use crate::{POOL, methods, model};
use diesel::RunQueryDsl;
use diesel::prelude::*;
use http::Method;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("delete" / i32)
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and(warp::path::end())
        .and_then(
            async move |payment_id: i32,
                        method: Method,
                        auth: String,
                        user_agent: String| {
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response();
                }
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_return();
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => int,
                    Err(_) => {
                        return methods::tokens::token_invalid_return();
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
                    Err(err) => {
                        match err {
                            VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response()
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response();
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response();
                            }
                        }

                        use crate::schema::payment_methods::dsl as pm_q;
                        let mut pool = POOL.get().unwrap();
                        let result = diesel::update
                            (
                                pm_q::payment_methods
                                    .find(&payment_id)
                            )
                            .set(pm_q::is_enabled.eq(false))
                            .execute(&mut pool);
                        match result {
                            Ok(count) => {
                                if count != 1{
                                    methods::standard_replies::bad_request("Payment method not found.")
                                } else {
                                    let msg = serde_json::json!({});
                                    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK).into_response(),))
                                }
                            }
                            Err(_) => {
                                methods::standard_replies::internal_server_error_response()
                            }
                        }
                    }
                };
            },
        )
}
