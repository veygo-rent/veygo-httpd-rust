use diesel::{RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use warp::Filter;
use warp::http::StatusCode;
use crate::db;
use crate::model::{AccessToken, PaymentMethod, Renter};
use diesel::prelude::*;
use tokio::task;
use crate::schema::access_tokens::dsl::*;
use crate::schema::payment_methods::dsl::*;
use crate::methods::tokens;

use crate::integration::stripe;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentMethodsRequestBody {
    pub token: String,
    pub md5: String,
    pub user_id: i32,
    pub pm_id: String,
    pub cardholder_name: String,
    pub nickname: Option<String>,
}

pub fn create_payment_method() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("create")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and(warp::path::end())
        .and_then(move |request_body: CreatePaymentMethodsRequestBody, client_type: Option<String>| {
            async move {
                let if_token_valid = tokens::verify_user_token(request_body.user_id.clone(), request_body.token.clone()).await;
                return match if_token_valid {
                    Err(_) => {
                        let error_msg = serde_json::json!({"token": &request_body.token, "error": "Token not in hex format"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                    }
                    Ok(token_bool) => {
                        if token_bool {
                            let md5_clone = request_body.md5.clone();
                            let card_in_db = task::spawn_blocking(move || {
                                diesel::select(diesel::dsl::exists(payment_methods.filter(md5.eq(md5_clone)))).get_result::<bool>(&mut db::get_connection_pool().get().unwrap())
                            }).await.unwrap().unwrap();
                            if !card_in_db {
                                let new_pm = stripe::create_new_payment_method(request_body.pm_id.as_str(), request_body.md5.clone(), request_body.cardholder_name.clone(), request_body.user_id.clone(), request_body.nickname).await;
                                match new_pm {
                                    Err(_) => {
                                        let error_msg = serde_json::json!({"token": &request_body.pm_id, "error": "PaymentMethods token invalid"});
                                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                    }
                                    Ok(new_pm) => {
                                        let new_pm_clone = new_pm.clone();
                                        let inserted_pm_card = task::spawn_blocking(move || {
                                            diesel::insert_into(payment_methods).values(&new_pm_clone).get_result::<PaymentMethod>(&mut db::get_connection_pool().get().unwrap()).unwrap()
                                        }).await.unwrap().to_public_payment_method();
                                        // attach payment method to customer
                                        let user_id_clone = request_body.user_id.clone();
                                        let current_renter = task::spawn_blocking(move || {
                                            use crate::schema::renters::dsl::*;
                                            renters.filter(id.eq(user_id_clone)).get_result::<Renter>(&mut db::get_connection_pool().get().unwrap())
                                        }).await.unwrap().unwrap();
                                        let stripe_customer_id = current_renter.stripe_id.clone().unwrap();
                                        let payment_method_id = new_pm.token.clone();
                                        let _attach_result = stripe::attach_payment_method_to_stripe_customer(stripe_customer_id, payment_method_id).await;
                                        let new_token = tokens::gen_token_object(request_body.user_id.clone(), client_type).await;
                                        let inserted_token = task::spawn_blocking(move || {
                                            diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap()).unwrap()
                                        }).await.unwrap().to_publish_access_token();
                                        tokens::rm_token_by_binary(hex::decode(request_body.token).unwrap()).await;
                                        let msg = serde_json::json!({"access_token": inserted_token, "payment_method": inserted_pm_card});
                                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),))
                                    }
                                }
                            } else {
                                let error_msg = serde_json::json!({"token": &request_body.token, "error": "PaymentMethods existed"});
                                Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                            }
                        } else {
                            let error_msg = serde_json::json!({"token": &request_body.token, "error": "User or token is invalid"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                    }
                }
            }
        })
}