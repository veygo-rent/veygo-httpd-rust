use std::collections::HashMap;
use crate::{connection_pool, methods, model, schema, helper_model};
use diesel::prelude::*;
use diesel::result::Error;
use warp::{Filter, Reply, http::{StatusCode, Method}};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("taxes")
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |query: HashMap<String, String>, method: Method, auth: String, user_agent: String| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response_405();
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
                            methods::standard_replies::internal_server_error_response_500(
                                String::from("apartment/get-taxes: Token verification unexpected error")
                            )
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent).await;

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("apartment/get-taxes: Token extension failed (returned false)")
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("apartment/get-taxes: Token extension error")
                            );
                        }
                    }

                    let user = methods::user::get_user_by_id(&access_token.user_id)
                        .await;

                    let Ok(user) = user else {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("apartment/get-taxes: Database error loading admin user")
                        );
                    };

                    let request_apt_id_str = query.get("apartment");

                    let mut pool = connection_pool().await.get().unwrap();
                    return if let Some(request_apt_id_str) = request_apt_id_str {
                        let Ok(request_apt_id) = request_apt_id_str.parse::<i32>() else  {
                            return methods::standard_replies::bad_request_400("Invalid apartment ID");
                        };

                        use schema::apartments::dsl as apt_q;
                        let apt =  apt_q::apartments
                            .find(request_apt_id)
                            .get_result::<model::Apartment>(&mut pool);

                        let apt = match apt {
                            Ok(apt) => { apt }
                            Err(e) => {
                                return match e {
                                    Error::NotFound => {
                                        let error_msg = helper_model::ErrorResponse {
                                            title: String::from("Permission Error"),
                                            message: String::from("Taxes not found or inaccessible"),
                                        };
                                        methods::standard_replies::response_with_obj(error_msg, StatusCode::FORBIDDEN)
                                    }
                                    _ => {
                                        methods::standard_replies::internal_server_error_response_500(
                                            String::from("apartment/get-taxes: Database error loading apartment")
                                        )
                                    }
                                }
                            }
                        };

                        match user.is_authorized_for(&apt).await {
                            Ok( is_authorized ) => {
                                if !is_authorized {
                                    let error_msg = helper_model::ErrorResponse {
                                        title: String::from("Permission Error"),
                                        message: String::from("Taxes not found or inaccessible"),
                                    };
                                    return methods::standard_replies::response_with_obj(error_msg, StatusCode::FORBIDDEN)
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("apartment/get-taxes: Database error loading apartment")
                                )
                            }
                        }

                        // is authorized

                        use schema::apartments_taxes::dsl as at_q;
                        use schema::taxes::dsl as t_q;

                        let all_taxes_belong_to_apt = at_q::apartments_taxes
                            .inner_join(t_q::taxes)
                            .filter(at_q::apartment_id.eq(&apt.id))
                            .order_by(t_q::id.asc())
                            .select(t_q::taxes::all_columns())
                            .get_results::<model::Tax>(&mut pool);

                        let all_taxes_belong_to_apt = match all_taxes_belong_to_apt {
                            Ok(res) => { res }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("apartment/get-taxes: Database error loading taxes")
                                )
                            }
                        };

                        methods::standard_replies::response_with_obj(all_taxes_belong_to_apt, StatusCode::OK)
                    } else {
                        if !user.is_operational_admin() {
                            let error_msg = helper_model::ErrorResponse {
                                title: String::from("Permission Error"),
                                message: String::from("Taxes not found or inaccessible"),
                            };
                            return methods::standard_replies::response_with_obj(error_msg, StatusCode::FORBIDDEN)
                        }

                        let request_page_id_str = query.get("page");
                        if let Some(request_page_id_str) = request_page_id_str &&
                            let Ok(request_page_id) = request_page_id_str.parse::<i64>() &&
                            request_page_id > 0 {
                            let per_page = 10;
                            let offset_num = (request_page_id - 1) * per_page;

                            use schema::taxes::dsl as t_q;
                            let taxes = t_q::taxes
                                .order_by(t_q::id.asc())
                                .offset(offset_num)
                                .limit(per_page)
                                .get_results::<model::Tax>(&mut pool);

                            let taxes = match taxes {
                                Ok(res) => { res }
                                Err(_) => {
                                    return methods::standard_replies::internal_server_error_response_500(
                                        String::from("apartment/get-taxes: Database error loading taxes")
                                    )
                                }
                            };

                            methods::standard_replies::response_with_obj(taxes, StatusCode::OK)
                        } else {
                            methods::standard_replies::bad_request_400("Invalid request parameter")
                        }
                    }
                }
            };
        })
}
