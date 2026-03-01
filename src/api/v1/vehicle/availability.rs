use crate::{POOL, methods, model, proj_config};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use rust_decimal::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use diesel::result::Error;
use warp::http::StatusCode;
use warp::{Filter, Reply, http::Method};
use crate::helper_model::VeygoError;

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
                    Err(err) => {
                        match err {
                            VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Token verification unexpected error"))
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Token extension failed (returned false)"));
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Token extension error"));
                            }
                        }

                        let user = match methods::user::get_user_by_id(&access_token.user_id).await {
                            Ok(usr) => { usr }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Database error loading renter"))
                            }
                        };

                        if body.apartment_id <= 1 {
                            // RETURN: FORBIDDEN
                            // apartment id should be greater than 1, since 1 is the HQ and is for mgmt only
                            return methods::standard_replies::apartment_not_allowed_response(body.apartment_id);
                        }

                        use crate::schema::apartments::dsl as apartments_query;
                        let apt = apartments_query::apartments
                            .find(&body.apartment_id)
                            .get_result::<model::Apartment>(&mut pool);
                        let apt = match apt {
                            Ok(apt) => { apt }
                            Err(err) => {
                                return match err {
                                    Error::NotFound => {
                                        methods::standard_replies::apartment_not_allowed_response(body.apartment_id)
                                    }
                                    _ => {
                                        return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Database error loading apartment"))
                                    }
                                }
                            }
                        };
                        if apt.uni_id != 1 && user.employee_tier != model::EmployeeTier::Admin && user.apartment_id != body.apartment_id {
                            // RETURN: FORBIDDEN
                            return methods::standard_replies::apartment_not_allowed_response(body.apartment_id);
                        }
                        if !apt.is_operating {
                            // RETURN: FORBIDDEN
                            return methods::standard_replies::apartment_not_operational();
                        }

                        use crate::schema::agreements::dsl as agreements_query;

                        let renter_agreements_blocking_count = agreements_query::agreements
                            .filter(agreements_query::renter_id.eq(&access_token.user_id))
                            .filter(agreements_query::status.eq(model::AgreementStatus::Rental))
                            .filter(
                                methods::diesel_fn::coalesce(agreements_query::actual_pickup_time, agreements_query::rsvp_pickup_time)
                                    .lt(body.end_time + Duration::minutes(proj_config::RSVP_BUFFER))
                                    .and(
                                        methods::diesel_fn::coalesce(
                                            agreements_query::actual_drop_off_time,
                                            methods::diesel_fn::greatest(agreements_query::rsvp_drop_off_time, diesel::dsl::now)
                                        )
                                            .gt(body.start_time - Duration::minutes(proj_config::RSVP_BUFFER))
                                    )
                            )
                            .count()
                            .get_result::<i64>(&mut pool);
                        let Ok(renter_agreements_blocking_count) = renter_agreements_blocking_count else {
                            return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Database error counting renter blocking agreements"))
                        };
                        if renter_agreements_blocking_count > 0 {
                            return methods::standard_replies::double_booking_not_allowed()
                        }

                        use crate::schema::locations::dsl as locations_query;
                        use crate::schema::vehicles::dsl as vehicles_query;
                        let all_vehicles = vehicles_query::vehicles
                            .inner_join(locations_query::locations)
                            .filter(locations_query::apartment_id.eq(&body.apartment_id))
                            .select((vehicles_query::vehicles::all_columns(), locations_query::locations::all_columns()))
                            .get_results::<(model::Vehicle, model::Location)>(&mut pool);
                        let Ok(all_vehicles) = all_vehicles else {
                            return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Database error loading vehicles and locations"))
                        };

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
                        for (vehicle, location) in all_vehicles {

                            let mut blocked_durations: Vec<BlockedRange> = Vec::new();

                            let agreements_blocking = agreements_query::agreements
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
                                .get_results::<model::Agreement>(&mut pool);

                            let Ok(agreements_blocking) = agreements_blocking else {
                                return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Database error loading vehicle blocking agreements"))
                            };

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
                        // RETURN: OK
                        #[derive(
                            Serialize, Deserialize,
                        )]
                        struct Availability {
                            offer: model::RateOffer,
                            vehicles: Vec<LocationWithVehicles>,
                        }
                        let new_rate_offer = model::NewRateOffer{
                            renter_id: user_id,
                            apartment_id: body.apartment_id,
                            multiplier: Decimal::new(100, 2),
                        };
                        use crate::schema::rate_offers::dsl as rate_offers_query;
                        let rate_offer_add_result = diesel::insert_into(rate_offers_query::rate_offers)
                            .values(&new_rate_offer)
                            .get_result::<model::RateOffer>(&mut pool);

                        let Ok(rate_offer) = rate_offer_add_result else {
                            return methods::standard_replies::internal_server_error_response(String::from("vehicle/availability: Database error inserting rate offer"))
                        };

                        let resp = Availability{ offer: rate_offer, vehicles: locations_with_vehicles };

                        methods::standard_replies::response_with_obj(resp, StatusCode::OK)
                    }
                }
            },
        )
}
