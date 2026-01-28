use crate::{POOL, methods, model};
use diesel::prelude::*;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UpdatePhoneBody {
    phone_number: String,
}

fn is_valid_phone_number(phone: &str) -> bool {
    lazy_static::lazy_static! {
        static ref PHONE_REGEX: Regex = Regex::new(
            r"^\d{10}$"  // Exactly 10 digits
        ).expect("Invalid phone number regex");
    }
    PHONE_REGEX.is_match(phone)
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("update-phone")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |body: UpdatePhoneBody,
                        auth: String,
                        user_agent: String| {
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_return();
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_return();
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                if !is_valid_phone_number(&body.phone_number) {
                    // invalid email or phone number format
                    return methods::standard_replies::bad_request("Please check your phone number format");
                };
                let if_token_valid = methods::tokens::verify_user_token(
                    &access_token.user_id,
                    &access_token.token,
                )
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

                        use crate::schema::renters::dsl as r_q;
                        let mut pool = POOL.get().unwrap();

                        let update_result = diesel::update
                            (
                                r_q::renters
                                    .find(&access_token.user_id)
                            )
                            .set((r_q::phone.eq(&body.phone_number), r_q::phone_is_verified.eq(false)))
                            .get_result::<model::Renter>(&mut pool);

                        let Ok(renter) = update_result else {
                            return methods::standard_replies::internal_server_error_response()
                        };

                        return methods::standard_replies::response_with_obj(renter, http::StatusCode::OK);
                    }
                };
            },
        )
}
