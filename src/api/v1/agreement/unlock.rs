use diesel::result::Error;
use warp::{Filter, Reply, http::Method, http::StatusCode};
use crate::{schema, helper_model, methods, model, connection_pool, integration};
use diesel::prelude::*;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path!("unlock" / String)
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |conf_id: String, method: Method, auth: String, user_agent: String| {
            // Checking method is GET
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response_405();
            }

            // Pool connection
            let mut pool = connection_pool().await.get().unwrap();

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
            let access_token = model::RequestToken {
                user_id,
                token: String::from(token_and_id[0]),
            };
            let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;

            match if_token_valid_result {
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
                                String::from("agreement/unlock: Token verification unexpected error"),
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
                                    String::from("agreement/unlock: Token extension failed (returned false)"),
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/unlock: Token extension error"),
                            );
                        }
                    }

                    // Get current user
                    let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await;

                    let user_in_request = match user_in_request {
                        Ok(temp) => { temp }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/unlock: Database error loading renter"),
                            );
                        }
                    };

                    use schema::agreements::dsl as ag_q;
                    let agreement = ag_q::agreements
                        .filter(ag_q::confirmation.eq(conf_id.to_uppercase()))
                        .filter(ag_q::actual_pickup_time.is_not_null())
                        .filter(ag_q::actual_drop_off_time.is_null())
                        .get_result::<model::Agreement>(&mut pool);

                    let agreement = match agreement {
                        Ok(agreement) => agreement,
                        Err(e) => {
                            return match e {
                                Error::NotFound => {
                                    methods::standard_replies::agreement_not_allowed_response()
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response_500(
                                        String::from("agreement/unlock: Database error loading agreement"),
                                    )
                                }
                            };
                        }
                    };
                    
                    if agreement.renter_id != user_id && !user_in_request.is_operational_admin() {
                        return methods::standard_replies::agreement_not_allowed_response()
                    }
                    
                    // Unlock vehicle
                    use crate::schema::vehicles::dsl as v_q;
                    let result = v_q::vehicles
                        .find(&agreement.vehicle_id)
                        .select((v_q::remote_mgmt, v_q::remote_mgmt_id))
                        .get_result::<(model::RemoteMgmtType, String)>(&mut pool);

                    let Ok((vehicle_remote_mgmt, mgmt_id)) = result else {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("agreement/unlock: Database error loading vehicle remote mgmt info"),
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

                    methods::standard_replies::response_with_obj(agreement, StatusCode::OK)
                }
            }
            
        })
}