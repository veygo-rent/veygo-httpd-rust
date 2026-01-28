use std::cmp::min;
use crate::{POOL, methods, model, helper_model, integration};
use diesel::prelude::*;
use diesel::expression_methods::NullableExpressionMethods;
use warp::{Filter, Rejection, Reply, http::{Method, StatusCode}};
use chrono::{DateTime, Duration, Utc};
use diesel::result::Error;
use rust_decimal::prelude::*;
use serde::Deserialize;
use stripe_core::PaymentIntentCaptureMethod;
use crate::helper_model::VeygoError;

#[derive(Debug, Deserialize)]
struct TeslaChargingSessionsResponse {
    data: Vec<TeslaChargingSessionMin>,
    status_code: i32,
}

#[derive(Debug, Deserialize)]
struct TeslaChargingSessionMin {
    start_date_time: DateTime<Utc>,
    location: TeslaChargingLocationMin,
    total_cost: TeslaTotalCostMin,
}

#[derive(Debug, Deserialize)]
struct TeslaChargingLocationMin {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TeslaTotalCostMin {
    excl_vat: f64,
    incl_vat: f64,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("check-in")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: helper_model::CheckInRequest, auth: String, user_agent: String| {

            // Checking method is POST
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }

            if body.agreement_id <= 0 || body.vehicle_snapshot_id <= 0 {
                return methods::standard_replies::bad_request("Bad request: wrong parameters. ")
            }

            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_return();
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => int,
                Err(_) => {
                    return methods::tokens::token_invalid_return();
                }
            };

            let access_token = model::RequestToken {
                user_id,
                token: String::from(token_and_id[0]),
            };
            let if_token_valid =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                    .await;

