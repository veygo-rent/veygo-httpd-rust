use diesel::{prelude::*};
use serde_derive::{Deserialize, Serialize};
use warp::Filter;
use tokio::task::{spawn_blocking};
use warp::http::StatusCode;
use crate::{model, POOL, methods};
use crate::model::{AccessToken, PaymentMethod, PublishPaymentMethod};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPaymentMethodsRequestBody {
    access_token: model::RequestBodyToken,
}

pub fn get_payment_methods() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |request_body: GetPaymentMethodsRequestBody, client_type: Option<String>| {
            async move {
                let if_token_valid_result = methods::tokens::verify_user_token(request_body.access_token.user_id.clone(), request_body.access_token.token.clone()).await;
                match if_token_valid_result {
                    Err(_err) => {
                        methods::tokens::token_not_hex_warp_return(&request_body.access_token.token)
                    }
                    Ok(if_token_valid) => {
                        if !if_token_valid {
                            methods::tokens::token_invalid_warp_return(&request_body.access_token.token)
                        } else {
                            // Token is valid -> user_id trusted
                            // gen new token
                            methods::tokens::rm_token_by_binary(hex::decode(request_body.access_token.token).unwrap()).await;
                            let new_token = methods::tokens::gen_token_object(request_body.access_token.user_id.clone(), client_type.clone()).await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut pool).unwrap().to_publish_access_token();

                            let id_clone = request_body.access_token.user_id.clone();
                            let mut pool = POOL.clone().get().unwrap();
                            let payment_method_query_result = spawn_blocking(move || {
                                use crate::schema::payment_methods::dsl::*;
                                payment_methods.filter(renter_id.eq(id_clone)).load::<PaymentMethod>(&mut pool)
                            }).await.unwrap().unwrap();
                            let publish_payment_methods: Vec<PublishPaymentMethod> = payment_method_query_result.iter().map(|x| x.to_public_payment_method().clone()).collect();
                            let msg = serde_json::json!({
                                                "access_token": new_token_in_db_publish,
                                                "payment_methods": publish_payment_methods,
                                            });
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),))
                        }
                    }
                }
            }
        })
}