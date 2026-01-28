use crate::{POOL, methods, model};
use diesel::{RunQueryDsl};
use warp::http::StatusCode;
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-taxes")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |auth: String, user_agent: String| {
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
                                "apartment/get-taxes: Token verification unexpected error",
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
                                    "apartment/get-taxes: Token extension failed (returned false)",
                                )
                                .await;
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                "apartment/get-taxes: Token extension error",
                            )
                            .await;
                        }
                    }

                    let admin = methods::user::get_user_by_id(&access_token.user_id)
                        .await;

                    let Ok(admin) = admin else {
                        return methods::standard_replies::internal_server_error_response(
                            "apartment/get-taxes: Database error loading admin user",
                        )
                        .await;
                    };

                    if !admin.is_operational_manager() {
                        return methods::standard_replies::admin_not_verified()
                    }

                    use crate::schema::taxes::dsl as tax_q;
                    let mut pool = POOL.get().unwrap();
                    let publish_taxes = tax_q::taxes
                        .get_results::<model::Tax>(&mut pool);

                    let publish_taxes = match publish_taxes {
                        Ok(taxes) => {
                            taxes
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                "apartment/get-taxes: Database error loading taxes",
                            )
                            .await;
                        }
                    };

                    methods::standard_replies::response_with_obj(publish_taxes, StatusCode::OK)
                }
            };
        })
}
