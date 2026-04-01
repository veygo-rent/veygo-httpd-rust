use chrono::{Datelike, Duration, Local, NaiveDate};
use warp::{Filter, Reply, http::Method, http::StatusCode};
use crate::{methods, model, POOL, schema};
use diesel::prelude::*;
use crate::helper_model::VeygoError;
use serde::Serialize;

#[derive(Serialize)]
struct RentersStats {
    total: i64,
    active: i64,
    active_paid: i64,
    pending_dl_approvals: i64,
    pending_lease_approvals: i64,
    pending_insurance_approvals: i64,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("renters")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, auth: String, user_agent: String| {
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

                match if_token_valid {
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
                                    String::from("admin/stats/renters: Token verification unexpected error"),
                                )
                            }
                        }
                    }
                    Ok((_token, token_id)) => {
                        let user = methods::user::get_user_by_id(&access_token.user_id)
                            .await;
                        let Ok(user) = user else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("admin/stats/renters: Database error loading renter by id"),
                            );
                        };

                        if !user.is_admin() {
                            return methods::standard_replies::user_not_admin()
                        }
                        if !user.is_operational_admin() {
                            return methods::standard_replies::admin_not_verified()
                        }

                        let result = methods::tokens::extend_token(token_id, &user_agent);
                        match result {
                            Err(_) => {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("admin/stats/renters: Token extension error"),
                                )
                            }
                            Ok(is_renewed) => {
                                if is_renewed {
                                    let mut pool = POOL.get().unwrap();

                                    use schema::renters::dsl as r_q;

                                    let total = r_q::renters
                                        .select(diesel::dsl::count_star())
                                        .get_result::<i64>(&mut pool);
                                    let Ok(total) = total else {
                                        return
                                            methods::standard_replies::internal_server_error_response(
                                                String::from("admin/stats/renters: DB error loading total renters count"),
                                            )
                                    };

                                    let today: NaiveDate = Local::now().date_naive();

                                    let today_year_i32: i32 = today.year();
                                    let today_month_i32: i32 = today.month() as i32;
                                    let today_day_i32: i32 = today.day() as i32;

                                    let first_day_of_next_month = if today_month_i32 == 12 {
                                        NaiveDate::from_ymd_opt(today_year_i32 + 1, 1, 1).unwrap()
                                    } else {
                                        NaiveDate::from_ymd_opt(today_year_i32, (today_month_i32 + 1) as u32, 1).unwrap()
                                    };

                                    let last_day_of_this_month_i32: i32 =
                                        (first_day_of_next_month - Duration::days(1)).day() as i32;

                                    let renter_renew_month_sql_int = diesel::dsl::sql::<diesel::sql_types::Integer>(
                                        "CAST(SUBSTRING(plan_expire_month_year, 1, 2) AS integer)"
                                    );
                                    let renter_renew_year_sql_int = diesel::dsl::sql::<diesel::sql_types::Integer>(
                                        "CAST(SUBSTRING(plan_expire_month_year, 3, 4) AS integer)"
                                    );

                                    let active_renters = r_q::renters
                                        .filter(
                                            renter_renew_year_sql_int.clone().gt(&today_year_i32)
                                                .or(
                                                    renter_renew_year_sql_int.eq(&today_year_i32)
                                                        .and(
                                                            renter_renew_month_sql_int.clone().gt(&today_month_i32)
                                                                .or(
                                                                    renter_renew_month_sql_int.clone().eq(&today_month_i32)
                                                                        .and(
                                                                            diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                                                                                "LEAST(CAST(plan_renewal_day AS integer), {}) >= {}",
                                                                                last_day_of_this_month_i32,
                                                                                today_day_i32,
                                                                            ))
                                                                        )
                                                                )
                                                        )
                                                )
                                        );

                                    let active = active_renters.clone()
                                        .select(diesel::dsl::count_star())
                                        .get_result::<i64>(&mut pool);
                                    let Ok(active) = active else {
                                        return
                                            methods::standard_replies::internal_server_error_response(
                                                String::from("admin/stats/renters: DB error loading active renters count"),
                                            )
                                    };

                                    let active_paid = active_renters
                                        .filter(r_q::plan_tier.ne(model::PlanTier::Free))
                                        .select(diesel::dsl::count_star())
                                        .get_result::<i64>(&mut pool);
                                    let Ok(active_paid) = active_paid else {
                                        return
                                            methods::standard_replies::internal_server_error_response(
                                                String::from("admin/stats/renters: DB error loading active renters count"),
                                            )
                                    };

                                    let pending_dl_approvals = r_q::renters
                                        .filter(r_q::drivers_license_expiration.is_null())
                                        .filter(r_q::drivers_license_image.is_not_null())
                                        .filter(
                                            r_q::requires_secondary_driver_lic.eq(false)
                                                .or(
                                                    r_q::requires_secondary_driver_lic.eq(true)
                                                        .and(r_q::drivers_license_image_secondary.is_not_null())
                                                )
                                        )
                                        .select(diesel::dsl::count_star())
                                        .get_result::<i64>(&mut pool);
                                    let Ok(pending_dl_approvals) = pending_dl_approvals else {
                                        return
                                            methods::standard_replies::internal_server_error_response(
                                                String::from("admin/stats/renters: DB error loading pending dl approval count"),
                                            )
                                    };

                                    let pending_lease_approvals = r_q::renters
                                        .filter(r_q::lease_agreement_expiration.is_null())
                                        .filter(r_q::lease_agreement_image.is_not_null())
                                        .select(diesel::dsl::count_star())
                                        .get_result::<i64>(&mut pool);
                                    let Ok(pending_lease_approvals) = pending_lease_approvals else {
                                        return
                                            methods::standard_replies::internal_server_error_response(
                                                String::from("admin/stats/renters: DB error loading pending lease approval count"),
                                            )
                                    };

                                    let pending_insurance_approvals = r_q::renters
                                        .filter(r_q::insurance_liability_expiration.is_null())
                                        .filter(r_q::insurance_id_image.is_not_null())
                                        .select(diesel::dsl::count_star())
                                        .get_result::<i64>(&mut pool);
                                    let Ok(pending_insurance_approvals) = pending_insurance_approvals else {
                                        return
                                            methods::standard_replies::internal_server_error_response(
                                                String::from("admin/stats/renters: DB error loading pending insurance approval count"),
                                            )
                                    };

                                    let msg = RentersStats {
                                        total,
                                        active,
                                        active_paid,
                                        pending_dl_approvals,
                                        pending_lease_approvals,
                                        pending_insurance_approvals,
                                    };

                                    methods::standard_replies::response_with_obj(&msg, StatusCode::OK)
                                } else {
                                    methods::standard_replies::internal_server_error_response(
                                        String::from("admin/stats/renters: Token extension failed (returned false)"),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        )
}