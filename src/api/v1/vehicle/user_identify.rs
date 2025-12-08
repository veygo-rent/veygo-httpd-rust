use crate::{POOL, methods, model, helper_model};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("user-identify")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, auth: String, user_agent: String| {
                // Checking method is GET
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response();
                }

                // Pool connection
                let mut pool = POOL.get().unwrap();

                // Checking token
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_wrapped_return();
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_wrapped_return();
                    }
                };
                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                match if_token_valid_result {
                    Err(_) => methods::tokens::token_not_hex_warp_return(),
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_not_hex_warp_return()
                        } else {
                            // Generate new token
                            let token_clone = access_token.clone();
                            methods::tokens::rm_token_by_binary(
                                hex::decode(token_clone.token).unwrap(),
                            ).await;
                            let new_token = methods::tokens::gen_token_object(
                                &access_token.user_id,
                                &user_agent,
                            ).await;
                            use crate::schema::access_tokens::dsl as access_token_query;
                            let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_token_query::access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .into();

                            // Get current user
                            let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();

                            use crate::schema::agreements::dsl as agreement_query;
                            use crate::schema::vehicles::dsl as vehicle_query;
                            let now = chrono::Utc::now();
                            let now_plus_buffer = now + chrono::Duration::minutes(15);
                            let current_vehicle_result = agreement_query::agreements
                                .inner_join(vehicle_query::vehicles)
                                .filter(agreement_query::renter_id.eq(&user_in_request.id))
                                .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                                .filter(agreement_query::actual_drop_off_time.is_null())
                                .filter(
                                    agreement_query::actual_pickup_time.is_not_null()
                                        .or(agreement_query::rsvp_drop_off_time.ge(now))
                                )
                                .filter(agreement_query::rsvp_pickup_time.le(&now_plus_buffer))
                                .order_by(agreement_query::rsvp_pickup_time.asc())
                                .select(
                                    vehicle_query::vehicles::all_columns()
                                )
                                .first::<model::Vehicle>(&mut pool);
                            if let Ok(vehicle) = current_vehicle_result {
                                
                            }
                            methods::standard_replies::not_implemented_response()
                        }
                    }
                }
            }
        )
}