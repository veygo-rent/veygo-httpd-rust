use std::str::FromStr;
use crate::{POOL, methods, model, helper_model, integration};
use diesel::prelude::*;
use diesel::expression_methods::NullableExpressionMethods;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;
use chrono::{DateTime, Datelike, Duration, Utc};
use hex::FromHexError;
use stripe::{ErrorType, PaymentIntentCaptureMethod, StripeError, PaymentIntentId};
use serde::Deserialize;

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
                        use crate::schema::vehicle_snapshots::dsl as v_s_q;

                        let five_minutes_ago = Utc::now() - Duration::minutes(5);

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
                            .filter(agreement_q::actual_pickup_time.is_not_null())
                            .filter(agreement_q::actual_drop_off_time.is_null())
                            .filter(v_s_q::time.ge(agreement_q::actual_pickup_time.assume_not_null()))
                            .filter(v_s_q::time.ge(five_minutes_ago))
                            .select((agreement_q::agreements::all_columns(), v_s_q::vehicle_snapshots::all_columns()))
                            .get_result::<(model::Agreement, model::VehicleSnapshot)>(&mut pool);

                        if ag_v_s_result.is_err() {
                            return methods::standard_replies::agreement_not_allowed_response(new_token_in_db_publish.clone())
                        }

                        let (mut agreement_to_be_checked_in, check_in_snapshot) = ag_v_s_result.unwrap();

                        // lock the vehicle

                        use crate::schema::vehicles::dsl as v_q;
                        let (vehicle_remote_mgmt, mgmt_id, vin_num) = v_q::vehicles
                            .find(&agreement_to_be_checked_in.vehicle_id)
                            .select((v_q::remote_mgmt, v_q::remote_mgmt_id, v_q::vin))
                            .get_result::<(model::RemoteMgmtType, String, String)>(&mut pool)
                            .unwrap();

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

                                if result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                                }

                                let resp = result.unwrap();
                                if resp.status() != StatusCode::OK {
                                    return methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                                }
                            }
                            _ => {}
                        }

                        // update agreement status to check in

                        agreement_to_be_checked_in.actual_drop_off_time = Some(Utc::now());
                        agreement_to_be_checked_in.vehicle_snapshot_after = Some(check_in_snapshot.id);
                        agreement_to_be_checked_in.save_changes::<model::Agreement>(&mut pool).unwrap();

                        // map unmapped charges to this agreement
                        // 1. current charges in the database

                        use crate::schema::charges::dsl as c_q;

                        let pickup: DateTime<Utc> = agreement_to_be_checked_in.actual_pickup_time.expect("pickup must exist");
                        let drop_off: DateTime<Utc> = agreement_to_be_checked_in.actual_drop_off_time.expect("drop off must exist");

                        diesel::update(c_q::charges)
                            .filter(c_q::agreement_id.is_null())
                            .filter(c_q::vehicle_id.eq(&agreement_to_be_checked_in.vehicle_id))
                            .filter(c_q::time.ge(&pickup))
                            .filter(c_q::time.le(&drop_off))
                            .set(c_q::agreement_id.eq(Some(&agreement_to_be_checked_in.id)))
                            .execute(&mut pool)
                            .unwrap();

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

                            if result.is_err() {
                                return methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                            }

                            let resp = result.unwrap();
                            if resp.status() != StatusCode::OK {
                                return methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                            }

                            let body_text = match resp.text().await {
                                Ok(t) => t,
                                Err(_) => {
                                    return methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                                }
                            };

                            let parsed: TeslaChargingSessionsResponse = match serde_json::from_str(&body_text) {
                                Ok(p) => p,
                                Err(_) => {
                                    return methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                                }
                            };

                            if parsed.status_code != 1000 {
                                return methods::standard_replies::internal_server_error_response(new_token_in_db_publish.clone())
                            }

                            let sessions_min: Vec<(DateTime<Utc>, String, f64, f64)> = parsed
                                .data
                                .into_iter()
                                .map(|s| (s.start_date_time, s.location.name, s.total_cost.excl_vat, s.total_cost.incl_vat))
                                .collect();

                            for (session_time, location, _excl_vat, incl_vat) in sessions_min {
                                let charging_note = format!("Tesla charging at {}", location);
                                let new_charge = model::NewCharge{
                                    name: charging_note,
                                    time: session_time,
                                    amount: incl_vat,
                                    note: None,
                                    agreement_id: Some(agreement_to_be_checked_in.id),
                                    vehicle_id: agreement_to_be_checked_in.vehicle_id,
                                    transponder_company_id: None,
                                    vehicle_identifier: None,
                                };

                                use crate::schema::charges::dsl as c_q;

                                let _ = diesel::insert_into(c_q::charges)
                                    .values(&new_charge)
                                    .execute(&mut pool);
                            }
                        }

                        // TODO: calculate total cost
                        // 1. total rental revenue before late return
                        // 2. total late return fee
                        // 3. total charges
                        // 4. total taxes
                        // 5. total cost = 1 + 2 + 3 + 4
                        
                        // TODO: capture the correct amount and process additional charges
                        // 1. calculate amount 

                        methods::standard_replies::not_implemented_response()
                    }
                }
                Err(_) => methods::tokens::token_not_hex_warp_return()
            }
        })
}