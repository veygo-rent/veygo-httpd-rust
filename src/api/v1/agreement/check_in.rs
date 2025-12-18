use crate::{POOL, methods, model, helper_model};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;
use chrono::{DateTime, Datelike, Duration, Utc};

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

                        let (agreement_to_be_checked_out, check_out_snapshot) = ag_v_s_result.unwrap();

                        let current_user = methods::user::get_user_by_id(&new_token.user_id).await.unwrap();

                        // Check if the renter has enough free hours to check in when using reward
                        if body.hours_using_reward <= 0.0 {
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
                        }

                        methods::standard_replies::not_implemented_response()
                    }
                }
            }
        })
}