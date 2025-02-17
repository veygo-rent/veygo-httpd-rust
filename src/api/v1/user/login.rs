use crate::model::{AccessToken, Renter};
use crate::schema::access_tokens::dsl::access_tokens;
use crate::db;
use bcrypt::verify;
use diesel::{ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use tokio::task;
use warp::http::StatusCode;
use warp::Filter;

#[derive(Deserialize, Serialize)]
struct LoginData {
    email: String,
    password: String,
}

pub fn user_login() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("login")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |login_data: LoginData, client_type: Option<String>| {
            async move {
                use crate::schema::renters::dsl::*;
                let pool = db::get_connection_pool();
                let input_email = login_data.email.clone();
                let input_password = login_data.password.clone();
                let result = task::spawn_blocking(move || {
                    let conn = &mut pool.get().unwrap();
                    renters.filter(student_email.eq(&login_data.email)).first::<Renter>(conn)
                }).await;

                match result {
                    Ok(Ok(renter)) => {
                        if verify(&input_password, &renter.password).unwrap_or(false) {
                            let _user_id = renter.id;
                            let new_access_token = crate::gen_token::gen_token_object(_user_id, client_type).await;
                            let _result: Result<QueryResult<AccessToken>, tokio::task::JoinError> = task::spawn_blocking(move || {
                                // Diesel operations are synchronous, so we use spawn_blocking
                                diesel::insert_into(access_tokens)
                                    .values(&new_access_token)
                                    .get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap()) // Get the inserted Renter
                            }).await;
                            match _result {
                                Ok(Ok(access_token)) => {
                                    let pub_token = access_token.to_publish_access_token();
                                    let renter_msg = serde_json::json!({
                                        "renter": {
                                            "id": renter.id,
                                            "name": renter.name,
                                            "student_email": renter.student_email,
                                            "student_email_expiration": renter.student_email_expiration,
                                            "phone": renter.phone,
                                            "phone_is_verified": renter.phone_is_verified,
                                            "date_of_birth": renter.date_of_birth,
                                            "profile_picture": renter.profile_picture,
                                            "gender": renter.gender,
                                            "date_of_registration": renter.date_of_registration,
                                            "drivers_license_number": renter.drivers_license_number,
                                            "drivers_license_state_region": renter.drivers_license_state_region,
                                            "drivers_license_image": renter.drivers_license_image,
                                            "drivers_license_image_secondary": renter.drivers_license_image_secondary,
                                            "drivers_license_expiration": renter.drivers_license_expiration,
                                            "insurance_id_image": renter.insurance_id_image,
                                            "insurance_id_expiration": renter.insurance_id_expiration,
                                            "lease_agreement_image": renter.lease_agreement_image,
                                            "apartment_id": renter.apartment_id,
                                            "lease_agreement_expiration": renter.lease_agreement_expiration,
                                            "billing_address": renter.billing_address,
                                            "signature_image": renter.signature_image,
                                            "signature_datetime": renter.signature_datetime,
                                            "plan_tier": renter.plan_tier,
                                            "plan_renewal_day": renter.plan_renewal_day,
                                            "plan_expire_month_year": renter.plan_expire_month_year,
                                            "plan_available_duration": renter.plan_available_duration,
                                            "is_plan_annual": renter.is_plan_annual
                                        },
                                        "access_token": pub_token,
                                    });
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&renter_msg), StatusCode::ACCEPTED),))
                                }
                                _ => {
                                    let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                                }
                            }
                        } else {
                            let error_msg = serde_json::json!({"email": &input_email, "password": &input_password, "error": "Credentials invalid. "});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                    }
                    Ok(Err(_)) => {
                        let error_msg = serde_json::json!({"email": &input_email, "password": &input_password, "error": "Credentials invalid. "});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                    }
                    Err(_) => {
                        let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                    }
                }
            }
        })
}
