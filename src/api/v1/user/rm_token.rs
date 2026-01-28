use crate::{methods, model};
use warp::{reply, Filter, Reply};
use warp::http::{StatusCode, Method};
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract=(impl Reply,), Error=warp::Rejection> + Clone {
    warp::path("remove-token")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and_then(async move |method: Method, auth: String| {
            if method != Method::DELETE {
                return methods::standard_replies::method_not_allowed_response();
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
            if let Ok((valid_token, _)) = if_token_valid {
                methods::tokens::rm_token(valid_token, access_token.user_id);
            }
            let msg = serde_json::json!({});
            Ok((with_status(reply::json(&msg), StatusCode::OK).into_response(),))
        })
}