use crate::{POOL, methods, model};
use diesel::prelude::*;
use warp::Filter;
use crate::helper_model::VeygoError;
use warp::http::{Method, StatusCode};

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get")
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
            let if_token_valid_result =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                    .await;
            match if_token_valid_result {
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
                                String::from("payment-method/get: Token verification unexpected error"),
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
                                    String::from("payment-method/get: Token extension failed (returned false)"),
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("payment-method/get: Token extension error"),
                            );
                        }
                    }

                    use crate::schema::payment_methods::dsl as pm_q;
                    let mut pool = POOL.get().unwrap();

                    let payment_methods_query_result = pm_q::payment_methods
                        .into_boxed()
                        .filter(pm_q::is_enabled)
                        .filter(pm_q::renter_id.eq(&access_token.user_id))
                        .order(pm_q::id.asc())
                        .load::<model::PaymentMethod>(&mut pool);

                    let payment_methods: Vec<model::PublishPaymentMethod> = match payment_methods_query_result {
                        Ok(pmt) => {
                            let payments: Vec<model::PublishPaymentMethod> = pmt
                                .iter()
                                .map(|x| x.clone().into())
                                .collect();
                            payments
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("payment-method/get: Database error loading payment methods"),
                            );
                        }
                    };

                    methods::standard_replies::response_with_obj(payment_methods, StatusCode::OK)
                }
            }
        })
}
