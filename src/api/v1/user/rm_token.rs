use crate::{methods, model};
use warp::{reply, Filter, Reply};
use warp::http::StatusCode;
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract=(impl Reply,), Error=warp::Rejection> + Clone {
    warp::path("remove-token")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::header::<String>("auth"))
        .and_then(async move |auth: String| {
            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_wrapped_return(&auth);
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => {
                    int
                }
                Err(_) => {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
            };
            let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
            let binary_token = hex::decode(access_token.token);
            if let Ok(token) = binary_token {
                methods::tokens::rm_token_by_binary(token).await;
            }
            Ok::<_, warp::Rejection>((with_status(reply(), StatusCode::OK).into_response(),))
        })
}