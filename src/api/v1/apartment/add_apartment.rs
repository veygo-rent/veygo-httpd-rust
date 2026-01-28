use crate::{POOL, methods, model, helper_model};
use diesel::prelude::*;
use diesel::result::Error;
use warp::{Filter, http::Method, http::StatusCode};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("add-apartment")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        apartment: model::NewApartment,
                        auth: String,
                        user_agent: String| {
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

                        use crate::schema::apartments::dsl::*;
                        let mut pool = POOL.get().unwrap();
                        let insert_result = diesel::insert_into(apartments)
                            .values(&apartment)
                            .get_result::<model::Apartment>(&mut pool);
                        return match insert_result {
                            Err(error) => {
                                match error {
                                    Error::DatabaseError(_, _) => {
                                        let err_msg = helper_model::ErrorResponse {
                                            title: "Unable To Create Apartment".to_string(),
                                            message: "Please check apartment details. ".to_string(),
                                        };
                                        methods::standard_replies::response_with_obj(err_msg, StatusCode::CONFLICT)
                                    }
                                    _ => {
                                        methods::standard_replies::internal_server_error_response()
                                    }
                                }
                            }
                            Ok(apt) => {
                                methods::standard_replies::response_with_obj(apt, StatusCode::CREATED)
                            }
                        };
                    }
                };
            },
        )
}
