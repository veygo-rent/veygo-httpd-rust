use crate::{methods, model, POOL};
use diesel::prelude::*;
use diesel::RunQueryDsl;
use serde_derive::{Deserialize, Serialize};
use tokio::task;
use warp::http::StatusCode;
use warp::{Filter, Rejection};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentMethodsRequestBody {
    access_token: model::RequestBodyToken,
    card_id: i32,
}

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("delete")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and(warp::path::end())
        .and_then(async move |request_body: CreatePaymentMethodsRequestBody, client_type: Option<String>| {
            let if_token_valid = methods::tokens::verify_user_token(request_body.access_token.user_id.clone(), request_body.access_token.token.clone()).await;
            return match if_token_valid {
                Err(_) => {
                    methods::tokens::token_not_hex_warp_return(&request_body.access_token.token)
                }
                Ok(token_bool) => {
                    if !token_bool {
                        methods::tokens::token_invalid_warp_return(&request_body.access_token.token)
                    } else {
                        // gen new token
                        methods::tokens::rm_token_by_binary(hex::decode(request_body.access_token.token.clone()).unwrap()).await;
                        let new_token = methods::tokens::gen_token_object(request_body.access_token.user_id.clone(), client_type.clone()).await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.clone().get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<model::AccessToken>(&mut pool).unwrap().to_publish_access_token();
                        // check if pm in question exists as an active pm
                        let pmt_id_clone = request_body.card_id.clone();
                        let if_pm_in_question_exists = task::spawn_blocking(move || {
                            use crate::schema::payment_methods::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            diesel::select(diesel::dsl::exists(payment_methods.filter(id.eq(pmt_id_clone)).filter(is_enabled.eq(true)))).get_result::<bool>(&mut pool)
                        }).await.unwrap().unwrap();
                        if !if_pm_in_question_exists {
                            let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Invalid Payment Method"});
                            return Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),));
                        }
                        // check if pm match user id
                        let pmt_id_clone = request_body.card_id.clone();
                        let mut pm = task::spawn_blocking(move || {
                            use crate::schema::payment_methods::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            payment_methods.filter(id.eq(pmt_id_clone)).get_result::<model::PaymentMethod>(&mut pool)
                        }).await.unwrap().unwrap();
                        if pm.renter_id != request_body.access_token.user_id {
                            let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Invalid Payment Method"});
                            return Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),));
                        }
                        pm.is_enabled = false;
                        use crate::schema::payment_methods::dsl::*;
                        let pmt_id_clone = request_body.card_id.clone();
                        diesel::update(payment_methods.find(pmt_id_clone)).set(&pm).execute(&mut pool).unwrap();
                        let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish});
                        return Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::OK),));
                    }
                }
            }
        })
}
