use crate::schema::renters::dsl::renters;
use crate::{POOL, methods, model, schema, helper_model};
use diesel::prelude::*;
use diesel::result::Error;
use warp::http::{StatusCode, Method};
use serde::{Deserialize, Serialize};
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

#[derive(Deserialize, Serialize)]
struct ChangePlanRequest {
    plan: model::PlanTier,
    is_plan_annual: bool,
    payment_method_id: Option<i32>,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("change-plan")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        request_body: ChangePlanRequest,
                        auth: String,
                        user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_return();
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_return();
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid = methods::tokens::verify_user_token(
                    &access_token.user_id,
                    &access_token.token,
                ).await;
                return match if_token_valid {
                    Err(err) => {
                        match err {
                            VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(
                                    "user/change-plan: Token verification unexpected error",
                                )
                                .await
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response(
                                        "user/change-plan: Token extension failed (returned false)",
                                    )
                                    .await;
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "user/change-plan: Token extension error",
                                )
                                .await;
                            }
                        }

                        // Get current user
                        let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await;

                        let mut user_in_request = match user_in_request {
                            Ok(temp) => { temp }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "user/change-plan: Database error loading renter",
                                )
                                .await
                            }
                        };

                        let mut pool = POOL.get().unwrap();

                        use schema::apartments::dsl as apt_q;
                        let apartment = apt_q::apartments
                            .filter(apt_q::id.eq(&user_in_request.apartment_id))
                            .get_result::<model::Apartment>(&mut pool);

                        let apartment = match apartment {
                            Ok(apt) => { apt }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "user/change-plan: Database error loading apartment",
                                )
                                .await
                            }
                        };

                        if !apartment.is_operating {
                            user_in_request.subscription_payment_method_id = None;
                            user_in_request.plan_tier = model::PlanTier::Free;
                            let result = diesel::update
                                (
                                    renters
                                        .find(access_token.user_id)
                                )
                                .set(&user_in_request)
                                .execute(&mut pool);
                            match result {
                                Ok(count) => {
                                    if count != 1 {
                                        return methods::standard_replies::internal_server_error_response(
                                            "user/change-plan: SQL error updating renter to Free (unexpected row count)",
                                        )
                                        .await
                                    }
                                }
                                Err(_) => {
                                    return methods::standard_replies::internal_server_error_response(
                                        "user/change-plan: Database error updating renter to Free",
                                    )
                                    .await
                                }
                            }
                            return methods::standard_replies::apartment_not_operational();
                        }

                        if request_body.plan == model::PlanTier::Free {
                            user_in_request.subscription_payment_method_id = None;
                            user_in_request.plan_tier = model::PlanTier::Free;
                            let result = diesel::update
                                (
                                    renters
                                        .find(access_token.user_id)
                                )
                                .set(&user_in_request)
                                .get_result::<model::Renter>(&mut pool);
                            return match result {
                                Ok(renter) => {
                                    let pub_renter: model::PublishRenter = renter.into();
                                    methods::standard_replies::response_with_obj(pub_renter, StatusCode::OK)
                                }
                                Err(_) => {
                                    methods::standard_replies::internal_server_error_response(
                                        "user/change-plan: SQL error saving renter Free plan",
                                    )
                                    .await
                                }
                            }
                        } else {
                            if let Some(pm_id) = request_body.payment_method_id {
                                use crate::schema::payment_methods::dsl as pm_q;
                                let payment_method = pm_q::payment_methods
                                    .find(&pm_id)
                                    .get_result::<model::PaymentMethod>(&mut pool);
                                match payment_method {
                                    Ok(_pm) => {
                                        let plan_rate = match request_body.plan {
                                            model::PlanTier::Free => None,
                                            model::PlanTier::Gold => apartment.gold_tier_rate,
                                            model::PlanTier::Silver => apartment.silver_tier_rate,
                                            model::PlanTier::Platinum => apartment.platinum_tier_rate,
                                        };
                                        let Some(_plan_rate) = plan_rate else {
                                            let msg = helper_model::ErrorResponse{
                                                title: "Plan Not Available".to_string(), message: "The plan is currently not available, please try a different plan. ".to_string()
                                            };
                                            return methods::standard_replies::response_with_obj(msg, StatusCode::FORBIDDEN)
                                        };


                                        // TODO: Calculate plan credits or start new plans


                                        methods::standard_replies::internal_server_error_response(
                                            "user/change-plan: Not implemented (plan change calculation)",
                                        )
                                        .await
                                    }
                                    Err(err) => {
                                        match err {
                                            Error::NotFound => {
                                                methods::standard_replies::card_invalid()
                                            }
                                            _ => {
                                                methods::standard_replies::internal_server_error_response(
                                                    "user/change-plan: Database error loading payment method",
                                                )
                                                .await
                                            }
                                        }
                                    }
                                }
                            } else {
                                methods::standard_replies::card_invalid()
                            }
                        }
                    }
                };
            },
        )
}
