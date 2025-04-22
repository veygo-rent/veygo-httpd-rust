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
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::<String>("content_type"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |token: String,
                        user_id: i32,
                        content_type: String,
                        client_type: Option<String>| {
                let access_token = model::RequestToken { user_id, token };
                let if_token_valid = methods::tokens::verify_user_token(
                    access_token.user_id.clone(),
                    access_token.token.clone(),
                )
                .await;
                let content_type_parsed_result = UploadedFileType::from_str(&*content_type);
                if content_type_parsed_result.is_err() {
                    return methods::standard_replies::internal_server_error_response();
                }
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                    Ok(token_is_valid) => {
                        if !token_is_valid {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
                        } else {
                            let id_clone = access_token.user_id.clone();
                            let user = methods::user::get_user_by_id(id_clone).await.unwrap();
                            let token_clone = access_token.clone();
                            methods::tokens::rm_token_by_binary(
                                hex::decode(token_clone.token).unwrap(),
                            )
                            .await;
                            let new_token = methods::tokens::gen_token_object(
                                access_token.user_id.clone(),
                                client_type.clone(),
                            )
                            .await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .to_publish_access_token();
                            let msg;
                            return match content_type_parsed_result.unwrap() {
                                UploadedFileType::DriversLicense => {
                                    if let Some(file) = user.drivers_license_image {
                                        let link =
                                            integration::gcloud_storage_veygo::get_signed_url(
                                                &file,
                                            )
                                            .await;
                                        msg = serde_json::json!({
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
                                        msg = serde_json::json!({
                                            "drivers_license": None::<Option<String>>,
                                        });
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
                                        msg = serde_json::json!({
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
                                        msg = serde_json::json!({
                                            "drivers_license_secondary": None::<Option<String>>,
                                        });
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
                                        msg = serde_json::json!({
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
                                        msg = serde_json::json!({
                                            "lease_agreement": None::<Option<String>>,
                                        });
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
                                        msg = serde_json::json!({
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
                                        msg = serde_json::json!({
                                            "insurance_id": None::<Option<String>>,
                                        });
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
