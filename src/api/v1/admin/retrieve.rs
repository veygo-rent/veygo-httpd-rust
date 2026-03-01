use http::StatusCode;
use crate::{methods, model};
use warp::{Filter, Reply, http::Method};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("retrieve")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, auth: String, user_agent: String| {
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
                    token: String::from(token_and_id[0]),
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
                                    String::from("admin/retrieve: Token verification unexpected error"),
                                )
                            }
                        }
                    }
                    Ok(token) => {
                        let user = methods::user::get_user_by_id(&access_token.user_id)
                            .await;
                        let Ok(user) = user else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("admin/retrieve: Database error loading renter by id"),
                            );
                        };

                        if !user.is_manager() {
                            return methods::standard_replies::user_not_admin();
                        }
                        let result = methods::tokens::extend_token(token.1, &user_agent);
                        match result {
                            Ok(is_renewed) => {
                                if is_renewed {
                                    methods::standard_replies::response_with_obj::<model::PublishRenter>(user.into(), StatusCode::OK)
                                } else {
                                    methods::standard_replies::internal_server_error_response(
                                        String::from("admin/retrieve: Token extension failed (returned false)"),
                                    )
                                }
                            }
                            Err(_) => {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("admin/retrieve: Token extension error"),
                                )
                            }
                        }
                    }
                };
            },
        )
}
