use crate::{POOL, methods, model, helper_model};
use diesel::RunQueryDsl;
use diesel::prelude::*;
use diesel::result::Error;
use warp::http::{StatusCode, Method};
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path!("get" / i32)
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |usr_id: i32, method:Method, auth: String, user_agent: String| {
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
                            methods::standard_replies::internal_server_error_response(
                                "user/get: Token verification unexpected error",
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
                                    "user/get: Token extension failed (returned false)",
                                )
                                .await;
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                "user/get: Token extension error",
                            )
                            .await;
                        }
                    }

                    let admin = methods::user::get_user_by_id(&access_token.user_id)
                        .await;

                    let Ok(admin) = admin else {
                        return methods::standard_replies::internal_server_error_response(
                            "user/get: Database error loading admin user",
                        )
                        .await;
                    };

                    if !admin.is_operational_manager() {
                        return methods::standard_replies::admin_not_verified()
                    }

                    use crate::schema::renters::dsl as r_q;
                    let mut pool = POOL.get().unwrap();

                    let user = if admin.is_operational_admin() {
                        r_q::renters
                            .find(&usr_id)
                            .get_result::<model::Renter>(&mut pool)
                    } else {
                        r_q::renters
                            .filter(r_q::apartment_id.eq(&admin.apartment_id))
                            .find(&usr_id)
                            .get_result::<model::Renter>(&mut pool)
                    };

                    let user = match user {
                        Ok(usr) => { usr }
                        Err(err) => {
                            return match err {
                                Error::NotFound => {
                                    let msg = helper_model::ErrorResponse{ title: "User Not Found".to_string(), message: "The user you are trying to access does not exist. ".to_string() };
                                    methods::standard_replies::response_with_obj(msg, StatusCode::NOT_FOUND)
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response(
                                        "user/get: Database error loading renter",
                                    )
                                    .await
                                }
                            }
                        }
                    };
                    
                    methods::standard_replies::response_with_obj(user, StatusCode::OK)
                }
            };
        })
}