            return match if_token_valid {
                Err(e) => {
                    match e {
                        VeygoError::TokenFormatError => {
                            methods::tokens::token_not_hex_warp_return()
                        }
                        VeygoError::InvalidToken => {
                            methods::tokens::token_invalid_return()
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response("agreement/check-in: Database connection error at token verification").await
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response("agreement/check-in: SQL error at extending token").await;
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response("agreement/check-in: Database connection error at extending token").await;
                        }
                    }

                    use crate::schema::agreements::dsl as agreement_q;
                    use crate::schema::vehicle_snapshots::dsl as v_s_q;

                    let mut pool = POOL.get().unwrap();

                    let five_minutes_ago = Utc::now() - Duration::minutes(5);

                    let ag_v_s_result = v_s_q::vehicle_snapshots
                        .inner_join(
                            agreement_q::agreements.on(
                                v_s_q::vehicle_id.eq(agreement_q::vehicle_id)
                                    .and(v_s_q::renter_id.eq(agreement_q::renter_id))
                            )
                        )
                        .filter(agreement_q::renter_id.eq(&user_id))
                        .filter(v_s_q::id.eq(&body.vehicle_snapshot_id))
                        .filter(agreement_q::id.eq(&body.agreement_id))
                        .filter(agreement_q::actual_pickup_time.is_not_null())
                        .filter(agreement_q::actual_drop_off_time.is_null())
                        .filter(v_s_q::time.ge(agreement_q::actual_pickup_time.assume_not_null()))
                        .filter(v_s_q::time.ge(five_minutes_ago))
                        .select((agreement_q::agreements::all_columns(), v_s_q::vehicle_snapshots::all_columns()))
                        .get_result::<(model::Agreement, model::VehicleSnapshot)>(&mut pool);

                    if let Err(err) = ag_v_s_result {
                        return match err {
                            Error::NotFound => {
                                methods::standard_replies::agreement_not_allowed_response()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response("agreement/check-in: Database connection error at loading vehicle snapshot").await
                            }
                        }
                    }

                    let (mut agreement_to_be_checked_in, check_in_snapshot) = ag_v_s_result.unwrap();

                    // lock the vehicle

                    use crate::schema::vehicles::dsl as v_q;
                    let result = v_q::vehicles
                        .find(&agreement_to_be_checked_in.vehicle_id)
                        .select((v_q::remote_mgmt, v_q::remote_mgmt_id, v_q::vin))
                        .get_result::<(model::RemoteMgmtType, String, String)>(&mut pool);

                    let Ok((vehicle_remote_mgmt, mgmt_id, vin_num)) = result else {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Database connection error at loading vehicle remote mgmt info",
                        )
                        .await;
                    };

                    match vehicle_remote_mgmt {
                        model::RemoteMgmtType::Tesla => {
                            // 1) Check online state via GET /api/1/vehicles/{vehicle_tag}
                            let status_path = format!("/api/1/vehicles/{}", mgmt_id);

                            for i in 0..16 { // up to ~10s total
                                if let Ok(response) = integration::tesla_curl::tesla_make_request(Method::GET, &status_path, None).await {
                                    if let Ok(body_text) = response.text().await {
                                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                                            let state = json
                                                .get("response")
                                                .and_then(|r| r.get("state"))
                                                .and_then(|s| s.as_str())
                                                .unwrap_or("");
                                            if state == "online" {
                                                break;
                                            }
                                            // Only on the first iteration, if offline, send wake_up once
                                            if i == 0 {
                                                let wake_path = format!("/api/1/vehicles/{}/wake_up", mgmt_id);
                                                let _ = integration::tesla_curl::tesla_make_request(Method::POST, &wake_path, None).await;
                                            }
                                        }
                                    }
                                }
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }
                            // 2) Proceed to lock/unlock once online (or after timeout anyway)
                            let cmd_path = format!("/api/1/vehicles/{}/command/door_lock", mgmt_id);
                            let result = integration::tesla_curl::tesla_make_request(Method::POST, &cmd_path, None).await;

                            let Ok(resp) = result else {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: Tesla API error at door_lock request",
                                )
                                .await;
                            };
                            if resp.status() != StatusCode::OK {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: Tesla API returned non-200 at door_lock request",
                                )
                                .await;
                            }
                        }
                        _ => {}
                    }

                    // update agreement status to check in (not in db)

                    agreement_to_be_checked_in.actual_drop_off_time = Some(Utc::now());
                    agreement_to_be_checked_in.vehicle_snapshot_after = Some(check_in_snapshot.id);

                    // map unmapped charges to this agreement
                    // 1. current charges in the database

                    use crate::schema::charges::dsl as c_q;

                    let pickup: DateTime<Utc> = agreement_to_be_checked_in.actual_pickup_time.unwrap();
                    let drop_off: DateTime<Utc> = agreement_to_be_checked_in.actual_drop_off_time.unwrap();

                    let result = diesel::update(c_q::charges)
                        .filter(c_q::agreement_id.is_null())
                        .filter(c_q::vehicle_id.eq(&agreement_to_be_checked_in.vehicle_id))
                        .filter(c_q::time.ge(&pickup))
                        .filter(c_q::time.le(&drop_off))
                        .set(c_q::agreement_id.eq(Some(&agreement_to_be_checked_in.id)))
                        .execute(&mut pool);

                    match result {
                        Ok(count) => {
                            if count == 0 {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: SQL error at mapping unmapped charges (no rows updated)",
                                )
                                .await
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: Database connection error at mapping unmapped charges",
                            )
                            .await
                        }
                    }

                    // 2. fetch tesla charging history

                    if vehicle_remote_mgmt == model::RemoteMgmtType::Tesla {
                        use chrono::SecondsFormat;
                        let date_from = pickup.to_rfc3339_opts(SecondsFormat::Secs, true);
                        let date_to   = drop_off.to_rfc3339_opts(SecondsFormat::Secs, true);

                        let charge_history_path = format!(
                            "/api/1/dx/charging/sessions?vin={}&date_from={}&date_to={}",
                            vin_num,
                            date_from,
                            date_to
                        );

                        let result = integration::tesla_curl::tesla_make_request(Method::GET, &charge_history_path, None).await;

                        let Ok(resp) = result else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: Tesla API error at fetching charging sessions",
                            )
                            .await
                        };

                        if resp.status() != StatusCode::OK {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: Tesla API returned non-200 at fetching charging sessions",
                            )
                            .await
                        }

                        let body_text = match resp.text().await {
                            Ok(t) => t,
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: Tesla API response body read error at charging sessions",
                                )
                                .await
                            }
                        };

                        let parsed: TeslaChargingSessionsResponse = match serde_json::from_str(&body_text) {
                            Ok(p) => p,
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: JSON parse error at charging sessions",
                                )
                                .await
                            }
                        };

                        if parsed.status_code != 1000 {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: Tesla charging sessions returned failure status_code",
                            )
                            .await
                        }

                        let sessions_min: Vec<(DateTime<Utc>, String, f64, f64)> = parsed
                            .data
                            .into_iter()
                            .map(|s| (s.start_date_time, s.location.name, s.total_cost.excl_vat, s.total_cost.incl_vat))
                            .collect();

                        for (session_time, location, _excl_vat, incl_vat) in sessions_min {
                            let charging_note = format!("Tesla charging at {}", location);
                            let incl_vat_opt = Decimal::try_from(incl_vat);
                            if incl_vat_opt.is_err() {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: Decimal conversion error for Tesla charging cost",
                                )
                                .await
                            }
                            let new_charge = model::NewCharge{
                                name: charging_note,
                                time: session_time,
                                amount: incl_vat_opt.unwrap(),
                                note: None,
                                agreement_id: Some(agreement_to_be_checked_in.id),
                                vehicle_id: agreement_to_be_checked_in.vehicle_id,
                                transponder_company_id: None,
                                vehicle_identifier: None,
                            };

                            use crate::schema::charges::dsl as c_q;

                            let res = diesel::insert_into(c_q::charges)
                                .values(&new_charge)
                                .execute(&mut pool);

                            if res.is_err() {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: SQL error at inserting Tesla charging charge",
                                )
                                .await
                            }
                        }
                    }

                    // Calculate total cost
                    // 0. total reward hours

                    use crate::schema::reward_transactions::dsl as r_q;

                    let total_reward_hours_result = r_q::reward_transactions
                        .filter(r_q::agreement_id.eq(&agreement_to_be_checked_in.id))
                        .select(diesel::dsl::sum(r_q::duration))
                        .get_result::<Option<Decimal>>(&mut pool);

                    let Ok(total_reward_hours_result) = total_reward_hours_result else {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Database connection error at summing reward hours",
                        )
                        .await
                    };

                    let total_reward_hours = if total_reward_hours_result.is_none() {
                        Decimal::new(0, 2)
                    } else {
                        total_reward_hours_result.unwrap()
                    };

                    // 1. total rental revenue before late return
                    // TODO: calculate insurance premium

                    let time_to_be_counted_as_if_return_on_time = min(drop_off, agreement_to_be_checked_in.rsvp_drop_off_time);

                    let total_duration = time_to_be_counted_as_if_return_on_time - pickup;
                    let billable_duration = methods::rental_rate::calculate_duration_after_reward(total_duration, total_reward_hours);

                    let billable_hours = methods::rental_rate::calculate_billable_duration_hours(billable_duration);

                    let rental_revenue_before_discounts = billable_hours
                        * agreement_to_be_checked_in.duration_rate
                        * agreement_to_be_checked_in.msrp_factor
                        * agreement_to_be_checked_in.utilization_factor;

                    let mut total_discount = Decimal::zero();

                    if let Some(m_discount) = agreement_to_be_checked_in.manual_discount {
                        total_discount += m_discount;
                    }

                    if let Some(discount_id) = agreement_to_be_checked_in.clone().promo_id {
                        use crate::schema::promos::dsl as pr_q;
                        let promo_result = pr_q::promos
                            .find(&discount_id)
                            .get_result::<model::Promo>(&mut pool);

                    if promo_result.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Database connection error at loading promo",
                        )
                        .await
                    }
                        let promo = promo_result.unwrap();

                        total_discount += promo.amount;
                    }

                    let rental_revenue = (rental_revenue_before_discounts - total_discount).max(Decimal::zero());

                    // 2. total late return fee

                    let late_hours = methods::rental_rate::calculate_late_hours(agreement_to_be_checked_in.rsvp_drop_off_time, drop_off);
                    let late_return_revenue = late_hours
                        * agreement_to_be_checked_in.duration_rate
                        * agreement_to_be_checked_in.msrp_factor
                        * agreement_to_be_checked_in.utilization_factor
                        * Decimal::new(2, 0);

                    // 3. total charges

                    let total_external_charges_result = c_q::charges
                        .filter(c_q::agreement_id.eq(&agreement_to_be_checked_in.id))
                        .select(diesel::dsl::sum(c_q::amount))
                        .get_result::<Option<Decimal>>(&mut pool);

                    let Ok(total_external_charges_result) = total_external_charges_result else {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Database connection error at summing external charges",
                        )
                        .await
                    };

                    let total_external_charges = if let Some(sum) = total_external_charges_result {
                        sum
                    } else {
                        Decimal::zero()
                    };

                    // 4. mileage package and over mileage charges

                    let mut included_miles = 10;

                    let mut mileage_package_cost = Decimal::zero();
                    let mut over_mileage_charges = Decimal::zero();

                    if let Some(mp_id) = agreement_to_be_checked_in.mileage_package_id {
                        use crate::schema::mileage_packages::dsl as mp_q;
                        let mp_result = mp_q::mileage_packages.find(&mp_id).get_result::<model::MileagePackage>(&mut pool);

                        let Ok(mp_result) = mp_result else {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: Database connection error at loading mileage package",
                            )
                            .await
                        };

                        let mp_rate = if let Some(overwrite_rate) = agreement_to_be_checked_in.mileage_package_overwrite {
                            overwrite_rate
                        } else {
                            agreement_to_be_checked_in.mileage_conversion
                                * agreement_to_be_checked_in.duration_rate
                                * agreement_to_be_checked_in.msrp_factor
                        };

                        mileage_package_cost = mp_rate
                            * Decimal::new(mp_result.miles as i64, 0)
                            * Decimal::new(mp_result.discounted_rate as i64, 2);

                        included_miles += mp_result.miles;
                    }

                    let odometer_after_result = v_s_q::vehicle_snapshots
                        .find(&agreement_to_be_checked_in.vehicle_snapshot_after.unwrap())
                        .select(v_s_q::odometer)
                        .get_result::<i32>(&mut pool);

                    let Ok(odometer_after) = odometer_after_result else {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Database connection error at loading odometer after",
                        )
                        .await
                    };

                    let odometer_before_result = v_s_q::vehicle_snapshots
                        .find(&agreement_to_be_checked_in.vehicle_snapshot_before.unwrap())
                        .select(v_s_q::odometer)
                        .get_result::<i32>(&mut pool);

                    let Ok(odometer_before) = odometer_before_result else {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Database connection error at loading odometer before",
                        )
                        .await
                    };

                    let total_driven = odometer_after - odometer_before;
                    if total_driven < 0 {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Invalid odometer delta (negative total driven)",
                        )
                        .await
                    }
                    if total_driven > included_miles {
                        let additional_miles = Decimal::new((total_driven - included_miles) as i64, 0);
                        let per_mile_cost = if let Some(overwrite_rate) = agreement_to_be_checked_in.mileage_rate_overwrite {
                            overwrite_rate
                        } else {
                            agreement_to_be_checked_in.mileage_conversion * agreement_to_be_checked_in.duration_rate * agreement_to_be_checked_in.msrp_factor
                        };

                        over_mileage_charges = additional_miles * per_mile_cost;
                    }

                    let total_mileage_charges = mileage_package_cost + over_mileage_charges;

                    // TODO: 5. low fuel charges

                    let total_low_fuel_charge = Decimal::zero();

                    // 6. total taxes

                    let mut percent_tax = Decimal::zero();
                    let mut daily_tax = Decimal::zero();
                    let mut fixed_tax = Decimal::zero();

                    let subjected_to_non_sales_tax = rental_revenue + late_return_revenue
                        + total_mileage_charges + total_low_fuel_charge;
                    let subjected_to_sales_tax = subjected_to_non_sales_tax
                        + total_external_charges;
                    let billable_days = methods::rental_rate::billable_days_count(total_duration);

                    use crate::schema::taxes::dsl as t_q;
                    use crate::schema::agreements_taxes::dsl as at_q;

                    let all_taxes_with_current_agreement_result = at_q::agreements_taxes
                        .inner_join(t_q::taxes)
                        .filter(at_q::agreement_id.eq(&agreement_to_be_checked_in.id))
                        .select(t_q::taxes::all_columns())
                        .get_results::<model::Tax>(&mut pool);

                    let Ok(all_taxes_with_current_agreement) = all_taxes_with_current_agreement_result else {
                        return methods::standard_replies::internal_server_error_response(
                            "agreement/check-in: Database connection error at loading agreement taxes",
                        )
                        .await
                    };

                    for tax in all_taxes_with_current_agreement {
                        match tax.tax_type {
                            model::TaxType::Percent => {
                                if tax.is_sales_tax {
                                    percent_tax += tax.multiplier * subjected_to_sales_tax;
                                } else {
                                    percent_tax += tax.multiplier * subjected_to_non_sales_tax;
                                }
                            }
                            model::TaxType::Daily => {
                                daily_tax += Decimal::new(billable_days as i64, 0) * tax.multiplier;
                            }
                            model::TaxType::Fixed => {
                                fixed_tax += tax.multiplier;
                            }
                        }
                    }

                    let total_tax = percent_tax + daily_tax + fixed_tax;

                    // 7. total cost = 1 + 2 + 3 + 4 + 5 + 6

                    let total_cost = (rental_revenue + late_return_revenue
                        + total_external_charges + total_mileage_charges + total_low_fuel_charge
                        + total_tax).round_dp(2);

                    // TODO: Capture the correct amount and process additional charges
                    // 1. calculate captured amount
                    use crate::schema::payments::dsl as p_q;
                    let total_captured_amount_result = p_q::payments
                        .filter(p_q::agreement_id.eq(&agreement_to_be_checked_in.id))
                        .filter(p_q::payment_type.eq_any(vec![
                            model::PaymentType::VeygoInsurance,
                            model::PaymentType::VeygoBadDebt,
                            model::PaymentType::RequiresCapture,
                            model::PaymentType::Succeeded
                        ]))
                        .select(diesel::dsl::sum(p_q::amount))
                        .get_result::<Option<Decimal>>(&mut pool);

                    let total_captured_amount = match total_captured_amount_result {
                        Ok(temp) => {
                            temp
                                .unwrap_or(Decimal::zero())
                                .round_dp(2)
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: Database connection error at summing captured payments",
                            )
                            .await
                        }
                    };

                    if total_captured_amount < total_cost {

                        let mut total_needed_to_be_captured_2dp = total_cost - total_captured_amount;
                        let all_pm_id_needed_to_be_completed = p_q::payments
                            .filter(p_q::agreement_id.eq(&agreement_to_be_checked_in.id))
                            .filter(p_q::payment_type.eq(model::PaymentType::RequiresCapture))
                            .filter(p_q::reference_number.is_not_null())
                            .select((p_q::id, p_q::reference_number, p_q::amount, p_q::amount_authorized))
                            .get_results::<(i32, Option<String>, Decimal, Decimal)>(&mut pool);

                        let Ok(all_pm_id_needed_to_be_completed) = all_pm_id_needed_to_be_completed else {
                            return methods::standard_replies::internal_server_error_response("agreement/check-in: Database connection error at loading uncaptured payments").await
                        };

                        for pm in all_pm_id_needed_to_be_completed {
                            let pi_id = pm.1.unwrap();
                            let p_id = pm.0;
                            if !total_needed_to_be_captured_2dp.is_zero() {
                                let can_capture_2dp = pm.3 - pm.2;
                                if can_capture_2dp >= total_needed_to_be_captured_2dp {
                                    // capture total_needed_to_be_captured
                                    let int_to_capture = total_needed_to_be_captured_2dp.mantissa() as i64;
                                    let pmi_cap = integration::stripe_veygo::capture_payment(&pi_id, Some((int_to_capture, true))).await;
                                    if pmi_cap.is_err() {
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: Stripe error at capturing payment",
                                        )
                                        .await
                                    }
                                    let pmi_cap = pmi_cap.unwrap();
                                    let pmi_status: model::PaymentType = pmi_cap.status.into();
                                    if pmi_status != model::PaymentType::Succeeded {
                                        let result = integration::stripe_veygo::capture_payment(&pi_id, Some((0, true))).await;
                                        if result.is_err() {
                                            return methods::standard_replies::internal_server_error_response(
                                                "agreement/check-in: Stripe error at finalizing capture",
                                            )
                                            .await
                                        }
                                    }
                                    let result = diesel::update(p_q::payments.find(&p_id))
                                        .set(
                                            (
                                                p_q::amount.eq(&Decimal::new(pmi_cap.amount_received, 2)),
                                                p_q::amount_authorized.eq(&Decimal::new(pmi_cap.amount, 2)),
                                                p_q::payment_type.eq(model::PaymentType::Succeeded)
                                            )
                                        )
                                        .execute(&mut pool);

                                    match result {
                                        Ok(count) => {
                                            if count == 0 {
                                                return methods::standard_replies::internal_server_error_response(
                                                    "agreement/check-in: SQL error at updating payment after capture (no rows updated)",
                                                )
                                                .await
                                            }
                                        }
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response(
                                                "agreement/check-in: Database connection error at updating payment after capture",
                                            )
                                            .await
                                        }
                                    }

                                    total_needed_to_be_captured_2dp = Decimal::zero();
                                } else {
                                    // capture can_capture_2dp
                                    let int_to_capture = can_capture_2dp.mantissa() as i64;
                                    let pmi_cap = integration::stripe_veygo::capture_payment(&pi_id, Some((int_to_capture, true))).await;
                                    if pmi_cap.is_err() {
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: Stripe error at capturing payment",
                                        )
                                        .await
                                    }
                                    let pmi_cap = pmi_cap.unwrap();
                                    let pmi_status: model::PaymentType = pmi_cap.status.into();
                                    if pmi_status != model::PaymentType::Succeeded {
                                        let result = integration::stripe_veygo::capture_payment(&pi_id, Some((0, true))).await;
                                        if result.is_err() {
                                            return methods::standard_replies::internal_server_error_response(
                                                "agreement/check-in: Stripe error at finalizing capture",
                                            )
                                            .await
                                        }
                                    }
                                    let result = diesel::update(p_q::payments.find(&p_id))
                                        .set(
                                            (
                                                p_q::amount.eq(&Decimal::new(pmi_cap.amount_received, 2)),
                                                p_q::amount_authorized.eq(&Decimal::new(pmi_cap.amount, 2)),
                                                p_q::payment_type.eq(model::PaymentType::Succeeded)
                                            )
                                        )
                                        .execute(&mut pool);

                                    match result {
                                        Ok(count) => {
                                            if count == 0 {
                                                return methods::standard_replies::internal_server_error_response(
                                                    "agreement/check-in: SQL error at updating payment after capture (no rows updated)",
                                                )
                                                .await
                                            }
                                        }
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response(
                                                "agreement/check-in: Database connection error at updating payment after capture",
                                            )
                                            .await
                                        }
                                    }

                                    total_needed_to_be_captured_2dp -= can_capture_2dp;
                                }
                            } else {
                                // captured enough, drop auth
                                let pi = if pm.2 != Decimal::zero() {
                                    let result = integration::stripe_veygo::capture_payment(&pi_id, Some((0, true))).await;
                                    if result.is_err() {
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: Stripe error at finalizing capture",
                                        )
                                        .await
                                    }
                                    result.unwrap()
                                } else {
                                    let result = integration::stripe_veygo::drop_auth(&pi_id).await;
                                    if result.is_err() {
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: Stripe error at dropping authorization",
                                        )
                                        .await
                                    }
                                    result.unwrap()
                                };
                                let result = diesel::update(p_q::payments.find(&p_id))
                                    .set(p_q::payment_type.eq::<model::PaymentType>(pi.status.into()))
                                    .execute(&mut pool);

                                match result {
                                    Ok(count) => {
                                        if count == 0 {
                                            return methods::standard_replies::internal_server_error_response(
                                                "agreement/check-in: SQL error at updating payment after capture (no rows updated)",
                                            )
                                            .await
                                        }
                                    }
                                    Err(_) => {
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: Database connection error at updating payment after capture",
                                        )
                                        .await
                                    }
                                }
                            }
                        }

                        // if total_needed_to_be_captured_2dp > 0, create a new Payment and capture the remaining amount
                        if total_needed_to_be_captured_2dp.gt(&Decimal::zero()) {
                            let pm_id = agreement_to_be_checked_in.payment_method_id;
                            use crate::schema::payment_methods::dsl as pm_q;
                            let p_id = pm_q::payment_methods
                                .find(&pm_id)
                                .select(pm_q::token)
                                .get_result::<String>(&mut pool);

                            let Ok(p_id) = p_id else {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: Database connection error at loading payment method token",
                                )
                                .await
                            };
                            // process total_needed_to_be_captured_2dp
                            let int_to_capture = total_needed_to_be_captured_2dp.mantissa() as i64;

                            use crate::schema::renters::dsl as r_q;
                            let stripe_id = r_q::renters
                                .find(&agreement_to_be_checked_in.renter_id)
                                .select(r_q::stripe_id)
                                .get_result::<String>(&mut pool);

                            let Ok(stripe_id) = stripe_id else {
                                return methods::standard_replies::internal_server_error_response(
                                    "agreement/check-in: Database connection error at loading renter stripe_id",
                                )
                                .await
                            };

                            let description = "RSVP #".to_owned() + &*agreement_to_be_checked_in.confirmation.clone();
                            let new_charge_result = integration::stripe_veygo::create_payment_intent(
                                &stripe_id, &p_id, int_to_capture, PaymentIntentCaptureMethod::Automatic, &description
                            ).await;
                            if let Err(e) = new_charge_result {
                                return match e {
                                    VeygoError::CardDeclined => {
                                        methods::standard_replies::card_declined()
                                    }
                                    _ => {
                                        methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: Stripe error at creating payment intent",
                                        )
                                        .await
                                    }
                                }
                            }
                            let new_charge = new_charge_result.unwrap();
                            let p_status: model::PaymentType = new_charge.status.into();
                            match p_status {
                                model::PaymentType::Canceled => {
                                    return methods::standard_replies::card_declined()
                                }
                                model::PaymentType::RequiresPaymentMethod => {
                                    return methods::standard_replies::card_declined()
                                }
                                model::PaymentType::Succeeded => {
                                    let new_payment = model::NewPayment {
                                        payment_type: model::PaymentType::Succeeded,
                                        amount: Decimal::new(new_charge.amount_received, 2),
                                        note: None,
                                        reference_number: Some(new_charge.id.to_string()),
                                        agreement_id: agreement_to_be_checked_in.id,
                                        renter_id: agreement_to_be_checked_in.renter_id,
                                        payment_method_id: Some(agreement_to_be_checked_in.payment_method_id),
                                        amount_authorized: Decimal::new(new_charge.amount, 2),
                                        capture_before: None,
                                        is_deposit: false,
                                    };

                                    let insert_result = diesel::insert_into(p_q::payments)
                                        .values(&new_payment)
                                        .execute(&mut pool);

                                    if insert_result.is_err() {
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: SQL error at inserting new payment",
                                        )
                                        .await
                                    }
                                }
                                _ => {
                                    return methods::standard_replies::internal_server_error_response(
                                        "agreement/check-in: Stripe error at creating payment intent",
                                    )
                                    .await
                                }
                            }
                        }

                    } else if total_captured_amount == total_cost {
                        let all_pm_id_needed_to_be_completed = p_q::payments
                            .filter(p_q::agreement_id.eq(&agreement_to_be_checked_in.id))
                            .filter(p_q::payment_type.eq(model::PaymentType::RequiresCapture))
                            .filter(p_q::reference_number.is_not_null())
                            .select((p_q::id, p_q::reference_number, p_q::amount))
                            .get_results::<(i32, Option<String>, Decimal)>(&mut pool);
                        if all_pm_id_needed_to_be_completed.is_err() {
                            return methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: Database connection error at loading payments requiring completion",
                            )
                            .await
                        }

                        let all_pm_id_needed_to_be_completed = all_pm_id_needed_to_be_completed.unwrap();
                        for pm in all_pm_id_needed_to_be_completed {
                            let pi_id = pm.1.unwrap();
                            let pm_id = pm.0;
                            let pi = if !pm.2.is_zero() {
                                let result = integration::stripe_veygo::capture_payment(&pi_id, Some((0, true))).await;
                                if result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        "agreement/check-in: Stripe error at finalizing capture",
                                    )
                                    .await
                                }
                                result.unwrap()
                            } else {
                                let result = integration::stripe_veygo::drop_auth(&pi_id).await;
                                if result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        "agreement/check-in: Stripe error at dropping authorization",
                                    )
                                    .await
                                }
                                result.unwrap()
                            };
                            let result = diesel::update(p_q::payments.find(&pm_id))
                                .set(p_q::payment_type.eq::<model::PaymentType>(pi.status.into()))
                                .execute(&mut pool);

                            match result {
                                Ok(count) => {
                                    if count == 0 {
                                        return methods::standard_replies::internal_server_error_response(
                                            "agreement/check-in: SQL error at updating payment after capture (no rows updated)",
                                        )
                                        .await
                                    }
                                }
                                Err(_) => {
                                    return methods::standard_replies::internal_server_error_response(
                                        "agreement/check-in: Database connection error at updating payment after capture",
                                    )
                                    .await
                                }
                            }
                        }
                    } else {
                        // TODO: refund over capture
                    }

                    // save checked in status
                    let save_result = agreement_to_be_checked_in.save_changes::<model::Agreement>(&mut pool);
                    match save_result {
                        Ok(ag) => {
                            methods::standard_replies::response_with_obj::<model::Agreement>(ag, StatusCode::OK)
                        }
                        Err(_err) => {
                            methods::standard_replies::internal_server_error_response(
                                "agreement/check-in: SQL error at saving agreement check-in",
                            )
                            .await
                        }
                    }
                }
            }
        })
}