use crate::model::{Apartment, PublishApartment};
use crate::{POOL, schema};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use tokio::task::spawn_blocking;
use warp::Filter;
use warp::http::StatusCode;

pub fn get_apartments()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get")
        .and(warp::get())
        .and(warp::path::end())
        .and_then(move || async move {
            use schema::apartments::dsl::*;
            let mut pool = POOL.clone().get().unwrap();
            let results = spawn_blocking(move || {
                apartments
                    .filter(is_operating.eq(true))
                    .load::<Apartment>(&mut pool)
            })
            .await
            .unwrap()
            .unwrap();

            let apt_publish: Vec<PublishApartment> = results
                .iter()
                .map(|x| x.to_publish_apartment().clone())
                .collect();
            let msg = serde_json::json!({"apartments": apt_publish});
            Ok::<_, warp::Rejection>((warp::reply::with_status(
                warp::reply::json(&msg),
                StatusCode::OK,
            ),))
        })
}
