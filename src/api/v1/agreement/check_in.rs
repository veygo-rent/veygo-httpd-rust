use std::str::FromStr;
use crate::{POOL, methods, model, helper_model, integration};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;
use chrono::{DateTime, Datelike, Duration, Utc};
use stripe::{ErrorType, PaymentIntentCaptureMethod, StripeError, PaymentIntentId};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("check-in")
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

            if body.agreement_id <= 0 || body.hours_using_reward < 0.0 || body.vehicle_snapshot_id <= 0 {
                return methods::standard_replies::bad_request("Bad request: wrong parameters. ")
            }

            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_wrapped_return();
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => int,
                Err(_) => {
                    return methods::tokens::token_invalid_wrapped_return();
                }
            };

            let access_token = model::RequestToken {
                user_id,
                token: token_and_id[0].parse().unwrap(),
            };
            let if_token_valid =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                    .await;

            return match if_token_valid {
                Err(_) => methods::tokens::token_not_hex_warp_return(),
                Ok(token_is_valid) => {
                    if !token_is_valid {
                        methods::tokens::token_invalid_wrapped_return()
                    } else {
                        // token is valid
                        let token_clone = access_token.clone();
                        methods::tokens::rm_token_by_binary(
                            hex::decode(token_clone.token).unwrap(),
                        ).await;

                        let new_token = methods::tokens::gen_token_object(
                            &access_token.user_id,
                            &user_agent,
                        ).await;

                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.get().unwrap();
                        let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                            .values(&new_token)
                            .get_result::<model::AccessToken>(&mut pool)
                            .unwrap()
                            .into();

                        use crate::schema::agreements::dsl as agreement_q;
                        use crate::schema::apartments::dsl as a_q;
                        use crate::schema::vehicle_snapshots::dsl as v_s_q;
                        let ag_v_s_result = v_s_q::vehicle_snapshots
                            .inner_join(
                                agreement_q::agreements.on(
                                v_s_q::vehicle_id.eq(agreement_q::vehicle_id)
                                    .and(v_s_q::renter_id.eq(agreement_q::renter_id))
                                )
                            )
                            .filter(agreement_q::renter_id.eq(&new_token.user_id))
                            .filter(v_s_q::id.eq(&body.vehicle_snapshot_id))
                            .filter(agreement_q::id.eq(&body.agreement_id))
                            .filter(v_s_q::time.ge(agreement_q::rsvp_pickup_time))
                            .filter(v_s_q::time.lt(agreement_q::rsvp_drop_off_time))
                            .select((agreement_q::agreements::all_columns(), v_s_q::vehicle_snapshots::all_columns()))
                            .get_result::<(model::Agreement, model::VehicleSnapshot)>(&mut pool);

                        if ag_v_s_result.is_err() {
                            return methods::standard_replies::agreement_not_allowed_response(new_token_in_db_publish.clone())
                        }

                        let (mut agreement_to_be_checked_out, check_out_snapshot) = ag_v_s_result.unwrap();

                        let current_user = methods::user::get_user_by_id(&new_token.user_id).await.unwrap();

                        let hours_reward_used = body.hours_using_reward;
                        let total_duration = agreement_to_be_checked_out.rsvp_drop_off_time - agreement_to_be_checked_out.rsvp_pickup_time;
                        let mut billable_duration = total_duration;

                        // Check if the renter has enough free hours to check in when using reward
                        if body.hours_using_reward > 0.0 {
                            let total_duration_in_hours = total_duration.num_minutes() as f64 / 60.0;

                            if hours_reward_used > total_duration_in_hours {
                                let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Check Out Not Allowed"),
                                    message: String::from("You are trying to redeem more hours than the rental period."),
                                };
                                // RETURN: FORBIDDEN
                                return Ok::<_, Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN)),));
                            }

                            let apartment: model::Apartment = a_q::apartments
                                .filter(a_q::id.eq(&current_user.apartment_id))
                                .get_result::<model::Apartment>(&mut pool)
                                .unwrap();

                            let current_time = Utc::now();
                            let current_naive_date = chrono::NaiveDate::from_ymd_opt(current_time.year(), current_time.month(), current_time.day()).unwrap();
                            let user_plan_renew_date = methods::user::user_plan_renewal_date(&current_user);

                            let is_active_plan = match user_plan_renew_date {
                                None => false,
                                Some(date) => date >= current_naive_date
                            };

                            let renter_allowed_total_free_hours: f64 = match current_user.plan_tier {
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
                                    message: String::from("You have exceeded your free hours limit. Please contact us to upgrade your plan. "),
                                };
                                // RETURN: FORBIDDEN
                                return Ok::<_, Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN)),));
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
                            let used_free_hours: f64 = reward_q::reward_transactions
                                .filter(reward_q::renter_id.eq(&new_token.user_id))
                                .filter(reward_q::transaction_time.ge(week_start))
                                .filter(reward_q::transaction_time.lt(week_end_exclusive))
                                .select(diesel::dsl::sum(reward_q::duration))
                                .first::<Option<f64>>(&mut pool)
                                .unwrap_or(None)
                                .unwrap_or(0.0);

                            if renter_allowed_total_free_hours - used_free_hours < body.hours_using_reward {
                                let err_msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Check Out Not Allowed"),
                                    message: String::from("You have exceeded your free hours limit. Please contact us to upgrade your plan. "),
                                };
                                // RETURN: FORBIDDEN
                                return Ok::<_, Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&err_msg), StatusCode::FORBIDDEN)),));
                            }
                            // Subtract reward time safely (prevent negative billable duration due to rounding)
                            let total_minutes: i64 = billable_duration.num_minutes().max(0);
                            let mut reward_minutes: i64 = (hours_reward_used.max(0.0) * 60.0).round() as i64;
                            if reward_minutes > total_minutes {
                                reward_minutes = total_minutes;
                            }
                            billable_duration = Duration::minutes(total_minutes - reward_minutes);
                        }

                        let billable_duration_hours: f64 = billable_duration.num_minutes() as f64 / 60.0;
                        let billable_days_count: i32 = (billable_duration_hours / 24.0).ceil() as i32;

                        // Tiered billing:
                        // - First 8 hours are billed 1:1
                        // - Hours after 8 up to the end of the first week (168 hours total) are billed at 0.25 per hour
                        // - Hours after 168 are billed at 0.15 per hour
                        let calculated_duration_hours: f64 = {
                            let h = billable_duration_hours.max(0.0);

                            // Tier 1: first 8 hours at 1x
                            let tier1_hours = h.min(8.0);

                            // Tier 2: from hour 9 up to hour 168 (i.e., next 160 hours) at 0.25x
                            let tier2_hours = (h - 8.0).clamp(0.0, 160.0);

                            // Tier 3: beyond 168 hours at 0.15x
                            let tier3_hours = (h - 168.0).max(0.0);

                            tier1_hours + (tier2_hours * 0.25) + (tier3_hours * 0.15)
                        };

                        let duration_revenue = calculated_duration_hours * agreement_to_be_checked_out.duration_rate * agreement_to_be_checked_out.msrp_factor * agreement_to_be_checked_out.utilization_factor;

                        use crate::schema::agreements_taxes::dsl as agreement_tax_q;
                        use crate::schema::taxes::dsl as tax_q;
                        use crate::schema::payments::dsl as payment_q;
                        use crate::schema::payment_methods::dsl as payment_method_q;
                        use crate::schema::renters::dsl as renter_q;

                        let taxes: Vec<model::Tax> = agreement_tax_q::agreements_taxes
                            .inner_join(tax_q::taxes)
                            .filter(agreement_tax_q::agreement_id.eq(&agreement_to_be_checked_out.id))
                            .select(tax_q::taxes::all_columns())
                            .get_results::<model::Tax>(&mut pool)
                            .unwrap_or_default();

                        let mut total_revenue: f64 = duration_revenue;

                        if let Some(mileage_package_id) = agreement_to_be_checked_out.mileage_package_id {
                            use crate::schema::mileage_packages::dsl as mp_q;
                            let mileage_package = mp_q::mileage_packages
                                .find(mileage_package_id)
                                .get_result::<model::MileagePackage>(&mut pool)
                                .unwrap();
                            let base_rate_for_mp: f64;
                            if let Some(overwrite) = agreement_to_be_checked_out.mileage_package_overwrite {
                                base_rate_for_mp = overwrite;
                            } else {
                                base_rate_for_mp = agreement_to_be_checked_out.duration_rate * agreement_to_be_checked_out.msrp_factor * agreement_to_be_checked_out.mileage_conversion;
                            }
                            total_revenue = total_revenue + base_rate_for_mp * mileage_package.miles as f64 * mileage_package.discounted_rate as f64 / 100.0;
                        }

                        let mut daily_taxes: f64 = 0.0;
                        let mut percent_taxes: f64 = 0.0;
                        let mut fixed_taxes: f64 = 0.0;

                        for tax in taxes {
                            match tax.tax_type {
                                model::TaxType::Fixed => {
                                    fixed_taxes = fixed_taxes + tax.multiplier;
                                },
                                model::TaxType::Percent => {
                                    percent_taxes = percent_taxes + total_revenue * tax.multiplier;
                                },
                                model::TaxType::Daily => {
                                    daily_taxes = daily_taxes + billable_days_count as f64 * tax.multiplier;
                                }
                            }
                        }

                        let payment_method_token: String = payment_method_q::payment_methods
                            .find(agreement_to_be_checked_out.payment_method_id)
                            .select(payment_method_q::token)
                            .get_result(&mut pool)
                            .unwrap();

                        let user_stripe_id: String = renter_q::renters
                            .find(&agreement_to_be_checked_out.renter_id)
                            .select(renter_q::stripe_id)
                            .get_result::<Option<String>>(&mut pool)
                            .unwrap()
                            .unwrap();

                        let total_should_bill: f64 = total_revenue + daily_taxes + percent_taxes + fixed_taxes;

                        let total_after_deposit: f64 = total_should_bill + 100.0;
                        let total_after_deposit_in_int = (total_after_deposit * 100.0).round() as i64;

                        let description = &("Temp Hold for Veygo Reservation #".to_owned() + &*agreement_to_be_checked_out.confirmation.clone());
                        let suffix: Option<&str> = Some(&*("RENTAL #".to_owned() + &*agreement_to_be_checked_out.confirmation.clone()));

                        let stripe_auth = integration::stripe_veygo::create_payment_intent(description, &user_stripe_id, &payment_method_token, &total_after_deposit_in_int, PaymentIntentCaptureMethod::Manual, suffix).await;

                        return match stripe_auth {
                            Err(error) => {
                                if let StripeError::Stripe(request_error) = error {
                                    eprintln!("Stripe API error: {:?}", request_error);
                                    if request_error.error_type == ErrorType::Card {
                                        return methods::standard_replies::card_declined_wrapped(new_token_in_db_publish);
                                    }
                                }
                                methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                            }
                            Ok(pmi) => {
                                // TODO: Unlock vehicle

                                use crate::schema::vehicles::dsl as v_q;
                                let (vehicle_remote_mgmt, mgmt_id) = v_q::vehicles
                                    .find(&agreement_to_be_checked_out.vehicle_id)
                                    .select((v_q::remote_mgmt, v_q::remote_mgmt_id))
                                    .get_result::<(model::RemoteMgmtType, String)>(&mut pool)
                                    .unwrap();

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
                                
                                let payments: Vec<(i32, Option<String>)> = payment_q::payments
                                    .filter(payment_q::agreement_id.eq(&agreement_to_be_checked_out.id))
                                    .filter(payment_q::payment_type.ne(model::PaymentType::Canceled))
                                    .filter(payment_q::is_deposit)
                                    .filter(payment_q::payment_method_id.is_not_null())
                                    .select((payment_q::id, payment_q::reference_number))
                                    .get_results(&mut pool)
                                    .unwrap_or_default();

                                // Create a payment record in the database

                                let new_payment = model::NewPayment {
                                    payment_type: pmi.status.into(),
                                    amount: 0.00,
                                    note: Some("Reservation charge".to_string()),
                                    reference_number: Some(pmi.id.to_string()),
                                    agreement_id: Some(agreement_to_be_checked_out.id.clone()),
                                    renter_id: agreement_to_be_checked_out.renter_id.clone(),
                                    payment_method_id: Some(agreement_to_be_checked_out.payment_method_id.clone()),
                                    amount_authorized: Option::from(total_after_deposit),
                                    capture_before: Option::from(methods::timestamps::from_seconds(pmi.clone().latest_charge.unwrap().into_object().unwrap().payment_method_details.unwrap().card.unwrap().capture_before.unwrap())),
                                    is_deposit: false,
                                };
                                let _payment_result = diesel::insert_into(payment_q::payments).values(&new_payment).get_result::<model::Payment>(&mut pool);

                                // Create a reward transaction record in the database if applicable

                                if body.hours_using_reward > 0.0 {
                                    let new_reward_transaction = model::NewRewardTransaction{
                                        agreement_id: Some(agreement_to_be_checked_out.id.clone()),
                                        duration: body.hours_using_reward,
                                        renter_id: agreement_to_be_checked_out.renter_id.clone(),
                                    };
                                    use crate::schema::reward_transactions::dsl as reward_q;
                                    let _reward_result = diesel::insert_into(reward_q::reward_transactions).values(&new_reward_transaction).get_result::<model::RewardTransaction>(&mut pool);
                                }

                                // Assign the snapshot, and the check-out time to the agreement record

                                agreement_to_be_checked_out.vehicle_snapshot_before = Some(check_out_snapshot.id);
                                agreement_to_be_checked_out.actual_pickup_time = Some(Utc::now());

                                let updated_agreement = diesel::update(agreement_q::agreements.find(&agreement_to_be_checked_out.id)).set(&agreement_to_be_checked_out).get_result::<model::Agreement>(&mut pool).unwrap();

                                // Drop auth charges

                                for payment in payments {
                                    let payment_id = PaymentIntentId::from_str(&payment.1.unwrap()).unwrap();
                                    let _drop_result = integration::stripe_veygo::drop_auth(&payment_id).await;
                                }

                                Ok::<_, Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&updated_agreement), StatusCode::OK)),))
                            }
                        }
                    }
                }
            }
        })
}