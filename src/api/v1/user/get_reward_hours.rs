use chrono::{DateTime, Datelike, Duration, Utc};
use warp::{Filter, Reply, http::{Method, StatusCode}};
use crate::{methods, model, schema, POOL, helper_model, helper_model::VeygoError};
use diesel::prelude::*;
use rust_decimal::prelude::*;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("reward-hour")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method,
                              auth: String,
                              user_agent: String| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response();
            }
            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_return();
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => {
                    int
                }
                Err(_) => {
                    return methods::tokens::token_invalid_return();
                }
            };

            let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
            let if_token_valid = methods::tokens::verify_user_token(
                &access_token.user_id,
                &access_token.token,
            ).await;
            
            return match if_token_valid {
                Err(err) => {
                    match err {
                        VeygoError::TokenFormatError => {
                            methods::tokens::token_not_hex_warp_return()
                        }
                        VeygoError::InvalidToken => {
                            methods::tokens::token_invalid_return()
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response(
                                String::from("user/get-reward-hours: Token verification unexpected error")
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
                                    String::from("user/get-reward-hours: Token extension failed (returned false)")
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("user/get-reward-hours: Token extension error")
                            );
                        }
                    };

                    let current_user = methods::user::get_user_by_id(&access_token.user_id).await;
                    if current_user.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("user/get-reward-hours: Database error loading renter"),
                        );
                    }
                    let current_user = current_user.unwrap();
                    
                    let user_plan_renew_date = current_user.plan_renewal_date();
                    
                    let current_time = Utc::now();
                    let current_naive_date = chrono::NaiveDate::from_ymd_opt(current_time.year(), current_time.month(), current_time.day()).unwrap();

                    let is_active_plan = match user_plan_renew_date {
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("user/get-reward-hours: Plan renewal date parse error"),
                            );
                        },
                        Ok(date) => date >= current_naive_date
                    };
                    
                    let mut pool = POOL.get().unwrap();

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

                    use schema::reward_transactions::dsl as reward_q;
                    let used_free_hours = reward_q::reward_transactions
                        .filter(reward_q::renter_id.eq(&access_token.user_id))
                        .filter(reward_q::transaction_time.ge(week_start))
                        .filter(reward_q::transaction_time.lt(week_end_exclusive))
                        .filter(reward_q::duration.gt(Decimal::zero()))
                        .select(diesel::dsl::sum(reward_q::duration))
                        .first::<Option<Decimal>>(&mut pool);
                    if used_free_hours.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("user/get-reward-hours: Database error summing used reward hours"),
                        )
                    }
                    let used_free_hours = used_free_hours.unwrap().unwrap_or(Decimal::zero());

                    let credit_hours = reward_q::reward_transactions
                        .filter(reward_q::renter_id.eq(&access_token.user_id))
                        .filter(reward_q::transaction_time.ge(week_start))
                        .filter(reward_q::transaction_time.lt(week_end_exclusive))
                        .filter(reward_q::duration.le(Decimal::zero()))
                        .select(diesel::dsl::sum(reward_q::duration))
                        .first::<Option<Decimal>>(&mut pool);
                    if credit_hours.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("user/get-reward-hours: Database error summing credited credit hours"),
                        )
                    }
                    let credit_hours = credit_hours.unwrap().unwrap_or(Decimal::zero());
                    
                    let msg = helper_model::RewardHoursSummaryResponse{ 
                        total: if is_active_plan { current_user.plan_total_availability + credit_hours } else { Decimal::zero() }, used: used_free_hours
                    };
                    
                    methods::standard_replies::response_with_obj(&msg, StatusCode::OK)
                }
            }
        })
}