use crate::{POOL, methods, model, helper_model};
use diesel::prelude::*;
use diesel::result::Error;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use warp::http::{StatusCode, Method};
use warp::reply::with_status;
use warp::{Filter, Reply};
use crate::helper_model::VeygoError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UpdateApartmentBody {
    student_email: String,
    apartment_id: i32,
}

fn email_belongs_to_domain(email: &str, domain: &str) -> bool {
    email.ends_with(&format!("@{}", domain))
}

fn is_valid_email(email: &str) -> bool {
    lazy_static::lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(
            r"(?i)^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$"
        ).expect("Invalid regex");
    }
    // Check overall length (RFC 5321 limit is 254, but some say 320)
    if email.len() > 254 {
        return false;
    }
    EMAIL_REGEX.is_match(email)
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("update-apartment")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, body: UpdateApartmentBody, auth: String, user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }
                if !is_valid_email(&body.student_email) {
                    return methods::standard_replies::bad_request("Email is invalid")
                }

                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_return();
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_return();
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid = methods::tokens::verify_user_token(
                    &access_token.user_id,
                    &access_token.token,
                )
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

                        let mut pool = POOL.get().unwrap();
                        use crate::schema::apartments::dsl as a_q;
                        let apartment_result = a_q::apartments.filter(a_q::is_operating).find(&body.apartment_id).get_result::<model::Apartment>(&mut pool);

                        return match apartment_result {
                            Ok(apartment) => {
                                if !email_belongs_to_domain(&body.student_email, &apartment.accepted_school_email_domain) {
                                    let error_msg = helper_model::ErrorResponse{
                                        title: String::from("Email Error"),
                                        message: String::from("Your email is not accepted by Veygo. "),
                                    };
                                    return Ok((with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN).into_response(),))
                                }

                                use crate::schema::renters::dsl as r_q;
                                let renter = r_q::renters.find(&access_token.user_id).get_result::<model::Renter>(&mut pool);
                                let Ok(mut renter) = renter else {
                                    return methods::standard_replies::internal_server_error_response()
                                };

                                renter.student_email = body.student_email.clone();
                                renter.apartment_id = body.apartment_id.clone();
                                renter.lease_agreement_image = None;
                                renter.lease_agreement_expiration = None;
                                renter.student_email_expiration = None;

                                let renter_updated = diesel::update(r_q::renters.find(&access_token.user_id))
                                    .set(&renter).get_result::<model::Renter>(&mut pool);

                                match renter_updated {
                                    Ok(renter) => {
                                        let pub_renter: model::PublishRenter = renter.into();
                                        methods::standard_replies::response_with_obj(&pub_renter, StatusCode::OK)
                                    }
                                    Err(_) => {
                                        methods::standard_replies::internal_server_error_response()
                                    }
                                }
                            }
                            Err(err) => {
                                match err {
                                    Error::NotFound => {
                                        let msg = helper_model::ErrorResponse{
                                            title: "Apartment Not Available".to_string(), message: "The apartment you are trying to access does not exist or is not available. ".to_string()
                                        };
                                        methods::standard_replies::response_with_obj(msg, StatusCode::FORBIDDEN)
                                    }
                                    _ => {
                                        methods::standard_replies::internal_server_error_response()
                                    }
                                }
                            }
                        };
                    }
                };
            },
        )
}
