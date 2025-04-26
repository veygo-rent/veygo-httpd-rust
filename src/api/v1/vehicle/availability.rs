use crate::methods::{tokens, user};
use crate::model::{AccessToken, Vehicle};
use crate::{POOL, model};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::task::spawn_blocking;
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct AvailabilityData {
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("availability")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(async move |body: AvailabilityData, token: String, user_id: i32, client_type: Option<String>| {
            let access_token = model::RequestToken {
                user_id,
                token,
            };
            let if_token_valid = tokens::verify_user_token(access_token.user_id.clone(), access_token.token.clone()).await;
            match if_token_valid {
                Ok(token_bool) => {
                    return if !token_bool {
                        tokens::token_invalid_wrapped_return(&access_token.token)
                    } else {
                        // Token is validated -> user_id is valid
                        let user_id_clone = access_token.user_id.clone();
                        let user = user::get_user_by_id(user_id_clone).await.unwrap();
                        let apartment_id_clone = user.apartment_id.clone();
                        let mut pool = POOL.clone().get().unwrap();
                        let vehicle_list = spawn_blocking(move || {
                            use crate::schema::vehicles::dsl::*;
                            use crate::model::Vehicle;
                            vehicles
                                .into_boxed().filter(apartment_id.eq(apartment_id_clone))
                                .filter(available.eq(true)).load::<Vehicle>(&mut pool).unwrap()
                        }).await.unwrap();

                        let apt_id = user.apartment_id;

                        let start_time: DateTime<Utc> = body.start_time;
                        let end_time: DateTime<Utc> = body.end_time;

                        let start_time_buffered = start_time - Duration::minutes(15);
                        let end_time_buffered = end_time + Duration::minutes(15);

                        let mut pool = POOL.clone().get().unwrap();
                        let conflicting_vehicle_ids = spawn_blocking({
                            move || {
                                use crate::schema::agreements::dsl::*;
                                use diesel::dsl::sql;

                                agreements
                                    .into_boxed()
                                    .filter(apartment_id.eq(apt_id))
                                    .filter(status.eq(model::AgreementStatus::Rental))
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
                                    .load::<i32>(&mut pool)
                            }
                        }).await.unwrap().unwrap();

                        let conflicting_set: HashSet<i32> = conflicting_vehicle_ids.into_iter().collect();
                        let available_vehicle_list: Vec<Vehicle> = vehicle_list
                            .into_iter()
                            .filter(|v| !conflicting_set.contains(&v.id))
                            .collect();
                        use crate::model::PublishVehicle;
                        let available_vehicle_list_publish: Vec<PublishVehicle> = available_vehicle_list.iter().map(|x| x.to_publish_vehicle().clone()).collect();

                        tokens::rm_token_by_binary(hex::decode(access_token.token).unwrap()).await;
                        let new_token = tokens::gen_token_object(access_token.user_id.clone(), client_type.clone()).await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.clone().get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut pool).unwrap().to_publish_access_token();
                        let msg = serde_json::json!({"available_vehicles": available_vehicle_list_publish});
                        Ok::<_, warp::Rejection>((tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&msg), StatusCode::OK)),))
                    }
                }
                Err(_msg) => {
                    tokens::token_not_hex_warp_return(&access_token.token)
                }
            }
        })
}
