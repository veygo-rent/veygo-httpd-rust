use crate::{POOL, integration, methods, model};
use chrono::{DateTime, Duration, Utc};
use diesel::RunQueryDsl;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
use serde_derive::{Deserialize, Serialize};
use stripe::ErrorType::InvalidRequest;
use stripe::{ErrorCode, PaymentIntentCaptureMethod, StripeError};
use warp::http::StatusCode;
use warp::{Filter, Reply};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct NewAgreementRequestBodyData {
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

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("new")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(async move |body: NewAgreementRequestBodyData, auth: String, client_type: Option<String>| {
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
            let if_token_valid = methods::tokens::verify_user_token(access_token.user_id.clone(), access_token.token.clone()).await;
            match if_token_valid {
                Err(_) => {
                    methods::tokens::token_not_hex_warp_return(&access_token.token)
                }
                Ok(token_bool) => {
                    if !token_bool {
                        methods::tokens::token_invalid_wrapped_return(&access_token.token)
                    } else {
                        // Token is valid, generate new publish token, user_id valid
                        methods::tokens::rm_token_by_binary(hex::decode(access_token.token).unwrap()).await;
                        let new_token = methods::tokens::gen_token_object(access_token.user_id.clone(), client_type.clone()).await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.clone().get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<model::AccessToken>(&mut pool).unwrap().to_publish_access_token();
                        let user_in_request = methods::user::get_user_by_id(access_token.user_id).await.unwrap();
                        // Check if Renter has an address
                        if user_in_request.billing_address.is_none() {
                            let error_msg = serde_json::json!({"error": "Unknown billing address"});
                            return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                        }
                        let user_address = user_in_request.billing_address.clone().unwrap();
                        // Check if Renter DL exp
                        if user_in_request.drivers_license_expiration.is_none() {
                            let error_msg = serde_json::json!({"error": "Drivers license not verified"});
                            return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                        }
                        let user_dl_expiration = user_in_request.drivers_license_expiration.unwrap();
                        let return_date = body.end_time.naive_utc().date();
                        if user_dl_expiration < return_date {
                            let error_msg = serde_json::json!({
                                "error": "Drivers license expired before return"
                            });
                            return Ok::<_, warp::Rejection>((
                                warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE).into_response(),
                            ));
                        }
                        // Check if Renter lease exp
                        if user_in_request.lease_agreement_expiration.is_none() {
                            let error_msg = serde_json::json!({"error": "Lease agreement not verified"});
                            return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                        }
                        let user_lease_expiration = user_in_request.lease_agreement_expiration.unwrap();
                        if user_lease_expiration < return_date {
                            let error_msg = serde_json::json!({
                                "error": "Lease agreement expired before return"
                            });
                            return Ok::<_, warp::Rejection>((
                                methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),
                            ));
                        }
                        // Check if liability is covered (liability & collision)
                        // liability
                        // TODO: Add apartment liability availability check
                        if user_in_request.insurance_liability_expiration.is_none() && !body.liability {
                            let error_msg = serde_json::json!({"error": "Liability coverage required"});
                            return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                        }
                        let user_liability_expiration = user_in_request.insurance_liability_expiration.unwrap();
                        if user_liability_expiration < return_date && !body.liability {
                            let error_msg = serde_json::json!({
                                "error": "Liability coverage expired before return"
                            });
                            return Ok::<_, warp::Rejection>((
                                methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),
                            ));
                        }
                        // collision
                        // TODO: Add credit card collision verification
                        if user_in_request.insurance_collision_expiration.is_none() && !body.pcdw {
                            let error_msg = serde_json::json!({"error": "Collision coverage required"});
                            return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                        }
                        let user_collision_expiration = user_in_request.insurance_collision_expiration.unwrap();
                        if user_collision_expiration < return_date && !body.pcdw {
                            let error_msg = serde_json::json!({
                                "error": "Collision coverage expired before return"
                            });
                            return Ok::<_, warp::Rejection>((
                                methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),
                            ));
                        }
                        // Check if renter in DNR
                        let if_in_dnr = methods::user::check_if_on_do_not_rent(&user_in_request).await;
                        if !if_in_dnr {
                            if body.start_time + Duration::hours(1) > body.end_time || body.start_time - Duration::hours(1) < Utc::now() {
                                let error_msg = serde_json::json!({"error": "Time invalid"});
                                return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                            }
                            // check vehicle::exist, vehicle.available, vehicle.apt_id == renter.apt_id => invalid_vehicle
                            use crate::schema::vehicles::dsl::*;
                            let renter_clone = user_in_request.clone();
                            let mut pool = POOL.clone().get().unwrap();
                            let vehicle_result = vehicles.filter(id.eq(&body.vehicle_id)).get_result::<crate::model::Vehicle>(&mut pool);
                            match vehicle_result {
                                Ok(vehicle) => {
                                    if vehicle.available && vehicle.apartment_id == renter_clone.id {
                                        // Vehicle checked, check pm::exist, if pm.is_enabled, pm.renter_id == user.id => invalid_payment_method
                                        use crate::schema::payment_methods::dsl::*;
                                        let mut pool = POOL.clone().get().unwrap();
                                        let pm_result = payment_methods.filter(id.eq(&body.payment_id)).get_result::<crate::model::PaymentMethod>(&mut pool);
                                        match pm_result {
                                            Ok(pm) => {
                                                if pm.is_enabled && pm.renter_id == user_in_request.id {
                                                    // vehicle and payment method are valid, check if time has any conflicts
                                                    let start_time_buffered = body.start_time - Duration::minutes(15);
                                                    let end_time_buffered = body.end_time + Duration::minutes(15);

                                                    let mut pool = POOL.clone().get().unwrap();
                                                    use crate::schema::agreements::dsl::*;
                                                    use diesel::dsl::sql;
                                                    let if_conflict = diesel::select(diesel::dsl::exists(
                                                        agreements
                                                            .into_boxed()
                                                            .filter(status.eq(crate::model::AgreementStatus::Rental))
                                                            .filter(vehicle_id.eq(&body.vehicle_id))
                                                            .filter(sql::<Bool>("COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                                                .bind::<Timestamptz, _>(start_time_buffered)
                                                                .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                                                .bind::<Timestamptz, _>(start_time_buffered)
                                                                .sql(" OR COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                                                .bind::<Timestamptz, _>(end_time_buffered)
                                                                .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                                                .bind::<Timestamptz, _>(end_time_buffered))
                                                    )).get_result::<bool>(&mut pool).unwrap();
                                                    if if_conflict {
                                                        let error_msg = serde_json::json!({"error": "Vehicle unavailable for the requested time"});
                                                        Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::CONFLICT)),))
                                                    } else {
                                                        let mut pool = POOL.clone().get().unwrap();
                                                        let apartment_id_clone = vehicle.apartment_id.clone();
                                                        use crate::schema::apartments::dsl::*;
                                                        let apt = apartments.into_boxed().filter(id.eq(apartment_id_clone)).get_result::<crate::model::Apartment>(&mut pool).unwrap();
                                                        let conf_id = methods::agreement::generate_unique_agreement_confirmation();
                                                        let new_agreement = crate::model::NewAgreement {
                                                            confirmation: conf_id,
                                                            status: crate::model::AgreementStatus::Rental,
                                                            user_name: renter_clone.name.clone(),
                                                            user_date_of_birth: renter_clone.date_of_birth.clone(),
                                                            user_email: renter_clone.student_email.clone(),
                                                            user_phone: renter_clone.phone.clone(),
                                                            user_billing_address: user_address,
                                                            rsvp_pickup_time: body.start_time,
                                                            rsvp_drop_off_time: body.end_time,
                                                            liability_protection_rate: if body.liability { apt.liability_protection_rate } else { 0.00 },
                                                            pcdw_protection_rate: if body.pcdw { apt.pcdw_protection_rate * vehicle.msrp_factor } else { 0.00 },
                                                            pcdw_ext_protection_rate: if body.pcdw_ext { apt.pcdw_ext_protection_rate * vehicle.msrp_factor } else { 0.00 },
                                                            rsa_protection_rate: if body.rsa { apt.rsa_protection_rate } else { 0.00 },
                                                            pai_protection_rate: if body.pai { apt.pai_protection_rate } else { 0.00 },
                                                            tax_rate: apt.sales_tax_rate,
                                                            msrp_factor: vehicle.msrp_factor,
                                                            duration_rate: apt.duration_rate * vehicle.msrp_factor,
                                                            apartment_id: vehicle.apartment_id,
                                                            vehicle_id: vehicle.id,
                                                            renter_id: renter_clone.id,
                                                            payment_method_id: body.payment_id,
                                                        };
                                                        let deposit_amount = new_agreement.duration_rate * (1.00 + apt.sales_tax_rate);
                                                        let deposit_amount_in_int = (deposit_amount * 100.0).round() as i64;
                                                        let stripe_auth = integration::stripe_veygo::create_payment_intent(
                                                            "Veygo Reservation #".to_owned() + &*new_agreement.confirmation.clone(), user_in_request.stripe_id.unwrap(), pm.token.clone(), deposit_amount_in_int, PaymentIntentCaptureMethod::Manual
                                                        ).await;
                                                        match stripe_auth {
                                                            Err(error) => {
                                                                if let StripeError::Stripe(request_error) = error {
                                                                    eprintln!("Stripe API error: {:?}", request_error);
                                                                    if request_error.code == Some(ErrorCode::CardDeclined) {
                                                                        return methods::standard_replies::card_declined_wrapped(new_token_in_db_publish);
                                                                    } else if request_error.error_type == InvalidRequest {
                                                                        let error_msg = serde_json::json!({"error": "Payment Methods token invalid"});
                                                                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                                                                    }
                                                                }
                                                                methods::standard_replies::internal_server_error_response()
                                                            }
                                                            Ok(pmi) => {
                                                                use crate::schema::agreements::dsl::*;
                                                                let mut pool = POOL.clone().get().unwrap();
                                                                let new_publish_agreement = diesel::insert_into(agreements).values(&new_agreement).get_result::<crate::model::Agreement>(&mut pool).unwrap();

                                                                let new_payment = crate::model::NewPayment {
                                                                    payment_type: crate::model::PaymentType::from_stripe_payment_intent_status(pmi.status),
                                                                    amount: 0.00,
                                                                    note: Some("Non refundable deposit".to_string()),
                                                                    reference_number: Some(pmi.id.to_string()),
                                                                    agreement_id: Some(new_publish_agreement.id.clone()),
                                                                    renter_id: new_publish_agreement.renter_id,
                                                                    payment_method_id: pm.id,
                                                                    amount_authorized: Option::from(deposit_amount),
                                                                    capture_before: Option::from(methods::timestamps::from_seconds(pmi.latest_charge.unwrap().into_object().unwrap().payment_method_details.unwrap().card.unwrap().capture_before.unwrap())),
                                                                };
                                                                use crate::schema::payments::dsl::*;
                                                                let _saved_payment = diesel::insert_into(payments).values(&new_payment).get_result::<crate::model::Payment>(&mut pool).unwrap();
                                                                let error_msg = serde_json::json!({"agreement": &new_publish_agreement});
                                                                Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::OK)),))
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    methods::standard_replies::card_invalid_wrapped(new_token_in_db_publish)
                                                }
                                            }
                                            Err(_) => {
                                                methods::standard_replies::card_invalid_wrapped(new_token_in_db_publish)
                                            }
                                        }
                                    } else {
                                        let error_msg = serde_json::json!({"error": "Vehicle unavailable"});
                                        Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                                    }
                                }
                                Err(_) => {
                                    let error_msg = serde_json::json!({"error": "Vehicle invalid"});
                                    Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                                }
                            }
                        } else {
                            let error_msg = serde_json::json!({"error": "User on do not rent list"});
                            Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                        }
                    }
                }
            }
        })
}
