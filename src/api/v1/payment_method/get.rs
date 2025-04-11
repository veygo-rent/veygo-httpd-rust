use crate::{POOL, methods, model};
use diesel::prelude::*;
use tokio::task::spawn_blocking;
use warp::Filter;
use warp::http::StatusCode;
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get")
        .and(warp::post())
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            move |token: String, user_id: i32, client_type: Option<String>| {
                async move {
                    let access_token = model::RequestToken { user_id, token };
                    let if_token_valid_result = methods::tokens::verify_user_token(
                        access_token.user_id.clone(),
                        access_token.token.clone(),
                    )
                    .await;
                    match if_token_valid_result {
                        Err(_err) => {
                            methods::tokens::token_not_hex_warp_return(&access_token.token)
                        }
                        Ok(if_token_valid) => {
                            if !if_token_valid {
                                methods::tokens::token_invalid_wrapped_return(&access_token.token)
                            } else {
                                // Token is valid -> user_id trusted
                                // gen new token
                                methods::tokens::rm_token_by_binary(
                                    hex::decode(access_token.token).unwrap(),
                                )
                                .await;
                                let new_token = methods::tokens::gen_token_object(
                                    access_token.user_id.clone(),
                                    client_type.clone(),
                                )
                                .await;
                                use crate::schema::access_tokens::dsl::*;
                                let mut pool = POOL.clone().get().unwrap();
                                let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                    .values(&new_token)
                                    .get_result::<model::AccessToken>(&mut pool)
                                    .unwrap()
                                    .to_publish_access_token();

                                let id_clone = access_token.user_id.clone();
                                let mut pool = POOL.clone().get().unwrap();
                                let payment_method_query_result = spawn_blocking(move || {
                                    use crate::schema::payment_methods::dsl::*;
                                    payment_methods
                                        .filter(is_enabled.eq(true))
                                        .filter(renter_id.eq(id_clone))
                                        .load::<model::PaymentMethod>(&mut pool)
                                })
                                .await
                                .unwrap()
                                .unwrap();
                                let publish_payment_methods: Vec<model::PublishPaymentMethod> =
                                    payment_method_query_result
                                        .iter()
                                        .map(|x| x.to_public_payment_method().clone())
                                        .collect();
                                let msg = serde_json::json!({
                                    "payment_methods": publish_payment_methods,
                                });
                                Ok::<_, warp::Rejection>((
                                    methods::tokens::wrap_json_reply_with_token(
                                        new_token_in_db_publish,
                                        with_status(warp::reply::json(&msg), StatusCode::CREATED),
                                    ),
                                ))
                            }
                        }
                    }
                }
            },
        )
}
