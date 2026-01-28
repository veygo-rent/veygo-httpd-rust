use crate::{POOL, integration, methods, model, proj_config, helper_model};
use chrono::{DateTime, Duration, Utc};
use diesel::RunQueryDsl;
use diesel::prelude::*;
use diesel::result::Error;
use serde_derive::{Deserialize, Serialize};
use stripe_core::{PaymentIntentCaptureMethod};
use warp::http::{Method, StatusCode};
use warp::{Filter, Reply};
use rust_decimal::prelude::*;

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
    rate_offer_id: Option<i32>,
    mileage_package_id: Option<i32>,
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
                    return methods::tokens::token_invalid_return();
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_return();
                    }
                };

                let access_token = model::RequestToken {
                    user_id,
                    token: String::from(token_and_id[0]),
                };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;

                return match if_token_valid_result {
                    Err(e) => {
                        match e {
                            helper_model::VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            helper_model::VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(
                                    "agreement/new: Token verification unexpected error",
                                )
                                .await
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);
                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response(
                                        "agreement/new: Token extension failed (returned false)",
                                    )
                                    .await;
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/new: Token extension error",
                                )
                                .await;
                            }
                        }

                        let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await;
                        let Ok(user_in_request) = user_in_request else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/new: Database error loading renter",
                            )
                            .await
                        };

                        // Check if Renter DL exp
                        let return_date = body.end_time.naive_utc().date();

                        if user_in_request.drivers_license_image.is_none() {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your driver's licence is not uploaded. Please submit your driver's licence. "),
                            };
                            // RETURN: NOT_ACCEPTABLE
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        } else if user_in_request.drivers_license_image_secondary.is_none() && user_in_request.requires_secondary_driver_lic {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your secondary driver's licence is not uploaded. Please submit your secondary driver's licence. "),
                            };
                            // RETURN: NOT_ACCEPTABLE
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        } else if user_in_request.drivers_license_expiration.is_none() {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your driver's licences are pending verification. If you are still encountering this issue, please reach out to us. "),
                            };
                            // RETURN: NOT_ACCEPTABLE
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        } else if user_in_request.drivers_license_expiration.unwrap() <= return_date {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your driver's licences expires before trip ends. Please re-submit your driver's licence. "),
                            };
                            // RETURN: NOT_ACCEPTABLE
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        }


                        // Check if Renter lease exp
                        use crate::schema::apartments::dsl as apartment_query;
                        let renter_apt = apartment_query::apartments
                            .find(&user_in_request.apartment_id)
                            .get_result::<model::Apartment>(&mut pool);

                        let Ok(renter_apt) = renter_apt else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/new: Database error loading renter apartment",
                            )
                            .await
                        };

                        if renter_apt.uni_id != 1 && user_in_request.lease_agreement_expiration.is_none() {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your lease agreement is not verified. Please submit your lease agreement. "),
                            };
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        }
                        if renter_apt.uni_id != 1 && user_in_request.lease_agreement_expiration.unwrap() <= return_date {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your lease agreement expires before trip ends. Please re-submit your lease agreement. "),
                            };
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        }

                        // Check if Renter has an address
                        let Some(billing_address) = user_in_request.clone().billing_address else {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your billing address is not verified. Please submit your driver's licence or lease agreement. "),
                            };
                            // RETURN: NOT_ACCEPTABLE
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        };

                        let dnr_records = user_in_request.get_dnr_count();
                        let Ok(record_count) = dnr_records else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/new: Database error checking DNR records",
                            )
                            .await
                        };
                        if record_count > 0 {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Do Not Rent Record Found"),
                                message: String::from("We found one or more dnr records, please contact us to resolve this! "),
                            };
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        }

                        use crate::schema::agreements::dsl as agreements_query;

                        let renter_agreements_blocking_count = agreements_query::agreements
                            .filter(agreements_query::renter_id.eq(&access_token.user_id))
                            .filter(agreements_query::status.eq(model::AgreementStatus::Rental))
                            .filter(
                                methods::diesel_fn::coalesce(agreements_query::actual_pickup_time, agreements_query::rsvp_pickup_time)
                                    .lt(body.end_time + Duration::minutes(15))
                                    .and(
                                        methods::diesel_fn::coalesce
                                            (
                                                agreements_query::actual_drop_off_time,
                                                methods::diesel_fn::greatest(agreements_query::rsvp_drop_off_time, diesel::dsl::now)
                                            )
                                            .gt(body.start_time - Duration::minutes(proj_config::RSVP_BUFFER))
                                    )
                            )
                            .count()
                            .get_result::<i64>(&mut pool);

                        let Ok(renter_agreements_blocking_count) = renter_agreements_blocking_count else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/new: Database error checking blocking agreements",
                            )
                            .await
                        };

                        if renter_agreements_blocking_count > 0 {
                            return methods::standard_replies::double_booking_not_allowed()
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

                        let vehicle_with_location = match vehicle_result {
                            Ok(result) => {
                                result
                            }
                            Err(err) => {
                                return match err {
                                    Error::NotFound => {
                                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                            title: String::from("Booking Not Allowed"),
                                            message: String::from("Booking this vehicle is currently not allowed. Please try again later. "),
                                        };
                                        methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                    }
                                    _ => {
                                        methods::standard_replies::double_booking_not_allowed()
                                    }
                                }
                            }
                        };

                        if vehicle_with_location.2.id <= 1 {
                            // RETURN: FORBIDDEN
                            return methods::standard_replies::apartment_not_allowed_response(vehicle_with_location.2.id);
                        }

                        if vehicle_with_location.2.uni_id != 1 && !user_in_request.is_operational_admin() && user_in_request.apartment_id != vehicle_with_location.2.id {
                            // RETURN: FORBIDDEN
                            return methods::standard_replies::apartment_not_allowed_response(vehicle_with_location.2.id);
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

                        let payment_method = match pm_result {
                            Ok(result) => {
                                result
                            }
                            Err(err) => {
                                return match err {
                                    Error::NotFound => {
                                        let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                            title: String::from("Booking Failed"),
                                            message: String::from("The credit card you used to book is invalid. "),
                                        };
                                        methods::standard_replies::response_with_obj(err_msg, StatusCode::PAYMENT_REQUIRED)
                                    }
                                    _ => {
                                        methods::standard_replies::double_booking_not_allowed()
                                    }
                                }
                            }
                        };

                        let start_time_buffered = body.start_time - Duration::minutes(proj_config::RSVP_BUFFER);
                        let end_time_buffered = body.end_time + Duration::minutes(proj_config::RSVP_BUFFER);

                        use crate::schema::apartments_taxes::dsl as apartments_taxes_query;
                        use crate::schema::taxes::dsl as t_q;

                        let taxes = apartments_taxes_query::apartments_taxes
                            .inner_join(t_q::taxes)
                            .filter(apartments_taxes_query::apartment_id.eq(&vehicle_with_location.2.id))
                            .select(t_q::taxes::all_columns())
                            .get_results::<model::Tax>(&mut pool);

                        let Ok(taxes) = taxes else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/new: Database error loading apartment taxes",
                            )
                            .await
                        };

                        let mut local_tax_rate_percent = Decimal::one();
                        let mut local_tax_rate_daily = Decimal::zero();
                        let mut local_tax_rate_fixed = Decimal::zero();

                        for tax_obj in &taxes {
                            match tax_obj.tax_type {
                                model::TaxType::Percent => {
                                    local_tax_rate_percent += tax_obj.multiplier;
                                },
                                model::TaxType::Daily => {
                                    local_tax_rate_daily += tax_obj.multiplier;
                                }
                                model::TaxType::Fixed => {
                                    local_tax_rate_fixed += tax_obj.multiplier;
                                }
                            }
                        }

                        let conf_id = methods::agreement::generate_unique_agreement_confirmation();
                        let Ok(conf_id) = conf_id else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/new: Failed to generate agreement confirmation",
                            )
                            .await
                        };

                        let deposit_amount_2dp = (vehicle_with_location.2.duration_rate * vehicle_with_location.0.msrp_factor * local_tax_rate_percent
                            + local_tax_rate_fixed + local_tax_rate_daily).round_dp(2);
                        let deposit_amount_in_int = deposit_amount_2dp.mantissa() as i64;
                        let description = "HOLD #".to_owned() + &*conf_id.clone();

                        let stripe_auth = integration::stripe_veygo::create_payment_intent(
                            &user_in_request.stripe_id, &payment_method.token, deposit_amount_in_int, PaymentIntentCaptureMethod::Manual, &description
                        ).await;

                        match stripe_auth {
                            Err(error) => {
                                match error {
                                    helper_model::VeygoError::CardDeclined => {
                                        methods::standard_replies::card_declined()
                                    }
                                    _ => {
                                        methods::standard_replies::internal_server_error_response(
                                            "agreement/new: Stripe error creating payment intent",
                                        )
                                        .await
                                    }
                                }
                            }
                            Ok(pmi) => {
                                use crate::schema::agreements::dsl as ag_q;
                                let is_conflict = diesel::select(diesel::dsl::exists(
                                    ag_q::agreements
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
                                )).get_result::<bool>(&mut pool);
                                let Ok(is_conflict) = is_conflict else {
                                    return methods::standard_replies::internal_server_error_response(
                                        "agreement/new: Database error checking vehicle conflict",
                                    )
                                    .await
                                };

                                if is_conflict {
                                    let _ = integration::stripe_veygo::drop_auth(&pmi.id).await;
                                    let err_msg = helper_model::ErrorResponse {
                                        title: "Vehicle Unavailable".to_string(),
                                        message: "Please try again later. ".to_string(),
                                    };
                                    return methods::standard_replies::response_with_obj(err_msg, StatusCode::CONFLICT)
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
                                    mileage_package_id: body.mileage_package_id,
                                    mileage_conversion: vehicle_with_location.2.mileage_conversion,
                                    mileage_rate_overwrite: vehicle_with_location.2.mileage_rate_overwrite,
                                    mileage_package_overwrite: vehicle_with_location.2.mileage_package_overwrite,
                                };

                                let new_publish_agreement_result = diesel::insert_into(ag_q::agreements)
                                    .values(&new_agreement)
                                    .get_result::<model::Agreement>(&mut pool);

                                let new_publish_agreement = match new_publish_agreement_result {
                                    Ok(result) => {
                                        result
                                    }
                                    Err(err) => {
                                        println!("{:?}", err);
                                        let _ = integration::stripe_veygo::drop_auth(&pmi.id).await;
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/new: SQL error inserting agreement",
                                        )
                                        .await;
                                    }
                                };


                                use crate::schema::payments::dsl as payment_query;
                                let new_payment = model::NewPayment {
                                    payment_type: pmi.clone().status.into(),
                                    amount: Decimal::zero(),
                                    note: Some("Non refundable deposit".to_string()),
                                    reference_number: Some(pmi.id.to_string()),
                                    agreement_id: new_publish_agreement.id,
                                    renter_id: user_in_request.id,
                                    payment_method_id: Some(payment_method.id),
                                    amount_authorized: Decimal::new(pmi.amount, 2),
                                    capture_before: Option::from(methods::timestamps::from_seconds(pmi.clone().latest_charge.unwrap().into_object().unwrap().payment_method_details.unwrap().card.unwrap().capture_before.unwrap())),
                                    is_deposit: true,
                                };
                                let payment_result = diesel::insert_into(payment_query::payments).values(&new_payment).get_result::<model::Payment>(&mut pool);

                                let Ok(payment) = payment_result else {
                                    let _ = integration::stripe_veygo::drop_auth(&pmi.id).await;
                                    let _ = diesel::delete(ag_q::agreements.find(&new_publish_agreement.id)).execute(&mut pool);
                                    return methods::standard_replies::internal_server_error_response(
                                        "agreement/new: SQL error inserting deposit payment",
                                    )
                                    .await;
                                };

                                use crate::schema::agreements_taxes::dsl as ag_tx_q;
                                for tax in &taxes {
                                    let new_agreement_tax = model::AgreementTax {
                                        agreement_id: new_publish_agreement.id,
                                        tax_id: tax.id,
                                    };
                                    let result = diesel::insert_into(ag_tx_q::agreements_taxes)
                                        .values(new_agreement_tax)
                                        .execute(&mut pool);

                                    match result {
                                        Ok(count) => {
                                            if count == 0 {
                                                let _ = diesel::delete(ag_tx_q::agreements_taxes.filter(ag_tx_q::agreement_id.eq(&new_publish_agreement.id))).execute(&mut pool);
                                                let _ = diesel::delete(payment_query::payments.find(&payment.id)).execute(&mut pool);
                                                let _ = diesel::delete(ag_q::agreements.find(&new_publish_agreement.id)).execute(&mut pool);
                                                let _ = integration::stripe_veygo::drop_auth(&pmi.id).await;
                                                return methods::standard_replies::internal_server_error_response(
                                                    "agreement/new: SQL error inserting agreement tax (no rows updated)",
                                                )
                                                .await
                                            }
                                        }
                                        Err(_) => {
                                            let _ = diesel::delete(ag_tx_q::agreements_taxes.filter(ag_tx_q::agreement_id.eq(&new_publish_agreement.id))).execute(&mut pool);
                                            let _ = diesel::delete(payment_query::payments.find(&payment.id)).execute(&mut pool);
                                            let _ = diesel::delete(ag_q::agreements.find(&new_publish_agreement.id)).execute(&mut pool);
                                            let _ = integration::stripe_veygo::drop_auth(&pmi.id).await;
                                            return methods::standard_replies::internal_server_error_response(
                                                "agreement/new: Database error inserting agreement tax",
                                            )
                                            .await
                                        }
                                    }
                                }

                                methods::standard_replies::response_with_obj(new_publish_agreement, StatusCode::CREATED)
                            }
                        }
                    }
                };
            },
        )
}
