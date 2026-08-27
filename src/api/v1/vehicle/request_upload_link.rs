use std::path::Path;
use diesel::prelude::*;
use warp::http::{Method, StatusCode};
use serde_derive::{Deserialize};
use sha2::{Sha256, Digest};
use warp::Filter;
use crate::{connection_pool, helper_model, integration, methods, model, schema};

#[derive(Deserialize, Clone)]
struct VehicleImageData {
    vehicle_vin: String,
    file_name: String,
}

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("request-upload-link")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: VehicleImageData, auth: String, user_agent: String| {
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response_405();
            }

            use schema::vehicles::dsl as v_q;
            let mut pool = connection_pool().await.get().unwrap();
            let vehicle_result = v_q::vehicles
                .filter(v_q::vin.eq(&body.vehicle_vin)).get_result::<model::Vehicle>(&mut pool);

            if vehicle_result.is_err() {
                return methods::standard_replies::bad_request_400("Vehicle does not exist")
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

            let token;
            let token_parsed_result = token_and_id[0].parse::<String>();
            token = match token_parsed_result {
                Ok(token) => token,
                Err(_) => {
                    return methods::tokens::token_invalid_return();
                }
            };
            let access_token = model::RequestToken {
                user_id,
                token,
            };

            let if_token_valid =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                    .await;

            return match if_token_valid {
                Err(err) => {
                    match err {
                        helper_model::VeygoError::TokenFormatError => {
                            methods::tokens::token_not_hex_warp_return()
                        }
                        helper_model::VeygoError::InvalidToken => {
                            methods::tokens::token_invalid_return()
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response_500(String::from("vehicle/request-upload-link: Token verification unexpected error"))
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent).await;

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response_500(String::from("vehicle/request-upload-link: Token extension failed (returned false)"));
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response_500(String::from("vehicle/request-upload-link: Token extension error"));
                        }
                    }

                    let path = Path::new(&body.file_name);
                    let ext = path.extension().unwrap_or("".as_ref()).to_str().unwrap_or("").to_uppercase();
                    let content_type = match ext.as_str() {
                        "PDF" => "application/pdf",
                        "JPG" | "JPEG" => "image/jpeg",
                        "PNG" => "image/png",
                        "CSV" => "text/csv",
                        "HEIC" => "image/heic",
                        _ => "application/octet-stream",
                    };
                    let u = uuid::Uuid::new_v4().to_string().to_uppercase();
                    let file_name_with_uuid = u + "." + ext.as_str();

                    let mut hasher = Sha256::new();
                    let data = vehicle.vin.into_bytes();
                    (&mut hasher).update(data);
                    let result = hasher.finalize();
                    let object_path: String = format!("vehicle_pictures/{}/", hex::encode_upper(result));
                    let stored_file_abs_path = format!("{}{}", object_path, file_name_with_uuid);

                    let link = integration::gcloud_storage_veygo::get_signed_upload_url(&stored_file_abs_path, content_type).await;

                    let file_link = helper_model::FileLink { file_link: link, file_name: file_name_with_uuid };
                    methods::standard_replies::response_with_obj(file_link, StatusCode::OK)
                }
            }
        })
}