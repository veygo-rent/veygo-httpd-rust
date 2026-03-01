use chrono::{DateTime, Utc};
use diesel::RunQueryDsl;
use http::{Method, StatusCode};
use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Reply};
use crate::{POOL, methods, model};
use diesel::prelude::*;
use diesel::result::Error;
use crate::helper_model::VeygoError;

#[derive(Deserialize, Serialize, Clone)]
struct PromoData {
    code: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    date_of_rental: DateTime<Utc>,
    apartment_id: i32,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("verify-promo")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method,
                              body: PromoData,
                              auth: String,
                              user_agent: String| {
            if method != Method::POST {
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
                            methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Token verification unexpected error"))
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Token extension failed (returned false)"));
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Token extension error"));
                        }
                    }
                    let mut pool = POOL.get().unwrap();

                    // check if the apartment is valid

                    if body.apartment_id <= 1 {
                        // RETURN: FORBIDDEN
                        // apartment id should be greater than 1, since 1 is the HQ and is for mgmt only
                        return methods::standard_replies::apartment_not_allowed_response(body.apartment_id);
                    }
                    use crate::schema::apartments::dsl as apartments_query;
                    let apt_in_request = apartments_query::apartments
                        .find(&body.apartment_id)
                        .get_result::<model::Apartment>(&mut pool);

                    let apt = match apt_in_request {
                        Ok(apt) => { apt }
                        Err(err) => {
                            return match err {
                                Error::NotFound => {
                                    methods::standard_replies::apartment_not_allowed_response(body.apartment_id)
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Database error loading apartment"))
                                }
                            }
                        }
                    };

                    use crate::schema::renters::dsl as r_q;
                    let renter = r_q::renters.find(&access_token.user_id).get_result::<model::Renter>(&mut pool);
                    let Ok(renter) = renter else {
                        return methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Database error loading renter"))
                    };
                    if apt.uni_id != 1 && renter.employee_tier != model::EmployeeTier::Admin && renter.apartment_id != body.apartment_id {
                        // RETURN: FORBIDDEN
                        return methods::standard_replies::apartment_not_allowed_response(body.apartment_id);
                    }
                    if !apt.is_operating {
                        // RETURN: FORBIDDEN
                        return methods::standard_replies::apartment_not_operational();
                    }

                    use crate::schema::promos::dsl as promos_query;
                    let promo = promos_query::promos
                        .filter(promos_query::code.eq(&body.code))
                        .filter(promos_query::is_enabled)
                        .filter(promos_query::exp.gt(&body.date_of_rental))
                        .get_result::<model::Promo>(&mut pool);
                    let promo = match promo {
                        Ok(promo) => { promo }
                        Err(err) => {
                            return match err {
                                Error::NotFound => {
                                    methods::standard_replies::promo_code_not_allowed_response(&body.code)
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Database error loading promo"))
                                }
                            }
                        }
                    };

                    // check if this renter already uses the promo code
                    use crate::schema::agreements::dsl as agreement_query;
                    let count_of_this_renter_usage = agreement_query::agreements
                        .filter(agreement_query::promo_id.eq(Some(&promo.code)))
                        .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                        .filter(agreement_query::renter_id.eq(&access_token.user_id))
                        .count()
                        .get_result::<i64>(&mut pool);
                    let Ok(count_of_this_renter_usage) = count_of_this_renter_usage else {
                        return methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Database error counting renter promo usage"))
                    };
                    if count_of_this_renter_usage >= 1 {
                        return methods::standard_replies::promo_code_not_allowed_response(&body.code);
                    }

                    // check if someone else already uses the promo code when it's one-time only
                    if promo.is_one_time {
                        let count_of_agreements = agreement_query::agreements
                            .filter(agreement_query::promo_id.eq(Some(&promo.code)))
                            .filter(agreement_query::status.ne(model::AgreementStatus::Canceled))
                            .count()
                            .get_result::<i64>(&mut pool);
                        let Ok(count_of_agreements) = count_of_agreements else {
                            return methods::standard_replies::internal_server_error_response(String::from("user/verify-promo: Database error counting promo usage"))
                        };
                        if count_of_agreements >= 1 {
                            return methods::standard_replies::promo_code_not_allowed_response(&body.code);
                        }
                    }

                    {
                        // check if the promo code is for a specific renter
                        if let Some(specified_user_id) = promo.user_id &&
                            renter.id != specified_user_id
                        {
                            return methods::standard_replies::promo_code_not_allowed_response(&body.code);
                        }
                        // check if the promo code is for a specific apartment
                        if let Some(specified_apartment_id) = promo.apt_id &&
                            !(apt.uni_id != 1 && apt.id == specified_apartment_id)
                        {
                            return methods::standard_replies::promo_code_not_allowed_response(&body.code);
                        }
                        // check if the promo code is for a specific university
                        if let Some(specified_uni_id) = promo.uni_id &&
                            !(apt.uni_id == 1 && apt.id == specified_uni_id || apt.uni_id == specified_uni_id)
                        {
                            return methods::standard_replies::promo_code_not_allowed_response(&body.code);
                        }
                    }

                    let pub_promo: model::PublishPromo = promo.into();
                    methods::standard_replies::response_with_obj(&pub_promo, StatusCode::OK)
                }
            }
        })
}
