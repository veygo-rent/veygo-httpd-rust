use std::collections::HashSet;
use crate::methods::tokens;
use crate::model::{AccessToken, Vehicle};
use crate::{db, model};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
use serde_derive::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use warp::http::StatusCode;
use warp::Filter;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct AvailabilityData {
    access_token: model::RequestBodyToken, // contains 'user_id' and 'token'
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

pub fn vehicle_availability(
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("availability")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |body: AvailabilityData, client_type: Option<String>| {
            async move {
                let if_token_valid = tokens::verify_user_token(body.access_token.user_id.clone(), body.access_token.token.clone()).await;
                match if_token_valid {
                    Ok(token_bool) => {
                        if !token_bool {
                            let error_msg = serde_json::json!({"token": &body.access_token.token, "error": "Token not valid"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        } else {
                            // Token is validated
                            let user_id_clone = body.access_token.user_id.clone();
                            let user = spawn_blocking(move || {
                                use crate::schema::renters::dsl::*;
                                use crate::model::Renter;
                                renters.filter(id.eq(user_id_clone)).get_result::<Renter>(&mut db::get_connection_pool().get().unwrap()).unwrap()
                            }).await.unwrap();
                            let apartment_id_clone = user.apartment_id.clone();
                            let vehicle_list = spawn_blocking(move || {
                                use crate::schema::vehicles::dsl::*;
                                use crate::model::Vehicle;
                                vehicles.filter(apartment_id.eq(apartment_id_clone)).filter(available.eq(true)).load::<Vehicle>(&mut db::get_connection_pool().get().unwrap()).unwrap()
                            }).await.unwrap();

                            let apt_id = user.apartment_id;

                            let start_time: DateTime<Utc> = body.start_time;
                            let end_time: DateTime<Utc>   = body.end_time;

                            let start_time_buffered = start_time - Duration::minutes(30);
                            let end_time_buffered   = end_time   + Duration::minutes(30);

                            let conflicting_vehicle_ids = spawn_blocking({
                                move || {
                                    use crate::schema::agreements::dsl::*;
                                    use diesel::dsl::sql;

                                    agreements
                                        .filter(apartment_id.eq(apt_id))
                                        .filter(
                                            // We chain .sql() and .bind() to handle multiple placeholders
                                            sql::<Bool>("COALESCE(actual_pickup_time, rsvp_pickup_time) < ")
                                                .bind::<Timestamptz, _>(end_time_buffered)
                                                .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) > ")
                                                .bind::<Timestamptz, _>(start_time_buffered)
                                        )
                                        .select(vehicle_id)
                                        .distinct()
                                        .load::<i32>(&mut db::get_connection_pool().get().unwrap())
                                }
                            }).await.unwrap().unwrap();

                            let conflicting_set: HashSet<i32> = conflicting_vehicle_ids.into_iter().collect();
                            let available_vehicle_list: Vec<Vehicle> = vehicle_list
                                .into_iter()
                                .filter(|v| !conflicting_set.contains(&v.id))
                                .collect();
                            use crate::model::PublishVehicle;
                            let available_vehicle_list_publish: Vec<PublishVehicle> = available_vehicle_list.iter().map(|x| x.to_publish_vehicle().clone()).collect();

                            tokens::rm_token_by_binary(hex::decode(body.access_token.token).unwrap()).await;
                            let new_token = tokens::gen_token_object(body.access_token.user_id.clone(), client_type.clone()).await;
                            use crate::schema::access_tokens::dsl::*;
                            let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap()).unwrap().to_publish_access_token();

                            let msg = serde_json::json!({"access_token": new_token_in_db_publish, "available_vehicles": available_vehicle_list_publish});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),))
                        }
                    }
                    Err(_msg) => {
                        let error_msg = serde_json::json!({"token": &body.access_token.token, "error": "Token not in hex format"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                    }
                }
            }
        })
}
