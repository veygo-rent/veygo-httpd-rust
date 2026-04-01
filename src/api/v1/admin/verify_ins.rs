use diesel::result::Error;
use diesel::prelude::*;
use http::{Method, StatusCode};
use sha2::{Digest, Sha256};
use warp::{Filter, Reply};
use crate::{helper_model, integration, methods, model, schema, POOL};
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("verify-insurance")
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: helper_model::VerifyInsuranceRequest, auth: String, user_agent: String| {
            if method != Method::PATCH {
                return methods::standard_replies::method_not_allowed_response();
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
                            methods::standard_replies::internal_server_error_response(
                                String::from("admin/verify-insurance: Token verification unexpected error"),
                            )
                        }
                    }
                }
                Ok((_token, token_id)) => {
                    let user = methods::user::get_user_by_id(&access_token.user_id)
                        .await;
                    let Ok(user) = user else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("admin/verify-insurance: Database error loading renter by id"),
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
                            methods::standard_replies::internal_server_error_response(
                                String::from("admin/verify-insurance: Token extension error"),
                            )
                        }
                        Ok(is_renewed) => {
                            if !is_renewed {
                                methods::standard_replies::internal_server_error_response(
                                    String::from("admin/verify-insurance: Token extension failed (returned false)"),
                                )
                            } else {
                                let renter_id = match body {
                                    helper_model::VerifyInsuranceRequest::Approved { renter_id, .. } => renter_id,
                                    helper_model::VerifyInsuranceRequest::Declined { renter_id, .. } => renter_id,
                                };

                                use schema::renters::dsl as r_q;

                                let mut pool = POOL.get().unwrap();

                                let renter = r_q::renters
                                    .filter(r_q::insurance_liability_expiration.is_null())
                                    .filter(r_q::insurance_id_image.is_not_null())
                                    .filter(r_q::id.eq(renter_id))
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
                                                methods::standard_replies::internal_server_error_response(
                                                    String::from("admin/verify-insurance: DB error loading renter by id"),
                                                )
                                            }
                                        }
                                    }
                                };
                                
                                match body { 
                                    helper_model::VerifyInsuranceRequest::Approved { insurance_liability_expiration, insurance_collision_valid, .. } => {
                                        renter.insurance_liability_expiration = Some(insurance_liability_expiration);
                                        renter.insurance_collision_valid = insurance_collision_valid;
                                    },
                                    helper_model::VerifyInsuranceRequest::Declined { reason, .. } => {
                                        let mut hasher = Sha256::new();
                                        let data = renter_id.to_le_bytes();
                                        (& mut hasher).update(data);
                                        let result = hasher.finalize();
                                        let file_name = renter.insurance_id_image.unwrap();
                                        let object_path: String = format!("user_docs/{}/{}", hex::encode_upper(result), file_name);
                                        integration::gcloud_storage_veygo::delete_object(object_path)
                                            .await;

                                        renter.insurance_id_image = None;

                                        let _reason = reason;
                                    }
                                }

                                let save_result = renter.save_changes::<model::Renter>(&mut pool);

                                if let Err(_err) = save_result {
                                    return methods::standard_replies::internal_server_error_response(
                                        format!("admin/verify-insurance: DB error saving renter by id: {}", _err),
                                    )
                                }

                                let next_renter = r_q::renters
                                    .filter(r_q::insurance_liability_expiration.is_null())
                                    .filter(r_q::insurance_id_image.is_not_null())
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
                                                methods::standard_replies::internal_server_error_response(
                                                    String::from("admin/verify-insurance: DB error loading renter"),
                                                )
                                            }
                                        }
                                    }
                                };

                                let next_renter_clone = next_renter.clone();

                                let doc_path_unsigned: String = next_renter.insurance_id_image.unwrap();

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