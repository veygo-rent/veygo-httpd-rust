use crate::{POOL, integration, methods, model};
use diesel::prelude::*;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct BodyData {
    verification_method: model::VerificationType,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("request-token")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |body: BodyData,
                        token: String,
                        user_id: i32,
                        client_type: Option<String>| {
                let access_token = model::RequestToken { user_id, token };
                let if_token_valid = methods::tokens::verify_user_token(
                    access_token.user_id.clone(),
                    access_token.token.clone(),
                )
                .await;
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
                        } else {
                            // gen new token
                            let token_clone = access_token.clone();
                            methods::tokens::rm_token_by_binary(
                                hex::decode(token_clone.token).unwrap(),
                            )
                            .await;
                            let user_id_clone = access_token.user_id.clone();
                            let new_token = methods::tokens::gen_token_object(
                                access_token.user_id.clone(),
                                client_type.clone(),
                            )
                            .await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .to_publish_access_token();
                            let otp = rand::rng().random_range(100000..=999999).to_string();
                            let to_be_inserted = model::NewVerification {
                                verification_method: body.verification_method,
                                renter_id: user_id_clone,
                                code: otp.clone(),
                            };
                            use crate::schema::verifications::dsl::*;
                            diesel::insert_into(verifications)
                                .values(&to_be_inserted)
                                .execute(&mut pool)
                                .unwrap();
                            let renter =
                                methods::user::get_user_by_id(user_id_clone).await.unwrap();
                            match body.verification_method {
                                model::VerificationType::Phone => {
                                    // TODO: Send phone 
                                }
                                model::VerificationType::Email => {
                                    let email = integration::sendgrid_veygo::make_email_obj(
                                        &renter.student_email,
                                        &renter.name,
                                    );
                                    integration::sendgrid_veygo::send_email(None, email, "Your Verification Code", &*otp, None, None).await.unwrap();
                                }
                            }
                            let msg = serde_json::json!({});
                            Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(
                                new_token_in_db_publish,
                                with_status(warp::reply::json(&msg), StatusCode::OK),
                            ),))
                        }
                    }
                };
            },
        )
}
