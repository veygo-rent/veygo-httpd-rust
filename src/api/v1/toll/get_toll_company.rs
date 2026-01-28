use crate::{POOL, methods, model};
use diesel::prelude::*;
use warp::Filter;
use warp::http::{StatusCode, Method};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-company")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, auth: String, user_agent: String| {
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

                    let admin = methods::user::get_user_by_id(&access_token.user_id)
                        .await;

                    let Ok(admin) = admin else {
                        return methods::standard_replies::internal_server_error_response();
                    };

                    if !admin.is_operational_admin() {
                        return methods::standard_replies::admin_not_verified()
                    }

                    use crate::schema::transponder_companies::dsl as tc_q;
                    let mut pool = POOL.get().unwrap();
                    let results = tc_q::transponder_companies
                        .get_results::<model::TransponderCompany>(&mut pool);

                    match results {
                        Ok(temp) => {
                            methods::standard_replies::response_with_obj(temp, StatusCode::OK)
                        }
                        Err(_) => {
                            methods::standard_replies::internal_server_error_response()
                        }
                    }
                }
            };
        })
}
