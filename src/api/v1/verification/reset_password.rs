use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use diesel::prelude::*;
use diesel::result::Error;
use http::StatusCode;
use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Reply, http::method::Method};
use crate::{helper_model, methods, model, schema, POOL};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct BodyData {
    email: String,
    code: String,
    new_password: String,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("reset-password")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and_then(async move | method: Method, body: BodyData | {
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }
            let mut pool = POOL.get().unwrap();
            use schema::renters::dsl as r_q;
            let usr_result = r_q::renters
                .filter(r_q::student_email.eq(&body.email))
                .select(r_q::id)
                .get_result::<i32>(&mut pool);

            let user_id = match usr_result {
                Ok(id) => { id }
                Err(err) => {
                    return match err {
                        Error::NotFound => {
                            let msg = helper_model::ErrorResponse{
                                title: "Cannot Reset Password".to_string(), message: "Please double check your credential. ".to_string()
                            };
                            return methods::standard_replies::response_with_obj(msg, StatusCode::NOT_ACCEPTABLE)
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response("verification/reset-password: Fetching user info error").await
                        }
                    }
                }
            };

            use crate::schema::verifications::dsl as verify_q;
            let now_utc = Utc::now();

            let delete_result = diesel::delete
                (
                    verify_q::verifications
                        .filter(verify_q::code.eq(&body.code))
                        .filter(verify_q::verification_method.eq(model::VerificationType::ResetPassword))
                        .filter(verify_q::renter_id.eq(&user_id))
                        .filter(verify_q::expires_at.ge(&now_utc))
                ).execute(&mut pool);

            let Ok(count) = delete_result else {
                return methods::standard_replies::internal_server_error_response("verification/reset-password: Verifying code error").await
            };

            if count >= 1 {
                let hashed_pass = hash(&body.new_password, DEFAULT_COST).unwrap();
                let result = diesel::update(r_q::renters.find(&user_id))
                    .set(r_q::password.eq(&hashed_pass))
                    .execute(&mut pool);
                match result {
                    Ok(updated_account_count) => {
                        if updated_account_count == 1 {
                            use schema::access_tokens::dsl as at_q;
                            let _ = diesel::delete(at_q::access_tokens.filter(at_q::user_id.eq(&user_id))).execute(&mut pool);
                            let msg = serde_json::json!({});
                            methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                        } else {
                            methods::standard_replies::internal_server_error_response(format!("{}{}{}", "verification/reset-password: [URGENT] Updating password error, ", updated_account_count, "account(s) updated").as_str()).await
                        }
                    }
                    Err(_) => {
                        methods::standard_replies::internal_server_error_response("verification/reset-password: Updating password error").await
                    }
                }
            } else {
                let msg = helper_model::ErrorResponse{
                    title: "Cannot Reset Password".to_string(), message: "Please double check your OTP code. ".to_string()
                };
                return methods::standard_replies::response_with_obj(msg, StatusCode::NOT_ACCEPTABLE)
            }
        })
}
