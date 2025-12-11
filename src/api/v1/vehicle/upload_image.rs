use crate::{POOL, integration, methods, model, schema, helper_model};
use bytes::{Bytes};
use diesel::prelude::*;
use http::Method;
use warp::Filter;
use warp::http::StatusCode;
use warp::reply::with_status;
use sha2::{Sha256, Digest};

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("upload-image")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::bytes())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("vehicle-vin"))
        .and(warp::header::<String>("file-name"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, body: Bytes, auth: String, vehicle_vin: String, file_name: String, user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }

                use schema::vehicles::dsl as v_q;
                let mut pool = POOL.get().unwrap();
                let vehicle_result = v_q::vehicles
                    .filter(v_q::vin.eq(&vehicle_vin)).get_result::<model::Vehicle>(&mut pool);

                if vehicle_result.is_err() {
                    return methods::standard_replies::bad_request("Vehicle does not exist")
                }

                let vehicle = vehicle_result.unwrap();

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
                            let mut hasher = Sha256::new();
                            let data = vehicle.vin.into_bytes();
                            hasher.update(data);
                            let result = hasher.finalize();
                            let object_path: String = format!("vehicle_pictures/{:X}/", result);
                            let file_bytes = body.to_vec();
                            let file_path = integration::gcloud_storage_veygo::upload_file(
                                object_path,
                                file_name,
                                file_bytes.clone(),
                            ).await;
                            let msg = helper_model::FilePath { file_path };
                            return Ok::<_, warp::Rejection>((
                                methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    with_status(warp::reply::json(&msg), StatusCode::OK),
                                ),
                            ));
                        }
                    }
                };
            },
        )
}
