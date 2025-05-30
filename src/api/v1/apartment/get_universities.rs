use crate::model::{Apartment, PublishApartment};
use crate::{POOL, schema};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use warp::Filter;
use warp::http::StatusCode;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-universities")
        .and(warp::get())
        .and(warp::path::end())
        .and_then(async move || {
            use schema::apartments::dsl::*;
            let mut pool = POOL.clone().get().unwrap();
            let results = apartments
                .into_boxed()
                .filter(is_operating.eq(true))
                .filter(is_uni.eq(true))
                .load::<Apartment>(&mut pool)
                .unwrap();

            let apt_publish: Vec<PublishApartment> = results
                .iter()
                .map(|x| x.to_publish_apartment().clone())
                .collect();
            let msg = serde_json::json!({"universities": apt_publish});
            Ok::<_, warp::Rejection>((warp::reply::with_status(
                warp::reply::json(&msg),
                StatusCode::OK,
            ),))
        })
}
