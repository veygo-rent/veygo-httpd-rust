use crate::{POOL, methods, model, integration};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("user-identify")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, auth: String, user_agent: String| {
                // Checking method is GET
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response();
                }

                // Pool connection
                let mut pool = POOL.get().unwrap();

                // Checking token
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_return();
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_return();
                    }
                };
                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                match if_token_valid_result {
                    Err(err) => {
                        match err {
                            VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response()
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response();
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response();
                            }
                        }

                        let user_in_request = methods::user::get_user_by_id(&access_token.user_id)
                            .await;
                        let Ok(user_in_request) = user_in_request else {
                            return methods::standard_replies::internal_server_error_response();
                        };

                        use crate::schema::agreements::dsl as agreement_query;
                        use crate::schema::vehicles::dsl as vehicle_query;
                        let now = chrono::Utc::now();
                        let now_plus_buffer = now + chrono::Duration::minutes(15);
                        let current_vehicle_result = agreement_query::agreements
                            .inner_join(vehicle_query::vehicles)
                            .filter(agreement_query::renter_id.eq(&user_in_request.id))
                            .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                            .filter(agreement_query::actual_drop_off_time.is_null())
                            .filter(
                                agreement_query::actual_pickup_time.is_not_null()
                                    .or(agreement_query::rsvp_drop_off_time.ge(now))
                            )
                            .filter(agreement_query::rsvp_pickup_time.le(&now_plus_buffer))
                            .order_by(agreement_query::rsvp_pickup_time.asc())
                            .select(
                                vehicle_query::vehicles::all_columns()
                            )
                            .first::<model::Vehicle>(&mut pool);



                        if let Ok(vehicle) = current_vehicle_result {
                            let previous_agreement_exist = diesel::select(diesel::dsl::exists(
                                agreement_query::agreements
                                    .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                                    .filter(agreement_query::vehicle_id.eq(&vehicle.id))
                                    .filter(agreement_query::renter_id.ne(&user_in_request.id))
                                    .filter(agreement_query::actual_pickup_time.is_not_null())
                                    .filter(agreement_query::actual_drop_off_time.is_null())
                            )).get_result::<bool>(&mut pool);
                            let Ok(previous_agreement_exist) = previous_agreement_exist else {
                                return methods::standard_replies::internal_server_error_response();
                            };

                            if previous_agreement_exist {
                                let msg = serde_json::json!({});
                                return methods::standard_replies::response_with_obj(msg, StatusCode::NOT_FOUND)
                            }

                            match vehicle.remote_mgmt {
                                model::RemoteMgmtType::Tesla => {
                                    let _handler = tokio::spawn(async move {
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

                                        // 2) Proceed to honk once online (or after timeout anyway)
                                        let cmd_path = format!("/api/1/vehicles/{}/command/honk_horn", vehicle.remote_mgmt_id);
                                        let _result = integration::tesla_curl::tesla_make_request(Method::POST, &cmd_path, None).await;
                                    });
                                },
                                _ => {

                                }
                            }
                            let msg = serde_json::json!({});
                            methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                        } else {
                            let msg = serde_json::json!({});
                            methods::standard_replies::response_with_obj(msg, StatusCode::NOT_FOUND)
                        }
                    }
                }
            }
        )
}