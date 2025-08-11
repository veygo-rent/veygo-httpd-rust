use crate::{POOL, integration, methods, model, proj_config};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use diesel::dsl::sql;
use diesel::RunQueryDsl;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
use serde_derive::{Deserialize, Serialize};
use stripe::ErrorType::InvalidRequest;
use stripe::{ErrorCode, PaymentIntentCaptureMethod, StripeError};
use warp::http::{Method, StatusCode};
use warp::{Filter, Reply};
use warp::reply::with_status;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct NewAgreementRequestBodyData {
    vehicle_id: i32,
    #[serde(with = "chrono::serde::ts_seconds")]
    start_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
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
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        mut body: NewAgreementRequestBodyData,
                        auth: String,
                        user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }
                let now = Utc::now();
                if body.start_time < now || body.end_time < now || body.start_time + Duration::minutes(proj_config::RSVP_BUFFER) > body.end_time {
                    let error_msg = serde_json::json!({"start_time": &body.start_time, "end_time": &body.end_time, "error": "Time is in valid"});
                    // RETURN: BAD_REQUEST
                    return Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST).into_response(),))
                }
                let mut pool = POOL.get().unwrap();
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_wrapped_return(&auth);
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                if if_token_valid_result.is_err() {
                    return methods::tokens::token_not_hex_warp_return(&access_token.token);
                }
                let token_bool = if_token_valid_result.unwrap();
                if !token_bool {
                    // RETURN: UNAUTHORIZED
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
                    let new_token_in_db_publish = diesel::insert_into(access_tokens)
                        .values(&new_token)
                        .get_result::<model::AccessToken>(&mut pool)
                        .unwrap()
                        .to_publish_access_token();

                    let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();
                    // Check if Renter has an address
                    if user_in_request.billing_address.is_none() {
                        let error_msg = serde_json::json!({"error": "Unknown billing address"});
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }

                    // Check if Renter DL exp
                    if user_in_request.drivers_license_expiration.is_none() {
                        let error_msg = serde_json::json!({"error": "Drivers license not verified"});
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }
                    let return_date = body.end_time.naive_utc().date();
                    if user_in_request.drivers_license_expiration.unwrap() <= return_date {
                        let error_msg = serde_json::json!({
                            "error": "Drivers license expired before return"
                        });
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((
                            with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE).into_response(),
                        ));
                    }

                    // Check if Renter lease exp
                    if user_in_request.lease_agreement_expiration.is_none() {
                        let error_msg = serde_json::json!({"error": "Lease agreement not verified"});
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }
                    if user_in_request.lease_agreement_expiration.unwrap() <= return_date {
                        let error_msg = serde_json::json!({
                            "error": "Lease agreement expired before return"
                        });
                        return Ok::<_, warp::Rejection>((
                            methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),
                        ));
                    }

                    let dnr_records = methods::user::get_dnr_record_for(&user_in_request);
                    if let Some(records) = dnr_records {
                        let error_msg = serde_json::json!({"error": "User on do not rent list", "do_not_rent_records": records});
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN)),));
                    }

                    use crate::schema::vehicles::dsl as vehicle_query;
                    use crate::schema::locations::dsl as location_query;
                    use crate::schema::apartments::dsl as apartment_query;
                    let vehicle_result = vehicle_query::vehicles
                        .inner_join(location_query::locations
                            .inner_join(apartment_query::apartments)
                        )
                        .filter(apartment_query::id.ne(1))
                        .filter(apartment_query::is_operating)
                        .filter(location_query::is_operational)
                        .filter(vehicle_query::id.eq(&body.vehicle_id))
                        .select(
                            (
                                vehicle_query::vehicles::all_columns(),
                                location_query::locations::all_columns(),
                                apartment_query::apartments::all_columns()
                            )
                        )
                        .get_result::<(model::Vehicle, model::Location, model::Apartment)>(&mut pool);

                    if vehicle_result.is_err() {
                        let error_msg = serde_json::json!({"error": "Vehicle unavailable"});
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::CONFLICT)),));
                    }
                    let vehicle_with_location = vehicle_result.unwrap();

                    if vehicle_with_location.2.id <= 1 {
                        // RETURN: FORBIDDEN
                        return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), vehicle_with_location.2.id);
                    }

                    if vehicle_with_location.2.uni_id != 1 && (user_in_request.employee_tier != model::EmployeeTier::Admin || user_in_request.apartment_id != vehicle_with_location.2.id) {
                        // RETURN: FORBIDDEN
                        return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), vehicle_with_location.2.id);
                    }

                    if vehicle_with_location.2.liability_protection_rate <= 0.0 {
                        body.liability = false;
                    }
                    if vehicle_with_location.2.pcdw_protection_rate <= 0.0 {
                        body.pcdw = false;
                    }
                    if vehicle_with_location.2.pcdw_ext_protection_rate <= 0.0 {
                        body.pcdw_ext = false;
                    }
                    if vehicle_with_location.2.rsa_protection_rate <= 0.0 {
                        body.rsa = false;
                    }
                    if vehicle_with_location.2.pai_protection_rate <= 0.0 {
                        body.pai = false;
                    }

                    use crate::schema::payment_methods::dsl as payment_method_query;
                    let pm_result = payment_method_query::payment_methods
                        .filter(payment_method_query::id.eq(&body.payment_id))
                        .filter(payment_method_query::renter_id.eq(&user_in_request.id))
                        .filter(payment_method_query::is_enabled)
                        .get_result::<model::PaymentMethod>(&mut pool);

                    if pm_result.is_err() {
                        let error_msg = serde_json::json!({"error": "Credit card is unavailable"});
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }
                    let payment_method = pm_result.unwrap();

                    // Check if liability is covered (liability & collision)
                    let requires_own = vehicle_with_location.0.requires_own_insurance;
                    // liability
                    let has_own_liability = user_in_request
                        .insurance_liability_expiration
                        .map(|d| d >= return_date)
                        .unwrap_or(false);
                    let has_lia = body.liability && !requires_own;

                    let liability_ok = if requires_own {
                        has_own_liability
                    } else {
                        has_lia || has_own_liability
                    };

                    if !liability_ok {
                        let error_msg = serde_json::json!({"error": "Liability coverage required"});
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }
                    // collision
                    let has_own_collision = user_in_request
                        .insurance_collision_expiration
                        .map(|d| d >= return_date)
                        .unwrap_or(false);
                    let has_card_cdw = payment_method.cdw_enabled; // credit-card CDW flag
                    let has_pcdw = body.pcdw && !requires_own;    // PCDW cannot satisfy if vehicle requires own insurance

                    let collision_ok = if requires_own {
                        // Vehicle requires renter's own policy to be valid through return
                        has_own_collision
                    } else {
                        // Any one of the protections suffices
                        has_pcdw || has_card_cdw || has_own_collision
                    };

                    if !collision_ok {
                        let error_msg = serde_json::json!({"error": "Collision coverage required"});
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }

                    let start_time_buffered = body.start_time - Duration::minutes(proj_config::RSVP_BUFFER);
                    let end_time_buffered = body.end_time + Duration::minutes(proj_config::RSVP_BUFFER);

                    let conf_id = methods::agreement::generate_unique_agreement_confirmation();
                    let deposit_amount = vehicle_with_location.2.duration_rate * vehicle_with_location.0.msrp_factor * (1.00 + 0.11);
                    let deposit_amount_in_int = (deposit_amount * 100.0).round() as i64;
                    let stripe_auth = integration::stripe_veygo::create_payment_intent(
                        &("Hold for Veygo Reservation #".to_owned() + &*conf_id.clone()), &user_in_request.stripe_id.unwrap(), &payment_method.token, &deposit_amount_in_int, PaymentIntentCaptureMethod::Manual,
                    ).await;

                    match stripe_auth {
                        Err(error) => {
                            if let StripeError::Stripe(request_error) = error {
                                eprintln!("Stripe API error: {:?}", request_error);
                                if request_error.code == Some(ErrorCode::CardDeclined) {
                                    return methods::standard_replies::card_declined_wrapped(new_token_in_db_publish);
                                } else if request_error.error_type == InvalidRequest {
                                    let error_msg = serde_json::json!({"error": "Payment Methods token invalid"});
                                    return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&error_msg), StatusCode::PAYMENT_REQUIRED)),));
                                }
                            }
                            return methods::standard_replies::internal_server_error_response();
                        }
                        Ok(pmi) => {
                            use crate::schema::agreements::dsl as agreement_query;
                            let if_conflict = diesel::select(diesel::dsl::exists(
                                agreement_query::agreements
                                    .into_boxed()
                                    .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                                    .filter(agreement_query::vehicle_id.eq(&body.vehicle_id))
                                    .filter(
                                        sql::<Bool>("(COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                            .bind::<Timestamptz, _>(end_time_buffered)
                                            .sql(") AND (COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                            .bind::<Timestamptz, _>(start_time_buffered)
                                            .sql(")")
                                    )
                            )).get_result::<bool>(&mut pool).unwrap();

                            if if_conflict {
                                let _ = integration::stripe_veygo::drop_auth(&pmi).await;
                                let error_msg = serde_json::json!({"error": "Vehicle unavailable"});
                                return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::CONFLICT)),));
                            }
                        }
                    }
                    methods::standard_replies::not_implemented_response()
                }


                //                                             let apartment_id_clone = vehicle.apartment_id.clone();
                //                                             use crate::schema::apartments::dsl::*;
                //                                             let apt = apartments.into_boxed().filter(id.eq(apartment_id_clone)).get_result::<crate::model::Apartment>(&mut pool).unwrap();
                //                                             let conf_id = methods::agreement::generate_unique_agreement_confirmation();
                //                                             let new_agreement = crate::model::NewAgreement {
                //                                                 confirmation: conf_id,
                //                                                 status: crate::model::AgreementStatus::Rental,
                //                                                 user_name: renter_clone.name.clone(),
                //                                                 user_date_of_birth: renter_clone.date_of_birth.clone(),
                //                                                 user_email: renter_clone.student_email.clone(),
                //                                                 user_phone: renter_clone.phone.clone(),
                //                                                 user_billing_address: user_address,
                //                                                 rsvp_pickup_time: body.start_time,
                //                                                 rsvp_drop_off_time: body.end_time,
                //                                                 liability_protection_rate: if body.liability { apt.liability_protection_rate } else { 0.00 },
                //                                                 pcdw_protection_rate: if body.pcdw { apt.pcdw_protection_rate * vehicle.msrp_factor } else { 0.00 },
                //                                                 pcdw_ext_protection_rate: if body.pcdw_ext { apt.pcdw_ext_protection_rate * vehicle.msrp_factor } else { 0.00 },
                //                                                 rsa_protection_rate: if body.rsa { apt.rsa_protection_rate } else { 0.00 },
                //                                                 pai_protection_rate: if body.pai { apt.pai_protection_rate } else { 0.00 },
                //                                                 taxes: apt.taxes,
                //                                                 msrp_factor: vehicle.msrp_factor,
                //                                                 duration_rate: apt.duration_rate * vehicle.msrp_factor,
                //                                                 apartment_id: vehicle.apartment_id,
                //                                                 vehicle_id: vehicle.id,
                //                                                 renter_id: renter_clone.id,
                //                                                 payment_method_id: body.payment_id,
                //                                                 promo_id: None,
                //                                             };
                //                                             //TODO update to real taxes
                //                                             let deposit_amount = new_agreement.duration_rate * (1.00 + 0.11);
                //                                             let deposit_amount_in_int = (deposit_amount * 100.0).round() as i64;
                //                                             let stripe_auth = integration::stripe_veygo::create_payment_intent(
                //                                                 &("Veygo Reservation #".to_owned() + &*new_agreement.confirmation.clone()), &user_in_request.stripe_id.unwrap(), &pm.token, &deposit_amount_in_int, PaymentIntentCaptureMethod::Manual,
                //                                             ).await;
                //                                             match stripe_auth {
                //                                                 Err(error) => {
                //                                                     if let StripeError::Stripe(request_error) = error {
                //                                                         eprintln!("Stripe API error: {:?}", request_error);
                //                                                         if request_error.code == Some(ErrorCode::CardDeclined) {
                //                                                             return methods::standard_replies::card_declined_wrapped(new_token_in_db_publish);
                //                                                         } else if request_error.error_type == InvalidRequest {
                //                                                             let error_msg = serde_json::json!({"error": "Payment Methods token invalid"});
                //                                                             return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),));
                //                                                         }
                //                                                     }
                //                                                     methods::standard_replies::internal_server_error_response()
                //                                                 }
                //                                                 Ok(pmi) => {
                //                                                     use crate::schema::agreements::dsl::*;
                //                                                     let new_publish_agreement = diesel::insert_into(agreements).values(&new_agreement).get_result::<crate::model::Agreement>(&mut pool).unwrap();
                //
                //                                                     let new_payment = crate::model::NewPayment {
                //                                                         payment_type: crate::model::PaymentType::from_stripe_payment_intent_status(pmi.status),
                //                                                         amount: 0.00,
                //                                                         note: Some("Non refundable deposit".to_string()),
                //                                                         reference_number: Some(pmi.id.to_string()),
                //                                                         agreement_id: Some(new_publish_agreement.id.clone()),
                //                                                         renter_id: new_publish_agreement.renter_id,
                //                                                         payment_method_id: pm.id,
                //                                                         amount_authorized: Option::from(deposit_amount),
                //                                                         capture_before: Option::from(methods::timestamps::from_seconds(pmi.latest_charge.unwrap().into_object().unwrap().payment_method_details.unwrap().card.unwrap().capture_before.unwrap())),
                //                                                     };
                //                                                     use crate::schema::payments::dsl::*;
                //                                                     let _saved_payment = diesel::insert_into(payments).values(&new_payment).get_result::<crate::model::Payment>(&mut pool).unwrap();
                //                                                     let error_msg = serde_json::json!({"agreement": &new_publish_agreement});
                //                                                     Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::OK)),))
                //                                                 }
                //                                             }
                //                                         }
                //                                     } else {
                //                                         methods::standard_replies::card_invalid_wrapped(new_token_in_db_publish)
                //                                     }
                //                                 }
                //                                 Err(_) => {
                //                                     methods::standard_replies::card_invalid_wrapped(new_token_in_db_publish)
                //                                 }
                //                             }
                //                         } else {
                //                             let error_msg = serde_json::json!({"error": "Vehicle unavailable"});
                //                             Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                //                         }
                //                     }
                //                     Err(_) => {
                //                         let error_msg = serde_json::json!({"error": "Vehicle invalid"});
                //                         Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                //                     }
                //                 }
                //             } else {
                //                 let error_msg = serde_json::json!({"error": "User on do not rent list"});
                //                 Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE)),))
                //             }
                //         }
                //     }
                // }
            },
        )
}
