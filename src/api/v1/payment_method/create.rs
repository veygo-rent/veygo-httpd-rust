use crate::model::{AccessToken, PaymentMethod};
use crate::{methods, model, POOL};
use diesel::prelude::*;
use diesel::RunQueryDsl;
use serde_derive::{Deserialize, Serialize};
use stripe::ErrorType::InvalidRequest;
use stripe::{ErrorCode, StripeError};
use tokio::task;
use warp::http::StatusCode;
use warp::Filter;

use crate::integration::stripe_veygo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentMethodsRequestBody {
    pm_id: String,
    cardholder_name: String,
    nickname: Option<String>,
}

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("create")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and(warp::path::end())
        .and_then(move |request_body: CreatePaymentMethodsRequestBody, token: String, user_id: i32, client_type: Option<String>| {
            async move {
                let access_token = model::RequestToken{
                    user_id,
                    token,
                };
                let if_token_valid = methods::tokens::verify_user_token(access_token.user_id.clone(), access_token.token.clone()).await;
                return match if_token_valid {
                    Err(_) => {
                        methods::tokens::token_not_hex_warp_return(&access_token.token)
                    }
                    Ok(token_bool) => {
                        if token_bool {
                            // gen new token
                            methods::tokens::rm_token_by_binary(hex::decode(access_token.token).unwrap()).await;
                            let new_token = methods::tokens::gen_token_object(access_token.user_id.clone(), client_type.clone()).await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut pool).unwrap().to_publish_access_token();

                            let new_pm_result = stripe_veygo::create_new_payment_method(request_body.pm_id.as_str(), request_body.cardholder_name.clone(), access_token.user_id.clone(), request_body.nickname).await;
                            match new_pm_result {
                                Ok(new_pm) => {
                                    let md5_clone = new_pm.md5.clone();
                                    let mut pool = POOL.clone().get().unwrap();
                                    let card_in_db = task::spawn_blocking(move || {
                                        use crate::schema::payment_methods::dsl::*;
                                        diesel::select(diesel::dsl::exists(payment_methods.filter(is_enabled.eq(true)).filter(md5.eq(md5_clone)))).get_result::<bool>(&mut pool)
                                    }).await.unwrap().unwrap();
                                    if card_in_db {
                                        let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "PaymentMethods existed"});
                                        return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),));
                                    }
                                    let new_pm_clone = new_pm.clone();
                                    // attach payment method to customer
                                    let user_id_clone = access_token.user_id.clone();
                                    let current_renter = methods::user::get_user_by_id(user_id_clone).await.unwrap();
                                    let stripe_customer_id = current_renter.stripe_id.clone().unwrap();
                                    let payment_method_id = new_pm.token.clone();
                                    let attach_result = stripe_veygo::attach_payment_method_to_stripe_customer(stripe_customer_id, payment_method_id).await;
                                    match attach_result {
                                        Ok(_) => {
                                            let mut pool = POOL.clone().get().unwrap();
                                            let inserted_pm_card = task::spawn_blocking(move || {
                                                use crate::schema::payment_methods::dsl::*;
                                                diesel::insert_into(payment_methods).values(&new_pm_clone).get_result::<PaymentMethod>(&mut pool).unwrap()
                                            }).await.unwrap().to_public_payment_method();
                                            let msg = serde_json::json!({"access_token": &new_token_in_db_publish, "payment_method": inserted_pm_card});
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),))
                                        }
                                        Err(error) => {
                                            if let StripeError::Stripe(request_error) = error {
                                                eprintln!("Stripe API error: {:?}", request_error);
                                                if request_error.code == Some(ErrorCode::CardDeclined) {
                                                    let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "PaymentMethods declined"});
                                                    return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),));
                                                } else if request_error.error_type == InvalidRequest {
                                                    let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "PaymentMethods token invalid"});
                                                    return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),));
                                                }
                                            }
                                            methods::standard_replies::internal_server_error_response_without_access_token()
                                        }
                                    }

                                }
                                Err(_) => {
                                    let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "PaymentMethods token invalid"});
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                }
                            }
                        } else {
                            methods::tokens::token_invalid_warp_return(&access_token.token)
                        }
                    }
                }
            }
        })
}
