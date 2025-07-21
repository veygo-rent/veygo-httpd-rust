use crate::{POOL, methods, model};
use diesel::prelude::*;
use warp::Filter;
use warp::http::StatusCode;
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get")
        .and(warp::get())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |auth: String, user_agent: String| {
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
            let if_token_valid_result =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                    .await;
            match if_token_valid_result {
                Err(_err) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                Ok(if_token_valid) => {
                    if !if_token_valid {
                        methods::tokens::token_invalid_wrapped_return(&access_token.token)
                    } else {
                        // Token is valid -> user_id trusted
                        // gen new token
                        let _ = methods::tokens::rm_token_by_binary(
                            hex::decode(access_token.token).unwrap(),
                        )
                        .await;
                        let new_token =
                            methods::tokens::gen_token_object(&access_token.user_id, &user_agent)
                                .await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens)
                            .values(&new_token)
                            .get_result::<model::AccessToken>(&mut pool)
                            .unwrap()
                            .to_publish_access_token();

                        let id_clone = access_token.user_id.clone();
                        let mut pool = POOL.get().unwrap();
                        use crate::schema::payment_methods::dsl::*;
                        let payment_method_query_result = payment_methods
                            .into_boxed()
                            .filter(is_enabled.eq(true))
                            .filter(renter_id.eq(id_clone))
                            .load::<model::PaymentMethod>(&mut pool)
                            .unwrap();
                        let publish_payment_methods: Vec<model::PublishPaymentMethod> =
                            payment_method_query_result
                                .iter()
                                .map(|x| x.to_public_payment_method().clone())
                                .collect();
                        let msg = serde_json::json!({
                            "payment_methods": publish_payment_methods,
                        });
                        Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(
                            new_token_in_db_publish,
                            with_status(warp::reply::json(&msg), StatusCode::OK),
                        ),))
                    }
                }
            }
        })
}
