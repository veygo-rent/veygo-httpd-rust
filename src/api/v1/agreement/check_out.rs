use crate::{POOL, methods, model, helper_model, integration};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;
use chrono::{DateTime, Datelike, Duration, Utc};
use diesel::result::Error;
use stripe_core::PaymentIntentCaptureMethod;
use rust_decimal::prelude::*;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("check-out")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: helper_model::CheckOutRequest, auth: String, user_agent: String| {

            // Checking method is POST
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }

            if body.agreement_id <= 0 || body.hours_using_reward < Decimal::zero() || body.vehicle_snapshot_id <= 0 {
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
                        helper_model::VeygoError::TokenFormatError => {
                            methods::tokens::token_not_hex_warp_return()
                        }
                        helper_model::VeygoError::InvalidToken => {
                            methods::tokens::token_invalid_return()
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-out: Token verification unexpected error"),
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
                                    String::from("agreement/check-out: Token extension failed (returned false)"),
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-out: Token extension error"),
                            );
                        }
                    }

                    let mut pool = POOL.get().unwrap();

                    let five_minutes_ago = Utc::now() - Duration::minutes(5);

                    use crate::schema::agreements::dsl as agreement_q;
                    use crate::schema::vehicle_snapshots::dsl as v_s_q;
                    let ag_v_s_result = v_s_q::vehicle_snapshots
                        .inner_join(
                            agreement_q::agreements.on(
                                v_s_q::vehicle_id.eq(agreement_q::vehicle_id)
                                    .and(v_s_q::renter_id.eq(agreement_q::renter_id))
                            )
                        )
                        .filter(agreement_q::renter_id.eq(&access_token.user_id))
                        .filter(v_s_q::id.eq(&body.vehicle_snapshot_id))
                        .filter(agreement_q::id.eq(&body.agreement_id))
                        .filter(agreement_q::actual_pickup_time.is_null())
                        .filter(agreement_q::actual_drop_off_time.is_null())
                        .filter(v_s_q::time.ge(agreement_q::rsvp_pickup_time))
                        .filter(v_s_q::time.lt(agreement_q::rsvp_drop_off_time))
                        .filter(v_s_q::time.ge(five_minutes_ago))
                        .select((agreement_q::agreements::all_columns(), v_s_q::vehicle_snapshots::all_columns()))
                        .get_result::<(model::Agreement, model::VehicleSnapshot)>(&mut pool);

                    if let Err(e) = ag_v_s_result {
                        return match e {
                            Error::NotFound => {
                                methods::standard_replies::agreement_not_allowed_response()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-out: Database error loading agreement and vehicle snapshot"),
                                )
                            }
                        }
                    }

                    let (mut agreement_to_be_checked_out, check_out_snapshot) = ag_v_s_result.unwrap();

                    let current_user = methods::user::get_user_by_id(&access_token.user_id).await;
                    if current_user.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-out: Database error loading renter"),
                        );
                    }
                    let current_user = current_user.unwrap();

                    // TODO: Calculate total cost
                    // 0. verify total reward hours

                    let hours_reward_used = body.hours_using_reward;
                    let total_duration = agreement_to_be_checked_out.rsvp_drop_off_time - agreement_to_be_checked_out.rsvp_pickup_time;
                    let mut billable_duration = total_duration;

                    // Check if the renter has enough free hours to check in when using reward
                    if body.hours_using_reward > Decimal::zero() {
                        let total_duration_in_hours = Decimal::new(total_duration.num_minutes(), 0)
                            / Decimal::new(60, 0);

                        if hours_reward_used > total_duration_in_hours {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Check Out Not Allowed"),
                                message: String::from("You are trying to redeem more hours than the rental period."),
                            };
                            // RETURN: FORBIDDEN
                            return Ok((with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN).into_response(),));
                        }

                        use crate::schema::apartments::dsl as a_q;

                        let apartment: model::Apartment = a_q::apartments
                            .filter(a_q::id.eq(&current_user.apartment_id))
                            .get_result::<model::Apartment>(&mut pool)
                            .unwrap();

                        let current_time = Utc::now();
                        let current_naive_date = chrono::NaiveDate::from_ymd_opt(current_time.year(), current_time.month(), current_time.day()).unwrap();
                        let user_plan_renew_date = current_user.plan_renewal_date();

                        let is_active_plan = match user_plan_renew_date {
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-out: Plan renewal date parse error"),
                                );
                            },
                            Ok(date) => date >= current_naive_date
                        };

                        let renter_allowed_total_free_hours: Decimal = match current_user.plan_tier {
                            model::PlanTier::Free => apartment.free_tier_hours,
                            model::PlanTier::Silver => {
                                if is_active_plan {
                                    apartment.silver_tier_hours.unwrap_or(apartment.free_tier_hours)
                                } else {
                                    apartment.free_tier_hours
                                }
                            },
                            model::PlanTier::Gold => {
                                if is_active_plan {
                                    apartment.gold_tier_hours.unwrap_or(apartment.silver_tier_hours.unwrap_or(apartment.free_tier_hours))
                                } else {
                                    apartment.free_tier_hours
                                }
                            },
                            model::PlanTier::Platinum => {
                                if is_active_plan {
                                    apartment.platinum_tier_hours.unwrap_or(apartment.gold_tier_hours.unwrap_or(apartment.silver_tier_hours.unwrap_or(apartment.free_tier_hours)))
                                } else {
                                    apartment.free_tier_hours
                                }
                            }
                        };

                        if renter_allowed_total_free_hours < body.hours_using_reward {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Check Out Not Allowed"),
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
                                String::from("agreement/check-out: Database error summing used reward hours"),
                            )
                        }
                        let used_free_hours = used_free_hours.unwrap().unwrap_or(Decimal::zero());

                        if renter_allowed_total_free_hours - used_free_hours < body.hours_using_reward {
                            let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                title: String::from("Check Out Not Allowed"),
                                message: String::from("You have exceeded your free hours limit. Please upgrade your plan. "),
                            };
                            // RETURN: FORBIDDEN
                            return Ok((with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN).into_response(),));
                        }
                        billable_duration = methods::rental_rate::calculate_duration_after_reward(billable_duration, hours_reward_used);
                    }

                    // 1. total rental revenue
                    let billable_days_count: i32 = methods::rental_rate::billable_days_count(total_duration);

                    let calculated_duration_hours: Decimal = methods::rental_rate::calculate_billable_duration_hours(billable_duration);

                    // subtracts promos
                    let duration_revenue = calculated_duration_hours * agreement_to_be_checked_out.duration_rate * agreement_to_be_checked_out.msrp_factor * agreement_to_be_checked_out.utilization_factor;

                    let mut duration_revenue_after_discount = duration_revenue;

                    if let Some(manual_promo) = agreement_to_be_checked_out.manual_discount {
                        duration_revenue_after_discount -= manual_promo;
                    }
                    if let Some(promo_str) = agreement_to_be_checked_out.promo_id.clone() {
                        use crate::schema::promos::dsl as promo_q;
                        let amount_result = promo_q::promos
                            .find(&promo_str)
                            .select(promo_q::amount)
                            .get_result::<Decimal>(&mut pool);

                        if amount_result.is_err() {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-out: Database error loading promo amount"),
                            )
                        }
                        duration_revenue_after_discount -= amount_result.unwrap();
                    }

                    // TODO: calculate insurance premium
                    let insurance_premium = Decimal::zero();



                    use crate::schema::agreements_taxes::dsl as agreement_tax_q;
                    use crate::schema::taxes::dsl as tax_q;
                    use crate::schema::payments::dsl as payment_q;
                    use crate::schema::payment_methods::dsl as payment_method_q;
                    use crate::schema::renters::dsl as renter_q;

                    // 2. add mileage package cost
                    let mp_cost = if let Some(mileage_package_id) = agreement_to_be_checked_out.mileage_package_id {
                        use crate::schema::mileage_packages::dsl as mp_q;
                        let mileage_package = mp_q::mileage_packages
                            .find(mileage_package_id)
                            .get_result::<model::MileagePackage>(&mut pool)
                            .unwrap();
                        let base_rate_for_mp = if let Some(overwrite) = agreement_to_be_checked_out.mileage_package_overwrite {
                            overwrite
                        } else {
                            agreement_to_be_checked_out.duration_rate * agreement_to_be_checked_out.msrp_factor * agreement_to_be_checked_out.mileage_conversion
                        };

                        base_rate_for_mp * Decimal::new(mileage_package.miles as i64, 0) * Decimal::new(mileage_package.discounted_rate as i64, 2)
                    } else {
                        Decimal::zero()
                    };

                    // 3. add taxes
                    let taxable = duration_revenue_after_discount + insurance_premium + mp_cost;

                    let taxes = agreement_tax_q::agreements_taxes
                        .inner_join(tax_q::taxes)
                        .filter(agreement_tax_q::agreement_id.eq(&agreement_to_be_checked_out.id))
                        .select(tax_q::taxes::all_columns())
                        .get_results::<model::Tax>(&mut pool);

                    if taxes.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from( "agreement/check-out: Database error loading agreement taxes"),
                        )
                    }
                    let taxes = taxes.unwrap();

                    let mut daily_taxes = Decimal::zero();
                    let mut percent_taxes = Decimal::zero();
                    let mut fixed_taxes = Decimal::zero();

                    for tax in taxes {
                        match tax.tax_type {
                            model::TaxType::Fixed => {
                                fixed_taxes += tax.multiplier;
                            },
                            model::TaxType::Percent => {
                                percent_taxes += taxable * tax.multiplier;
                            },
                            model::TaxType::Daily => {
                                daily_taxes += Decimal::new(billable_days_count as i64, 0) * tax.multiplier;
                            }
                        }
                    }

                    let payment_method_token: String = payment_method_q::payment_methods
                        .find(agreement_to_be_checked_out.payment_method_id)
                        .select(payment_method_q::token)
                        .get_result(&mut pool)
                        .unwrap();

                    let user_stripe_id = renter_q::renters
                        .find(&agreement_to_be_checked_out.renter_id)
                        .select(renter_q::stripe_id)
                        .get_result::<String>(&mut pool);

                    if user_stripe_id.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from( "agreement/check-out: Database error loading renter stripe_id"),
                        )
                    }
                    let user_stripe_id = user_stripe_id.unwrap();

                    let total_should_bill = (duration_revenue_after_discount + mp_cost
                        + insurance_premium + daily_taxes + percent_taxes + fixed_taxes).round_dp(2);

                    let total_after_deposit = total_should_bill + Decimal::new(10000, 2);
                    let total_after_deposit_in_int = total_after_deposit.mantissa() as i64;

                    let description = "RSVP #".to_owned() + &*agreement_to_be_checked_out.confirmation.clone();

                    let stripe_auth = integration::stripe_veygo::create_payment_intent(
                        &user_stripe_id, &payment_method_token, total_after_deposit_in_int, PaymentIntentCaptureMethod::Manual, &description
                    ).await;

                    return match stripe_auth {
                        Err(error) => {
                            match error {
                                helper_model::VeygoError::CardDeclined => {
                                    methods::standard_replies::card_declined()
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response(
                                        String::from( "agreement/check-out: Stripe error creating payment intent"),
                                    )
                                }
                            }
                        }
                        Ok(pmi) => {
                            // Assign the snapshot, and the check-out time to the agreement record
                            agreement_to_be_checked_out.vehicle_snapshot_before = Some(check_out_snapshot.id);
                            agreement_to_be_checked_out.actual_pickup_time = Some(Utc::now());

                            {
                                // Create a payment record in the database
                                let new_payment = model::NewPayment {
                                    payment_type: pmi.clone().status.into(),
                                    amount: Decimal::zero(),
                                    note: Some("Reservation charge".to_string()),
                                    reference_number: Some(pmi.id.to_string()),
                                    agreement_id: agreement_to_be_checked_out.id,
                                    renter_id: agreement_to_be_checked_out.renter_id.clone(),
                                    payment_method_id: Some(agreement_to_be_checked_out.payment_method_id.clone()),
                                    amount_authorized: Decimal::new(pmi.amount, 2),
                                    capture_before: Option::from(methods::timestamps::from_seconds(pmi.clone().latest_charge.unwrap().into_object().unwrap().payment_method_details.unwrap().card.unwrap().capture_before.unwrap())),
                                    is_deposit: false,
                                };
                                let payment_result = diesel::insert_into(payment_q::payments)
                                    .values(&new_payment)
                                    .get_result::<model::Payment>(&mut pool);
                                if payment_result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-out: SQL error inserting reservation payment"),
                                    )
                                }
                            }

                            // Create a reward transaction record in the database if applicable
                            if body.hours_using_reward > Decimal::zero() {
                                let new_reward_transaction = model::NewRewardTransaction{
                                    agreement_id: Some(agreement_to_be_checked_out.id.clone()),
                                    duration: body.hours_using_reward,
                                    renter_id: agreement_to_be_checked_out.renter_id.clone(),
                                };
                                use crate::schema::reward_transactions::dsl as reward_q;
                                let reward_result = diesel::insert_into(reward_q::reward_transactions)
                                    .values(&new_reward_transaction)
                                    .get_result::<model::RewardTransaction>(&mut pool);
                                if reward_result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-out: SQL error inserting reward transaction"),
                                    )
                                }
                            }

                            // save the agreement record
                            let updated_agreement = diesel::update
                                (
                                    agreement_q::agreements
                                        .find(&agreement_to_be_checked_out.id)
                                )
                                .set(&agreement_to_be_checked_out)
                                .get_result::<model::Agreement>(&mut pool);

                            let Ok(updated_agreement) = updated_agreement else {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-out: SQL error saving agreement check-out"),
                                )
                            };

                            // Drop auth charges
                            let payments = payment_q::payments
                                .filter(payment_q::agreement_id.eq(&agreement_to_be_checked_out.id))
                                .filter(payment_q::payment_type.eq(model::PaymentType::RequiresCapture))
                                .filter(payment_q::is_deposit)
                                .filter(payment_q::payment_method_id.is_not_null())
                                .select((payment_q::id, payment_q::reference_number))
                                .get_results::<(i32, Option<String>)>(&mut pool);

                            if payments.is_err() {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-out: Database error loading payments requiring drop_auth"),
                                )
                            }
                            let payments = payments.unwrap();

                            for payment in payments {
                                let pi_id = payment.1.unwrap();
                                let drop_result = integration::stripe_veygo::drop_auth(&pi_id).await;
                                if drop_result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-out: Stripe error dropping authorization"),
                                    )
                                }
                            }

                            // Unlock vehicle
                            use crate::schema::vehicles::dsl as v_q;
                            let result = v_q::vehicles
                                .find(&agreement_to_be_checked_out.vehicle_id)
                                .select((v_q::remote_mgmt, v_q::remote_mgmt_id))
                                .get_result::<(model::RemoteMgmtType, String)>(&mut pool);

                            let Ok((vehicle_remote_mgmt, mgmt_id)) = result else {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-out: Database error loading vehicle remote mgmt info"),
                                )
                            };

                            match vehicle_remote_mgmt {
                                model::RemoteMgmtType::Tesla => {
                                    let _handler = tokio::spawn(async move {
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
                                        let cmd_path = format!("/api/1/vehicles/{}/command/door_unlock", mgmt_id);
                                        let _result = integration::tesla_curl::tesla_make_request(Method::POST, &cmd_path, None).await;
                                    });
                                }
                                _ => {}
                            }

                            methods::standard_replies::response_with_obj(updated_agreement, StatusCode::OK)
                        }
                    }
                }
            }
        })
}