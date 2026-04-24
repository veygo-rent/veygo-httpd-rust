use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, prelude::*};
use warp::{Filter, Reply};
use warp::http::{Method, StatusCode};
use crate::{connection_pool, helper_model, methods, model};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("upcoming")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        auth: String,
                        user_agent: String| {
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response_405();
                }
                let mut pool = connection_pool().await.get().unwrap();
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_return();
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_return();
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                match if_token_valid_result {
                    Err(e) => {
                        match e {
                            helper_model::VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            helper_model::VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/get_upcoming: Token verification unexpected error"),
                                )
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent).await;

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response_500(
                                        String::from("agreement/get_upcoming: Token extension failed (returned false)"),
                                    );
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/get_upcoming: Token extension error"),
                                );
                            }
                        }

                        let now = chrono::Utc::now();

                        use crate::schema::agreements::dsl as agreement_query;
                        use crate::schema::apartments::dsl as apartment_query;
                        use crate::schema::locations::dsl as location_query;
                        use crate::schema::vehicles::dsl as vehicle_query;
                        let agreements = agreement_query::agreements
                            .inner_join(
                                location_query::locations
                                    .inner_join(apartment_query::apartments)
                            )
                            .inner_join(vehicle_query::vehicles)
                            .filter(agreement_query::renter_id.eq(&access_token.user_id))
                            .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                            .filter(agreement_query::actual_pickup_time.is_null())
                            .filter(agreement_query::rsvp_drop_off_time.ge(now))
                            .order_by(agreement_query::rsvp_pickup_time.asc())
                            .select((
                                agreement_query::agreements::all_columns(),
                                apartment_query::name,
                                apartment_query::timezone,
                                vehicle_query::name
                            ))
                            .get_results::<(model::Agreement, String, String, String)>(&mut pool);

                        match agreements {
                            Ok(ags) => {
                                let trips: Vec<helper_model::TripInfo> = ags
                                    .into_iter()
                                    .map(|ag| helper_model::TripInfo {
                                        agreement: ag.0,
                                        apartment_timezone: ag.2,
                                        location_name: ag.1,
                                        vehicle_name: ag.3,
                                    })
                                    .collect();
                                methods::standard_replies::response_with_obj(trips, StatusCode::OK)
                            }
                            Err(_) => {
                                methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/get_upcoming: Database error loading agreements"),
                                )
                            }
                        }
                    }
                }
            }
        )
}