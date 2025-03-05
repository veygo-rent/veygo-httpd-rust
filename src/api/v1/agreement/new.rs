use chrono::{DateTime, Utc, Duration};
use diesel::RunQueryDsl;
use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Rejection};
use warp::http::StatusCode;
use crate::{model, POOL};
use crate::{methods, integration};
use crate::model::{AccessToken, Apartment, NewAgreement, PaymentMethod, Vehicle};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
use stripe::{ErrorCode, StripeError};
use tokio::task::spawn_blocking;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct NewAgreementRequestBodyData {
    access_token: model::RequestBodyToken, // contains 'user_id' and 'token'
    vehicle_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    payment_id: i32,
    liability: bool,
    pcdw: bool,
    pcdw_ext: bool,
    rsa: bool,
    pai: bool,
}

pub fn new_agreement(
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("new")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |body: NewAgreementRequestBodyData, client_type: Option<String>| async move {
            let if_token_valid = methods::tokens::verify_user_token(body.access_token.user_id.clone(), body.access_token.token.clone()).await;
            match if_token_valid {
                Err(_) => {
                    methods::tokens::token_not_hex_warp_return(&body.access_token.token)
                }
                Ok(token_bool) => {
                    if !token_bool {
                        methods::tokens::token_invalid_warp_return(&body.access_token.token)
                    } else {
                        // Token is valid, generate new publish token, user_id valid
                        methods::tokens::rm_token_by_binary(hex::decode(body.access_token.token).unwrap()).await;
                        let new_token = methods::tokens::gen_token_object(body.access_token.user_id.clone(), client_type.clone()).await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.clone().get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut pool).unwrap().to_publish_access_token();
                        let user_in_request = methods::user::get_user_by_id(body.access_token.user_id).await.unwrap();
                        // Check if renter in DNR
                        let if_in_dnr = methods::user::check_if_on_do_not_rent(&user_in_request).await;
                        if !if_in_dnr {
                            if body.start_time + Duration::hours(1) > body.end_time || body.start_time - Duration::hours(1) < Utc::now() {
                                let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Time invalid"});
                                return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN),))
                            }
                            // check vehicle::exist, vehicle.available, vehicle.apt_id == renter.apt_id => invalid_vehicle
                            use crate::schema::vehicles::dsl::*;
                            let renter_clone = user_in_request.clone();
                            let mut pool = POOL.clone().get().unwrap();
                            let vehicle_result = spawn_blocking(move || {
                                vehicles.filter(id.eq(&body.vehicle_id)).get_result::<Vehicle>(&mut pool)
                            }).await.unwrap();
                            match vehicle_result {
                                Ok(vehicle) => {
                                    if vehicle.available && vehicle.apartment_id == renter_clone.id {
                                        // Vehicle checked, check pm::exist, if pm.is_enabled, pm.renter_id == user.id => invalid_payment_method
                                        use crate::schema::payment_methods::dsl::*;
                                        let mut pool = POOL.clone().get().unwrap();
                                        let pm_result = spawn_blocking(move || {
                                            payment_methods.filter(id.eq(&body.payment_id)).get_result::<PaymentMethod>(&mut pool)
                                        }).await.unwrap();
                                        match pm_result {
                                            Ok(pm) => {
                                                if pm.is_enabled && pm.renter_id == user_in_request.id {
                                                    // vehicle and payment method are valid, check if time has any conflicts
                                                    let start_time_buffered = body.start_time - Duration::minutes(30);
                                                    let end_time_buffered   = body.end_time   + Duration::minutes(30);

                                                    let mut pool = POOL.clone().get().unwrap();
                                                    use crate::schema::agreements::dsl::*;
                                                    use diesel::dsl::sql;
                                                    let if_conflict = diesel::select(diesel::dsl::exists(
                                                        agreements.filter(sql::<Bool>("COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                                            .bind::<Timestamptz, _>(start_time_buffered)
                                                            .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                                            .bind::<Timestamptz, _>(start_time_buffered)
                                                            .sql(" OR COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                                            .bind::<Timestamptz, _>(end_time_buffered)
                                                            .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                                            .bind::<Timestamptz, _>(end_time_buffered))
                                                    )).get_result::<bool>(&mut pool).unwrap();
                                                    if if_conflict {
                                                        let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Vehicle unavailable for the requested time"});
                                                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::CONFLICT),))
                                                    } else {
                                                        let mut pool = POOL.clone().get().unwrap();
                                                        let apartment_id_clone = vehicle.apartment_id.clone();
                                                        let apt = spawn_blocking(move || {
                                                            use crate::schema::apartments::dsl::*;
                                                            apartments.filter(id.eq(apartment_id_clone)).get_result::<Apartment>(&mut pool)
                                                        }).await.unwrap().unwrap();
                                                        let conf_id = methods::agreement::generate_unique_agreement_confirmation();
                                                        let new_agreement = NewAgreement {
                                                            confirmation: conf_id,
                                                            status: crate::model::AgreementStatus::Rental,
                                                            user_name: renter_clone.name.clone(),
                                                            user_date_of_birth: renter_clone.date_of_birth.clone(),
                                                            user_email: renter_clone.student_email.clone(),
                                                            user_phone: renter_clone.phone.clone(),
                                                            user_billing_address: "2101 Cumberland Ave".to_string(),
                                                            rsvp_pickup_time: body.start_time,
                                                            rsvp_drop_off_time: body.end_time,
                                                            liability_protection_rate: if body.liability { apt.liability_protection_rate } else { 0.00 },
                                                            pcdw_protection_rate: if body.pcdw { apt.pcdw_protection_rate } else { 0.00 },
                                                            pcdw_ext_protection_rate: if body.pcdw_ext { apt.pcdw_ext_protection_rate } else { 0.00 },
                                                            rsa_protection_rate: if body.rsa { apt.rsa_protection_rate } else { 0.00 },
                                                            pai_protection_rate: if body.pai { apt.pai_protection_rate } else { 0.00 },
                                                            tax_rate: apt.sales_tax_rate,
                                                            msrp_factor: vehicle.msrp_factor,
                                                            duration_rate: apt.duration_rate,
                                                            apartment_id: vehicle.apartment_id,
                                                            vehicle_id: vehicle.id,
                                                            renter_id: renter_clone.id,
                                                            payment_method_id: body.payment_id,
                                                        };
                                                        // TODO: auth deposit
                                                        let fifty_dollar_auth = integration::stripe_veygo::create_payment_intent(
                                                            new_agreement.confirmation.clone(), user_in_request.stripe_id.unwrap(), pm.token.clone(), 500
                                                        ).await;
                                                        match fifty_dollar_auth {
                                                            Err(error) => {
                                                                match error {
                                                                    StripeError::Stripe(request_error) => {
                                                                        eprintln!("Stripe API error: {:?}", request_error);
                                                                        if request_error.code == Some(ErrorCode::CardDeclined) {
                                                                            return methods::standard_replys::card_declined(&new_token_in_db_publish);
                                                                        }
                                                                    }
                                                                    StripeError::QueryStringSerialize(ser_err) => {
                                                                        eprintln!("Query string serialization error: {:?}", ser_err);
                                                                    }
                                                                    StripeError::JSONSerialize(json_err) => {
                                                                        eprintln!("JSON serialization error: {}", json_err.to_string());
                                                                    }
                                                                    StripeError::UnsupportedVersion => {
                                                                        eprintln!("Unsupported Stripe API version");
                                                                    }
                                                                    StripeError::ClientError(msg) => {
                                                                        eprintln!("Client error: {}", msg);
                                                                    }
                                                                    StripeError::Timeout => {
                                                                        eprintln!("Stripe request timed out");
                                                                    }
                                                                }
                                                                methods::standard_replys::internal_server_error_response(&new_token_in_db_publish)
                                                            }
                                                            Ok(_) => {
                                                                let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Credit card declined"});
                                                                Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::OK),))
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Payment Method unavailable"});
                                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::CONFLICT),))
                                                }
                                            }
                                            Err(_) => {
                                                let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Payment Method invalid"});
                                                Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST),))
                                            }
                                        }
                                    } else {
                                        let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Vehicle unavailable"});
                                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::CONFLICT),))
                                    }
                                }
                                Err(_) => {
                                    let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "Vehicle invalid"});
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST),))
                                }
                            }
                        } else {
                            let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "User on do not rent list"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN),))
                        }
                    }
                }
            }
        })
}