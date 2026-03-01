use crate::{integration, methods, model, helper_model};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use sha2::{Sha256, Digest};
use warp::Filter;
use warp::http::StatusCode;
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
    warp::path("get-files")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("file-type"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |auth: String, file_type: String, user_agent: String| {
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
                let content_type_parsed_result = UploadedFileType::from_str(&*file_type);
                if content_type_parsed_result.is_err() {
                    return methods::standard_replies::bad_request("File type not supported");
                }
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
                                methods::standard_replies::internal_server_error_response(
                                    String::from("user/get-files: Token verification unexpected error")
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
                                        String::from("user/get-files: Token extension failed (returned false)")
                                    );
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    String::from("user/get-files: Token extension error")
                                );
                            }
                        }

                        let user = methods::user::get_user_by_id(&access_token.user_id)
                            .await;
                        let Ok(user) = user else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("user/get-files: Database error loading renter")
                            );
                        };

                        let Ok(content_type_parsed_result) = content_type_parsed_result else {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("user/get-files: File type parse state invalid")
                            );
                        };

                        match content_type_parsed_result {
                            UploadedFileType::DriversLicense => {
                                if let Some(file) = user.drivers_license_image {
                                    let mut hasher = Sha256::new();
                                    let data = user.id.to_le_bytes();
                                    hasher.update(data);
                                    let result = hasher.finalize();
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    let link = integration::gcloud_storage_veygo::get_signed_url(
                                        &object_path,
                                    ).await;
                                    let msg = helper_model::FileLink{ file_link: link };
                                    methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                                } else {
                                    let err_msg = helper_model::ErrorResponse {
                                        title: "File Not Exist".to_string(),
                                        message: "Cannot locate drivers license. ".to_string(),
                                    };
                                    methods::standard_replies::response_with_obj(err_msg, StatusCode::NOT_ACCEPTABLE)
                                }
                            }
                            UploadedFileType::DriversLicenseSecondary => {
                                if let Some(file) = user.drivers_license_image_secondary {
                                    let mut hasher = Sha256::new();
                                    let data = user.id.to_le_bytes();
                                    hasher.update(data);
                                    let result = hasher.finalize();
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    let link = integration::gcloud_storage_veygo::get_signed_url(
                                        &object_path,
                                    ).await;
                                    let msg = helper_model::FileLink{ file_link: link };
                                    methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                                } else {
                                    let err_msg = helper_model::ErrorResponse {
                                        title: "File Not Exist".to_string(),
                                        message: "Cannot locate secondary drivers license. ".to_string(),
                                    };
                                    methods::standard_replies::response_with_obj(err_msg, StatusCode::NOT_ACCEPTABLE)
                                }
                            }
                            UploadedFileType::LeaseAgreement => {
                                if let Some(file) = user.lease_agreement_image {
                                    let mut hasher = Sha256::new();
                                    let data = user.id.to_le_bytes();
                                    hasher.update(data);
                                    let result = hasher.finalize();
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    let link = integration::gcloud_storage_veygo::get_signed_url(
                                        &object_path,
                                    ).await;
                                    let msg = helper_model::FileLink{ file_link: link };
                                    methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                                } else {
                                    let err_msg = helper_model::ErrorResponse {
                                        title: "File Not Exist".to_string(),
                                        message: "Cannot locate lease agreement. ".to_string(),
                                    };
                                    methods::standard_replies::response_with_obj(err_msg, StatusCode::NOT_ACCEPTABLE)
                                }
                            }
                            UploadedFileType::ProofOfInsurance => {
                                if let Some(file) = user.insurance_id_image {
                                    let mut hasher = Sha256::new();
                                    let data = user.id.to_le_bytes();
                                    hasher.update(data);
                                    let result = hasher.finalize();
                                    let object_path: String = format!("user_docs/{:X}/{}", result, file);
                                    let link = integration::gcloud_storage_veygo::get_signed_url(
                                        &object_path,
                                    ).await;
                                    let msg = helper_model::FileLink{ file_link: link };
                                    methods::standard_replies::response_with_obj(msg, StatusCode::OK)
                                } else {
                                    let err_msg = helper_model::ErrorResponse {
                                        title: "File Not Exist".to_string(),
                                        message: "Cannot locate proof of insurance. ".to_string(),
                                    };
                                    methods::standard_replies::response_with_obj(err_msg, StatusCode::NOT_ACCEPTABLE)
                                }
                            }
                        }
                    }
                };
            },
        )
}
