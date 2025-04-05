use crate::model::Renter;
use crate::{methods, model, POOL};
use diesel::prelude::*;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use tokio::task;
use warp::http::StatusCode;
use warp::Filter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CreatePaymentMethodsRequestBody {
    access_token: model::RequestBodyToken,
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

pub fn update() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("update-apartment")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |body: CreatePaymentMethodsRequestBody, client_type: Option<String>| {
                let if_token_valid = methods::tokens::verify_user_token(
                    body.access_token.user_id.clone(),
                    body.access_token.token.clone(),
                )
                .await;
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(&body.access_token.token),
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_invalid_warp_return(&body.access_token.token)
                        } else {
                            // gen new token
                            let body_clone = body.clone();
                            methods::tokens::rm_token_by_binary(
                                hex::decode(body_clone.access_token.token).unwrap(),
                            )
                            .await;
                            let new_token = methods::tokens::gen_token_object(
                                body.access_token.user_id.clone(),
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
                            let body_clone = body.clone();
                            let apartment_result = task::spawn_blocking(move || {
                                use crate::schema::apartments::dsl::*;
                                apartments.find(body_clone.apartment_id).get_result::<model::Apartment>(&mut pool)
                            }).await.unwrap();
                            match apartment_result {
                                Err(_) => {
                                    // Wrong apartment ID
                                    let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "apartment": &body.apartment_id, "msg": "Wrong apartment ID"});
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST),))
                                }
                                Ok(apartment) => {
                                    if !is_valid_email(&body.student_email) {
                                        let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "email": &body.student_email});
                                        return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                    }
                                    if !email_belongs_to_domain(&body.student_email, &apartment.accepted_school_email_domain) {
                                        let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "email": &body.student_email, "accepted_domain": &apartment.accepted_school_email_domain});
                                        return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST),));
                                    }
                                    let mut pool = POOL.clone().get().unwrap();
                                    use crate::schema::renters::dsl::*;
                                    let clone_of_user_id = body.access_token.user_id.clone();
                                    let renter_updated = diesel::update(renters.find(clone_of_user_id))
                                        .set((
                                            student_email.eq(body.student_email.clone()),
                                            apartment_id.eq(body.apartment_id.clone()),
                                        )).get_result::<Renter>(&mut pool).unwrap().to_publish_renter();
                                    let renter_msg = serde_json::json!({
                                        "renter": renter_updated,
                                        "access_token": new_token_in_db_publish,
                                    });
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&renter_msg), StatusCode::OK),))
                                }
                            }
                        }
                    }
                };
            },
        )
}
