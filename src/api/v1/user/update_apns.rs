use crate::{POOL, methods, model};
use diesel::prelude::*;
use warp::http::{Method};
use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UpdateApnsBody {
    apns: String,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("update-apns")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, body: UpdateApnsBody, auth: String, user_agent: String| {
                if method != Method::POST {
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
                                methods::standard_replies::internal_server_error_response(
                                    "user/update-apns: Token verification unexpected error",
                                )
                                .await
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response(
                                        "user/update-apns: Token extension failed (returned false)",
                                    )
                                    .await;
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "user/update-apns: Token extension error",
                                )
                                .await;
                            }
                        }

                        use crate::schema::renters::dsl as r_q;
                        let mut pool = POOL.get().unwrap();

                        let update_result = diesel::update
                            (
                                r_q::renters
                                    .find(&access_token.user_id)
                            )
                            .set(r_q::apple_apns.eq(Some(&body.apns)))
                            .get_result::<model::Renter>(&mut pool);

                        let Ok(renter) = update_result else {
                            return methods::standard_replies::internal_server_error_response(
                                "user/update-apns: SQL error updating apple_apns",
                            )
                            .await;
                        };

                        let renter: model::PublishRenter = renter.into();
                        return methods::standard_replies::response_with_obj(renter, http::StatusCode::OK);
                    }
                };
            },
        )
}
