use crate::{POOL, methods, model, integration};
use diesel::prelude::*;
use serde_derive::Deserialize;
use smartcar::vehicle::Vehicle;
use warp::{Filter, http::Method, http::StatusCode, reply::with_status};

#[derive(Deserialize)]
struct RequestBody {
    vehicle_id: i32,
    to_lock: bool,
}

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("lock-with-sc")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method,
                              body: RequestBody,
                              auth: String,
                              user_agent: String| {
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }
            let mut pool = POOL.get().unwrap();
            use crate::schema::vehicles::dsl as vehicle_query;
            let vehicle_result = vehicle_query::vehicles.filter(vehicle_query::id.eq(&body.vehicle_id)).get_result::<model::Vehicle>(&mut pool);
            if vehicle_result.is_err() {
                return methods::standard_replies::bad_request("Vehicle not found.");
            }
            let vehicle = vehicle_result.unwrap();
            let vehicle_auth_and_id = vehicle.remote_mgmt_id.split("$").collect::<Vec<&str>>();
            if vehicle_auth_and_id.len() != 2 {
                return methods::standard_replies::bad_request("Vehicle sc token issue");
            }
            let refresh_token = vehicle_auth_and_id[0];
            let vehicle_sc_id = vehicle_auth_and_id[1];

            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_wrapped_return(&auth);
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => int,
                Err(_) => {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
            };

            let access_token = model::RequestToken {
                user_id,
                token: token_and_id[0].parse().unwrap(),
            };
            let if_token_valid =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;

            match if_token_valid {
                Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                Ok(token_is_valid) => {
                    if !token_is_valid {
                        methods::tokens::token_invalid_wrapped_return(&access_token.token)
                    } else {
                        let access_opt = integration::smartcar_veygo::renew_access_token(refresh_token).await;
                        if access_opt.is_none() {
                            return methods::standard_replies::bad_request("Vehicle sc token issue");
                        }
                        let access = access_opt.unwrap();
                        // Token is valid
                        let admin = methods::user::get_user_by_id(&access_token.user_id)
                            .await
                            .unwrap();
                        let token_clone = access_token.clone();
                        methods::tokens::rm_token_by_binary(
                            hex::decode(token_clone.token).unwrap(),
                        ).await;
                        let new_token = methods::tokens::gen_token_object(
                            &access_token.user_id,
                            &user_agent,
                        ).await;
                        use crate::schema::access_tokens::dsl::*;

                        let new_token_in_db_publish = diesel::insert_into(access_tokens)
                            .values(&new_token)
                            .get_result::<model::AccessToken>(&mut pool)
                            .unwrap()
                            .to_publish_access_token();
                        if !methods::user::user_is_operational_admin(&admin) {
                            let token_clone = new_token_in_db_publish.clone();
                            return methods::standard_replies::user_not_admin_wrapped_return(
                                token_clone,
                            );
                        }
                        let vehicle = Vehicle::new(vehicle_sc_id, &access.access_token);
                        let result = if body.to_lock {
                            vehicle.lock().await
                        } else {
                            vehicle.unlock().await
                        };

                        let new_token = access.refresh_token + "$" + vehicle_sc_id;

                        let updated_vehicle = diesel::update(vehicle_query::vehicles.filter(vehicle_query::id.eq(&body.vehicle_id)))
                            .set(vehicle_query::remote_mgmt_id.eq(&new_token))
                            .get_result::<model::Vehicle>(&mut pool).unwrap().to_publish_admin_vehicle();

                        let status_code = match result {
                            Err(_) => StatusCode::BAD_GATEWAY,
                            Ok(_) => StatusCode::OK,
                        };

                        let msg = serde_json::json!({"updated_vehicle": &updated_vehicle});
                        Ok::<_, warp::Rejection>((
                            methods::tokens::wrap_json_reply_with_token(
                                new_token_in_db_publish,
                                with_status(
                                    warp::reply::json(&msg),
                                    status_code,
                                ),
                            ),
                        ))
                    }
                }
            }
        })
}