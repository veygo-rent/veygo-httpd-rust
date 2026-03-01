use askama::Template;
use http::StatusCode;
use warp::{Filter, Reply, http::method::Method};
use crate::{schema, methods, model, POOL, integration};
use diesel::prelude::*;
use diesel::result::Error;
use rand::{RngExt};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct BodyData {
    email: String,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("request-password-token")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and_then(|method: Method, body: BodyData| async move {
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }

            let mut pool = POOL.get().unwrap();
            use schema::renters::dsl as r_q;
            let usr_result = r_q::renters
                .filter(r_q::student_email.eq(&body.email))
                .select((r_q::id, r_q::student_email, r_q::name))
                .get_result::<(i32, String, String)>(&mut pool);

            let (renter_id, renter_email, renter_name) = match usr_result {
                Ok(info) => { info }
                Err(err) => {
                    return match err {
                        Error::NotFound => {
                            let msg = serde_json::json!({});
                            methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response(String::from("verification/request-password-token: Fetching user info error"))
                        }
                    }
                }
            };

            let otp = rand::rng().random_range(10000000..=99999999).to_string();

            let new_verification = model::NewVerification {
                verification_method: model::VerificationType::ResetPassword,
                renter_id,
                code: otp.clone(),
            };

            use schema::verifications::dsl as v_q;
            let result = diesel::insert_into(v_q::verifications)
                .values(&new_verification)
                .execute(&mut pool);

            let Ok(_) = result else {
                return methods::standard_replies::internal_server_error_response(String::from("verification/request-password-token: Cannot insert OTP"))
            };

            let email = integration::sendgrid_veygo::make_email_obj(
                &renter_email,
                &renter_name,
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
                return methods::standard_replies::internal_server_error_response(String::from("verification/request-token: SendGrid error sending verification email"));
            }

            let msg = serde_json::json!({});
            methods::standard_replies::response_with_obj(msg, StatusCode::OK)
        })
}