use crate::{POOL, methods, model};
use diesel::RunQueryDsl;
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use stripe::ErrorType::InvalidRequest;
use stripe::{ErrorCode, StripeError};
use warp::Filter;
use warp::http::StatusCode;

use crate::integration::stripe_veygo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentMethodsRequestBody {
    pm_id: String,
    cardholder_name: String,
    nickname: Option<String>,
}

pub fn main() -> impl Filter<Extract=(impl warp::Reply,), Error=warp::Rejection> + Clone {
    warp::path("create")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and(warp::path::end())
        .and_then(async move |request_body: CreatePaymentMethodsRequestBody, auth: String, user_agent: String| {
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
            let if_token_valid = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
            return match if_token_valid {
                Err(_) => {
                    methods::tokens::token_not_hex_warp_return(&access_token.token)
                }
                Ok(token_bool) => {
                    if token_bool {
                        // gen new token
                        let _ = methods::tokens::rm_token_by_binary(hex::decode(access_token.token).unwrap()).await;
                        let new_token = methods::tokens::gen_token_object(&access_token.user_id, &user_agent).await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.get().unwrap();
                        let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                            .values(&new_token)
                            .get_result::<model::AccessToken>(&mut pool)
                            .unwrap()
                            .into();

                        let new_pm_result = stripe_veygo::create_new_payment_method(&request_body.pm_id, &request_body.cardholder_name, &access_token.user_id, &request_body.nickname).await;
                        match new_pm_result {
                            Ok(new_pm) => {
                                use crate::schema::payment_methods::dsl::*;
                                let card_in_db = diesel::select(diesel::dsl::exists(payment_methods.into_boxed().filter(is_enabled.eq(true)).filter(fingerprint.eq(&new_pm.fingerprint)))).get_result::<bool>(&mut pool)
                                    .unwrap();
                                if card_in_db {
                                    let error_msg = serde_json::json!({"error": "PaymentMethods existed"});
                                    return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                                }
                                // attach payment method to customer
                                let current_renter = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();
                                let attach_result = stripe_veygo::attach_payment_method_to_stripe_customer(&current_renter.stripe_id.unwrap(), &new_pm.token).await;
                                match attach_result {
                                    Ok(_) => {
                                        use crate::schema::payment_methods::dsl::*;
                                        let inserted_pm_card: model::PublishPaymentMethod = diesel::insert_into(payment_methods)
                                            .values(&new_pm)
                                            .get_result::<model::PaymentMethod>(&mut pool)
                                            .unwrap()
                                            .into();

                                        let msg = serde_json::json!({
                                            "new_payment_method": inserted_pm_card,
                                        });
                                        Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&msg), StatusCode::CREATED)),))
                                    }
                                    Err(error) => {
                                        if let StripeError::Stripe(request_error) = error {
                                            eprintln!("Stripe API error: {:?}", request_error);
                                            if request_error.code == Some(ErrorCode::CardDeclined) {
                                                return methods::standard_replies::card_declined_wrapped(new_token_in_db_publish);
                                            } else if request_error.error_type == InvalidRequest {
                                                let error_msg = serde_json::json!({"error": "Payment Methods token invalid"});
                                                return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                                            }
                                        }
                                        methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                                    }
                                }
                            }
                            Err(_) => {
                                let error_msg = serde_json::json!({"error": "Payment Methods token invalid"});
                                Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                            }
                        }
                    } else {
                        methods::tokens::token_invalid_wrapped_return(&access_token.token)
                    }
                }
            };
        })
}
