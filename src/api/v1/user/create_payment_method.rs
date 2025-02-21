use chrono::Utc;
use diesel::{RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use tokio::task::{spawn_blocking};
use warp::Filter;
use warp::http::StatusCode;
use crate::db;
use crate::model::{AccessToken, PaymentMethod};
use diesel::prelude::*;
use crate::schema::access_tokens::dsl::*;
use crate::schema::payment_methods::dsl::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentMethodsRequestBody {
    pub token: String,
    pub user_id: i32,
    pub pm_id: String,
    pub cardholder_name: String,
    pub nickname: Option<String>,
}

pub fn create_payment_method() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("create-payment-method")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and(warp::path::end())
        .and_then(move |request_body: CreatePaymentMethodsRequestBody, client_type: Option<String>| {
            async move {
                let token_try_decode = hex::decode(request_body.token.clone());
                let token_hex: Vec<u8>;
                match token_try_decode {
                    Err(_) => {
                        let error_msg = serde_json::json!({"token": &request_body.token, "error": "Token not in hex format. "});
                        return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),));
                    }
                    Ok(token_u8) => {
                        token_hex = token_u8;
                    }
                };

                // get access_token object
                let user_id_clone = request_body.user_id.clone();
                let token_in_db_result = spawn_blocking(move || {
                    access_tokens.filter(user_id.eq(user_id_clone)).filter(crate::schema::access_tokens::dsl::token.eq(token_hex)).first::<AccessToken>(&mut db::get_connection_pool().get().unwrap())
                }).await;

                // check access token
                match token_in_db_result {
                    Err(_) => {
                        let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                    }
                    Ok(Err(_)) => {
                        let error_msg = serde_json::json!({"token": &request_body.token, "error": "Token invalid. "});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                    }
                    Ok(Ok(token_in_db)) => {
                        let token_exp = token_in_db.exp;
                        if token_exp >= Utc::now() {
                            // valid token
                            // generate new token
                            let new_token = crate::methods::tokens::gen_token_object(user_id_clone, client_type).await;
                            diesel::delete(access_tokens.filter(crate::schema::access_tokens::dsl::id.eq(token_in_db.id))).get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap()).unwrap();
                            let new_token_query_result = diesel::insert_into(access_tokens).values(new_token).get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap());
                            match new_token_query_result {
                                Err(_) => {
                                    let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                                }
                                Ok(new_token_in_db) => {
                                    let publish_token = new_token_in_db.to_publish_access_token();

                                    let converting_pm_id_to_new_payment_result = crate::integration::stripe::create_new_payment_method(request_body.pm_id.as_str(), request_body.cardholder_name, request_body.user_id, request_body.nickname).await;
                                    match converting_pm_id_to_new_payment_result {
                                        Err(_) => {
                                            let error_msg = serde_json::json!({"access_token": publish_token, "message": "Invalid Payment Method", "pm_id": request_body.pm_id});
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                        }
                                        Ok(new_payment) => {
                                            let insert_payment_method_result = diesel::insert_into(payment_methods).values(new_payment).get_result::<PaymentMethod>(&mut db::get_connection_pool().get().unwrap());
                                            match insert_payment_method_result {
                                                Err(error) => {
                                                    let error_msg = serde_json::json!({"access_token": publish_token, "pm_id": request_body.pm_id, "message": error.to_string()});
                                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                                }
                                                Ok(pm_method) => {
                                                    let pub_pm_method = pm_method.to_public_payment_method();
                                                    let msg = serde_json::json!({"access_token": publish_token, "payment_method": pub_pm_method});
                                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // expired token
                            let error_msg = serde_json::json!({"token": &request_body.token, "error": "Token invalid. "});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                    }
                }
            }
        })
}