use crate::{POOL, methods, model};
use diesel::RunQueryDsl;
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::{Filter, Rejection};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentMethodsRequestBody {
    card_id: i32,
}

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
    warp::path("delete")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and(warp::path::end())
        .and_then(
            async move |request_body: CreatePaymentMethodsRequestBody,
                        auth: String,
                        user_agent: String| {
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => int,
                    Err(_) => {
                        return methods::tokens::token_invalid_wrapped_return(&auth);
                    }
                };

                let access_token = model::RequestToken {
                    user_id,
                    token: token_and_id[0].parse().unwrap(),
                };
                let if_token_valid =
                    methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                        .await;
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
                        } else {
                            // gen new token
                            methods::tokens::rm_token_by_binary(
                                hex::decode(access_token.token.clone()).unwrap(),
                            )
                            .await;
                            let new_token = methods::tokens::gen_token_object(
                                &access_token.user_id,
                                &user_agent,
                            )
                            .await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.get().unwrap();
                            let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .into();
                            // check if the pm in question exists as an active pm
                            let if_pm_in_question_exists = {
                                use crate::schema::payment_methods::dsl::*;
                                let mut pool = POOL.get().unwrap();
                                diesel::select(diesel::dsl::exists(
                                    payment_methods
                                        .filter(id.eq(&request_body.card_id))
                                        .filter(is_enabled.eq(true)),
                                ))
                                .get_result::<bool>(&mut pool)
                            }
                            .unwrap();
                            if !if_pm_in_question_exists {
                                let error_msg =
                                    serde_json::json!({"error": "Invalid Payment Method"});
                                return Ok::<_, Rejection>((
                                    methods::tokens::wrap_json_reply_with_token(
                                        new_token_in_db_publish,
                                        warp::reply::with_status(
                                            warp::reply::json(&error_msg),
                                            StatusCode::NOT_ACCEPTABLE,
                                        ),
                                    ),
                                ));
                            }
                            // check if pm match user id
                            let mut pm = payment_methods
                                .filter(
                                    crate::schema::payment_methods::id.eq(&request_body.card_id),
                                )
                                .get_result::<model::PaymentMethod>(&mut pool)
                                .unwrap();
                            if pm.renter_id != access_token.user_id {
                                let error_msg =
                                    serde_json::json!({"error": "Invalid Payment Method"});
                                return Ok::<_, Rejection>((
                                    methods::tokens::wrap_json_reply_with_token(
                                        new_token_in_db_publish,
                                        warp::reply::with_status(
                                            warp::reply::json(&error_msg),
                                            StatusCode::NOT_ACCEPTABLE,
                                        ),
                                    ),
                                ));
                            }
                            pm.is_enabled = false;
                            use crate::schema::payment_methods::dsl::*;
                            let pmt_id_clone = request_body.card_id.clone();
                            diesel::update(payment_methods.find(pmt_id_clone))
                                .set(&pm)
                                .execute(&mut pool)
                                .unwrap();
                            let payment_method_query_result = payment_methods
                                .into_boxed()
                                .filter(is_enabled.eq(true))
                                .filter(renter_id.eq(&access_token.user_id))
                                .load::<model::PaymentMethod>(&mut pool)
                                .unwrap();
                            let publish_payment_methods: Vec<model::PublishPaymentMethod> =
                                payment_method_query_result
                                    .iter()
                                    .map(|x| model::PublishPaymentMethod::from(x.clone()))
                                    .collect();
                            let msg = serde_json::json!({
                                "payment_methods": publish_payment_methods,
                            });
                            return Ok::<_, Rejection>((
                                methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    warp::reply::with_status(
                                        warp::reply::json(&msg),
                                        StatusCode::OK,
                                    ),
                                ),
                            ));
                        }
                    }
                };
            },
        )
}
