use warp::{reply, Filter, Reply, http::{Method, StatusCode}};
use warp::reply::with_status;
use crate::{methods, model, schema, connection_pool};
use diesel::prelude::*;

pub fn main() -> impl Filter<Extract=(impl Reply,), Error=warp::Rejection> + Clone {
    warp::path::end()
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and_then(async move |method: Method, auth: String| {
            if method != Method::DELETE {
                return methods::standard_replies::method_not_allowed_response_405();
            }
            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_return();
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => {
                    int
                }
                Err(_) => {
                    return methods::tokens::token_invalid_return();
                }
            };
            let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
            let if_token_valid =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                    .await;
            if let Ok((_valid_token, _token_id)) = if_token_valid {
                use schema::access_tokens::dsl as at_q;
                let mut pool = connection_pool().await.get().unwrap();
                let _ = diesel::delete(at_q::access_tokens.filter(at_q::user_id.eq(user_id))).execute(&mut pool);
                let msg = serde_json::json!({});
                Ok((with_status(reply::json(&msg), StatusCode::OK).into_response(),))
            } else {
                methods::tokens::token_invalid_return()
            }
        })
}