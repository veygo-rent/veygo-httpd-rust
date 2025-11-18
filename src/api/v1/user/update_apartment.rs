use crate::{POOL, methods, model};
use diesel::prelude::*;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UpdateApartmentBody {
    student_email: String,
    apartment_id: i32,
}

fn email_belongs_to_domain(email: &str, domain: &str) -> bool {
    email.ends_with(&format!("@{}", domain))
}

fn is_valid_email(email: &str) -> bool {
    lazy_static::lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(
            r"(?i)^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$"
        ).expect("Invalid regex");
    }
    // Check overall length (RFC 5321 limit is 254, but some say 320)
    if email.len() > 254 {
        return false;
    }
    EMAIL_REGEX.is_match(email)
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("update-apartment")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |body: UpdateApartmentBody, auth: String, user_agent: String| {
                if !is_valid_email(&body.student_email) {
                    return methods::standard_replies::bad_request("Email is invalid")
                }

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
                    &access_token.user_id,
                    &access_token.token,
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
                            ).await;
                            let new_token = methods::tokens::gen_token_object(
                                &access_token.user_id,
                                &user_agent,
                            ).await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.get().unwrap();
                            let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .into();
                            use crate::schema::apartments::dsl::*;
                            let apartment_result = apartments.find(&body.apartment_id).get_result::<model::Apartment>(&mut pool);
                            match apartment_result {
                                Err(_) => {
                                    // Wrong apartment ID
                                    let error_msg = serde_json::json!({"apartment": &body.apartment_id, "error": "Wrong apartment ID"});
                                    Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                                }
                                Ok(apartment) => {
                                    if !email_belongs_to_domain(&body.student_email, &apartment.accepted_school_email_domain) {
                                        let error_msg = serde_json::json!({"email": &body.student_email, "accepted_domain": &apartment.accepted_school_email_domain});
                                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                                    }
                                    let mut pool = POOL.get().unwrap();
                                    use crate::schema::renters::dsl::*;
                                    let mut user_update = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();
                                    user_update.student_email = body.student_email.clone();
                                    user_update.apartment_id = body.apartment_id.clone();
                                    user_update.lease_agreement_image = None;
                                    user_update.lease_agreement_expiration = None;
                                    user_update.student_email_expiration = None;
                                    user_update.drivers_license_number = None;
                                    user_update.drivers_license_state_region = None;
                                    user_update.drivers_license_image = None;
                                    user_update.drivers_license_image_secondary = None;
                                    user_update.drivers_license_expiration = None;
                                    let renter_updated: model::PublishRenter = diesel::update(renters.find(&access_token.user_id))
                                        .set(&user_update).get_result::<model::Renter>(&mut pool).unwrap().into();
                                    return methods::standard_replies::renter_wrapped(new_token_in_db_publish, &renter_updated);
                                }
                            }
                        }
                    }
                };
            },
        )
}
