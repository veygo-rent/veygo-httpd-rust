use crate::{POOL, integration, methods, model};
use bytes::BufMut;
use diesel::prelude::*;
use futures::TryStreamExt;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use http::Method;
use warp::Filter;
use warp::http::StatusCode;
use warp::multipart::FormData;
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
    warp::path("upload-file")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::multipart::form().max_length(5 * 1024 * 1024))
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("file-type"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, form: FormData, auth: String, file_type: String, user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }

                let content_type_parsed_result = UploadedFileType::from_str(&*file_type);
                if content_type_parsed_result.is_err() {
                    return methods::standard_replies::bad_request("File type not supported");
                }

                let field_names: Vec<_> = form
                    .and_then(|mut field| async move {
                        let mut bytes: Vec<u8> = Vec::new();

                        // field.data() only returns a piece of the content, you should call over it until it replies None
                        while let Some(content) = field.data().await {
                            let content = content.unwrap();
                            bytes.put(content);
                        }
                        Ok((field.filename().unwrap().to_string(), bytes))
                    })
                    .try_collect()
                    .await
                    .unwrap();
                let file_count = field_names.len() as i32;
                if file_count != 1 {
                    return methods::standard_replies::bad_request("Please upload exactly one file");
                };
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
                            let mut user = methods::user::get_user_by_id(&access_token.user_id)
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
                            let object_path: String = "user_docs/".to_string();
                            let file_path = integration::gcloud_storage_veygo::upload_file(
                                object_path,
                                field_names[0].0.to_string(),
                                field_names[0].1.clone(),
                            )
                            .await;
                            match content_type_parsed_result.unwrap() {
                                UploadedFileType::DriversLicense => {
                                    if let Some(file) = user.drivers_license_image {
                                        integration::gcloud_storage_veygo::delete_object(file)
                                            .await;
                                    }
                                    if let Some(file) = user.drivers_license_image_secondary {
                                        integration::gcloud_storage_veygo::delete_object(file)
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
                                        integration::gcloud_storage_veygo::delete_object(file)
                                            .await;
                                    }
                                    user.drivers_license_image_secondary = Some(file_path);
                                    user.drivers_license_expiration = None;
                                    user.drivers_license_number = None;
                                    user.drivers_license_state_region = None;
                                }
                                UploadedFileType::LeaseAgreement => {
                                    if let Some(file) = user.lease_agreement_image {
                                        integration::gcloud_storage_veygo::delete_object(file)
                                            .await;
                                    }
                                    user.lease_agreement_image = Some(file_path);
                                    user.lease_agreement_expiration = None;
                                }
                                UploadedFileType::ProofOfInsurance => {
                                    if let Some(file) = user.insurance_id_image {
                                        integration::gcloud_storage_veygo::delete_object(file)
                                            .await;
                                    }
                                    user.insurance_id_image = Some(file_path);
                                    user.insurance_collision_expiration = None;
                                    user.insurance_liability_expiration = None;
                                }
                            }
                            use crate::schema::renters::dsl::*;
                            let mut pool = POOL.get().unwrap();
                            let renter_updated: model::PublishRenter = diesel::update(renters.find(access_token.user_id))
                                .set(&user)
                                .get_result::<model::Renter>(&mut pool)
                                .unwrap()
                                .into();
                            return Ok::<_, warp::Rejection>((
                                methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    with_status(warp::reply::json(&renter_updated), StatusCode::OK),
                                ),
                            ));
                        }
                    }
                };
            },
        )
}
