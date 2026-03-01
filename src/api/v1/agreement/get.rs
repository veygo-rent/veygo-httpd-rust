use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use warp::{Filter, Reply};
use warp::http::{Method, StatusCode};
use crate::{helper_model, methods, model, POOL};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("get")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        auth: String,
                        user_agent: String| {
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response();
                }
                let mut pool = POOL.get().unwrap();
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_return();
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_return();
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                match if_token_valid_result {
                    Err(e) => {
                        match e {
                            helper_model::VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            helper_model::VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/get: Token verification unexpected error"),
                                )
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
                                        String::from("agreement/get: Token extension failed (returned false)"),
                                    );
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/get: Token extension error"),
                                );
                            }
                        }

                        use crate::schema::agreements::dsl as agreement_query;
                        let agreements = agreement_query::agreements
                            .filter(agreement_query::renter_id.eq(&access_token.user_id))
                            .get_results::<model::Agreement>(&mut pool);

                        match agreements {
                            Ok(ags) => {
                                methods::standard_replies::response_with_obj(ags, StatusCode::OK)
                            }
                            Err(_) => {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/get: Database error loading agreements"),
                                )
                            }
                        }
                    }
                }
            }
        )
}