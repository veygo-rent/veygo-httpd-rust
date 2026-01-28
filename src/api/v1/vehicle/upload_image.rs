use crate::{POOL, integration, methods, model, schema, helper_model};
use bytes::{Bytes};
use diesel::prelude::*;
use warp::Filter;
use warp::http::{StatusCode, Method};
use sha2::{Sha256, Digest};
use crate::helper_model::VeygoError;

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
                    token: token_and_id[0].parse().unwrap(),
                };
                let if_token_valid =
                    methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                        .await;
                return match if_token_valid {
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

                        let mut hasher = Sha256::new();
                        let data = vehicle.vin.into_bytes();
                        (&mut hasher).update(data);
                        let result = hasher.finalize();
                        let object_path: String = format!("vehicle_pictures/{:X}/", result);

                        let file_bytes = body.to_vec();
                        let file_path = integration::gcloud_storage_veygo::upload_file(
                            object_path,
                            file_name,
                            file_bytes.clone(),
                        ).await;

                        let msg = helper_model::FilePath { file_path };
                        methods::standard_replies::response_with_obj(msg, StatusCode::CREATED)
                    }
                };
            },
        )
}
