use std::str::FromStr;
use serde_derive::{Deserialize};
use warp::{Filter, Reply, http::Method, http::StatusCode};
use diesel::prelude::*;
use diesel::result::Error;
use sha2::{Digest, Sha256};
use crate::{helper_model, integration, methods, model, schema, POOL};
use crate::helper_model::VeygoError;

#[derive(Deserialize)]
enum TypeOfDocument {
    DriversLicense,
    LeaseAgreement,
    ProofOfInsurance,
}

impl FromStr for TypeOfDocument {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DriversLicense" => Ok(TypeOfDocument::DriversLicense),
            "LeaseAgreement" => Ok(TypeOfDocument::LeaseAgreement),
            "ProofOfInsurance" => Ok(TypeOfDocument::ProofOfInsurance),
            _ => Err(()),
        }
    }
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("renter-need-verify")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("renter-type"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, auth: String, renter_type: String, user_agent: String| {
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response_405();
                }

                let renter_type = TypeOfDocument::from_str(&*renter_type);
                let Ok(renter_type) = renter_type else {
                    return methods::standard_replies::bad_request_400("Renter type not supported");
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
                    token: String::from(token_and_id[0]),
                };
                let if_token_valid =
                    methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                        .await;

                match if_token_valid {
                    Err(err) => {
                        match err {
                            VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response_500(
                                    String::from("admin/renter-need-verify: Token verification unexpected error"),
                                )
                            }
                        }
                    }
                    Ok((_token, token_id)) => {
                        let user = methods::user::get_user_by_id(&access_token.user_id)
                            .await;
                        let Ok(user) = user else {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("admin/renter-need-verify: Database error loading admin by id"),
                            );
                        };

                        if !user.is_admin() {
                            return methods::standard_replies::user_not_admin()
                        }
                        if !user.is_operational_admin() {
                            return methods::standard_replies::admin_not_verified()
                        }

                        let result = methods::tokens::extend_token(token_id, &user_agent);

                        match result {
                            Err(_) => {
                                methods::standard_replies::internal_server_error_response_500(
                                    String::from("admin/renter-need-verify: Token extension error"),
                                )
                            }
                            Ok(is_renewed) => {
                                if !is_renewed {
                                    methods::standard_replies::internal_server_error_response_500(
                                        String::from("admin/renter-need-verify: Token extension failed (returned false)"),
                                    )
                                } else {
                                    use schema::renters::dsl as r_q;

                                    let mut pool = POOL.get().unwrap();

                                    let renter = match renter_type {
                                        TypeOfDocument::DriversLicense => {
                                            r_q::renters
                                                .filter(r_q::drivers_license_expiration.is_null())
                                                .filter(r_q::drivers_license_image.is_not_null())
                                                .filter(
                                                    r_q::requires_secondary_driver_lic.eq(false)
                                                        .or(
                                                            r_q::requires_secondary_driver_lic.eq(true)
                                                                .and(r_q::drivers_license_image_secondary.is_not_null())
                                                        )
                                                )
                                                .into_boxed()
                                        }
                                        TypeOfDocument::LeaseAgreement => {
                                            r_q::renters
                                                .filter(r_q::lease_agreement_expiration.is_null())
                                                .filter(r_q::lease_agreement_image.is_not_null())
                                                .into_boxed()
                                        }
                                        TypeOfDocument::ProofOfInsurance => {
                                            r_q::renters
                                                .filter(r_q::insurance_liability_expiration.is_null())
                                                .filter(r_q::insurance_id_image.is_not_null())
                                                .into_boxed()
                                        }
                                    }.limit(1)
                                        .get_result::<model::Renter>(&mut pool);

                                    let renter = match renter {
                                        Ok(renter) => renter,
                                        Err(err) => {
                                            return match err {
                                                Error::NotFound => {
                                                    let msg = serde_json::json!({});
                                                    methods::standard_replies::response_with_obj(msg, StatusCode::NOT_FOUND)
                                                }
                                                _ => {
                                                    methods::standard_replies::internal_server_error_response_500(
                                                        String::from("admin/renter-need-verify: DB error loading renter"),
                                                    )
                                                }
                                            }
                                        }
                                    };

                                    let renter_clone = renter.clone();

                                    let doc_path_unsigned: String = match renter_type {
                                        TypeOfDocument::DriversLicense => {
                                            if renter.requires_secondary_driver_lic {
                                                renter.drivers_license_image_secondary.unwrap()
                                            } else {
                                                renter.drivers_license_image.unwrap()
                                            }
                                        }
                                        TypeOfDocument::LeaseAgreement => {
                                            renter.lease_agreement_image.unwrap()
                                        }
                                        TypeOfDocument::ProofOfInsurance => {
                                            renter.insurance_id_image.unwrap()
                                        }
                                    };

                                    let mut hasher = Sha256::new();
                                    let data = renter.id.to_le_bytes();
                                    hasher.update(data);
                                    let result = hasher.finalize();
                                    let object_path: String = format!("user_docs/{}/{}", hex::encode_upper(result), doc_path_unsigned);
                                    let link = integration::gcloud_storage_veygo::get_signed_url(
                                        &object_path,
                                    ).await;

                                    let link = helper_model::FileLink{ file_link: link };
                                    let msg = helper_model::RenterNeedVerify { renter: renter_clone.into(), file_link: link };

                                    methods::standard_replies::response_with_obj(&msg, StatusCode::OK)
                                }
                            }
                        }
                    }
                }
            }
        )
}
