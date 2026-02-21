use crate::{POOL, integration, methods, model, helper_model};
use askama::Template;
use diesel::prelude::*;
use rand::{RngExt};
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct BodyData {
    verification_method: model::VerificationType,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("request-token")
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
                                methods::standard_replies::internal_server_error_response(
                                    "verification/request-token: Token verification unexpected error",
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
                                        "verification/request-token: Token extension failed (returned false)",
                                    )
                                    .await;
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "verification/request-token: Token extension error",
                                )
                                .await;
                            }
                        }

                        if body.verification_method == model::VerificationType::ResetPassword {
                            let msg = helper_model::ErrorResponse{
                                title: "Cannot Request OTP".to_string(), message: "Please do not request password reset code here. ".to_string()
                            };
                            return methods::standard_replies::response_with_obj(msg, StatusCode::NOT_ACCEPTABLE)
                        }

                        let otp = rand::rng().random_range(10000000..=99999999).to_string();
                        let to_be_inserted = model::NewVerification {
                            verification_method: body.verification_method,
                            renter_id: access_token.user_id,
                            code: otp.clone(),
                        };

                        let renter = methods::user::get_user_by_id(&access_token.user_id)
                            .await;
                        let Ok(renter) = renter else {
                            return methods::standard_replies::internal_server_error_response(
                                "verification/request-token: Database error loading renter",
                            )
                            .await;
                        };

                        use crate::schema::verifications::dsl as v_q;
                        let mut pool = POOL.get().unwrap();
                        let result = diesel::insert_into(v_q::verifications)
                            .values(&to_be_inserted)
                            .execute(&mut pool);

                        if result.is_err() {
                            return methods::standard_replies::internal_server_error_response(
                                "verification/request-token: SQL error inserting verification",
                            )
                            .await;
                        }

                        match body.verification_method {
                            model::VerificationType::Phone => {
                                let phone = &renter.phone;
                                let call_result = integration::twilio_veygo::call_otp(
                                    phone, &*otp)
                                    .await;
                                if call_result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        "verification/request-token: Twilio error sending OTP",
                                    )
                                    .await;
                                }
                            }
                            model::VerificationType::Email => {
                                let email = integration::sendgrid_veygo::make_email_obj(
                                    &renter.student_email,
                                    &renter.name,
                                );
                                #[derive(Template)]
                                #[template(path = "email_verification.html")]
                                struct EmailVerificationTemplate<'a> {
                                    verification_code: &'a str,
                                }
                                let email_content = EmailVerificationTemplate { verification_code: &otp };
                                let email_result = integration::sendgrid_veygo::send_email(
                                    None,
                                    email,
                                    "Your Verification Code",
                                    &email_content.render().unwrap(),
                                    None,
                                    None,
                                )
                                    .await;
                                if email_result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        "verification/request-token: SendGrid error sending verification email",
                                    )
                                    .await;
                                }
                            }
                            model::VerificationType::ResetPassword => {
                                return methods::standard_replies::internal_server_error_response(
                                    "verification/request-token: Should not request reset password code here",
                                ).await;
                            }
                        }

                        let msg = serde_json::json!({});
                        methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                    }
                };
            },
        )
}
