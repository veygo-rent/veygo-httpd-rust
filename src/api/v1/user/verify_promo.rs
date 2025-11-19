use chrono::{DateTime, Utc};
use diesel::RunQueryDsl;
use http::{Method, StatusCode};
use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Rejection, Reply};
use crate::{POOL, methods, model};
use diesel::prelude::*;

#[derive(Deserialize, Serialize, Clone)]
struct PromoData {
    code: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    date_of_rental: DateTime<Utc>,
    apartment_id: i32, // could be university or apartment id
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
                return methods::tokens::token_invalid_wrapped_return(&auth);
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => int,
                Err(_) => {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
            };

            let access_token = model::RequestToken {
                user_id,
                token: token_and_id[0].parse().unwrap(),
            };
            let if_token_valid =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
                    .await;
            if if_token_valid.is_err() {
                return methods::tokens::token_not_hex_warp_return(&access_token.token);
            }
            let token_is_valid = if_token_valid.unwrap();
            if !token_is_valid {
                return methods::tokens::token_invalid_wrapped_return(&access_token.token);
            }

            // token is valid, proceed to verify promo code
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
            use crate::schema::access_tokens::dsl as access_tokens_query;
            let mut pool = POOL.get().unwrap();
            let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens_query::access_tokens)
                .values(&new_token)
                .get_result::<model::AccessToken>(&mut pool)
                .unwrap()
                .into();

            // check if the apartment is valid

            if body.apartment_id <= 1 {
                // RETURN: FORBIDDEN
                // apartment id should be greater than 1, since 1 is the HQ and is for mgmt only
                return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), body.apartment_id);
            }
            use crate::schema::apartments::dsl as apartments_query;
            let apt_in_request = apartments_query::apartments
                .filter(apartments_query::id.eq(&body.apartment_id))
                .get_result::<model::Apartment>(&mut pool);
            if apt_in_request.is_err() {
                // RETURN: FORBIDDEN
                return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), body.apartment_id);
            }
            let apt = apt_in_request.unwrap();
            if apt.uni_id.is_some() && user.employee_tier != model::EmployeeTier::Admin && user.apartment_id != body.apartment_id {
                // RETURN: FORBIDDEN
                return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), body.apartment_id);
            }
            if !apt.is_operating {
                // RETURN: FORBIDDEN
                return methods::standard_replies::apartment_not_operational_wrapped(new_token_in_db_publish.clone());
            }


            use crate::schema::promos::dsl as promos_query;
            let promo_result = promos_query::promos
                .filter(promos_query::code.eq(&body.code))
                .filter(promos_query::is_enabled)
                .filter(promos_query::exp.gt(&body.date_of_rental))
                .get_result::<model::Promo>(&mut pool);
            return match promo_result {
                Err(_) => methods::standard_replies::promo_code_not_allowed_response(new_token_in_db_publish.clone(), &body.code),
                Ok(promo) => {
                    // check if this renter already uses the promo code
                    use crate::schema::agreements::dsl as agreement_query;
                    let count_of_this_renter_usage: i64 = agreement_query::agreements
                        .filter(agreement_query::promo_id.eq(Some(&promo.code)))
                        .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                        .filter(agreement_query::renter_id.eq(&user.id))
                        .count()
                        .get_result::<i64>(&mut pool)
                        .unwrap();
                    if count_of_this_renter_usage >= 1 {
                        return methods::standard_replies::promo_code_not_allowed_response(new_token_in_db_publish.clone(), &body.code);
                    }
                    // check if someone else already uses the promo code when it's one-time only
                    if promo.is_one_time {
                        let count_of_agreements: i64 = agreement_query::agreements
                            .filter(agreement_query::promo_id.eq(Some(&promo.code)))
                            .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                            .count()
                            .get_result::<i64>(&mut pool)
                            .unwrap();
                        if count_of_agreements >= 1 {
                            return methods::standard_replies::promo_code_not_allowed_response(new_token_in_db_publish.clone(), &body.code);
                        }
                    }
                    {
                        // check if the promo code is for a specific renter
                        if let Some(specified_user_id) = promo.user_id && user.id != specified_user_id {
                            return methods::standard_replies::promo_code_not_allowed_response(new_token_in_db_publish.clone(), &body.code);
                        }
                        // check if the promo code is for a specific apartment
                        if let Some(specified_apartment_id) = promo.apt_id && body.apartment_id != specified_apartment_id {
                            return methods::standard_replies::promo_code_not_allowed_response(new_token_in_db_publish.clone(), &body.code);
                        }
                        // check if the promo code is for a specific university
                        if let Some(specified_uni_id) = promo.uni_id && apt.uni_id != Some(specified_uni_id) && apt.id != specified_uni_id {
                            return methods::standard_replies::promo_code_not_allowed_response(new_token_in_db_publish.clone(), &body.code);
                        }
                    }
                    let pub_promo: model::PublishPromo = promo.into();
                    let msg = serde_json::json!({"promo": pub_promo});
                    let with_status = warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK);

                    Ok::<_, Rejection>((methods::tokens::wrap_json_reply_with_token(
                        new_token_in_db_publish,
                        with_status,
                    ),))
                }
            };
        })
}
