use crate::{POOL, methods, model, helper_model};
use diesel::RunQueryDsl;
use diesel::prelude::*;
use http::Method;
use serde_derive::{Deserialize, Serialize};
use warp::{Filter};
use warp::http::StatusCode;
use crate::helper_model::VeygoError;
use crate::integration::stripe_veygo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentMethodsRequestBody {
    pm_id: String,
    cardholder_name: String,
    nickname: Option<String>,
}

pub fn main() -> impl Filter<Extract=(impl warp::Reply,), Error=warp::Rejection> + Clone {
    warp::path("create")
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and(warp::path::end())
        .and_then(async move |method: Method,
                              request_body: CreatePaymentMethodsRequestBody,
                              auth: String, user_agent: String| {
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

            let access_token = model::RequestToken {
                user_id,
                token: String::from(token_and_id[0]),
            };

            let if_token_valid = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
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
                            methods::standard_replies::internal_server_error_response()
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response();
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response();
                        }
                    }

                    let new_pm_result = stripe_veygo::retrieve_payment_method_from_stripe
                        (
                            &request_body.pm_id,
                            &request_body.cardholder_name,
                            &access_token.user_id,
                            &request_body.nickname
                        )
                        .await;

                    match new_pm_result {
                        Ok(new_pm) => {
                            use crate::schema::payment_methods::dsl as pm_q;
                            let mut pool = POOL.get().unwrap();
                            let card_in_db = diesel::select
                                (
                                    diesel::dsl::exists(pm_q::payment_methods.into_boxed()
                                        .filter(pm_q::is_enabled)
                                        .filter(pm_q::fingerprint.eq(&new_pm.fingerprint))
                                )
                            ).get_result::<bool>(&mut pool);

                            let Ok(card_in_db) = card_in_db else {
                                return methods::standard_replies::internal_server_error_response()
                            };
                            if card_in_db {
                                let err_msg = helper_model::ErrorResponse {
                                    title: "Payment Method Existed".to_string(),
                                    message: "Please try a different credit card. ".to_string(),
                                };
                                return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                            }

                            use crate::schema::renters::dsl as renter_q;
                            let stripe_id = renter_q::renters
                                .find(&access_token.user_id)
                                .select(renter_q::stripe_id)
                                .get_result::<String>(&mut pool);

                            let Ok(stripe_id) = stripe_id else {
                                return methods::standard_replies::internal_server_error_response()
                            };

                            let attach_result = stripe_veygo::attach_payment_method_to_stripe_customer(&stripe_id, &new_pm.token).await;

                            match attach_result {
                                Ok(_) => {
                                    use crate::schema::payment_methods::dsl as pm_q;
                                    let inserted_pm_card = diesel::insert_into(pm_q::payment_methods)
                                        .values(&new_pm)
                                        .get_result::<model::PaymentMethod>(&mut pool);
                                    
                                    return match inserted_pm_card {
                                        Ok(pm) => {
                                            let pub_pm: model::PublishPaymentMethod = pm.into();
                                            methods::standard_replies::response_with_obj(pub_pm, StatusCode::CREATED)
                                        }
                                        Err(_) => {
                                            methods::standard_replies::internal_server_error_response()
                                        }
                                    }
                                }
                                Err(err) => {
                                    match err {
                                        VeygoError::CardDeclined => {
                                            methods::standard_replies::card_declined()
                                        }
                                        _ => {
                                            methods::standard_replies::internal_server_error_response()
                                        }
                                    }
                                }
                            }


                        }
                        Err(err) => {
                            return match err {
                                VeygoError::CardNotSupported => {
                                    methods::standard_replies::card_invalid()
                                }
                                VeygoError::InputDataError => {
                                    methods::standard_replies::card_invalid()
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response()
                                }
                            }
                        }
                    }
                }
            };
        })
}
