use chrono::{Utc};
use diesel::{prelude::*, ExpressionMethods, QueryResult, RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use warp::Filter;
use tokio::task::{spawn_blocking, JoinError};
use warp::http::StatusCode;
use crate::db;
use crate::model::{AccessToken, PaymentMethod};
use crate::schema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPaymentMethodsRequestBody {
    pub token: String,
    pub user_id: i32,
}

pub fn get_payment_methods() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get-payment-methods")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |request_body: GetPaymentMethodsRequestBody, client_type: Option<String>| {
            async move {
                // convert the access token String
                use schema::access_tokens::dsl::*;
                let _token_try_decode = hex::decode(request_body.token.clone());
                let mut _token: Vec<u8>;
                match _token_try_decode {
                    Err(_) => {
                        let error_msg = serde_json::json!({"token": &request_body.token, "error": "Token not in hex format. "});
                        return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),));
                    }
                    Ok(token_u8) => {
                        _token = token_u8;
                    }
                }

                // get access_token object
                let user_id_clone = request_body.user_id.clone();
                let token_in_db_result: Result<QueryResult<AccessToken>, JoinError> = spawn_blocking(move || {
                    access_tokens.filter(token.eq(_token)).first::<AccessToken>(&mut db::get_connection_pool().get().unwrap())
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
                            let new_token = crate::gen_token::gen_token_object(user_id_clone, client_type).await;
                            diesel::delete(access_tokens.filter(id.eq(token_in_db.id))).get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap()).unwrap();
                            let new_token_query_result = diesel::insert_into(access_tokens).values(new_token).get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap());
                            match new_token_query_result {
                                Err(_) => {
                                    let error_msg = serde_json::json!({"token": &request_body.token, "error": "Token invalid. "});
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                }
                                Ok(new_token_in_db) => {
                                    let publish_token = new_token_in_db.to_publish_access_token();
                                    let payment_method_query_result_async = spawn_blocking(move || {
                                        use crate::schema::payment_methods::dsl::*;
                                        payment_methods.filter(renter_id.eq(user_id_clone)).load::<PaymentMethod>(&mut db::get_connection_pool().get().unwrap())
                                    }).await;
                                    match payment_method_query_result_async {
                                        Err(_) => {
                                            let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                                        }
                                        Ok(Ok(payment_results)) => {
                                            let msg = serde_json::json!({
                                                "access_token": publish_token,
                                                "payment_methods": payment_results,
                                            });
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&msg), StatusCode::ACCEPTED),))
                                        }
                                        Ok(Err(_)) => {
                                            let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
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