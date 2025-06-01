use crate::{POOL, methods, model};
use chrono::{Datelike, Utc};
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

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
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_wrapped_return(&auth);
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
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
                            let new_token = methods::tokens::gen_token_object(
                                access_token.user_id.clone(),
                                user_agent.clone(),
                            )
                                .await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .to_publish_access_token();
                            let mut renter = methods::user::get_user_by_id(access_token.user_id)
                                .await
                                .unwrap();
                            use crate::schema::verifications::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let now_utc = Utc::now();
                            let if_verify = diesel::select(diesel::dsl::exists(
                                verifications
                                    .into_boxed()
                                    .filter(verification_method.eq(body.verification_method))
                                    .filter(renter_id.eq(access_token.user_id))
                                    .filter(expires_at.ge(now_utc))
                                    .filter(code.eq(body.code.clone())),
                            )).get_result::<bool>(&mut pool).unwrap();
                            if if_verify {
                                use crate::schema::verifications::dsl::*;
                                diesel::delete(verifications
                                    .filter(verification_method.eq(body.verification_method))
                                    .filter(renter_id.eq(access_token.user_id))
                                    .filter(code.eq(body.code.clone()))
                                ).execute(&mut pool).unwrap();
                                renter = match body.verification_method {
                                    model::VerificationType::Phone => {
                                        renter.phone_is_verified = true;
                                        renter
                                    }
                                    model::VerificationType::Email => {
                                        let now = Utc::now().date_naive();
                                        let two_years_from_now = now.with_year(now.year() + 2).unwrap();
                                        renter.student_email_expiration = Some(two_years_from_now);
                                        renter
                                    }
                                };
                                use crate::schema::renters::dsl::*;
                                let new_renter = diesel::update(renters.find(&renter.id.clone())).set(renter).get_result::<model::Renter>(&mut pool).unwrap().to_publish_renter();
                                let msg = serde_json::json!({"verified_renter": new_renter});
                                Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    with_status(warp::reply::json(&msg), StatusCode::OK),
                                ),))
                            } else {
                                let msg = serde_json::json!({"code": body.code, "error": "Cannot be verified"});
                                Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    with_status(warp::reply::json(&msg), StatusCode::NOT_ACCEPTABLE),
                                ),))
                            }
                        }
                    }
                };
            },
        )
}
