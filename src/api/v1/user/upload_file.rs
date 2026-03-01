use crate::{POOL, integration, methods, model};
use bytes::{Bytes};
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use http::Method;
use warp::Filter;
use warp::http::StatusCode;
use sha2::{Sha256, Digest};
use crate::helper_model::VeygoError;

#[derive(Serialize, Deserialize)]
enum UploadedFileType {
    DriversLicense,
    DriversLicenseSecondary,
    LeaseAgreement,
    ProofOfInsurance,
}

impl FromStr for UploadedFileType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DriversLicense" => Ok(UploadedFileType::DriversLicense),
            "DriversLicenseSecondary" => Ok(UploadedFileType::DriversLicenseSecondary),
            "LeaseAgreement" => Ok(UploadedFileType::LeaseAgreement),
            "ProofOfInsurance" => Ok(UploadedFileType::ProofOfInsurance),
            _ => Err(()),
        }
    }
}

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("upload-file")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::bytes())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("file-type"))
        .and(warp::header::<String>("file-name"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, body: Bytes, auth: String, file_type: String, file_name: String, user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }

                let content_type_parsed_result = UploadedFileType::from_str(&*file_type);
                let Ok(content_type) = content_type_parsed_result else {
                    return methods::standard_replies::bad_request("File type not supported");
                };

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
                                methods::standard_replies::internal_server_error_response(String::from("user/upload-file: Token verification unexpected error"))
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response(String::from("user/upload-file: Token extension failed (returned false)"));
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(String::from("user/upload-file: Token extension error"));
                            }
                        }

                        let user = methods::user::get_user_by_id(&access_token.user_id).await;
                        let Ok(mut user) = user else {
                            return methods::standard_replies::internal_server_error_response(String::from("user/upload-file: Database error loading renter"))
                        };

                        let mut hasher = Sha256::new();
                        let data = user.id.to_le_bytes();
                        (& mut hasher).update(data);
                        let result = hasher.finalize();
                        let object_path: String = format!("user_docs/{:X}/", result);

                        let file_bytes = body.to_vec();
                        let file_path = integration::gcloud_storage_veygo::upload_file
                            (
                                object_path,
                                file_name,
                                file_bytes.clone(),
                            )
                            .await;

                        match content_type {
                            UploadedFileType::DriversLicense => {
                                if let Some(file) = user.drivers_license_image {
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    integration::gcloud_storage_veygo::delete_object(object_path)
                                        .await;
                                }
                                if let Some(file) = user.drivers_license_image_secondary {
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    integration::gcloud_storage_veygo::delete_object(object_path)
                                        .await;
                                }
                                user.drivers_license_image = Some(file_path);
                                user.drivers_license_expiration = None;
                                user.drivers_license_number = None;
                                user.drivers_license_state_region = None;
                                user.drivers_license_image_secondary = None;
                                user.requires_secondary_driver_lic = false;
                            }
                            UploadedFileType::DriversLicenseSecondary => {
                                if let Some(file) = user.drivers_license_image_secondary {
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    integration::gcloud_storage_veygo::delete_object(object_path)
                                        .await;
                                }
                                user.drivers_license_image_secondary = Some(file_path);
                                user.drivers_license_expiration = None;
                                user.drivers_license_number = None;
                                user.drivers_license_state_region = None;
                            }
                            UploadedFileType::LeaseAgreement => {
                                if let Some(file) = user.lease_agreement_image {
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    integration::gcloud_storage_veygo::delete_object(object_path)
                                        .await;
                                }
                                user.lease_agreement_image = Some(file_path);
                                user.lease_agreement_expiration = None;
                            }
                            UploadedFileType::ProofOfInsurance => {
                                if let Some(file) = user.insurance_id_image {
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    integration::gcloud_storage_veygo::delete_object(object_path)
                                        .await;
                                }
                                user.insurance_id_image = Some(file_path);
                                user.insurance_collision_expiration = None;
                                user.insurance_liability_expiration = None;
                            }
                        }

                        use crate::schema::renters::dsl as r_q;
                        let mut pool = POOL.get().unwrap();

                        let update_result = diesel::update
                            (
                                r_q::renters
                                    .find(&access_token.user_id)
                            )
                            .set(&user)
                            .get_result::<model::Renter>(&mut pool);

                        let Ok(renter) = update_result else {
                            return methods::standard_replies::internal_server_error_response(String::from("user/upload-file: SQL error saving renter uploaded file"))
                        };

                        return methods::standard_replies::response_with_obj(renter, StatusCode::OK);
                    }
                };
            },
        )
}
