use crate::integration::stripe_veygo;
use crate::model;
use crate::{POOL, methods};
use bcrypt::{DEFAULT_COST, hash};
use chrono::{Datelike, NaiveDate, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

#[derive(Deserialize, Serialize, Clone, Debug)]
struct CreateUserData {
    name: String,
    student_email: String,
    password: String,
    phone: String,
    date_of_birth: NaiveDate,
}

fn get_email_domain(email: &str) -> Option<String> {
    email.split('@').nth(1).map(|s| s.to_lowercase())
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
    if email.len() > 254 {
        return false;
    }
    lazy_static::lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(
            r"(?i)^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9-](?:[a-z0-9-]{0,61}[a-z0-9])+(?:\.[a-z0-9-](?:[a-z0-9-]{0,61}[a-z0-9])+)+$"
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

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("create")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |mut renter_create_data: CreateUserData, user_agent: String| {
            use crate::schema::renters::dsl::*;
            let mut pool = POOL.get().unwrap();

            let email_clone = renter_create_data.student_email.clone();
            let phone_clone = renter_create_data.phone.clone();

            if !is_valid_email(&renter_create_data.student_email) || !is_valid_phone_number(&renter_create_data.phone) {
                // invalid email or phone number format
                let error_msg = serde_json::json!({"email": &renter_create_data.student_email, "phone": &renter_create_data.phone, "error": "Please check your email and phone number format"});
                Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST).into_response(),))
            } else {
                // valid email
                let result = renters.filter(student_email.eq(&email_clone)
                    .or(phone.eq(&phone_clone))).get_result::<model::Renter>(&mut pool);
                match result {
                    Ok(_user) => {
                        // credential existed
                        let error_msg = serde_json::json!({"email": &renter_create_data.student_email, "phone": &renter_create_data.phone, "error": "Invalid email or phone number"});
                        Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE).into_response(),))
                    }
                    Err(_) => {
                        // new customer
                        if !is_at_least_18(&renter_create_data.date_of_birth) {
                            // Renter is NOT old enough
                            let error_msg = serde_json::json!({"date_of_birth": &renter_create_data.date_of_birth, "error": "Please make sure you are at least 18 years old"});
                            Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE).into_response(),))
                        } else {
                            // Renter is old enough
                            use crate::schema::apartments::dsl::*;

                            let input_email_domain = get_email_domain(&email_clone).unwrap();
                            let result;
                            result = if input_email_domain != "veygo.rent" { apartments.filter(uni_id.eq(0)).filter(accepted_school_email_domain.eq(&input_email_domain)).get_result::<model::Apartment>(&mut pool) } else { apartments.filter(accepted_school_email_domain.eq(&input_email_domain)).get_result::<model::Apartment>(&mut pool) };
                            match result {
                                // Matched Apartment Found
                                Ok(apartment) => {
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

                                    let emp_tier: model::EmployeeTier;
                                    if &apartment.accepted_school_email_domain == "veygo.rent" {
                                        emp_tier = model::EmployeeTier::Admin;
                                    } else {
                                        emp_tier = model::EmployeeTier::User;
                                    }

                                    let to_be_inserted = model::NewRenter {
                                        name: renter_create_data.name,
                                        student_email: renter_create_data.student_email,
                                        password: renter_create_data.password,
                                        phone: renter_create_data.phone,
                                        date_of_birth: renter_create_data.date_of_birth,
                                        apartment_id: apartment.id,
                                        plan_renewal_day: plan_renewal_day_string,
                                        plan_expire_month_year: plan_expire_month_year_string,
                                        plan_available_duration: apartment.free_tier_hours,
                                        employee_tier: emp_tier,
                                    };
                                    let mut renter = diesel::insert_into(renters)
                                        .values(&to_be_inserted)
                                        .get_result::<model::Renter>(&mut pool).unwrap();

                                    let stripe_result = stripe_veygo::create_stripe_customer(&renter.name, &renter.phone, &renter.student_email).await;
                                    match stripe_result {
                                        Ok(stripe_customer) => {
                                            let stripe_customer_id = stripe_customer.id.to_string();
                                            let renter_id_to_add_stripe = renter.id.clone();
                                            let new_renter = diesel::update(renters.find(renter_id_to_add_stripe)).set(stripe_id.eq(stripe_customer_id)).get_result::<model::Renter>(&mut pool).unwrap();
                                            renter = new_renter;
                                        }
                                        Err(_) => {
                                            use crate::schema::renters::dsl::*;
                                            diesel::delete(renters.filter(id.eq(renter.id))).execute(&mut pool).unwrap();
                                            return methods::standard_replies::internal_server_error_response();
                                        }
                                    }
                                    let user_id_data = renter.id;
                                    let new_access_token = methods::tokens::gen_token_object(&user_id_data, &user_agent).await;
                                    use crate::schema::access_tokens::dsl::*;
                                    let insert_token_result = diesel::insert_into(access_tokens)
                                        .values(&new_access_token)
                                        .get_result::<model::AccessToken>(&mut pool)
                                        .unwrap();

                                    let pub_token = insert_token_result.to_publish_access_token();
                                    let pub_renter = renter.to_publish_renter();
                                    let renter_msg = serde_json::json!({
                                            "renter": pub_renter,
                                        });
                                    Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(pub_token, with_status(warp::reply::json(&renter_msg), StatusCode::CREATED)),))
                                }
                                Err(_) => {
                                    // Matched Apartment Not Found
                                    let error_msg = serde_json::json!({"email": &renter_create_data.student_email, "error": "Email not accepted"});
                                    Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE).into_response(),))
                                }
                            }
                        }
                    }
                }
            }
        })
}
