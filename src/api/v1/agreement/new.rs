use crate::{POOL, integration, methods, model, proj_config, helper_model};
use chrono::{DateTime, Duration, Utc};
use diesel::associations::HasTable;
use diesel::RunQueryDsl;
use diesel::prelude::*;
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
                    // RETURN: BAD_REQUEST
                    return methods::standard_replies::bad_request("Time is invalid")
                }
                let mut pool = POOL.get().unwrap();
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_wrapped_return();
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_wrapped_return();
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                if if_token_valid_result.is_err() {
                    return methods::tokens::token_not_hex_warp_return();
                }
                let token_bool = if_token_valid_result.unwrap();
                if !token_bool {
                    // RETURN: UNAUTHORIZED
                    methods::tokens::token_invalid_wrapped_return()
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
                    let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                        .values(&new_token)
                        .get_result::<model::AccessToken>(&mut pool)
                        .unwrap()
                        .into();

                    let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();

                    // Check if Renter DL exp
                    let return_date = body.end_time.naive_utc().date();

                    if user_in_request.drivers_license_image.is_none() {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Your driver's licence is not uploaded. Please submit your driver's licence. "),
                        };
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),));
                    } else if user_in_request.drivers_license_image_secondary.is_none() && user_in_request.requires_secondary_driver_lic {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Your secondary driver's licence is not uploaded. Please submit your secondary driver's licence. "),
                        };
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),));
                    } else if user_in_request.drivers_license_expiration.is_none() {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Your driver's licences are pending verification. If you are still encountering this issue, please reach out to us. "),
                        };
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),));
                    } else if user_in_request.drivers_license_expiration.unwrap() <= return_date {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Your driver's licences expires before trip ends. Please re-submit your driver's licence. "),
                        };
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }

                    // Check if Renter lease exp
                    use crate::schema::apartments::dsl as apartment_query;
                    let renter_apt: model::Apartment = apartment_query::apartments
                        .filter(apartment_query::id.eq(&user_in_request.apartment_id))
                        .get_result(&mut pool).unwrap();

                    if renter_apt.uni_id != 1 && user_in_request.lease_agreement_expiration.is_none() {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Your lease agreement is not verified. Please submit your lease agreement. "),
                        };
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }
                    if renter_apt.uni_id != 1 && user_in_request.lease_agreement_expiration.unwrap() <= return_date {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Your lease agreement expires before trip ends. Please re-submit your lease agreement. "),
                        };
                        return Ok::<_, warp::Rejection>((
                            methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),
                        ));
                    }

                    // Check if Renter has an address
                    let Some(billing_address) = user_in_request.clone().billing_address else {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Your billing address is not verified. Please submit your driver's licence or lease agreement. "),
                        };
                        // RETURN: NOT_ACCEPTABLE
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),));
                    };

                    let dnr_records = methods::user::get_dnr_record_for(&user_in_request);
                    if let Some(records) = dnr_records && !records.is_empty() {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Do Not Rent Record Found"),
                            message: String::from("We found one or more dnr records, please contact us to resolve this! "),
                        };
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN)),));
                    }
                    
                    use crate::schema::agreements::dsl as agreements_query;

                    let renter_agreements_blocking_count = agreements_query::agreements
                        .filter(agreements_query::renter_id.eq(&access_token.user_id))
                        .filter(agreements_query::status.eq(model::AgreementStatus::Rental))
                        .filter(
                            methods::diesel_fn::coalesce(agreements_query::actual_pickup_time, agreements_query::rsvp_pickup_time)
                                .lt(body.end_time + Duration::minutes(15))
                                .and(
                                    methods::diesel_fn::coalesce(
                                        agreements_query::actual_drop_off_time,
                                        methods::diesel_fn::greatest(agreements_query::rsvp_drop_off_time, diesel::dsl::now)
                                    )
                                        .gt(body.start_time - Duration::minutes(15))
                                )
                        )
                        .count()
                        .get_result(&mut pool).unwrap_or(0);

                    if renter_agreements_blocking_count > 0 {
                        return methods::standard_replies::double_booking_not_allowed_wrapped(new_token_in_db_publish.clone())
                    }

                    use crate::schema::vehicles::dsl as vehicle_query;
                    use crate::schema::locations::dsl as location_query;
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
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Not Allowed"),
                            message: String::from("Booking this vehicle is currently not allowed. Please try again later. "),
                        };
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::CONFLICT)),));
                    }
                    let vehicle_with_location: (model::Vehicle, model::Location, model::Apartment) = vehicle_result.unwrap();

                    if vehicle_with_location.2.id <= 1 {
                        // RETURN: FORBIDDEN
                        return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), vehicle_with_location.2.id);
                    }

                    if vehicle_with_location.2.uni_id != 1 && user_in_request.employee_tier != model::EmployeeTier::Admin && user_in_request.apartment_id != vehicle_with_location.2.id {
                        // RETURN: FORBIDDEN
                        return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), vehicle_with_location.2.id);
                    }

                    if vehicle_with_location.2.liability_protection_rate.is_none() {
                        body.liability = false;
                    }
                    if vehicle_with_location.2.pcdw_protection_rate.is_none() {
                        body.pcdw = false;
                    }
                    if vehicle_with_location.2.pcdw_ext_protection_rate.is_none() {
                        body.pcdw_ext = false;
                    }
                    if vehicle_with_location.2.rsa_protection_rate.is_none() {
                        body.rsa = false;
                    }
                    if vehicle_with_location.2.pai_protection_rate.is_none() {
                        body.pai = false;
                    }

                    use crate::schema::payment_methods::dsl as payment_method_query;
                    let pm_result = payment_method_query::payment_methods
                        .filter(payment_method_query::id.eq(&body.payment_id))
                        .filter(payment_method_query::renter_id.eq(&user_in_request.id))
                        .filter(payment_method_query::is_enabled)
                        .get_result::<model::PaymentMethod>(&mut pool);

                    if pm_result.is_err() {
                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Booking Failed"),
                            message: String::from("The credit card you used to book is invalid. "),
                        };
                        return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE)),));
                    }
                    let payment_method = pm_result.unwrap();

                    let start_time_buffered = body.start_time - Duration::minutes(proj_config::RSVP_BUFFER);
                    let end_time_buffered = body.end_time + Duration::minutes(proj_config::RSVP_BUFFER);

                    use crate::schema::apartments_taxes::dsl as apartments_taxes_query;

                    let tax_ids: Vec<i32> = apartments_taxes_query::apartments_taxes::table().select(apartments_taxes_query::tax_id)
                        .filter(apartments_taxes_query::apartment_id.eq(&vehicle_with_location.2.id))
                        .get_results::<i32>(&mut pool)
                        .unwrap_or_default();

                    use crate::schema::taxes::dsl as tax_query;
                    let tax_objs = tax_query::taxes
                        .filter(tax_query::id.eq_any(tax_ids))
                        .filter(tax_query::is_effective)
                        .get_results::<model::Tax>(&mut pool)
                        .unwrap_or_default();

                    let mut local_tax_rate_multiplier = 0.00;
                    let mut _local_tax_rate_daily = 0.0;
                    let mut local_tax_rate_fixed = 0.0;
                    let mut local_tax_id: Vec<Option<i32>> = Vec::new();
                    for tax_obj in tax_objs {
                        match tax_obj.tax_type {
                            model::TaxType::Percent => {
                                local_tax_rate_multiplier += tax_obj.multiplier;
                            },
                            model::TaxType::Daily => {
                                _local_tax_rate_daily += tax_obj.multiplier;
                            }
                            model::TaxType::Fixed => {
                                local_tax_rate_fixed += tax_obj.multiplier;
                            }
                        }
                        (&mut local_tax_id).push(Some(tax_obj.id));
                    }

                    let conf_id = methods::agreement::generate_unique_agreement_confirmation();
                    let deposit_amount = vehicle_with_location.2.duration_rate * vehicle_with_location.0.msrp_factor * (1.00 + local_tax_rate_multiplier)
                        + local_tax_rate_fixed;
                    let deposit_amount_in_int = (deposit_amount * 100.0).round() as i64;
                    let description = &("Hold for Veygo Reservation #".to_owned() + &*conf_id.clone());
                    let suffix: Option<&str> = Some(&*("DEPOSIT #".to_owned() + &*conf_id.clone()));
                    let stripe_auth = integration::stripe_veygo::create_payment_intent(description, &user_in_request.stripe_id.unwrap(), &payment_method.token, &deposit_amount_in_int, PaymentIntentCaptureMethod::Manual, suffix).await;

                    match stripe_auth {
                        Err(error) => {
                            if let StripeError::Stripe(request_error) = error {
                                eprintln!("Stripe API error: {:?}", request_error);
                                if request_error.code == Some(ErrorCode::CardDeclined) {
                                    return methods::standard_replies::card_declined_wrapped(new_token_in_db_publish);
                                } else if request_error.error_type == InvalidRequest {
                                    let err_msg = helper_model::ErrorResponse {
                                        title: "Unable To Book".to_string(),
                                        message: "System error, please contact us. ".to_string(),
                                    };
                                    return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::INTERNAL_SERVER_ERROR)),));
                                }
                            }
                            let err_msg = helper_model::ErrorResponse {
                                title: "Unable To Book".to_string(),
                                message: "System error, please contact us. ".to_string(),
                            };
                            return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::INTERNAL_SERVER_ERROR)),));
                        }
                        Ok(pmi) => {
                            use crate::schema::agreements::dsl as ag_q;
                            let is_conflict = diesel::select(diesel::dsl::exists(
                                ag_q::agreements
                                    .into_boxed()
                                    .filter(ag_q::status.eq(model::AgreementStatus::Rental))
                                    .filter(ag_q::vehicle_id.eq(&body.vehicle_id))
                                    .filter(
                                        methods::diesel_fn::coalesce(ag_q::actual_pickup_time, ag_q::rsvp_pickup_time)
                                            .lt(end_time_buffered)
                                            .and(
                                                methods::diesel_fn::coalesce(
                                                    ag_q::actual_drop_off_time,
                                                    methods::diesel_fn::greatest(ag_q::rsvp_drop_off_time, diesel::dsl::now)
                                                ).gt(start_time_buffered)
                                            )
                                    )
                            )).get_result::<bool>(&mut pool).unwrap();

                            if is_conflict {
                                let _ = integration::stripe_veygo::drop_auth(&pmi).await;
                                let err_msg = helper_model::ErrorResponse {
                                    title: "Vehicle Unavailable".to_string(),
                                    message: "Please try again later. ".to_string(),
                                };
                                return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::INTERNAL_SERVER_ERROR)),));
                            }

                            let new_agreement = model::NewAgreement {
                                confirmation: conf_id,
                                status: model::AgreementStatus::Rental,
                                user_name: user_in_request.name.clone(),
                                user_date_of_birth: user_in_request.date_of_birth.clone(),
                                user_email: user_in_request.student_email.clone(),
                                user_phone: user_in_request.phone.clone(),
                                user_billing_address: billing_address,
                                rsvp_pickup_time: body.start_time,
                                rsvp_drop_off_time: body.end_time,
                                liability_protection_rate: if body.liability { vehicle_with_location.2.liability_protection_rate } else { None },
                                pcdw_protection_rate: if body.pcdw && vehicle_with_location.2.pcdw_protection_rate.is_some() { Some(vehicle_with_location.2.pcdw_protection_rate.unwrap() * vehicle_with_location.0.msrp_factor) } else { None },
                                pcdw_ext_protection_rate: if body.pcdw_ext && vehicle_with_location.2.pcdw_ext_protection_rate.is_some() { Some(vehicle_with_location.2.pcdw_ext_protection_rate.unwrap() * vehicle_with_location.0.msrp_factor) } else { None },
                                rsa_protection_rate: if body.rsa { vehicle_with_location.2.rsa_protection_rate } else { None },
                                pai_protection_rate: if body.pai { vehicle_with_location.2.pai_protection_rate } else { None },
                                msrp_factor: vehicle_with_location.0.msrp_factor,
                                duration_rate: vehicle_with_location.2.duration_rate * vehicle_with_location.0.msrp_factor,
                                vehicle_id: vehicle_with_location.0.id,
                                renter_id: user_in_request.id,
                                payment_method_id: body.payment_id,
                                promo_id: None,
                                manual_discount: None,
                                location_id: vehicle_with_location.1.id,
                                mileage_package_id: None,
                                mileage_conversion: vehicle_with_location.2.mileage_conversion,
                                mileage_rate_overwrite: None,
                                mileage_package_overwrite: None
                            };

                            let new_publish_agreement_result = diesel::insert_into(ag_q::agreements).values(&new_agreement).get_result::<model::Agreement>(&mut pool);
                            if new_publish_agreement_result.is_err() {
                                let _ = integration::stripe_veygo::drop_auth(&pmi).await;
                                let err_msg = helper_model::ErrorResponse {
                                    title: "Unable To Book".to_string(),
                                    message: "System error, please contact us. ".to_string(),
                                };
                                return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg),
                                                                                                                                                  StatusCode::INTERNAL_SERVER_ERROR)),));
                            }
                            let new_publish_agreement = new_publish_agreement_result.unwrap();
                            use crate::schema::payments::dsl as payment_query;
                            let new_payment = model::NewPayment {
                                payment_type: model::PaymentType::from_stripe_payment_intent_status(pmi.status),
                                amount: 0.00,
                                note: Some("Non refundable deposit".to_string()),
                                reference_number: Some(pmi.id.to_string()),
                                agreement_id: Some(new_publish_agreement.id.clone()),
                                renter_id: user_in_request.id,
                                payment_method_id: payment_method.id,
                                amount_authorized: Option::from(deposit_amount),
                                capture_before: Option::from(methods::timestamps::from_seconds(pmi.clone().latest_charge.unwrap().into_object().unwrap().payment_method_details.unwrap().card.unwrap().capture_before.unwrap())),
                                is_deposit: true,
                            };
                            let payment_result = diesel::insert_into(payment_query::payments).values(&new_payment).get_result::<model::Payment>(&mut pool);
                            if payment_result.is_err() {
                                let _ = integration::stripe_veygo::drop_auth(&pmi).await;
                                let _ = diesel::delete(ag_q::agreements).filter(ag_q::id.eq(&new_publish_agreement.id)).execute(&mut pool);
                                let err_msg = helper_model::ErrorResponse {
                                    title: "Unable To Book".to_string(),
                                    message: "System error, please contact us. ".to_string(),
                                };
                                return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg),
                                                                                                                                                  StatusCode::INTERNAL_SERVER_ERROR)),));
                            }
                            let msg = serde_json::json!({"agreement": &new_publish_agreement});
                            Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&msg), StatusCode::OK)),))
                        }
                    }
                }
            },
        )
}
