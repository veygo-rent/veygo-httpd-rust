use crate::model::AccessToken;
use crate::{POOL, methods, model};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
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
        .and_then(async move |method:Method, body: AvailabilityData, auth: String, user_agent: String| {
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
                Ok(int) => {
                    int
                }
                Err(_) => {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
            };

            let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
            let if_token_valid = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
            match if_token_valid {
                Ok(token_bool) => {
                    return if !token_bool {
                        methods::tokens::token_invalid_wrapped_return(&access_token.token)
                    } else {
                        // Token is validated -> user_id is valid
                        let user = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();
                        let apartment_id_clone = user.apartment_id.clone();
                        let mut pool = POOL.get().unwrap();
                        use crate::schema::vehicles::dsl::*;
                        use crate::model::Vehicle;
                        let vehicle_list = vehicles
                                .into_boxed().filter(crate::schema::vehicles::apartment_id.eq(apartment_id_clone))
                                .filter(available.eq(true)).load::<Vehicle>(&mut pool).unwrap();

                        let apt_id = user.apartment_id;

                        let start_time: DateTime<Utc> = body.start_time;
                        let end_time: DateTime<Utc> = body.end_time;

                        let start_time_buffered = start_time - Duration::minutes(15);
                        let end_time_buffered = end_time + Duration::minutes(15);

                        let mut pool = POOL.get().unwrap();
                        use crate::schema::agreements::dsl::*;
                        use diesel::dsl::sql;
                        let conflicting_vehicle_ids = agreements
                            .into_boxed()
                            .filter(crate::schema::agreements::apartment_id.eq(apt_id))
                            .filter(status.eq(crate::model::AgreementStatus::Rental))
                            .filter(
                                // We chain .sql() and .bind() to handle multiple placeholders
                                sql::<Bool>("COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                    .bind::<Timestamptz, _>(start_time_buffered)
                                    .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                    .bind::<Timestamptz, _>(start_time_buffered)
                                    .sql(" OR COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                    .bind::<Timestamptz, _>(end_time_buffered)
                                    .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                    .bind::<Timestamptz, _>(end_time_buffered)
                            )
                            .select(vehicle_id)
                            .distinct()
                            .load::<i32>(&mut pool).unwrap();

                        let conflicting_set: HashSet<i32> = conflicting_vehicle_ids.into_iter().collect();
                        let available_vehicle_list: Vec<Vehicle> = vehicle_list
                            .into_iter()
                            .filter(|v| !conflicting_set.contains(&v.id))
                            .collect();
                        use crate::model::PublishVehicle;
                        let available_vehicle_list_publish: Vec<PublishVehicle> = available_vehicle_list.iter().map(|x| x.to_publish_vehicle().clone()).collect();

                        let _ = methods::tokens::rm_token_by_binary(hex::decode(access_token.token).unwrap()).await;
                        let new_token = methods::tokens::gen_token_object(&access_token.user_id, &user_agent).await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut pool).unwrap().to_publish_access_token();
                        let msg = serde_json::json!({"available_vehicles": available_vehicle_list_publish});
                        Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&msg), StatusCode::OK)),))
                    }
                }
                Err(_msg) => {
                    methods::tokens::token_not_hex_warp_return(&access_token.token)
                }
            }
        })
}
