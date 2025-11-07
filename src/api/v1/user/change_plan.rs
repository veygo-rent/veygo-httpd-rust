use crate::schema::renters::dsl::renters;
use crate::{POOL, methods, model, schema};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use warp::{Filter, Reply};

#[derive(Deserialize, Serialize)]
struct ChangePlanRequest {
    plan: model::PlanTier,
    is_plan_annual: bool,
    payment_method_id: Option<i32>,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("change-plan")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |request_body: ChangePlanRequest,
                        auth: String,
                        user_agent: String| {
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_wrapped_return(&auth);
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid = methods::tokens::verify_user_token(
                    &access_token.user_id,
                    &access_token.token,
                ).await;
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
                        } else {
                            // gen new token
                            methods::tokens::rm_token_by_binary(
                                hex::decode(access_token.token).unwrap(),
                            ).await;
                            let new_token = methods::tokens::gen_token_object(
                                &access_token.user_id,
                                &user_agent,
                            ).await;
                            use schema::access_tokens::dsl::*;
                            let mut pool = POOL.get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .to_publish_access_token();
                            let mut user = methods::user::get_user_by_id(&access_token.user_id)
                                .await
                                .unwrap();
                            use schema::apartments::dsl::*;
                            let apartment: model::Apartment = apartments
                                .into_boxed()
                                .filter(schema::apartments::columns::id.eq(&user.apartment_id))
                                .get_result::<model::Apartment>(&mut pool)
                                .unwrap();
                            if !&apartment.is_operating {
                                return methods::standard_replies::apartment_not_operational_wrapped(new_token_in_db_publish);
                            }
                            if request_body.plan == model::PlanTier::Free {
                                // request downgrade will be automatically executed when the old plan expires
                                user.subscription_payment_method_id = None;
                                let pub_user = diesel::update(renters.find(access_token.user_id.clone())).set(&user).get_result::<model::Renter>(&mut pool).unwrap().to_publish_renter();
                                methods::standard_replies::renter_wrapped(new_token_in_db_publish, &pub_user)
                            } else {
                                if let Some(_pm_id) = request_body.payment_method_id {
                                    if user.plan_tier == model::PlanTier::Free {
                                        // TODO if the old plan is free, setting up a brand new plan

                                        let plan_rate: Option<f64> = match request_body.plan {
                                            model::PlanTier::Free => Some(0.00),
                                            model::PlanTier::Gold => apartment.gold_tier_rate,
                                            model::PlanTier::Silver => apartment.silver_tier_rate,
                                            model::PlanTier::Platinum => apartment.platinum_tier_rate,
                                        };

                                        let plan_cost = plan_rate.unwrap_or_default();

                                        if plan_cost == 0.00 {
                                            // TODO 
                                            return methods::standard_replies::not_implemented_response();
                                        }

                                        methods::standard_replies::not_implemented_response()
                                    } else {
                                        // TODO Change exp date and tier level
                                        let _plan_exp_ddmmyyyy = user.plan_renewal_day.clone() + &user.plan_expire_month_year;
                                        let _old_plan = user.plan_tier.clone();
                                        let _if_annual = user.is_plan_annual.clone();
                                        methods::standard_replies::not_implemented_response()
                                    }
                                } else {
                                    methods::standard_replies::card_invalid_wrapped(new_token_in_db_publish)
                                }
                            }
                        }
                    }
                };
            },
        )
}
