use crate::{POOL, methods, model};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use diesel::dsl::sql;
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
                    return methods::standard_replies::method_not_allowed_response();
                }
                let mut pool = POOL.get().unwrap();
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_wrapped_return(&auth);
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;

                match if_token_valid_result {
                    Err(_) => {
                        methods::tokens::token_not_hex_warp_return(&access_token.token)
                    }
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
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
                            let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .to_publish_access_token();

                            if body.apartment_id <= 1 {
                                return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), body.apartment_id);
                            }
                            let user = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();
                            use crate::schema::apartments::dsl as apartments_query;
                            let apt_in_request = apartments_query::apartments.filter(apartments_query::id.eq(&body.apartment_id)).get_result::<model::Apartment>(&mut pool);
                            if apt_in_request.is_err() {
                                return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), body.apartment_id);
                            }
                            let apt = apt_in_request.unwrap();
                            if apt.uni_id != 1 && (user.employee_tier != model::EmployeeTier::Admin || user.apartment_id != body.apartment_id) {
                                return methods::standard_replies::apartment_not_allowed_response(new_token_in_db_publish.clone(), body.apartment_id);
                            }
                            if !apt.is_operating {
                                return methods::standard_replies::apartment_not_operational_wrapped(new_token_in_db_publish.clone());
                            }
                            use crate::schema::locations::dsl as locations_query;
                            use crate::schema::vehicles::dsl as vehicles_query;
                            let all_vehicles = vehicles_query::vehicles
                                .inner_join(locations_query::locations)
                                .filter(locations_query::apartment_id.eq(&body.apartment_id))
                                .select((vehicles_query::vehicles::all_columns(), locations_query::locations::all_columns()))
                                .get_results::<(model::Vehicle, model::Location)>(&mut pool).unwrap_or_default();

                            let start_time_buffered = body.start_time - Duration::minutes(15);
                            let end_time_buffered = body.end_time + Duration::minutes(15);

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
                                vehicle: model::Vehicle,
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
                                        sql::<Bool>("(COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                            .bind::<Timestamptz, _>(end_time_buffered)
                                            .sql(") AND (COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                            .bind::<Timestamptz, _>(start_time_buffered)
                                            .sql(")")
                                    )
                                    .get_results::<model::Agreement>(&mut pool).unwrap_or_default();

                                for agreement in agreements_blocking {
                                    let pickup_time = agreement.actual_pickup_time.unwrap_or(agreement.rsvp_pickup_time);
                                    let drop_off_time = agreement.actual_drop_off_time.unwrap_or(agreement.rsvp_drop_off_time);

                                    let blocking_start_time = {
                                        if pickup_time >= start_time_buffered {
                                            pickup_time
                                        } else {
                                            start_time_buffered
                                        }
                                    };
                                    let blocking_end_time = {
                                        if drop_off_time <= end_time_buffered {
                                            drop_off_time
                                        } else {
                                            end_time_buffered
                                        }
                                    };

                                    (&mut blocked_durations).push(BlockedRange{ start_time: blocking_start_time, end_time: blocking_end_time });
                                }
                                let entry = (&mut vehicles_by_location).entry(location.id).or_insert_with(|| LocationWithVehicles {
                                    location,
                                    vehicles: Vec::new(),
                                });
                                (&mut entry.vehicles).push(VehicleWithBlockedDurations { vehicle, blocked_durations });
                            }
                            let locations_with_vehicles: Vec<LocationWithVehicles> = vehicles_by_location.into_values().collect();
                            let msg = serde_json::json!({"vehicles": locations_with_vehicles});
                            Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&msg), StatusCode::OK)),))
                        }
                    }
                }
            },
        )
}
