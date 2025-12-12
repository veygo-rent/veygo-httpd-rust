use crate::{POOL, methods, model, integration, helper_model};
use diesel::prelude::*;
use warp::{Filter, http::Method, http::StatusCode, reply::with_status};
use sha2::{Sha256, Digest};
use futures::{stream::FuturesUnordered, StreamExt};

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("set-sc-token")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move | method: Method, body: helper_model::GenerateSnapshot,
                               auth: String, user_agent: String| {

            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }

            use crate::schema::vehicles::dsl as v_q;
            let mut pool = POOL.get().unwrap();
            let vehicle_result = v_q::vehicles
                .filter(v_q::vin.eq(&body.vehicle_vin)).get_result::<model::Vehicle>(&mut pool);
            if vehicle_result.is_err() {
                return methods::standard_replies::bad_request("Vehicle does not exist")
            }
            let vehicle: model::Vehicle = vehicle_result.unwrap();

            let mut hasher = Sha256::new();
            let data = vehicle.vin.into_bytes();
            hasher.update(data);
            let result = hasher.finalize();
            let object_pwd: String = format!("vehicle_pictures/{:X}/", result);

            let left_image_path: String = format!("{}{}", &object_pwd, &body.left_image_path);
            let right_image_path: String = format!("{}{}", &object_pwd, &body.right_image_path);
            let front_image_path: String = format!("{}{}", &object_pwd, &body.front_image_path);
            let back_image_path: String = format!("{}{}", &object_pwd, &body.back_image_path);
            let front_right_image_path: String = format!("{}{}", &object_pwd, &body.front_right_image_path);
            let front_left_image_path: String = format!("{}{}", &object_pwd, &body.front_left_image_path);
            let back_right_image_path: String = format!("{}{}", &object_pwd, &body.back_right_image_path);
            let back_left_image_path: String = format!("{}{}", &object_pwd, &body.back_left_image_path);

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

            while let Some((label, path, ok)) = futures.next().await {
                if !ok {
                    return methods::standard_replies::bad_request(
                        &format!("{} does not exist: {}", label, path),
                    );
                }
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
                        )
                            .await;
                        let new_token = methods::tokens::gen_token_object(
                            &access_token.user_id,
                            &user_agent,
                        )
                            .await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.get().unwrap();
                        let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                            .values(&new_token)
                            .get_result::<model::AccessToken>(&mut pool)
                            .unwrap()
                            .into();

                        let snapshot_to_be_inserted = model::NewVehicleSnapshot {
                            left_image: body.left_image_path.clone(),
                            right_image: body.right_image_path.clone(),
                            front_image: body.front_image_path.clone(),
                            back_image: body.back_image_path.clone(),
                            odometer: vehicle.odometer,
                            level: vehicle.tank_level_percentage,
                            vehicle_id: vehicle.id,
                            rear_right: body.back_right_image_path.clone(),
                            rear_left: body.back_left_image_path.clone(),
                            front_right: body.front_right_image_path.clone(),
                            front_left: body.front_left_image_path.clone(),
                            dashboard: None,
                        };

                        methods::standard_replies::not_implemented_response()
                    }
                }
            }
        })
}