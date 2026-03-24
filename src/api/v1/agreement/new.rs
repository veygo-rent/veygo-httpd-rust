use std::cmp::max;
use crate::{POOL, integration, methods, model, proj_config, helper_model, schema, helper_model::VeygoError};
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use diesel::prelude::*;
use diesel::result::Error;
use stripe_core::{PaymentIntentCaptureMethod};
use warp::{Filter, Reply, http::{Method, StatusCode}};
use rust_decimal::prelude::*;
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("new")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        body: helper_model::NewAgreementRequest,
                        auth: String,
                        user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }

                if body.hours_using_reward < Decimal::zero() {
                    return methods::standard_replies::bad_request("Invalid reward hour request")
                }

                let now = Utc::now();
                let next_quarter = {
                    let minute = now.minute();
                    let next_minute = ((minute / 15) + 1) * 15;
                    let base = now
                        .with_second(0)
                        .and_then(|dt| dt.with_nanosecond(0))
                        .unwrap();

                    if next_minute == 60 {
                        (base + Duration::hours(1)).with_minute(0).unwrap()
                    } else {
                        base.with_minute(next_minute).unwrap()
                    }
                };
                let min_start_time = next_quarter + Duration::minutes(15);
                let max_start_time = min_start_time + Duration::weeks(6);
                let trip_duration = body.end_time - body.start_time;
                let min_trip_duration = Duration::minutes(30);
                let max_trip_duration = Duration::weeks(4);

                let is_quarter_aligned = |dt: DateTime<Utc>| {
                    dt.second() == 0 && dt.nanosecond() == 0 && dt.minute() % 15 == 0
                };

                if !is_quarter_aligned(body.start_time)
                    || !is_quarter_aligned(body.end_time)
                    || body.start_time < min_start_time
                    || body.start_time > max_start_time
                    || trip_duration < min_trip_duration
                    || trip_duration > max_trip_duration
                {
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
                            VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/new: Token verification unexpected error")
                                )
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
                                        String::from("agreement/new: Token extension failed (returned false)")
                                    );
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/new: Token extension error")
                                );
                            }
                        }

                        let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await;
                        let Ok(user_in_request) = user_in_request else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/new: Database error loading renter")
                            )
                        };

                        let current_time = Utc::now();
                        let current_naive_date = chrono::NaiveDate::from_ymd_opt(current_time.year(), current_time.month(), current_time.day()).unwrap();
                        let user_plan_renew_date = user_in_request.plan_renewal_date();

                        let is_active_plan = match user_plan_renew_date {
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/new: Plan renewal date parse error"),
                                );
                            },
                            Ok(date) => date >= current_naive_date
                        };

                        if !is_active_plan {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your membership has expired. Please contact Veygo for assistance. "),
                            };
                            // RETURN: FORBIDDEN
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        }

                        // Check if Renter DL exp
                        let return_date = body.end_time.naive_utc().date();

                        if user_in_request.drivers_license_image.is_none() {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Booking Not Allowed"),
                                message: String::from("Your driver's licence is not uploaded. Please submit your driver's licence. "),
                            };
                            // RETURN: FORBIDDEN
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
                                String::from("agreement/new: Database error checking DNR records")
                            )
                        };
                        if record_count > 0 {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Do Not Rent Record Found"),
                                message: String::from("We found one or more dnr records, please contact us to resolve this! "),
                            };
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                        }

                        use schema::agreements::dsl as agreements_query;

                        let renter_agreements_blocking_count = agreements_query::agreements
                            .filter(agreements_query::renter_id.eq(&access_token.user_id))
                            .filter(agreements_query::status.eq(model::AgreementStatus::Rental))
                            .filter(
                                methods::diesel_fn::coalesce(agreements_query::actual_pickup_time, agreements_query::rsvp_pickup_time)
                                    .lt(body.end_time + Duration::minutes(proj_config::RSVP_BUFFER))
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
                                String::from("agreement/new: Database error checking blocking agreements")
                            )
                        };

                        if renter_agreements_blocking_count > 0 {
                            return methods::standard_replies::double_booking_not_allowed()
                        }

                        use schema::apartments::dsl as apartment_query;
                        use schema::vehicles::dsl as vehicle_query;
                        use schema::locations::dsl as location_query;
                        let vehicle_result = vehicle_query::vehicles
                            .inner_join(location_query::locations
                                .inner_join(apartment_query::apartments)
                            )
                            .filter(apartment_query::id.ne(1))
                            .filter(apartment_query::is_operating)
                            .filter(location_query::is_operational)
                            .filter(vehicle_query::id.eq(&body.vehicle_id))
                            .filter(vehicle_query::available)
                            .select(
                                (
                                    vehicle_query::vehicles::all_columns(),
                                    location_query::locations::all_columns(),
                                    apartment_query::apartments::all_columns()
                                )
                            )
                            .get_result::<(model::Vehicle, model::Location, model::Apartment)>(&mut pool);

                        let (req_vehicle, req_location, req_apt) = match vehicle_result {
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

                        let verified_promo: Option<model::Promo> = match body.promo_code.clone() {
                            None => { None }
                            Some(code) => {
                                use crate::schema::promos::dsl as promos_query;
                                let promo = promos_query::promos
                                    .filter(promos_query::code.eq(&code))
                                    .filter(promos_query::is_enabled)
                                    .filter(promos_query::exp.gt(&body.start_time))
                                    .get_result::<model::Promo>(&mut pool);
                                let promo: model::Promo = match promo {
                                    Ok(promo) => { promo }
                                    Err(err) => {
                                        return match err {
                                            Error::NotFound => {
                                                methods::standard_replies::promo_code_not_allowed_response(&code)
                                            }
                                            _ => {
                                                methods::standard_replies::internal_server_error_response(
                                                    String::from("agreement/new: Database error loading promo")
                                                )
                                            }
                                        }
                                    }
                                };

                                // check if this renter already uses the promo code
                                use crate::schema::agreements::dsl as agreement_query;
                                let count_of_this_renter_usage = agreement_query::agreements
                                    .filter(agreement_query::promo_id.eq(Some(&promo.code)))
                                    .filter(agreement_query::status.ne(model::AgreementStatus::Canceled))
                                    .filter(agreement_query::renter_id.eq(&access_token.user_id))
                                    .count()
                                    .get_result::<i64>(&mut pool);
                                let Ok(count_of_this_renter_usage) = count_of_this_renter_usage else {
                                    return methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/new: Database error counting renter promo usage")
                                    )
                                };
                                if count_of_this_renter_usage >= 1 {
                                    return methods::standard_replies::promo_code_not_allowed_response(&code);
                                }

                                // check if someone else already uses the promo code when it's one-time only
                                if promo.is_one_time {
                                    let count_of_agreements = agreement_query::agreements
                                        .filter(agreement_query::promo_id.eq(Some(&promo.code)))
                                        .filter(agreement_query::status.ne(model::AgreementStatus::Canceled))
                                        .count()
                                        .get_result::<i64>(&mut pool);
                                    let Ok(count_of_agreements) = count_of_agreements else {
                                        return methods::standard_replies::internal_server_error_response(
                                            String::from("agreement/new: Database error counting promo usage")
                                        )
                                    };
                                    if count_of_agreements >= 1 {
                                        return methods::standard_replies::promo_code_not_allowed_response(&code);
                                    }
                                }

                                // check if the promo code is for a specific renter
                                if let Some(specified_user_id) = promo.user_id &&
                                    access_token.user_id != specified_user_id
                                {
                                    return methods::standard_replies::promo_code_not_allowed_response(&code);
                                }
                                // check if the promo code is for a specific apartment
                                if let Some(specified_apartment_id) = promo.apt_id &&
                                    !(req_apt.id == specified_apartment_id && req_apt.uni_id != 1)
                                {
                                    return methods::standard_replies::promo_code_not_allowed_response(&code);
                                }
                                // check if the promo code is for a specific university
                                if let Some(specified_uni_id) = promo.uni_id &&
                                    !(req_apt.id == specified_uni_id && req_apt.uni_id == 1 ||
                                        req_apt.uni_id == specified_uni_id)
                                {
                                    return methods::standard_replies::promo_code_not_allowed_response(&code);
                                }

                                Some(promo)
                            }
                        };

                        let is_auth = user_in_request.is_authorized_for(&req_apt);
                        let is_auth = match is_auth {
                            Ok(is_auth) => { is_auth }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/new: Database error loading authorization")
                                )
                            }
                        };

                        if !is_auth {
                            return methods::standard_replies::apartment_not_allowed_response(req_apt.id);
                        }

                        let ag_lia = match body.liability {
                            true => {
                                if let Some(rate) = req_apt.liability_protection_rate {
                                    Some(rate)
                                } else {
                                    let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                        title: String::from("Options Not Allowed"),
                                        message: String::from("Liability insurance is not offered at this location. "),
                                    };
                                    return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                }
                            }
                            false => {
                                None
                            }
                        };

                        let ag_rsa = match body.rsa {
                            true => {
                                if let Some(rate) = req_apt.rsa_protection_rate {
                                    Some(rate)
                                } else {
                                    let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                        title: String::from("Options Not Allowed"),
                                        message: String::from("Roadside Assistance is not offered at this location. "),
                                    };
                                    return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                }
                            }
                            false => {
                                None
                            }
                        };

                        let (ag_pcdw, ag_pcdw_ext) = match (body.pcdw, body.pcdw_ext) {
                            (false, false) => { (None, None) },
                            (true, true) => {
                                if let Some(pcdw) = req_apt.pcdw_protection_rate &&
                                    let Some(pcdw_ext) = req_apt.pcdw_ext_protection_rate {
                                    (Some(pcdw), Some(pcdw_ext))
                                } else {
                                    let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                        title: String::from("Options Not Allowed"),
                                        message: String::from("Collision Damage Waiver is not offered at this location. "),
                                    };
                                    return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                }
                            },
                            (true, false) => {
                                if let Some(pcdw) = req_apt.pcdw_protection_rate {
                                    (Some(pcdw), None)
                                } else {
                                    let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                        title: String::from("Options Not Allowed"),
                                        message: String::from("Partial Collision Damage Waiver is not offered at this location. "),
                                    };
                                    return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                }
                            },
                            (false, true) => {
                                if let Some(pcdw_ext) = req_apt.pcdw_ext_protection_rate {
                                    (None, Some(pcdw_ext))
                                } else {
                                    let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                        title: String::from("Options Not Allowed"),
                                        message: String::from("Limited Collision Damage Waiver is not offered at this location. "),
                                    };
                                    return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                }
                            },
                        };

                        let ag_pai = match body.pai {
                            true => {
                                if let Some(rate) = req_apt.pai_protection_rate {
                                    Some(rate)
                                } else {
                                    let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                        title: String::from("Options Not Allowed"),
                                        message: String::from("Personal Accident Insurance is not offered at this location. "),
                                    };
                                    return methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                }
                            }
                            false => {
                                None
                            }
                        };

                        if body.hours_using_reward > Decimal::ZERO {

                            let total_duration = body.end_time - body.start_time;
                            let total_duration_in_hours = Decimal::new(total_duration.num_minutes(), 0)
                                / Decimal::new(60, 0);

                            if body.hours_using_reward > total_duration_in_hours {
                                let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Booking Not Allowed"),
                                    message: String::from("You are trying to redeem more hours than the rental period."),
                                };
                                // RETURN: FORBIDDEN
                                return Ok((with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN).into_response(),));
                            }

                            let init_allowance = match user_in_request.plan_tier {
                                model::PlanTier::Free => { req_apt.free_tier_hours }
                                model::PlanTier::Silver => { req_apt.silver_tier_hours }
                                model::PlanTier::Gold => { req_apt.gold_tier_hours }
                                model::PlanTier::Platinum => { req_apt.platinum_tier_hours }
                            };

                            if init_allowance < body.hours_using_reward {
                                let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Booking Not Allowed"),
                                    message: String::from("You have exceeded your free hours limit. Please upgrade your plan. "),
                                };
                                // RETURN: FORBIDDEN
                                return Ok((with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN).into_response(),));
                            }

                            let (week_start, week_end_exclusive) = {
                                let days_from_monday = current_naive_date.weekday().num_days_from_monday() as i64;
                                let monday = current_naive_date - Duration::days(days_from_monday);
                                let monday_start = DateTime::<Utc>::from_naive_utc_and_offset(
                                    monday.and_hms_opt(0, 0, 0).unwrap(),
                                    Utc,
                                );
                                let next_monday = monday_start + Duration::days(7);
                                (monday_start, next_monday)
                            };

                            use crate::schema::reward_transactions::dsl as reward_q;
                            let used_free_hours = reward_q::reward_transactions
                                .filter(reward_q::renter_id.eq(&access_token.user_id))
                                .filter(reward_q::transaction_time.ge(week_start))
                                .filter(reward_q::transaction_time.lt(week_end_exclusive))
                                .select(diesel::dsl::sum(reward_q::duration))
                                .first::<Option<Decimal>>(&mut pool);
                            if used_free_hours.is_err() {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/new: Database error summing used reward hours"),
                                )
                            }
                            let used_free_hours = used_free_hours.unwrap().unwrap_or(Decimal::zero());

                            if init_allowance - used_free_hours < body.hours_using_reward {
                                let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Booking Not Allowed"),
                                    message: String::from("You have exceeded your free hours limit. Please upgrade your plan. "),
                                };
                                // RETURN: FORBIDDEN
                                return Ok((with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN).into_response(),));
                            }
                        }

                        use schema::payment_methods::dsl as payment_method_query;
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
                                        methods::standard_replies::internal_server_error_response(String::from("agreement/new: Database error loading credit card"))
                                    }
                                }
                            }
                        };

                        // Check conflict

                        let start_time_buffered = body.start_time - Duration::minutes(proj_config::RSVP_BUFFER);
                        let end_time_buffered = body.end_time + Duration::minutes(proj_config::RSVP_BUFFER);

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
                                String::from("agreement/new: Database error checking vehicle conflict")
                            )
                        };

                        if is_conflict {
                            let err_msg = helper_model::ErrorResponse {
                                title: "Vehicle Unavailable".to_string(),
                                message: "Please try again later. ".to_string(),
                            };
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::CONFLICT)
                        }


                        // Calculate total cost

                        // 0. calculate rate offer
                        let rate_offer: Decimal = {
                            use schema::rate_offers::dsl as ro_q;
                            let non_expired_rate_offer_dec = ro_q::rate_offers
                                .filter(ro_q::exp.gt(current_time))
                                .filter(ro_q::renter_id.eq(user_in_request.id))
                                .find(body.rate_offer_id)
                                .select(ro_q::multiplier)
                                .get_result::<Decimal>(&mut pool);

                            match non_expired_rate_offer_dec {
                                Ok(offer) => { offer }
                                Err(err) => {
                                    return match err {
                                        Error::NotFound => {
                                            // rate offer not valid or not found
                                            let err_msg = helper_model::ErrorResponse {
                                                title: "Rate Not Available".to_string(),
                                                message: "Please get a new quote".to_string(),
                                            };
                                            methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                        }
                                        _ => {
                                            // diesel db error
                                            methods::standard_replies::internal_server_error_response(String::from("agreement/new: Database error loading rate offer"))
                                        }
                                    }
                                }
                            }
                        };

                        // 1. total rental revenue
                        let total_hours_reserved = Decimal::new(trip_duration.num_minutes(), 0) / Decimal::new(60, 0);
                        let total_hours_reserved_round_up = total_hours_reserved.round_dp_with_strategy(0, RoundingStrategy::AwayFromZero);
                        let raw_hours_after_applying_credit = methods::rental_rate::calculate_duration_after_reward(trip_duration, body.hours_using_reward);
                        let billable_days_count: i32 = methods::rental_rate::billable_days_count(trip_duration);
                        let billable_duration_hours: Decimal = methods::rental_rate::calculate_billable_duration_hours(raw_hours_after_applying_credit);

                        let duration_revenue = billable_duration_hours * req_apt.duration_rate * req_vehicle.msrp_factor * rate_offer;

                        let duration_revenue_after_promo = match verified_promo.clone() {
                            None => { duration_revenue }
                            Some(promo) => {
                                max(Decimal::zero(), duration_revenue - promo.amount)
                            }
                        };

                        // 2. total insurance revenue

                        let insurance_revenue = {
                            total_hours_reserved_round_up * (
                                ag_lia.unwrap_or(Decimal::zero())
                                + ag_pcdw.unwrap_or(Decimal::zero())
                                + ag_pcdw_ext.unwrap_or(Decimal::zero())
                                + ag_rsa.unwrap_or(Decimal::zero())
                                + ag_pai.unwrap_or(Decimal::zero())
                            )
                        };

                        // 3. mileage package revenue

                        let mileage_package_cost = match body.mileage_package_id {
                            None => {
                                // didn't select mp
                                Decimal::zero()
                            }
                            Some(mp_id) => {
                                use schema::mileage_packages::dsl as mp_q;
                                let mp_result = mp_q::mileage_packages
                                    .filter(mp_q::is_active)
                                    .find(mp_id)
                                    .select((mp_q::miles, mp_q::discounted_rate))
                                    .get_result::<(i32, i32)>(&mut pool);
                                match mp_result {
                                    Ok((mileage, discount_rate)) => {
                                        let base_rate_for_mp = if let Some(overwrite) = req_apt.mileage_package_overwrite {
                                            overwrite
                                        } else {
                                            req_apt.duration_rate * req_vehicle.msrp_factor * req_apt.mileage_conversion
                                        };

                                        base_rate_for_mp * Decimal::new(mileage as i64, 0) * Decimal::new(discount_rate as i64, 2)
                                    }
                                    Err(err) => {
                                        return match err {
                                            Error::NotFound => {
                                                let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                                    title: String::from("Booking Not Allowed"),
                                                    message: String::from("Invalid mileage package option selected"),
                                                };
                                                methods::standard_replies::response_with_obj(err_msg, StatusCode::FORBIDDEN)
                                            }
                                            _ => {
                                                methods::standard_replies::internal_server_error_response(
                                                    String::from( "agreement/new: Database error loading mileage package"),
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        };

                        // 4. taxes

                        use crate::schema::apartments_taxes::dsl as apartments_taxes_query;
                        use crate::schema::taxes::dsl as t_q;

                        let taxes = apartments_taxes_query::apartments_taxes
                            .inner_join(t_q::taxes)
                            .filter(apartments_taxes_query::apartment_id.eq(&req_apt.id))
                            .select(t_q::taxes::all_columns())
                            .get_results::<model::Tax>(&mut pool);

                        let Ok(taxes) = taxes else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/new: Database error loading apartment taxes")
                            )
                        };

                        let mut local_tax_rate_percent = Decimal::zero();
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

                        // 5. subtotals
                        let total_subject_to_rental_tax = duration_revenue_after_promo
                            + insurance_revenue + mileage_package_cost;

                        let total_percentage_tax = total_subject_to_rental_tax * local_tax_rate_percent;
                        let total_daily_tax = Decimal::new(billable_days_count as i64, 0) * local_tax_rate_daily;
                        let total_fixed_tax = local_tax_rate_fixed;

                        let total_stripe_amount = total_subject_to_rental_tax
                            + total_percentage_tax + total_daily_tax + total_fixed_tax;
                        let mut total_stripe_amount_2dp = total_stripe_amount.round_dp(2);
                        (&mut total_stripe_amount_2dp).rescale(2);
                        let total_stripe_amount_cent = total_stripe_amount_2dp.mantissa();

                        // insert agreement

                        let conf_id = methods::agreement::generate_unique_agreement_confirmation();
                        let Ok(conf_id) = conf_id else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/new: Failed to generate agreement confirmation")
                            )
                        };

                        let promo_id = match verified_promo {
                            None => { None }
                            Some( promo ) => { Some( promo.code ) }
                        };
                        let new_agreement = model::NewAgreement {
                            confirmation: conf_id.clone(),
                            status: model::AgreementStatus::Rental,
                            user_name: user_in_request.name.clone(),
                            user_date_of_birth: user_in_request.date_of_birth.clone(),
                            user_email: user_in_request.student_email.clone(),
                            user_phone: user_in_request.phone.clone(),
                            user_billing_address: billing_address,
                            rsvp_pickup_time: body.start_time,
                            rsvp_drop_off_time: body.end_time,
                            liability_protection_rate: ag_lia,
                            pcdw_protection_rate: ag_pcdw,
                            pcdw_ext_protection_rate: ag_pcdw_ext,
                            rsa_protection_rate: ag_rsa,
                            pai_protection_rate: ag_pai,
                            msrp_factor: req_vehicle.msrp_factor,
                            duration_rate: req_apt.duration_rate,
                            vehicle_id: req_vehicle.id,
                            renter_id: user_in_request.id,
                            payment_method_id: body.payment_id,
                            promo_id,
                            manual_discount: None,
                            location_id: req_location.id,
                            mileage_package_id: body.mileage_package_id,
                            mileage_conversion: req_apt.mileage_conversion,
                            mileage_rate_overwrite: req_apt.mileage_rate_overwrite,
                            mileage_package_overwrite: req_apt.mileage_package_overwrite,
                            minimum_earning_rate: total_subject_to_rental_tax,
                        };

                        let inserted_agreement = diesel::insert_into(ag_q::agreements)
                            .values(&new_agreement)
                            .get_result::<model::Agreement>(&mut pool);

                        let Ok(inserted_agreement) = inserted_agreement else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/new: Failed to insert agreement error")
                            )
                        };

                        use crate::schema::agreements_taxes::dsl as ag_tx_q;
                        for tax in &taxes {
                            let new_agreement_tax = model::AgreementTax {
                                agreement_id: inserted_agreement.id,
                                tax_id: tax.id,
                            };
                            let result = diesel::insert_into(ag_tx_q::agreements_taxes)
                                .values(new_agreement_tax)
                                .get_result::<model::AgreementTax>(&mut pool);
                            if result.is_err() {
                                let _ = diesel::delete(ag_tx_q::agreements_taxes
                                    .filter(ag_tx_q::agreement_id.eq(inserted_agreement.id)))
                                    .execute(&mut pool);
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/new: Failed to insert agreement tax record error")
                                )
                            }
                        }

                        use schema::reward_transactions::dsl as rt_q;

                        if body.hours_using_reward > Decimal::zero() {
                            let new_reward_trans = model::NewRewardTransaction {
                                agreement_id: Some(inserted_agreement.id),
                                renter_id: user_in_request.id,
                                duration: body.hours_using_reward,
                            };

                            let result = diesel::insert_into(rt_q::reward_transactions)
                                .values(&new_reward_trans)
                                .get_result::<model::RewardTransaction>(&mut pool);
                            if result.is_err() {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/new: Failed to insert reward transaction record error")
                                )
                            }
                        }

                        // stripe auth

                        let description = "RSVP #".to_owned() + &*conf_id.clone();
                        let stripe_auth = integration::stripe_veygo::create_payment_intent(
                            &user_in_request.stripe_id, &payment_method.token, total_stripe_amount_cent as i64, PaymentIntentCaptureMethod::Manual, &description
                        ).await;

                        use crate::schema::payments::dsl as payment_query;
                        match stripe_auth {
                            Ok(pmi) => {
                                // auth successful
                                let new_payment = model::NewPayment {
                                    payment_type: model::PaymentType::Succeeded,
                                    amount: total_stripe_amount_2dp,
                                    note: None,
                                    reference_number: Some(pmi.id.to_string()),
                                    agreement_id: inserted_agreement.id,
                                    renter_id: user_in_request.id,
                                    payment_method_id: Some(payment_method.id),
                                    amount_authorized: total_stripe_amount_2dp,
                                    capture_before: None,
                                };

                                let payment_result = diesel::insert_into(payment_query::payments)
                                    .values(&new_payment).get_result::<model::Payment>(&mut pool);

                                match payment_result {
                                    Ok(pmt) => {
                                        // insert successful
                                        let intent_id = pmt.clone().reference_number.unwrap();
                                        let _ = integration::stripe_veygo::capture_payment(&intent_id, None).await;

                                        methods::standard_replies::response_with_obj(inserted_agreement, StatusCode::CREATED)
                                    }
                                    Err(_) => {
                                        methods::standard_replies::internal_server_error_response(
                                            String::from("agreement/new: Failed to insert payment error")
                                        )
                                    }
                                }
                            }
                            Err(v_err) => {
                                let _ = diesel::delete(payment_query::payments)
                                    .filter(payment_query::agreement_id.eq(inserted_agreement.id))
                                    .execute(&mut pool);
                                let _ = diesel::delete(ag_tx_q::agreements_taxes)
                                    .filter(ag_tx_q::agreement_id.eq(inserted_agreement.id))
                                    .execute(&mut pool);
                                let _ = diesel::delete(rt_q::reward_transactions)
                                    .filter(rt_q::agreement_id.eq(inserted_agreement.id))
                                    .execute(&mut pool);
                                let _ = diesel::delete(ag_q::agreements)
                                    .filter(ag_q::id.eq(inserted_agreement.id))
                                    .execute(&mut pool);

                                return match v_err {
                                    VeygoError::CardDeclined => {
                                        methods::standard_replies::card_declined()
                                    }
                                    _ => {
                                        methods::standard_replies::internal_server_error_response(
                                            String::from("agreement/new: Stripe error creating payment intent")
                                        )
                                    }
                                }
                            }
                        }
                    }
                };
            },
        )
}
