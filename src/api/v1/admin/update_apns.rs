use crate::{POOL, methods, model};
use diesel::prelude::*;
use http::{Method, StatusCode};
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
                    },
                    Ok(token) => {
                        let result = methods::tokens::extend_token(token.1, &user_agent);
                        
                        match result {
                            Ok(is_renewed) => {
                                if is_renewed {
                                    let usr_in_question =
                                        methods::user::get_user_by_id(&access_token.user_id)
                                            .await
                                            .unwrap();
                                    if !usr_in_question.is_manager() {
                                        return methods::standard_replies::user_not_admin();
                                    }

                                    let mut pool = POOL.get().unwrap();
                                    use crate::schema::renters::dsl as r_q;
                                    let renter_updated = 
                                        diesel::update(r_q::renters.find(&access_token.user_id))
                                            .set(r_q::admin_apple_apns.eq(Some(&body.apns)))
                                            .get_result::<model::Renter>(&mut pool);
                                    
                                    match renter_updated {
                                        Ok(renter) => {
                                            let pub_renter: model::PublishRenter = renter.into();
                                            methods::standard_replies::response_with_obj(pub_renter, StatusCode::OK)
                                        }
                                        Err(_) => {
                                            methods::standard_replies::internal_server_error_response()
                                        }
                                    }
                                } else {
                                    methods::standard_replies::internal_server_error_response()
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
