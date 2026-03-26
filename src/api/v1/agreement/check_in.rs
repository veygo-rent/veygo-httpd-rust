use std::cmp::{max};
use crate::{POOL, methods, model, helper_model, integration, schema, proj_config};
use diesel::prelude::*;
use diesel::expression_methods::NullableExpressionMethods;
use warp::{Filter, Rejection, Reply, http::{Method, StatusCode}};
use chrono::{DateTime, Duration, Utc};
use diesel::result::Error;
use futures::{stream::FuturesUnordered, StreamExt};
use rust_decimal::prelude::*;
use sha2::{Sha256, Digest};
use stripe_core::{ PaymentIntentCaptureMethod };
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("check-in")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: helper_model::CheckInOutRequest, auth: String, user_agent: String| {

            let mut pool = POOL.get().unwrap();
            
            // Checking method is POST
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }

            let agreement_id = match body {
                helper_model::CheckInOutRequest::WithSnapshotId { agreement_id, .. } => { agreement_id }
                helper_model::CheckInOutRequest::WithImagePath { agreement_id, .. } => { agreement_id }
            };

            match body {
                helper_model::CheckInOutRequest::WithSnapshotId { agreement_id, vehicle_snapshot_id } => {
                    if agreement_id <= 0 || vehicle_snapshot_id <= 0 {
                        return methods::standard_replies::bad_request("wrong parameters. ")
                    }
                }
                helper_model::CheckInOutRequest::WithImagePath {
                    agreement_id,
                    ref left_image_path,
                    ref right_image_path,
                    ref front_image_path,
                    ref back_image_path,
                    ref front_right_image_path,
                    ref front_left_image_path,
                    ref back_right_image_path,
                    ref back_left_image_path,
                } => {
                    if agreement_id <= 0 {
                        return methods::standard_replies::bad_request("wrong parameters. ")
                    }

                    use schema::agreements::dsl as ag_q;
                    use schema::vehicles::dsl as veh_q;

                    let vin_num = ag_q::agreements
                        .find(&agreement_id)
                        .inner_join(veh_q::vehicles)
                        .select(veh_q::vin)
                        .get_result::<String>(&mut pool);

                    let vin_num = match vin_num {
                        Ok(vin) => { vin }
                        Err(err) => {
                            return match err {
                                Error::NotFound => {
                                    methods::standard_replies::bad_request("Loading vehicles failed")
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-in: DB Error loading VIN number"),
                                    )
                                }
                            }
                        }
                    };

                    let mut hasher = Sha256::new();
                    let data = vin_num.clone().into_bytes();
                    hasher.update(data);
                    let result = hasher.finalize();
                    let object_pwd: String = format!("vehicle_pictures/{:X}/", result);

                    let left_image_path: String = format!("{}{}", &object_pwd, &left_image_path);
                    let right_image_path: String = format!("{}{}", &object_pwd, &right_image_path);
                    let front_image_path: String = format!("{}{}", &object_pwd, &front_image_path);
                    let back_image_path: String = format!("{}{}", &object_pwd, &back_image_path);
                    let front_right_image_path: String = format!("{}{}", &object_pwd, &front_right_image_path);
                    let front_left_image_path: String = format!("{}{}", &object_pwd, &front_left_image_path);
                    let back_right_image_path: String = format!("{}{}", &object_pwd, &back_right_image_path);
                    let back_left_image_path: String = format!("{}{}", &object_pwd, &back_left_image_path);

                    // Validate that all referenced images exist in GCS (run checks concurrently)
                    let checks: [(String, String); 8] = [
                        ("Left Image".to_string(), left_image_path.clone()),
                        ("Right Image".to_string(), right_image_path.clone()),
                        ("Front Image".to_string(), front_image_path.clone()),
                        ("Back Image".to_string(), back_image_path.clone()),
                        ("Front-Right Image".to_string(), front_right_image_path.clone()),
                        ("Front-Left Image".to_string(), front_left_image_path.clone()),
                        ("Back-Right Image".to_string(), back_right_image_path.clone()),
                        ("Back-Left Image".to_string(), back_left_image_path.clone()),
                    ];

                    let mut futures = FuturesUnordered::new();
                    for (label, path) in checks {
                        futures.push(async move {
                            let ok = integration::gcloud_storage_veygo::check_exists(path.clone()).await;
                            (label, path, ok)
                        });
                    }

                    while let Some((label, _path, ok)) = futures.next().await {
                        if !ok {
                            return methods::standard_replies::bad_request(
                                &format!("{} does not exist", label),
                            );
                        }
                    }
                }
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
                            methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Database connection error at token verification"))
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: SQL error at extending token"));
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Database connection error at extending token"));
                        }
                    }

                    let vehicle_current_latitude: f64;
                    let vehicle_current_longitude: f64;

                    let vehicle_snapshot_id = match body {
                        helper_model::CheckInOutRequest::WithSnapshotId { vehicle_snapshot_id, .. } => {

                            use schema::agreements::dsl as ag_q;
                            use schema::vehicles::dsl as veh_q;

                            let vehicle = ag_q::agreements
                                .find(&agreement_id)
                                .inner_join(veh_q::vehicles)
                                .select(veh_q::vehicles::all_columns())
                                .get_result::<model::Vehicle>(&mut pool);

                            let Ok(mut vehicle) = vehicle else {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Loading vehicle error DB"),
                                );
                            };


                            let (_fuel, _odo, latitude, longitude) = match vehicle.remote_mgmt {
                                model::RemoteMgmtType::Tesla => {
                                    // 1) Check online state via GET /api/1/vehicles/{vehicle_tag}
                                    let status_path = format!("/api/1/vehicles/{}", vehicle.remote_mgmt_id);

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
                                                        let wake_path = format!("/api/1/vehicles/{}/wake_up", vehicle.remote_mgmt_id);
                                                        let _ = integration::tesla_curl::tesla_make_request(Method::POST, &wake_path, None).await;
                                                    }
                                                }
                                            }
                                        }
                                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                    }

                                    // Fetch live Tesla vehicle data (odometer + battery level)
                                    let vehicle_tag = &vehicle.remote_mgmt_id;
                                    let tesla_path = format!("/api/1/vehicles/{}/vehicle_data?endpoints=location_data%3Bcharge_state%3Bvehicle_state", vehicle_tag);

                                    let tesla_resp = match integration::tesla_curl::tesla_make_request(Method::GET, &tesla_path, None).await {
                                        Ok(r) => r,
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Tesla API error fetching vehicle_data"));
                                        }
                                    };

                                    if !tesla_resp.status().is_success() {
                                        return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Tesla API returned non-success for vehicle_data"));
                                    }

                                    let tesla_body: helper_model::TeslaVehicleDataEnvelope = match tesla_resp.json().await {
                                        Ok(b) => b,
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Tesla API response JSON decode error"));
                                        }
                                    };

                                    let odometer_i32: i32 = tesla_body.response.vehicle_state.odometer.round() as i32;
                                    let battery_level_i32: i32 = tesla_body.response.charge_state.battery_level;

                                    let lat = tesla_body.response.drive_state.latitude;
                                    let lon = tesla_body.response.drive_state.longitude;

                                    vehicle.odometer = odometer_i32;
                                    vehicle.tank_level_percentage = battery_level_i32;

                                    let _ = vehicle.save_changes::<model::Vehicle>(&mut pool);

                                    (battery_level_i32, odometer_i32, lat, lon)
                                }
                                _ => {
                                    return methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-in: Vehicle not supported for remote return"),
                                    )
                                }
                            };

                            vehicle_current_latitude = latitude;
                            vehicle_current_longitude = longitude;

                            vehicle_snapshot_id
                        }
                        helper_model::CheckInOutRequest::WithImagePath {
                            agreement_id,
                            ref left_image_path,
                            ref right_image_path,
                            ref front_image_path,
                            ref back_image_path,
                            ref front_right_image_path,
                            ref front_left_image_path,
                            ref back_right_image_path,
                            ref back_left_image_path,
                        } => {
                            let usr_in_question =
                                methods::user::get_user_by_id(&access_token.user_id)
                                    .await;
                            let Ok(usr_in_question) = usr_in_question else {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Loading user error"),
                                );
                            };
                            if !usr_in_question.is_email_verified() {
                                return methods::standard_replies::user_email_not_verified();
                            }

                            use schema::agreements::dsl as ag_q;
                            use schema::vehicles::dsl as veh_q;

                            let vehicle = ag_q::agreements
                                .find(&agreement_id)
                                .inner_join(veh_q::vehicles)
                                .select(veh_q::vehicles::all_columns())
                                .get_result::<model::Vehicle>(&mut pool);

                            let Ok(mut vehicle) = vehicle else {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Loading vehicle error DB"),
                                );
                            };


                            let (fuel, odo, latitude, longitude) = match vehicle.remote_mgmt {
                                model::RemoteMgmtType::Tesla => {
                                    // 1) Check online state via GET /api/1/vehicles/{vehicle_tag}
                                    let status_path = format!("/api/1/vehicles/{}", vehicle.remote_mgmt_id);

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
                                                        let wake_path = format!("/api/1/vehicles/{}/wake_up", vehicle.remote_mgmt_id);
                                                        let _ = integration::tesla_curl::tesla_make_request(Method::POST, &wake_path, None).await;
                                                    }
                                                }
                                            }
                                        }
                                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                    }

                                    // Fetch live Tesla vehicle data (odometer + battery level)
                                    let vehicle_tag = &vehicle.remote_mgmt_id;
                                    let tesla_path = format!("/api/1/vehicles/{}/vehicle_data?endpoints=location_data%3Bcharge_state%3Bvehicle_state", vehicle_tag);

                                    let tesla_resp = match integration::tesla_curl::tesla_make_request(Method::GET, &tesla_path, None).await {
                                        Ok(r) => r,
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Tesla API error fetching vehicle_data"));
                                        }
                                    };

                                    if !tesla_resp.status().is_success() {
                                        return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Tesla API returned non-success for vehicle_data"));
                                    }

                                    let tesla_body: helper_model::TeslaVehicleDataEnvelope = match tesla_resp.json().await {
                                        Ok(b) => b,
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Tesla API response JSON decode error"));
                                        }
                                    };

                                    let odometer_i32: i32 = tesla_body.response.vehicle_state.odometer.round() as i32;
                                    let battery_level_i32: i32 = tesla_body.response.charge_state.battery_level;

                                    let lat = tesla_body.response.drive_state.latitude;
                                    let lon = tesla_body.response.drive_state.longitude;

                                    vehicle.odometer = odometer_i32;
                                    vehicle.tank_level_percentage = battery_level_i32;

                                    let _ = vehicle.save_changes::<model::Vehicle>(&mut pool);

                                    (battery_level_i32, odometer_i32, lat, lon)
                                }
                                _ => {
                                    return methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-in: Vehicle not supported for remote return"),
                                    )
                                }
                            };

                            vehicle_current_latitude = latitude;
                            vehicle_current_longitude = longitude;

                            let snapshot_to_be_inserted = model::NewVehicleSnapshot {
                                left_image: left_image_path.clone(),
                                right_image: right_image_path.clone(),
                                front_image: front_image_path.clone(),
                                back_image: back_image_path.clone(),
                                odometer: odo,
                                level: fuel,
                                vehicle_id: vehicle.id,
                                rear_right: back_right_image_path.clone(),
                                rear_left: back_left_image_path.clone(),
                                front_right: front_right_image_path.clone(),
                                front_left: front_left_image_path.clone(),
                                dashboard: None,
                                renter_id: access_token.user_id,
                            };

                            use crate::schema::vehicle_snapshots::dsl as v_s_q;

                            let v_snap = diesel::insert_into(v_s_q::vehicle_snapshots)
                                .values(&snapshot_to_be_inserted)
                                .get_result::<model::VehicleSnapshot>(&mut pool);

                            match v_snap {
                                Ok(vs) => {
                                    vs.id
                                }
                                Err(_) => {
                                    return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: SQL error inserting vehicle snapshot"))
                                }
                            }
                        }
                    };

                    use schema::agreements::dsl as agreement_q;
                    use schema::vehicle_snapshots::dsl as v_s_q;
                    use schema::vehicles::dsl as v_q;
                    use schema::renters::dsl as renter_q;
                    use schema::payment_methods::dsl as pm_q;

                    let five_minutes_ago = Utc::now() - Duration::minutes(5);

                    let ag_v_s_result = v_s_q::vehicle_snapshots
                        .inner_join(
                            agreement_q::agreements.on(
                                v_s_q::vehicle_id.eq(agreement_q::vehicle_id)
                                    .and(v_s_q::renter_id.eq(agreement_q::renter_id))
                            )
                                .inner_join(v_q::vehicles)
                                .inner_join(renter_q::renters)
                                .inner_join(pm_q::payment_methods)
                        )
                        .filter(agreement_q::renter_id.eq(&user_id))
                        .filter(agreement_q::status.eq(model::AgreementStatus::Rental))
                        .filter(v_s_q::id.eq(&vehicle_snapshot_id))
                        .filter(agreement_q::id.eq(&agreement_id))
                        .filter(agreement_q::actual_pickup_time.is_not_null())
                        .filter(agreement_q::actual_drop_off_time.is_null())
                        .filter(v_s_q::time.ge(agreement_q::actual_pickup_time.assume_not_null()))
                        .filter(v_s_q::time.ge(five_minutes_ago))
                        .select((agreement_q::agreements::all_columns(), v_q::vehicles::all_columns(), v_s_q::vehicle_snapshots::all_columns(), renter_q::stripe_id, pm_q::token))
                        .get_result::<(model::Agreement, model::Vehicle, model::VehicleSnapshot, String, String)>(&mut pool);

                    if let Err(err) = ag_v_s_result {
                        return match err {
                            Error::NotFound => {
                                let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Check In Not Allowed"),
                                    message: String::from("Agreement or vehicle snapshot is not valid"),
                                };
                                methods::standard_replies::response_with_obj(&msg, StatusCode::FORBIDDEN)
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Database connection error at loading vehicle snapshot"))
                            }
                        }
                    }

                    let (mut agreement_to_be_checked_in, vehicle,check_in_snapshot, stripe_id, payment_method_id) = ag_v_s_result.unwrap();

                    let (lower_latitude, higher_latitude, lower_longitude, higher_longitude) = {
                        let location_id = &vehicle.location_id;
                        use schema::locations::dsl as l_q;
                        use schema::apartments::dsl as a_q;
                        let result = l_q::locations
                            .find(location_id)
                            .inner_join(a_q::apartments)
                            .select((
                                l_q::latitude_lower_bound,
                                l_q::latitude_higher_bound,
                                l_q::longitude_lower_bound,
                                l_q::longitude_higher_bound,
                                a_q::latitude_lower_bound,
                                a_q::latitude_higher_bound,
                                a_q::longitude_lower_bound,
                                a_q::longitude_higher_bound,
                            ))
                            .get_result::<(Option<f64>, Option<f64>, Option<f64>, Option<f64>, f64, f64, f64, f64)>(&mut pool);

                        let Ok(result) = result else {
                            return methods::standard_replies::internal_server_error_response(String::from("agreement/check-in: Database connection error at loading location boundry"))
                        };

                        let lower_latitude = result.0.unwrap_or(result.4);
                        let higher_latitude = result.1.unwrap_or(result.5);
                        let lower_longitude = result.2.unwrap_or(result.6);
                        let higher_longitude = result.3.unwrap_or(result.7);

                        (lower_latitude, higher_latitude, lower_longitude, higher_longitude)
                    };

                    if !(lower_latitude <= vehicle_current_latitude && vehicle_current_latitude <= higher_latitude
                        && lower_longitude <= vehicle_current_longitude && vehicle_current_longitude <= higher_longitude) {
                        let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                            title: String::from("Check In Not Allowed"),
                            message: String::from("Please return to the specific location"),
                        };
                        return methods::standard_replies::response_with_obj(&msg, StatusCode::FORBIDDEN)
                    }

                    // lock the vehicle

                    match vehicle.remote_mgmt {
                        model::RemoteMgmtType::Tesla => {
                            // 1) Check online state via GET /api/1/vehicles/{vehicle_tag}
                            let status_path = format!("/api/1/vehicles/{}", vehicle.remote_mgmt_id);

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
                                                let wake_path = format!("/api/1/vehicles/{}/wake_up", vehicle.remote_mgmt_id);
                                                let _ = integration::tesla_curl::tesla_make_request(Method::POST, &wake_path, None).await;
                                            }
                                        }
                                    }
                                }
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }
                            // 2) Proceed to lock/unlock once online (or after timeout anyway)
                            let cmd_path = format!("/api/1/vehicles/{}/command/door_lock", vehicle.remote_mgmt_id);
                            let result = integration::tesla_curl::tesla_make_request(Method::POST, &cmd_path, None).await;

                            let Ok(resp) = result else {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Tesla API error at door_lock request")
                                );
                            };
                            if resp.status() != StatusCode::OK {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Tesla API returned non-200 at door_lock request")
                                );
                            }
                        }
                        _ => {}
                    }

                    // update agreement status to check in (not in db)

                    agreement_to_be_checked_in.actual_drop_off_time = Some(Utc::now());
                    agreement_to_be_checked_in.vehicle_snapshot_after = Some(check_in_snapshot.id);

                    // map unmapped charges to this agreement
                    // 1. current charges in the database

                    use schema::charges::dsl as c_q;

                    let pickup: DateTime<Utc> = agreement_to_be_checked_in.actual_pickup_time.unwrap();
                    let drop_off: DateTime<Utc> = agreement_to_be_checked_in.actual_drop_off_time.unwrap();

                    let result = diesel::update(c_q::charges)
                        .filter(c_q::agreement_id.is_null())
                        .filter(c_q::vehicle_id.eq(&agreement_to_be_checked_in.vehicle_id))
                        .filter(c_q::time.ge(&pickup))
                        .filter(c_q::time.le(&drop_off))
                        .set(c_q::agreement_id.eq(Some(&agreement_to_be_checked_in.id)))
                        .execute(&mut pool);

                    if result.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: Database connection error at mapping unmapped charges")
                        )
                    }

                    // 2. fetch tesla charging history

                    if vehicle.remote_mgmt == model::RemoteMgmtType::Tesla {
                        use chrono::SecondsFormat;
                        let date_from = pickup.to_rfc3339_opts(SecondsFormat::Secs, true);
                        let date_to   = drop_off.to_rfc3339_opts(SecondsFormat::Secs, true);

                        let charge_history_path = format!(
                            "/api/1/dx/charging/sessions?vin={}&date_from={}&date_to={}",
                            vehicle.remote_mgmt_id,
                            date_from,
                            date_to
                        );

                        let result = integration::tesla_curl::tesla_make_request(Method::GET, &charge_history_path, None).await;

                        let Ok(resp) = result else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-in: Tesla API error at fetching charging sessions")
                            )
                        };

                        if resp.status() != StatusCode::OK {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-in: Tesla API returned non-200 at fetching charging sessions")
                            )
                        }

                        let body_text = match resp.text().await {
                            Ok(t) => t,
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Tesla API response body read error at charging sessions")
                                )
                            }
                        };

                        let parsed: helper_model::TeslaChargingSessionsResponse = match serde_json::from_str(&body_text) {
                            Ok(p) => p,
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: JSON parse error at charging sessions")
                                )
                            }
                        };

                        if parsed.status_code != 1000 {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-in: Tesla charging sessions returned failure status_code")
                            )
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
                                    String::from("agreement/check-in: Decimal conversion error for Tesla charging cost")
                                )
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
                                is_taxed: true,
                            };

                            use schema::charges::dsl as c_q;

                            let res = diesel::insert_into(c_q::charges)
                                .values(&new_charge)
                                .get_result::<model::Charge>(&mut pool);

                            if let Err(err) = res {
                                match err {
                                    Error::DatabaseError(_, _) => {
                                        continue;
                                    }
                                    _ => {
                                        return methods::standard_replies::internal_server_error_response(
                                            String::from("agreement/check-in: DB error inserting charges")
                                        )
                                    }
                                }
                            }
                        }
                    }

                    // Calculate total cost

                    // 0. rate offer
                    let rate_offer = agreement_to_be_checked_in.utilization_factor;

                    // 1. total rental revenue
                    let trip_duration = agreement_to_be_checked_in.rsvp_drop_off_time
                        - agreement_to_be_checked_in.rsvp_pickup_time;
                    let trip_duration_including_late_return =
                        max(agreement_to_be_checked_in.actual_drop_off_time.unwrap(), agreement_to_be_checked_in.rsvp_drop_off_time)
                        - agreement_to_be_checked_in.rsvp_pickup_time;

                    let total_hours_reserved = Decimal::new(trip_duration.num_minutes(), 0) / Decimal::new(60, 0);

                    let total_hours_driven = Decimal::new(trip_duration_including_late_return.num_minutes(), 0) / Decimal::new(60, 0);
                    let total_hours_driven_round_up = total_hours_driven.round_dp_with_strategy(0, RoundingStrategy::AwayFromZero);

                    let late_hours = (total_hours_driven - total_hours_reserved)
                        .round_dp_with_strategy(0, RoundingStrategy::AwayFromZero);

                    use schema::reward_transactions::dsl as re_q;
                    let reward_used_sum = re_q::reward_transactions
                        .filter(re_q::renter_id.eq(agreement_to_be_checked_in.renter_id))
                        .filter(re_q::agreement_id.eq(agreement_to_be_checked_in.id))
                        .select(diesel::dsl::sum(re_q::duration))
                        .get_result::<Option<Decimal>>(&mut pool);

                    let Ok(reward_used_sum) = reward_used_sum else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: Database connection error at summing reward hours")
                        )
                    };

                    let raw_hours_after_applying_credit = match reward_used_sum {
                        None => { trip_duration }
                        Some(credit) => {
                            methods::rental_rate::calculate_duration_after_reward(trip_duration, credit)
                        }
                    };

                    let billable_days_count_including_late_return: i32 = methods::rental_rate::billable_days_count(trip_duration_including_late_return);
                    let billable_duration_hours: Decimal = methods::rental_rate::calculate_billable_duration_hours(raw_hours_after_applying_credit);

                    let duration_revenue = billable_duration_hours * agreement_to_be_checked_in.duration_rate * agreement_to_be_checked_in.msrp_factor * rate_offer;
                    let duration_revenue_after_promo = match agreement_to_be_checked_in.clone().promo_id {
                        None => { duration_revenue }
                        Some(promo) => {
                            use schema::promos::dsl as p_q;
                            let discount = p_q::promos
                                .find(&promo)
                                .select(p_q::amount)
                                .get_result::<Decimal>(&mut pool);

                            let Ok(discount) = discount else {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Database connection error looking up promo amount")
                                )
                            };

                            max(Decimal::zero(), duration_revenue - discount)
                        }
                    };

                    // does not including late return
                    let duration_revenue_after_promo = match agreement_to_be_checked_in.clone().manual_discount {
                        None => { duration_revenue_after_promo }
                        Some(discount) => {
                            max(Decimal::zero(), duration_revenue - discount)
                        }
                    };

                    // late return fee is calculated separately
                    let late_return_fee = Decimal::new(2, 0) * late_hours * agreement_to_be_checked_in.duration_rate * agreement_to_be_checked_in.msrp_factor * rate_offer;

                    let total_rental_revenue = duration_revenue_after_promo + late_return_fee;

                    // 2. total insurance revenue

                    // calculated to include late return
                    let total_insurance_revenue = {
                        total_hours_driven_round_up * (
                            agreement_to_be_checked_in.liability_protection_rate.unwrap_or(Decimal::zero())
                                + agreement_to_be_checked_in.pcdw_protection_rate.unwrap_or(Decimal::zero())
                                + agreement_to_be_checked_in.pcdw_ext_protection_rate.unwrap_or(Decimal::zero())
                                + agreement_to_be_checked_in.pai_protection_rate.unwrap_or(Decimal::zero())
                                + agreement_to_be_checked_in.rsa_protection_rate.unwrap_or(Decimal::zero())
                        )
                    };

                    // 3. mileage package revenue

                    let (total_mileage_package_revenue, miles_allowed) = match agreement_to_be_checked_in.mileage_package_id {
                        None => {
                            // didn't select mp
                            (Decimal::zero(), 10)
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
                                    let base_rate_for_mp = if let Some(overwrite) = agreement_to_be_checked_in.mileage_package_overwrite {
                                        overwrite
                                    } else {
                                        agreement_to_be_checked_in.duration_rate * agreement_to_be_checked_in.msrp_factor * agreement_to_be_checked_in.mileage_conversion
                                    };

                                    (
                                        base_rate_for_mp * Decimal::new(mileage as i64, 0) * Decimal::new(discount_rate as i64, 2),
                                        mileage + 10
                                    )
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
                                                String::from( "agreement/check-in: Database error loading mileage package"),
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    };

                    // 4. charges


                    // eg. toll road, tesla supercharging
                    let total_taxed_charges_cost = c_q::charges
                        .filter(c_q::agreement_id.eq(agreement_to_be_checked_in.id))
                        .filter(c_q::is_taxed)
                        .select(diesel::dsl::sum(c_q::amount))
                        .get_result::<Option<Decimal>>(&mut pool);

                    let Ok(total_taxed_charges_cost) = total_taxed_charges_cost else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: Database connection error at summing external charges cost")
                        )
                    };

                    let total_taxed_charges_cost = match total_taxed_charges_cost {
                        None => { Decimal::ZERO }
                        Some(cost) => { cost }
                    };

                    // eg. parking citations and fines
                    let total_not_taxed_charges_cost = c_q::charges
                        .filter(c_q::agreement_id.eq(agreement_to_be_checked_in.id))
                        .filter(c_q::is_taxed.eq(false))
                        .select(diesel::dsl::sum(c_q::amount))
                        .get_result::<Option<Decimal>>(&mut pool);

                    let Ok(total_not_taxed_charges_cost) = total_not_taxed_charges_cost else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: Database connection error at summing external charges cost")
                        )
                    };

                    let total_not_taxed_charges_cost = match total_not_taxed_charges_cost {
                        None => { Decimal::ZERO }
                        Some(cost) => { cost }
                    };

                    // 5. low fuel & over mileage

                    let check_in_percent = check_in_snapshot.level;
                    let check_out_percent = {
                        let check_out_snapshot_id = agreement_to_be_checked_in.vehicle_snapshot_before.unwrap();
                        let check_out_percent = v_s_q::vehicle_snapshots
                            .find(check_out_snapshot_id)
                            .select(v_s_q::level)
                            .get_result::<i32>(&mut pool);
                        match check_out_percent {
                            Ok(check_out_percent) => { check_out_percent }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-in: Database connection error at fetching check out percent")
                                )
                            }
                        }
                    };

                    let missing_fuel_level = max(0, check_out_percent - check_in_percent) as i64;

                    let low_fuel_revenue = Decimal::new(missing_fuel_level, 0) * proj_config::PRICE_PER_CENT_ON_GAS;
                    let over_mileage_revenue = {
                        let check_in_odo = check_in_snapshot.odometer;

                        let check_out_snap_id = agreement_to_be_checked_in.vehicle_snapshot_before.unwrap();
                        let check_out_odo = v_s_q::vehicle_snapshots
                            .find(check_out_snap_id)
                            .select(v_s_q::odometer)
                            .get_result::<i32>(&mut pool);
                        let Ok(check_out_odo) = check_out_odo else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-in: Database connection error looking up check out odometer")
                            )
                        };

                        let total_driven = check_in_odo - check_out_odo;
                        if total_driven <= miles_allowed {
                            Decimal::ZERO
                        } else {
                            let over_mileage = total_driven - miles_allowed;
                            let mileage_rate: Decimal = {
                                if let Some(overwrite) = agreement_to_be_checked_in.mileage_rate_overwrite {
                                    overwrite
                                } else {
                                    agreement_to_be_checked_in.duration_rate * agreement_to_be_checked_in.msrp_factor * agreement_to_be_checked_in.mileage_conversion
                                }
                            };
                            Decimal::new(over_mileage as i64, 0) * mileage_rate
                        }
                    };

                    // 6. taxes

                    use schema::agreements_taxes::dsl as agreements_taxes_query;
                    use schema::taxes::dsl as t_q;

                    let taxes = agreements_taxes_query::agreements_taxes
                        .inner_join(t_q::taxes)
                        .filter(agreements_taxes_query::agreement_id.eq(&agreement_to_be_checked_in.id))
                        .select(t_q::taxes::all_columns())
                        .get_results::<model::Tax>(&mut pool);

                    let Ok(taxes) = taxes else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: Database error loading apartment taxes")
                        )
                    };

                    let mut local_tax_rate_daily = Decimal::zero();
                    let mut local_tax_rate_fixed = Decimal::zero();

                    let mut local_tax_rate_percent_sales = Decimal::zero();
                    let mut local_tax_rate_percent_non_sales = Decimal::zero();

                    for tax_obj in &taxes {
                        match tax_obj.tax_type {
                            model::TaxType::Percent => {
                                if tax_obj.is_sales_tax {
                                    local_tax_rate_percent_sales += tax_obj.multiplier;
                                } else {
                                    local_tax_rate_percent_non_sales += tax_obj.multiplier;
                                }
                            },
                            model::TaxType::Daily => {
                                local_tax_rate_daily += tax_obj.multiplier;
                            }
                            model::TaxType::Fixed => {
                                local_tax_rate_fixed += tax_obj.multiplier;
                            }
                        }
                    }

                    // 7. summarize

                    let total_subject_to_non_sales_tax = total_rental_revenue
                        + total_insurance_revenue + total_mileage_package_revenue
                        + low_fuel_revenue + over_mileage_revenue;
                    let total_subject_to_sales_tax = total_subject_to_non_sales_tax + total_taxed_charges_cost;

                    let total_revenue = total_subject_to_sales_tax;

                    let total_percentage_tax = total_subject_to_non_sales_tax * local_tax_rate_percent_non_sales
                        + total_subject_to_sales_tax * local_tax_rate_percent_sales;
                    let total_daily_tax = Decimal::new(billable_days_count_including_late_return as i64, 0) * local_tax_rate_daily;
                    let total_fixed_tax = local_tax_rate_fixed;

                    let total_stripe_amount = total_revenue + total_percentage_tax
                        + total_daily_tax + total_fixed_tax + total_not_taxed_charges_cost;
                    let total_stripe_amount_2dp = total_stripe_amount.round_dp(2);

                    // settle payments

                    use schema::payments::dsl as pmt_q;
                    let paid_amount = pmt_q::payments
                        .filter(pmt_q::agreement_id.eq(agreement_to_be_checked_in.id))
                        .select(diesel::dsl::sum(pmt_q::amount - pmt_q::refund_amount))
                        .get_result::<Option<Decimal>>(&mut pool);

                    let Ok(paid_amount) = paid_amount else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: Database connection error at summing paid amount")
                        )
                    };

                    let paid_amount = paid_amount.unwrap_or(Decimal::zero());

                    let auth_hold_pmt = pmt_q::payments
                        .find(agreement_to_be_checked_in.deposit_pmt_id.unwrap())
                        .get_result::<model::Payment>(&mut pool);
                    let Ok(mut auth_hold_pmt) = auth_hold_pmt else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: Database connection error at loading auth hold payment")
                        )
                    };

                    let outstanding_balance = total_stripe_amount_2dp - paid_amount;

                    if outstanding_balance < Decimal::zero() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-in: outstanding balance negative")
                        )
                    } else if outstanding_balance == Decimal::zero() {
                        auth_hold_pmt.capture_before = None;
                        auth_hold_pmt.payment_type = model::PaymentType::Canceled;
                        let _ = auth_hold_pmt.save_changes::<model::Payment>(&mut pool);
                        let _ = integration::stripe_veygo::drop_auth(&auth_hold_pmt.reference_number.unwrap()).await;
                    } else if outstanding_balance <= auth_hold_pmt.amount_authorized {
                        let mut outstanding_balance_2dp = outstanding_balance.round_dp(2);
                        outstanding_balance_2dp.rescale(2);
                        auth_hold_pmt.amount = outstanding_balance_2dp;
                        auth_hold_pmt.capture_before = None;
                        auth_hold_pmt.payment_type = model::PaymentType::Succeeded;
                        let _ = auth_hold_pmt.save_changes::<model::Payment>(&mut pool);
                        let _ = integration::stripe_veygo::capture_payment(&auth_hold_pmt.reference_number.unwrap(), Some((outstanding_balance_2dp.mantissa() as i64, true))).await;
                    } else {
                        auth_hold_pmt.amount = auth_hold_pmt.amount_authorized;
                        auth_hold_pmt.capture_before = None;
                        auth_hold_pmt.payment_type = model::PaymentType::Succeeded;
                        let _ = auth_hold_pmt.save_changes::<model::Payment>(&mut pool);
                        let _ = integration::stripe_veygo::capture_payment(&auth_hold_pmt.reference_number.unwrap(), None).await;

                        let still_need_to_process = outstanding_balance - auth_hold_pmt.amount;
                        let mut still_need_to_process_2dp = still_need_to_process.round_dp(2);
                        still_need_to_process_2dp.rescale(2);

                        let description = "RSVP #".to_owned() + &*agreement_to_be_checked_in.confirmation.clone();
                        let pmi = integration::stripe_veygo::create_payment_intent(&stripe_id, &payment_method_id, still_need_to_process_2dp.mantissa() as i64, PaymentIntentCaptureMethod::Automatic, &description).await;

                        match pmi {
                            Ok(pmi) => {
                                let new_payment = model::NewPayment {
                                    payment_type: model::PaymentType::Succeeded,
                                    amount: still_need_to_process_2dp,
                                    note: None,
                                    reference_number: Some(pmi.id.to_string()),
                                    agreement_id: agreement_to_be_checked_in.id,
                                    renter_id: agreement_to_be_checked_in.renter_id,
                                    payment_method_id: Some(agreement_to_be_checked_in.payment_method_id),
                                    amount_authorized: still_need_to_process_2dp,
                                    capture_before: None,
                                };

                                let payment_result = diesel::insert_into(pmt_q::payments)
                                    .values(&new_payment).get_result::<model::Payment>(&mut pool);

                                if payment_result.is_err() {
                                    return methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-in: DB saving final payment error, payment collected")
                                    )
                                }
                            }
                            Err(err) => {
                                return if err == VeygoError::CardDeclined {
                                    methods::standard_replies::card_declined()
                                } else {
                                    methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-in: Stripe cannot process outstanding payment")
                                    )
                                }
                            }
                        }
                    }

                    let new_ag = agreement_to_be_checked_in.save_changes::<model::Agreement>(&mut pool);
                    match new_ag {
                        Ok(ag) => {
                            methods::standard_replies::response_with_obj(&ag, StatusCode::OK)
                        }
                        Err(_) => {
                            methods::standard_replies::internal_server_error_response(
                                String::from("agreement/check-in: could not save agreement updates")
                            )
                        }
                    }
                }
            }
        })
}