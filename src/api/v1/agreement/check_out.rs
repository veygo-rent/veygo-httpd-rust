use crate::{methods, model, helper_model, integration, schema, proj_config, connection_pool};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use chrono::{Duration, Utc};
use diesel::result::Error;
use futures::{ StreamExt, stream::FuturesUnordered };
use stripe_core::{PaymentIntentCaptureMethod};
use rust_decimal::prelude::*;
use sha2::{Sha256, Digest};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("check-out")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: helper_model::CheckInOutRequest, auth: String, user_agent: String| {

            let mut pool = connection_pool().await.get().unwrap();

            // Checking method is POST
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response_405();
            }

            let agreement_id = match body {
                helper_model::CheckInOutRequest::WithSnapshotId { agreement_id, .. } => { agreement_id }
                helper_model::CheckInOutRequest::WithImagePath { agreement_id, .. } => { agreement_id }
            };

            match body {
                helper_model::CheckInOutRequest::WithSnapshotId { agreement_id, vehicle_snapshot_id } => {
                    if agreement_id <= 0 || vehicle_snapshot_id <= 0 {
                        return methods::standard_replies::bad_request_400("wrong parameters. ")
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
                        return methods::standard_replies::bad_request_400("wrong parameters. ")
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
                                    methods::standard_replies::bad_request_400("Loading vehicles failed")
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response_500(
                                        String::from("agreement/check-out: DB Error loading VIN number"),
                                    )
                                }
                            }
                        }
                    };

                    let mut hasher = Sha256::new();
                    let data = vin_num.clone().into_bytes();
                    hasher.update(data);
                    let result = hasher.finalize();
                    let object_pwd: String = format!("vehicle_pictures/{}/", hex::encode_upper(result));

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
                            return methods::standard_replies::bad_request_400(
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
                        helper_model::VeygoError::TokenFormatError => {
                            methods::tokens::token_not_hex_warp_return()
                        }
                        helper_model::VeygoError::InvalidToken => {
                            methods::tokens::token_invalid_return()
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/check-out: Token verification unexpected error"),
                            )
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent).await;

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/check-out: Token extension failed (returned false)"),
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/check-out: Token extension error"),
                            );
                        }
                    }

                    let vehicle_snapshot_id = match body {
                        helper_model::CheckInOutRequest::WithSnapshotId { vehicle_snapshot_id, .. } => { vehicle_snapshot_id }
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
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/check-out: Loading user error"),
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
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/check-out: Loading vehicle error DB"),
                                );
                            };


                            let (fuel, odo) = match vehicle.remote_mgmt {
                                model::RemoteMgmtType::Tesla => {
                                    // 1) Check online state via GET /api/1/vehicles/{vehicle_tag}
                                    let status_path = format!("/api/1/vehicles/{}", vehicle.remote_mgmt_id);

                                    for i in 0..16 { // up to ~10s total
                                        if let Ok(response) = integration::tesla_veygo::tesla_make_request(Method::GET, &status_path, None).await {
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
                                                        let _ = integration::tesla_veygo::tesla_make_request(Method::POST, &wake_path, None).await;
                                                    }
                                                }
                                            }
                                        }
                                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                    }

                                    // Fetch live Tesla vehicle data (odometer + battery level)
                                    let vehicle_tag = &vehicle.remote_mgmt_id;
                                    let tesla_path = format!("/api/1/vehicles/{}/vehicle_data?endpoints=location_data%3Bcharge_state%3Bvehicle_state", vehicle_tag);

                                    let tesla_resp = match integration::tesla_veygo::tesla_make_request(Method::GET, &tesla_path, None).await {
                                        Ok(r) => r,
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response_500(String::from("agreement/check-out: Tesla API error fetching vehicle_data"));
                                        }
                                    };

                                    if !tesla_resp.status().is_success() {
                                        return methods::standard_replies::internal_server_error_response_500(String::from("agreement/check-out: Tesla API returned non-success for vehicle_data"));
                                    }

                                    let tesla_body: helper_model::TeslaVehicleDataEnvelope = match tesla_resp.json().await {
                                        Ok(b) => b,
                                        Err(_) => {
                                            return methods::standard_replies::internal_server_error_response_500(String::from("agreement/check-out: Tesla API response JSON decode error"));
                                        }
                                    };

                                    let odometer_i32: i32 = tesla_body.response.vehicle_state.odometer.round() as i32;
                                    let battery_level_i32: i32 = tesla_body.response.charge_state.battery_level;

                                    vehicle.odometer = odometer_i32;
                                    vehicle.tank_level_percentage = battery_level_i32;

                                    let _ = vehicle.save_changes::<model::Vehicle>(&mut pool);

                                    (battery_level_i32, odometer_i32)
                                }
                                _ => {
                                    return methods::standard_replies::internal_server_error_response_500(
                                        String::from("agreement/check-out: Vehicle not supported for remote pickup"),
                                    )
                                }
                            };


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
                                    return methods::standard_replies::internal_server_error_response_500(String::from("agreement/check-out: SQL error inserting vehicle snapshot"))
                                }
                            }
                        }
                    };

                    let five_minutes_ago = Utc::now() - Duration::minutes(5);

                    use schema::agreements::dsl as agreement_q;
                    use schema::vehicle_snapshots::dsl as v_s_q;

                    let ag_v_s_result = v_s_q::vehicle_snapshots
                        .inner_join(
                            agreement_q::agreements.on(
                                v_s_q::vehicle_id.eq(agreement_q::vehicle_id)
                                    .and(v_s_q::renter_id.eq(agreement_q::renter_id))
                            )
                        )
                        .filter(agreement_q::renter_id.eq(&access_token.user_id))
                        .filter(agreement_q::status.eq(model::AgreementStatus::Rental))
                        .filter(v_s_q::id.eq(&vehicle_snapshot_id))
                        .filter(agreement_q::id.eq(&agreement_id))
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
                                let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Check Out Not Allowed"),
                                    message: String::from("Agreement or vehicle snapshot is not valid"),
                                };
                                methods::standard_replies::response_with_obj(&msg, StatusCode::FORBIDDEN)
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/check-out: Database error loading agreement and vehicle snapshot"),
                                )
                            }
                        }
                    }

                    let (agreement_to_be_checked_out, check_out_snapshot) = ag_v_s_result.unwrap();

                    let total_on_rental = agreement_q::agreements
                        .filter(agreement_q::status.eq(model::AgreementStatus::Rental))
                        .filter(agreement_q::vehicle_id.eq(agreement_to_be_checked_out.vehicle_id))
                        .filter(agreement_q::actual_pickup_time.is_not_null())
                        .filter(agreement_q::actual_drop_off_time.is_null())
                        .filter(agreement_q::id.ne(&agreement_id))
                        .count()
                        .get_result::<i64>(&mut pool);

                    let Ok(total_on_rental) = total_on_rental else {
                        return methods::standard_replies::internal_server_error_response_500(String::from("agreement/check-out: SQL error loading agreement count"))
                    };

                    if total_on_rental != 0 {
                        let msg = helper_model::ErrorResponse {
                            title: "Unable to Check Out".to_string(),
                            message: "Vehicle currently on rent".to_string(),
                        };
                        return methods::standard_replies::response_with_obj(&msg, StatusCode::FORBIDDEN);
                    }

                    let current_user = methods::user::get_user_by_id(&access_token.user_id).await;
                    if current_user.is_err() {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("agreement/check-out: Database error loading renter"),
                        );
                    }
                    let current_user = current_user.unwrap();

                    // hold deposit
                    // stripe auth

                    use schema::payment_methods::dsl as pm_q;
                    let pm = pm_q::payment_methods
                        .find(agreement_to_be_checked_out.payment_method_id)
                        .select(pm_q::token)
                        .get_result::<String>(&mut pool);
                    let Ok(pm_str) = pm else {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("agreement/check-out: Database error loading payment method"),
                        );
                    };

                    let mut deposit = Decimal::new(proj_config::DEPOSIT_AMOUNT, 0);
                    deposit.rescale(2);

                    let description = "RSVP #".to_owned() + &*agreement_to_be_checked_out.confirmation.clone();
                    let stripe_auth = integration::stripe_veygo::create_payment_intent(
                        &current_user.stripe_id, &pm_str, deposit.mantissa() as i64, PaymentIntentCaptureMethod::Manual, &description
                    ).await;

                    match stripe_auth {
                        Ok(pmi) => {
                            let now = Utc::now();
                            let _ = diesel::update(pm_q::payment_methods.filter(pm_q::token.eq(&pm_str)))
                                .set(pm_q::last_used_date_time.eq(now))
                                .execute(&mut pool);
                            
                            use schema::payments::dsl as p_q;
                            let new_deposit = model::NewPayment {
                                payment_type: pmi.clone().status.into(),
                                amount: Decimal::ZERO,
                                note: None,
                                reference_number: Some(pmi.id.to_string()),
                                agreement_id: agreement_to_be_checked_out.id,
                                renter_id: agreement_to_be_checked_out.renter_id,
                                payment_method_id: Some(agreement_to_be_checked_out.payment_method_id),
                                amount_authorized: deposit,
                                capture_before: Option::from(methods::timestamps::from_seconds(pmi.clone().latest_charge.unwrap().into_object().unwrap().payment_method_details.unwrap().card.unwrap().capture_before.unwrap())),
                            };

                            let result = diesel::insert_into(p_q::payments)
                                .values(&new_deposit)
                                .get_result::<model::Payment>(&mut pool);

                            let Ok(inserted_payment) = result else {
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/check-out: Database error inserting payment"),
                                );
                            };

                            let now = Some(Utc::now());
                            let ag = diesel::update(agreement_q::agreements)
                                .filter(agreement_q::id.eq(agreement_to_be_checked_out.id))
                                .set((
                                    agreement_q::deposit_pmt_id.eq(inserted_payment.id),
                                    agreement_q::vehicle_snapshot_before.eq(check_out_snapshot.id),
                                    agreement_q::actual_pickup_time.eq(now)
                                ))
                                .get_result::<model::Agreement>(&mut pool);

                            match ag {
                                Ok(ag) => {
                                    // Unlock vehicle
                                    use crate::schema::vehicles::dsl as v_q;
                                    let result = v_q::vehicles
                                        .find(&agreement_to_be_checked_out.vehicle_id)
                                        .select((v_q::remote_mgmt, v_q::remote_mgmt_id))
                                        .get_result::<(model::RemoteMgmtType, String)>(&mut pool);

                                    let Ok((vehicle_remote_mgmt, mgmt_id)) = result else {
                                        return methods::standard_replies::internal_server_error_response_500(
                                            String::from("agreement/check-out: Database error loading vehicle remote mgmt info"),
                                        )
                                    };

                                    match vehicle_remote_mgmt {
                                        model::RemoteMgmtType::Tesla => {
                                            let _handler = tokio::spawn(async move {
                                                // 1) Check online state via GET /api/1/vehicles/{vehicle_tag}
                                                let status_path = format!("/api/1/vehicles/{}", mgmt_id);

                                                for i in 0..16 {
                                                    if let Ok(response) = integration::tesla_veygo::tesla_make_request(Method::GET, &status_path, None).await {
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
                                                                    let _ = integration::tesla_veygo::tesla_make_request(Method::POST, &wake_path, None).await;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                                }
                                                // 2) Proceed to lock/unlock once online (or after timeout anyway)
                                                let cmd_path = format!("/api/1/vehicles/{}/command/door_unlock", mgmt_id);
                                                let _result = integration::tesla_veygo::tesla_make_request(Method::POST, &cmd_path, None).await;
                                            });
                                        }
                                        _ => {}
                                    }

                                    methods::standard_replies::response_with_obj(ag, StatusCode::OK)
                                }
                                Err(_) => {
                                    methods::standard_replies::internal_server_error_response_500(
                                        String::from("agreement/check-out: DB error cannot update agreement")
                                    )
                                }
                            }
                        }
                        Err(err) => {
                            return match err {
                                helper_model::VeygoError::CardDeclined => {
                                    methods::standard_replies::card_declined_402()
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response_500(
                                        String::from("agreement/check-out: Stripe error creating payment intent")
                                    )
                                }
                            }
                        }
                    }
                }
            }
        })
}