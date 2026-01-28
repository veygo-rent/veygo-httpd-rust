use crate::{POOL, methods, model, helper_model};
use chrono::{Datelike, Utc};
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct BodyData {
    verification_method: model::VerificationType,
    code: String,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("verify-token")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |body: BodyData,
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

                        let mut pool = POOL.get().unwrap();

                        use crate::schema::verifications::dsl as verify_q;
                        let now_utc = Utc::now();

                        let delete_result = diesel::delete
                        (
                            verify_q::verifications
                                .filter(verify_q::verification_method.eq(&body.verification_method))
                                .filter(verify_q::renter_id.eq(&access_token.user_id))
                                .filter(verify_q::expires_at.ge(&now_utc))
                                .filter(verify_q::code.eq(&body.code))
                        ).execute(&mut pool);

                        match delete_result {
                            Ok(count) => {
                                if count >= 1 {
                                    use crate::schema::renters::dsl as r_q;
                                    let updated_renter = match body.verification_method {
                                        model::VerificationType::Email => {
                                            let now = Utc::now().date_naive();
                                            let one_years_from_now = now.with_year(now.year() + 1).unwrap();
                                            diesel::update
                                                (
                                                    r_q::renters
                                                        .find(&access_token.user_id)
                                                )
                                                .set(r_q::student_email_expiration.eq(Some(one_years_from_now)))
                                                .get_result::<model::Renter>(&mut pool)
                                        }
                                        model::VerificationType::Phone => {
                                            diesel::update
                                                (
                                                    r_q::renters
                                                        .find(&access_token.user_id)
                                                )
                                                .set(r_q::phone_is_verified.eq(true))
                                                .get_result::<model::Renter>(&mut pool)
                                        }
                                    };

                                    let Ok(updated_renter) = updated_renter else {
                                        return methods::standard_replies::internal_server_error_response()
                                    };

                                    methods::standard_replies::response_with_obj(updated_renter, StatusCode::OK)
                                } else {
                                    let msg = helper_model::ErrorResponse{
                                        title: "Cannot Verify OTP".to_string(), message: "The OTP you provided is not valid. ".to_string()
                                    };
                                    methods::standard_replies::response_with_obj(msg, StatusCode::NOT_ACCEPTABLE)
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
