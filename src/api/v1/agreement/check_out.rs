use crate::{POOL, methods, model, helper_model, integration, schema, proj_config};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use chrono::{Duration, Utc};
use diesel::result::Error;
use stripe_core::{PaymentIntentCaptureMethod};
use rust_decimal::prelude::*;
use crate::helper_model::VeygoError;

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

            if body.agreement_id <= 0 || body.vehicle_snapshot_id <= 0 {
                return methods::standard_replies::bad_request("wrong parameters. ")
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
                                let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
                                    title: String::from("Check Out Not Allowed"),
                                    message: String::from("Agreement or vehicle snapshot is not valid"),
                                };
                                methods::standard_replies::response_with_obj(&msg, StatusCode::FORBIDDEN)
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("agreement/check-out: Database error loading agreement and vehicle snapshot"),
                                )
                            }
                        }
                    }

                    let (agreement_to_be_checked_out, check_out_snapshot) = ag_v_s_result.unwrap();

                    let current_user = methods::user::get_user_by_id(&access_token.user_id).await;
                    if current_user.is_err() {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("agreement/check-out: Database error loading renter"),
                        );
                    }
                    let current_user = current_user.unwrap();

                    // TODO: hold deposit
                    // stripe auth

                    use schema::payment_methods::dsl as pm_q;
                    let pm = pm_q::payment_methods
                        .find(agreement_to_be_checked_out.payment_method_id)
                        .select(pm_q::token)
                        .get_result::<String>(&mut pool);
                    let Ok(pm_str) = pm else {
                        return methods::standard_replies::internal_server_error_response(
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
                                return methods::standard_replies::internal_server_error_response(
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
                                        return methods::standard_replies::internal_server_error_response(
                                            String::from("agreement/check-out: Database error loading vehicle remote mgmt info"),
                                        )
                                    };

                                    match vehicle_remote_mgmt {
                                        model::RemoteMgmtType::Tesla => {
                                            let _handler = tokio::spawn(async move {
                                                // 1) Check online state via GET /api/1/vehicles/{vehicle_tag}
                                                let status_path = format!("/api/1/vehicles/{}", mgmt_id);

                                                for i in 0..16 {
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

                                    methods::standard_replies::response_with_obj(ag, StatusCode::OK)
                                }
                                Err(_) => {
                                    methods::standard_replies::internal_server_error_response(
                                        String::from("agreement/check-out: DB error cannot update agreement")
                                    )
                                }
                            }
                        }
                        Err(err) => {
                            return match err {
                                VeygoError::CardDeclined => {
                                    methods::standard_replies::card_declined()
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response(
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