use std::ops::Add;
use crate::db;
use crate::model::{AccessToken, NewAccessToken, Renter};
use crate::model::{Apartment, NewRenter};
use crate::schema::access_tokens::dsl::access_tokens;
use crate::schema::apartments::dsl::apartments;
use bcrypt::{hash, DEFAULT_COST};
use chrono::{DateTime, NaiveDate, Utc};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, QueryDsl, QueryResult,
    RunQueryDsl,
};
use regex::Regex;
use tokio::task;
use warp::http::StatusCode;
use warp::Filter;

fn email_belongs_to_domain(email: &str, domain: &str) -> bool {
    email.ends_with(&format!("@{}", domain))
}

fn is_at_least_18(dob: &NaiveDate) -> bool {
    let today = Utc::now().date_naive();

    if let Some(eighteen_years_ago) = today.years_since(dob.clone()) {
        eighteen_years_ago >= 18
    } else {
        false
    }
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

fn is_valid_phone_number(phone: &str) -> bool {
    lazy_static::lazy_static! {
        static ref PHONE_REGEX: Regex = Regex::new(
            r"^\d{10}$"  // Exactly 10 digits
        ).expect("Invalid phone number regex");
    }
    PHONE_REGEX.is_match(phone)
}

pub fn create_user() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path!("create-user")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |mut renter_create: NewRenter, client_type: Option<String>| {
            async move {
                use crate::schema::renters::dsl::*;
                let pool = db::get_connection_pool();

                // Clone necessary fields *before* the spawn_blocking closure
                let email_clone = renter_create.student_email.clone();
                let phone_clone = renter_create.phone.clone();
                let apartment_id_clone = renter_create.apartment_id; // i32 implements Copy, so no need to clone

                if !is_valid_email(&renter_create.student_email) || !is_valid_phone_number(&renter_create.phone) {
                    // invalid email or phone number format
                    let error_msg = serde_json::json!({"email": &renter_create.student_email, "phone": &renter_create.phone, "msg": "Please check your email and phone number format. "});
                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                } else {
                    // valid email
                    let result = task::spawn_blocking(move || {
                        let conn = &mut pool.get().unwrap();
                        renters.filter(student_email.eq(email_clone)
                            .or(phone.eq(phone_clone))).first::<Renter>(conn)
                    }).await;
                    match result {
                        Ok(Ok(_user)) => {
                            // credential existed
                            let error_msg = serde_json::json!({"email": &renter_create.student_email, "phone": &renter_create.phone, "msg": "Invalid email or phone number. "});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                        Ok(Err(_)) => {
                            // new customer
                            if !is_at_least_18(&renter_create.date_of_birth) {
                                // Renter is NOT old enough
                                let error_msg = serde_json::json!({"date of birth": &renter_create.date_of_birth, "msg": "Please make sure you are at least 18 years old. "});
                                Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                            } else {
                                // Renter is old enough
                                let result = task::spawn_blocking(move || {
                                    apartments.find(apartment_id_clone).first::<Apartment>(&mut db::get_connection_pool().get().unwrap())
                                }).await;
                                match result {
                                    // Apartment exists
                                    Ok(Ok(apartment)) => {
                                        if email_belongs_to_domain(&renter_create.student_email, &apartment.accepted_school_email_domain) {
                                            // email correct
                                            let hashed_pass = hash(&renter_create.password, DEFAULT_COST).unwrap();
                                            renter_create.password = hashed_pass;
                                            let to_be_inserted = renter_create.clone();
                                            let _result: Result<QueryResult<Renter>, tokio::task::JoinError> = task::spawn_blocking(move || {
                                                // Diesel operations are synchronous, so we use spawn_blocking
                                                diesel::insert_into(renters)
                                                    .values(&to_be_inserted)
                                                    .get_result::<Renter>(&mut db::get_connection_pool().get().unwrap()) // Get the inserted Renter
                                            }).await; //Awaiting a JoinHandle, not diesel query.
                                            match _result {
                                                Ok(Ok(renter)) => {
                                                    let _token = crate::gen_token::generate_unique_token().await;
                                                    let _user_id = renter.id;
                                                    let mut _exp: DateTime<Utc> = Utc::now().add(chrono::Duration::seconds(600));
                                                    if let Some(client_type) = client_type {
                                                        if client_type == "veygo-app" {
                                                            _exp = Utc::now().add(chrono::Duration::days(28));
                                                        }
                                                    }
                                                    let new_access_token = NewAccessToken {
                                                        user_id: _user_id,
                                                        token: _token,
                                                        exp: _exp,
                                                    };
                                                    let _result: Result<QueryResult<AccessToken>, tokio::task::JoinError> = task::spawn_blocking(move || {
                                                        // Diesel operations are synchronous, so we use spawn_blocking
                                                        diesel::insert_into(access_tokens)
                                                            .values(&new_access_token)
                                                            .get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap()) // Get the inserted Renter
                                                    }).await;
                                                    match _result {
                                                        Ok(Ok(access_token)) => {
                                                            let pub_token = access_token.to_publish_access_token();
                                                            let pub_renter = renter.to_publish_renter();
                                                            let renter_msg = serde_json::json!({
                                                                "renter": pub_renter,
                                                                "access_token": pub_token,
                                                            });
                                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&renter_msg), StatusCode::CREATED),))
                                                        }
                                                        _ => {
                                                            let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                                                }
                                            }
                                        } else {
                                            let error_msg = serde_json::json!({"email": &renter_create.student_email, "accepted domain": &apartment.accepted_school_email_domain});
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                        }
                                    }
                                    Ok(Err(_)) => {
                                        // Wrong apartment ID
                                        let error_msg = serde_json::json!({"apartment": &renter_create.apartment_id, "msg": "Wrong apartment ID. "});
                                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                    }
                                    Err(_) => {
                                        // System error
                                        let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // System error
                            let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                        }
                    }
                }
            }
        })
}
