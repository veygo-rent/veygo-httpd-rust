use crate::{POOL, integration, methods, model};
use diesel::RunQueryDsl;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use warp::Filter;
use warp::http::StatusCode;
use warp::reply::with_status;

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
                let content_type_parsed_result = UploadedFileType::from_str(&*file_type);
                if content_type_parsed_result.is_err() {
                    return methods::standard_replies::bad_request("File type not supported");
                }
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(),
                    Ok(token_is_valid) => {
                        if !token_is_valid {
                            methods::tokens::token_invalid_wrapped_return()
                        } else {
                            let user = methods::user::get_user_by_id(&access_token.user_id)
                                .await
                                .unwrap();
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
                            return match content_type_parsed_result.unwrap() {
                                UploadedFileType::DriversLicense => {
                                    if let Some(file) = user.drivers_license_image {
                                        let link =
                                            integration::gcloud_storage_veygo::get_signed_url(
                                                &file,
                                            )
                                            .await;
                                        let msg = serde_json::json!({
                                            "drivers_license": link,
                                        });
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::OK,
                                                ),
                                            ),
                                        ))
                                    } else {
                                        let msg = model::ErrorResponse {
                                            title: "Upload Failed".to_string(),
                                            message: "Cannot upload drivers license. ".to_string(),
                                        };
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::NOT_ACCEPTABLE,
                                                ),
                                            ),
                                        ))
                                    }
                                }
                                UploadedFileType::DriversLicenseSecondary => {
                                    if let Some(file) = user.drivers_license_image_secondary {
                                        let link =
                                            integration::gcloud_storage_veygo::get_signed_url(
                                                &file,
                                            )
                                            .await;
                                        let msg = serde_json::json!({
                                            "drivers_license_secondary": link,
                                        });
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::OK,
                                                ),
                                            ),
                                        ))
                                    } else {
                                        let msg = model::ErrorResponse {
                                            title: "Upload Failed".to_string(),
                                            message: "Cannot upload secondary drivers license. ".to_string(),
                                        };
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::NOT_ACCEPTABLE,
                                                ),
                                            ),
                                        ))
                                    }
                                }
                                UploadedFileType::LeaseAgreement => {
                                    if let Some(file) = user.lease_agreement_image {
                                        let link =
                                            integration::gcloud_storage_veygo::get_signed_url(
                                                &file,
                                            )
                                            .await;
                                        let msg = serde_json::json!({
                                            "lease_agreement": link,
                                        });
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::OK,
                                                ),
                                            ),
                                        ))
                                    } else {
                                        let msg = model::ErrorResponse {
                                            title: "Upload Failed".to_string(),
                                            message: "Cannot upload lease agreement. ".to_string(),
                                        };
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::NOT_ACCEPTABLE,
                                                ),
                                            ),
                                        ))
                                    }
                                }
                                UploadedFileType::ProofOfInsurance => {
                                    if let Some(file) = user.insurance_id_image {
                                        let link =
                                            integration::gcloud_storage_veygo::get_signed_url(
                                                &file,
                                            )
                                            .await;
                                        let msg = serde_json::json!({
                                            "insurance_id": link,
                                        });
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::OK,
                                                ),
                                            ),
                                        ))
                                    } else {
                                        let msg = model::ErrorResponse {
                                            title: "Upload Failed".to_string(),
                                            message: "Cannot upload proof of insurance. ".to_string(),
                                        };
                                        Ok::<_, warp::Rejection>((
                                            methods::tokens::wrap_json_reply_with_token(
                                                new_token_in_db_publish,
                                                with_status(
                                                    warp::reply::json(&msg),
                                                    StatusCode::NOT_ACCEPTABLE,
                                                ),
                                            ),
                                        ))
                                    }
                                }
                            };
                        }
                    }
                };
            },
        )
}
