use crate::integration::stripe_veygo;
use crate::model;
use crate::{methods, POOL};
use bcrypt::{hash, DEFAULT_COST};
use chrono::{Datelike, NaiveDate, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use tokio::task;
use warp::http::StatusCode;
use warp::Filter;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct CreateUserData {
    name: String,
    student_email: String,
    password: String,
    phone: String,
    date_of_birth: NaiveDate,
    apartment_id: i32,
}

fn email_belongs_to_domain(email: &str, domain: &str) -> bool {
    email.ends_with(&format!("@{}", domain))
}

fn is_at_least_18(dob: &NaiveDate) -> bool {
    let today = Utc::now().date_naive();

    // Try to compute the 18th birthday by replacing the year
    let eighteenth_birthday = dob
        .with_year(dob.year() + 18)
        // If dob is Feb 29 and the target year isn't a leap year, fallback to Feb 28.
        .unwrap_or_else(|| {
            NaiveDate::from_ymd_opt(dob.year() + 18, 2, 28)
                .expect("Feb 28 should always be a valid date")
        });

    today >= eighteenth_birthday
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

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("create")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |mut renter_create_data: CreateUserData, client_type: Option<String>| {
            async move {
                use crate::schema::renters::dsl::*;
                let mut pool = POOL.clone().get().unwrap();

                // Clone necessary fields *before* the spawn_blocking closure
                let email_clone = renter_create_data.student_email.clone();
                let phone_clone = renter_create_data.phone.clone();
                let apartment_id_clone = renter_create_data.apartment_id; // i32 implements Copy, so no need to clone

                if !is_valid_email(&renter_create_data.student_email) || !is_valid_phone_number(&renter_create_data.phone) {
                    // invalid email or phone number format
                    let error_msg = serde_json::json!({"email": &renter_create_data.student_email, "phone": &renter_create_data.phone, "error": "Please check your email and phone number format"});
                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST),))
                } else {
                    // valid email
                    let result = task::spawn_blocking(move || {
                        renters.filter(student_email.eq(email_clone)
                            .or(phone.eq(phone_clone))).get_result::<model::Renter>(&mut pool)
                    }).await.unwrap();
                    match result {
                        Ok(_user) => {
                            // credential existed
                            let error_msg = serde_json::json!({"email": &renter_create_data.student_email, "phone": &renter_create_data.phone, "error": "Invalid email or phone number"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                        Err(_) => {
                            // new customer
                            if !is_at_least_18(&renter_create_data.date_of_birth) {
                                // Renter is NOT old enough
                                let error_msg = serde_json::json!({"date_of_birth": &renter_create_data.date_of_birth, "error": "Please make sure you are at least 18 years old"});
                                Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                            } else {
                                // Renter is old enough
                                let mut pool = POOL.clone().get().unwrap();
                                let result = task::spawn_blocking(move || {
                                    use crate::schema::apartments::dsl::*;
                                    apartments.find(apartment_id_clone).first::<model::Apartment>(&mut pool)
                                }).await.unwrap();
                                match result {
                                    // Apartment exists
                                    Ok(apartment) => {
                                        if email_belongs_to_domain(&renter_create_data.student_email, &apartment.accepted_school_email_domain) {
                                            // email correct
                                            let hashed_pass = hash(&renter_create_data.password, DEFAULT_COST).unwrap();
                                            renter_create_data.password = hashed_pass;
                                            // Get today's date.
                                            let today = Utc::now().date_naive();

                                            // For the plan renewal day, keep today’s day as a two-digit string.
                                            let plan_renewal_day_string = format!("{:02}", today.day());

                                            // Calculate next month and its year.
                                            let (next_month, next_year) = if today.month() == 12 {
                                                (1, today.year() + 1)
                                            } else {
                                                (today.month() + 1, today.year())
                                            };

                                            // Format plan_expire_month_year as MMYYYY.
                                            let plan_expire_month_year_string = format!("{:02}{}", next_month, next_year);

                                            let to_be_inserted = model::NewRenter {
                                                name: renter_create_data.name,
                                                student_email: renter_create_data.student_email,
                                                password: renter_create_data.password,
                                                phone: renter_create_data.phone,
                                                date_of_birth: renter_create_data.date_of_birth,
                                                apartment_id: renter_create_data.apartment_id,
                                                plan_renewal_day: plan_renewal_day_string,
                                                plan_expire_month_year: plan_expire_month_year_string,
                                                plan_available_duration: apartment.free_tier_hours,
                                            };
                                            let mut pool = POOL.clone().get().unwrap();
                                            let mut renter = task::spawn_blocking(move || {
                                                diesel::insert_into(renters)
                                                    .values(&to_be_inserted)
                                                    .get_result::<model::Renter>(&mut pool) // Get the inserted Renter
                                            }).await.unwrap().unwrap(); //Awaiting a JoinHandle, not diesel query.

                                            let stripe_name = renter.name.clone();
                                            let stripe_phone = renter.phone.clone();
                                            let stripe_email = renter.student_email.clone();
                                            let stripe_result = stripe_veygo::create_stripe_customer(stripe_name, stripe_phone, stripe_email).await;
                                            match stripe_result {
                                                Ok(stripe_customer) => {
                                                    let stripe_customer_id = stripe_customer.id.to_string();
                                                    let renter_id_to_add_stripe = renter.id.clone();
                                                    let mut pool = POOL.clone().get().unwrap();
                                                    let new_renter = diesel::update(renters.find(renter_id_to_add_stripe)).set(stripe_id.eq(stripe_customer_id)).get_result::<model::Renter>(&mut pool).unwrap();
                                                    renter = new_renter;
                                                }
                                                Err(_) => {
                                                    use crate::schema::renters::dsl::*;
                                                    let mut pool = POOL.clone().get().unwrap();
                                                    diesel::delete(renters.filter(id.eq(renter.id))).execute(&mut pool).unwrap();
                                                    return methods::standard_replys::internal_server_error_response_without_access_token();
                                                }
                                            }
                                            let user_id_data = renter.id;
                                            let new_access_token = crate::methods::tokens::gen_token_object(user_id_data, client_type).await;
                                            let mut pool = POOL.clone().get().unwrap();
                                            let insert_token_result = task::spawn_blocking(move || {
                                                use crate::schema::access_tokens::dsl::*;
                                                diesel::insert_into(access_tokens)
                                                    .values(&new_access_token)
                                                    .get_result::<model::AccessToken>(&mut pool) // Get the inserted Renter
                                            }).await.unwrap().unwrap();

                                            let pub_token = insert_token_result.to_publish_access_token();
                                            let pub_renter = renter.to_publish_renter();
                                            let renter_msg = serde_json::json!({
                                                                "renter": pub_renter,
                                                                "access_token": pub_token,
                                                            });
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&renter_msg), StatusCode::CREATED),))
                                        } else {
                                            let error_msg = serde_json::json!({"email": &renter_create_data.student_email, "accepted_domain": &apartment.accepted_school_email_domain, "error": "Email not accepted"});
                                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                        }
                                    }
                                    Err(_) => {
                                        // Wrong apartment ID
                                        let error_msg = serde_json::json!({"apartment": &renter_create_data.apartment_id, "error": "Wrong apartment ID"});
                                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
}
