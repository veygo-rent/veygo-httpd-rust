use crate::{POOL, methods, model, proj_config};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply, http::Method};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct AvailabilityData {
    #[serde(with = "chrono::serde::ts_seconds")]
    start_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    end_time: DateTime<Utc>,
    apartment_id: i32,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("availability")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        body: AvailabilityData,
                        auth: String,
                        user_agent: String| {
                if method != Method::POST {
                    // RETURN: METHOD_NOT_ALLOWED
                    return methods::standard_replies::method_not_allowed_response();
                }
                let now = Utc::now();
                if body.start_time < now || body.end_time < now || body.start_time + Duration::minutes(proj_config::RSVP_BUFFER) > body.end_time {
                    // RETURN: BAD_REQUEST
                    return methods::standard_replies::bad_request("Time is invalid")
                }
                let mut pool = POOL.get().unwrap();
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
                    Err(_) => {
                        methods::tokens::token_not_hex_warp_return()
                    }
                    Ok(token_bool) => {
                        if !token_bool {
                            // RETURN: UNAUTHORIZED
                            methods::tokens::token_invalid_wrapped_return()
                        } else {
                            // gen new token
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
                            let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .into();

                            let user = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();
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
                            use crate::schema::locations::dsl as locations_query;
                            use crate::schema::vehicles::dsl as vehicles_query;
                            let all_vehicles = vehicles_query::vehicles
                                .inner_join(locations_query::locations)
                                .filter(locations_query::apartment_id.eq(&body.apartment_id))
                                .select((vehicles_query::vehicles::all_columns(), locations_query::locations::all_columns()))
                                .get_results::<(model::Vehicle, model::Location)>(&mut pool).unwrap_or_default();

                            let time_delta = body.end_time - body.start_time;
                            let start_time = body.start_time - Duration::hours(1);
                            let end_time = body.start_time + Duration::days(time_delta.num_days() + 1);

                            let start_time_buffered = start_time - Duration::minutes(proj_config::RSVP_BUFFER);
                            let end_time_buffered = end_time + Duration::minutes(proj_config::RSVP_BUFFER);

                            #[derive(
                                Serialize, Deserialize,
                            )]
                            struct BlockedRange {
                                #[serde(with = "chrono::serde::ts_seconds")]
                                start_time: DateTime<Utc>,
                                #[serde(with = "chrono::serde::ts_seconds")]
                                end_time: DateTime<Utc>,
                            }
                            #[derive(
                                Serialize, Deserialize,
                            )]
                            struct VehicleWithBlockedDurations {
                                vehicle: model::PublishRenterVehicle,
                                blocked_durations: Vec<BlockedRange>,
                            }
                            #[derive(
                                Serialize, Deserialize,
                            )]
                            struct LocationWithVehicles {
                                location: model::Location,
                                vehicles: Vec<VehicleWithBlockedDurations>,
                            }
                            let mut vehicles_by_location: HashMap<i32, LocationWithVehicles> = HashMap::new();
                            use crate::schema::agreements::dsl as agreements_query;
                            for (vehicle, location) in all_vehicles {

                                let mut blocked_durations: Vec<BlockedRange> = Vec::new();

                                let agreements_blocking: Vec<model::Agreement> = agreements_query::agreements
                                    .filter(agreements_query::vehicle_id.eq(&vehicle.id))
                                    .filter(agreements_query::status.eq(model::AgreementStatus::Rental))
                                    .filter(
                                        methods::diesel_fn::coalesce(agreements_query::actual_pickup_time, agreements_query::rsvp_pickup_time)
                                            .lt(end_time_buffered)
                                            .and(
                                                methods::diesel_fn::coalesce(
                                                    agreements_query::actual_drop_off_time,
                                                    methods::diesel_fn::greatest(agreements_query::rsvp_drop_off_time, diesel::dsl::now)
                                                )
                                                    .gt(start_time_buffered)
                                            )
                                    )
                                    .get_results::<model::Agreement>(&mut pool).unwrap_or_default();

                                for agreement in agreements_blocking {
                                    let pickup_time = agreement.actual_pickup_time.unwrap_or(agreement.rsvp_pickup_time) - Duration::minutes(proj_config::RSVP_BUFFER);
                                    let drop_off_time = agreement.actual_drop_off_time.unwrap_or(agreement.rsvp_drop_off_time) + Duration::minutes(proj_config::RSVP_BUFFER);

                                    let blocking_start_time = {
                                        if pickup_time >= start_time {
                                            pickup_time
                                        } else {
                                            start_time
                                        }
                                    };
                                    let blocking_end_time = {
                                        if drop_off_time <= end_time {
                                            drop_off_time
                                        } else {
                                            end_time
                                        }
                                    };

                                    (&mut blocked_durations).push(BlockedRange{ start_time: blocking_start_time, end_time: blocking_end_time });
                                }
                                let entry = (&mut vehicles_by_location).entry(location.id).or_insert_with(|| LocationWithVehicles {
                                    location,
                                    vehicles: Vec::new(),
                                });
                                (&mut entry.vehicles).push(VehicleWithBlockedDurations { vehicle: vehicle.into(), blocked_durations });
                            }
                            let locations_with_vehicles: Vec<LocationWithVehicles> = vehicles_by_location.into_values().collect();
                            let msg = serde_json::json!({"vehicles": locations_with_vehicles});
                            // RETURN: OK
                            Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&msg), StatusCode::OK)),))
                        }
                    }
                }
            },
        )
}
