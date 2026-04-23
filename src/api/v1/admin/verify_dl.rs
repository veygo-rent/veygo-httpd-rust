use askama::Template;
use diesel::prelude::*;
use diesel::result::Error;
use sha2::{Digest, Sha256};
use warp::{Filter, Reply, http::Method, http::StatusCode};
use crate::{helper_model, integration, methods, model, schema, POOL};
use crate::helper_model::{VeygoError};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("verify-drivers-license")
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: helper_model::VerifyDriversLicenseRequest, auth: String, user_agent: String| {
            if method != Method::PATCH {
                return methods::standard_replies::method_not_allowed_response_405();
            }

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
                                String::from("admin/verify-drivers-license: Token verification unexpected error"),
                            )
                        }
                    }
                }
                Ok((_token, token_id)) => {
                    let user = methods::user::get_user_by_id(&access_token.user_id)
                        .await;
                    let Ok(user) = user else {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("admin/verify-drivers-license: Database error loading renter by id"),
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
                                String::from("admin/verify-drivers-license: Token extension error"),
                            )
                        }
                        Ok(is_renewed) => {
                            if !is_renewed {
                                methods::standard_replies::internal_server_error_response_500(
                                    String::from("admin/verify-drivers-license: Token extension failed (returned false)"),
                                )
                            } else {
                                use schema::renters::dsl as r_q;

                                let mut pool = POOL.get().unwrap();

                                let renter_id  = match body {
                                    helper_model::VerifyDriversLicenseRequest::DeclinePrimary { renter_id, .. } => { renter_id }
                                    helper_model::VerifyDriversLicenseRequest::DeclineSecondary { renter_id, .. } => { renter_id }
                                    helper_model::VerifyDriversLicenseRequest::RequireSecondary { renter_id, .. } => { renter_id }
                                    helper_model::VerifyDriversLicenseRequest::Approved { renter_id, .. } => { renter_id }
                                };

                                let renter = match body {
                                    helper_model::VerifyDriversLicenseRequest::DeclinePrimary { .. } => {
                                        r_q::renters
                                            .filter(r_q::drivers_license_expiration.is_null())
                                            .filter(r_q::drivers_license_image.is_not_null())
                                            .filter(r_q::requires_secondary_driver_lic.eq(false))
                                            .into_boxed()
                                    }
                                    helper_model::VerifyDriversLicenseRequest::DeclineSecondary { .. } => {
                                        r_q::renters
                                            .filter(r_q::drivers_license_expiration.is_null())
                                            .filter(r_q::drivers_license_image.is_not_null())
                                            .filter(r_q::drivers_license_image_secondary.is_not_null())
                                            .filter(r_q::requires_secondary_driver_lic.eq(true))
                                            .into_boxed()
                                    }
                                    helper_model::VerifyDriversLicenseRequest::RequireSecondary { .. } => {
                                        r_q::renters
                                            .filter(r_q::drivers_license_expiration.is_null())
                                            .filter(r_q::drivers_license_image.is_not_null())
                                            .filter(r_q::requires_secondary_driver_lic.eq(false))
                                            .into_boxed()
                                    }
                                    helper_model::VerifyDriversLicenseRequest::Approved { .. } => {
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
                                }.filter(r_q::id.eq(renter_id))
                                    .get_result::<model::Renter>(&mut pool);

                                let mut renter = match renter {
                                    Ok(renter) => { renter }
                                    Err(err) => {
                                        return match err {
                                            Error::NotFound => {
                                                let msg = helper_model::ErrorResponse {
                                                    title: "Renter Not Found".to_string(),
                                                    message: "Renter not found or already verified.".to_string()
                                                };
                                                methods::standard_replies::response_with_obj(&msg, StatusCode::NOT_FOUND)
                                            }
                                            _ => {
                                                methods::standard_replies::internal_server_error_response_500(
                                                    String::from("admin/verify-drivers-license: DB error loading renter by id"),
                                                )
                                            }
                                        }
                                    }
                                };

                                match body.clone() {
                                    helper_model::VerifyDriversLicenseRequest::DeclinePrimary { renter_id, reason, .. } => {
                                        let mut hasher = Sha256::new();
                                        let data = renter_id.to_le_bytes();
                                        (& mut hasher).update(data);
                                        let result = hasher.finalize();
                                        let file_name = renter.drivers_license_image.unwrap();
                                        let object_path: String = format!("user_docs/{}/{}", hex::encode_upper(result), file_name);
                                        integration::gcloud_storage_veygo::delete_object(object_path)
                                            .await;

                                        renter.drivers_license_image = None;

                                        let renter_moved = renter.clone();

                                        tokio::spawn(async move {
                                            let email = integration::sendgrid_veygo::make_email_obj(&renter_moved.student_email, &renter_moved.name);
                                            let email_content = helper_model::DocumentRejectionTemplate { document_name: "Driver's License", reason: &reason };
                                            let _email_result = integration::sendgrid_veygo::send_email(
                                                None,
                                                email,
                                                "Your Document is Declined",
                                                &email_content.render().unwrap(),
                                                None,
                                                None,
                                            ).await;
                                        });
                                    }
                                    helper_model::VerifyDriversLicenseRequest::DeclineSecondary { renter_id, reason, .. } => {
                                        let mut hasher = Sha256::new();
                                        let data = renter_id.to_le_bytes();
                                        (& mut hasher).update(data);
                                        let result = hasher.finalize();
                                        let file_name = renter.drivers_license_image_secondary.unwrap();
                                        let object_path: String = format!("user_docs/{}/{}", hex::encode_upper(result), file_name);
                                        integration::gcloud_storage_veygo::delete_object(object_path)
                                            .await;

                                        renter.drivers_license_image_secondary = None;

                                        let renter_moved = renter.clone();

                                        tokio::spawn(async move {
                                            let email = integration::sendgrid_veygo::make_email_obj(&renter_moved.student_email, &renter_moved.name);
                                            let email_content = helper_model::DocumentRejectionTemplate { document_name: "Driver's License", reason: &reason };
                                            let _email_result = integration::sendgrid_veygo::send_email(
                                                None,
                                                email,
                                                "Your Document is Declined",
                                                &email_content.render().unwrap(),
                                                None,
                                                None,
                                            ).await;
                                        });
                                    }
                                    helper_model::VerifyDriversLicenseRequest::RequireSecondary { drivers_license_number, drivers_license_state_region, reason, .. } => {
                                        renter.drivers_license_number = drivers_license_number;
                                        renter.drivers_license_state_region = drivers_license_state_region;
                                        renter.requires_secondary_driver_lic = true;

                                        let _reason = reason;
                                    }
                                    helper_model::VerifyDriversLicenseRequest::Approved {
                                        drivers_license_number,
                                        drivers_license_state_region,
                                        drivers_license_expiration,
                                        renter_address,
                                        .. } => {
                                        renter.drivers_license_number = drivers_license_number;
                                        renter.drivers_license_state_region = drivers_license_state_region;
                                        renter.drivers_license_expiration = Some(drivers_license_expiration);
                                        if renter.billing_address != renter_address {
                                            renter.billing_address = renter_address.clone();

                                            if let Some(addr) = renter_address {
                                                let stripe_id = &renter.stripe_id;
                                                let _ = integration::stripe_veygo::update_stripe_customer_address(stripe_id, addr).await;
                                            }
                                        }
                                    }
                                };

                                let save_result = renter.save_changes::<model::Renter>(&mut pool);

                                if let Err(err) = save_result {
                                    return methods::standard_replies::internal_server_error_response_500(
                                        format!("admin/verify-drivers-license: DB error saving renter by id: {}", err),
                                    )
                                }

                                match body {
                                    helper_model::VerifyDriversLicenseRequest::Approved { .. } => {
                                        tokio::spawn(async move {
                                            if let Some(renter_app_apns) = renter.apple_apns {
                                                let _ = integration::apns_veygo::send_notification(
                                                    &renter_app_apns, "Congrats", "Your driver's license has been approved", false
                                                ).await;
                                            }
                                        });
                                    }
                                    helper_model::VerifyDriversLicenseRequest::RequireSecondary { .. } => {
                                        tokio::spawn(async move {
                                            if let Some(renter_app_apns) = renter.apple_apns {
                                                let _ = integration::apns_veygo::send_notification(
                                                    &renter_app_apns, "Next Step Required", "Please submit a supplementary document for your license", false
                                                ).await;
                                            }
                                        });
                                    }
                                    _ => {
                                        tokio::spawn(async move {
                                            if let Some(renter_app_apns) = renter.apple_apns {
                                                let _ = integration::apns_veygo::send_notification(
                                                    &renter_app_apns, "Bad News", "Your driver's license has been declined", false
                                                ).await;
                                            }
                                        });
                                    }
                                };

                                let next_renter = r_q::renters
                                    .filter(r_q::drivers_license_expiration.is_null())
                                    .filter(r_q::drivers_license_image.is_not_null())
                                    .filter(
                                        r_q::requires_secondary_driver_lic.eq(false)
                                            .or(
                                                r_q::requires_secondary_driver_lic.eq(true)
                                                    .and(r_q::drivers_license_image_secondary.is_not_null())
                                            )
                                    )
                                    .limit(1)
                                    .get_result::<model::Renter>(&mut pool);

                                let next_renter = match next_renter {
                                    Ok(renter) => renter,
                                    Err(err) => {
                                        return match err {
                                            Error::NotFound => {
                                                let msg = helper_model::ErrorResponse {
                                                    title: "Renters All Verified".to_string(),
                                                    message: "You are all caught up".to_string()
                                                };
                                                methods::standard_replies::response_with_obj(msg, StatusCode::NOT_FOUND)
                                            }
                                            _ => {
                                                methods::standard_replies::internal_server_error_response_500(
                                                    String::from("admin/verify-drivers-license: DB error loading renter"),
                                                )
                                            }
                                        }
                                    }
                                };

                                let next_renter_clone = next_renter.clone();

                                let doc_path_unsigned: String = if next_renter.requires_secondary_driver_lic
                                {
                                    next_renter.drivers_license_image_secondary.unwrap()
                                } else {
                                    next_renter.drivers_license_image.unwrap()
                                };

                                let mut hasher = Sha256::new();
                                let data = next_renter.id.to_le_bytes();
                                hasher.update(data);
                                let result = hasher.finalize();
                                let object_path: String = format!("user_docs/{}/{}", hex::encode_upper(result), doc_path_unsigned);
                                let link = integration::gcloud_storage_veygo::get_signed_url(
                                    &object_path,
                                ).await;

                                let link = helper_model::FileLink{ file_link: link };
                                let msg = helper_model::RenterNeedVerify { renter: next_renter_clone.into(), file_link: link };

                                methods::standard_replies::response_with_obj(&msg, StatusCode::OK)
                            }
                        }
                    }
                }
            }
        })
}