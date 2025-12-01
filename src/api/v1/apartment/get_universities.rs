use crate::model::Apartment;
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
            let mut pool = POOL.get().unwrap();
            let results: Vec<Apartment> = apartments
                .into_boxed()
                .filter(is_operating.eq(true))
                .filter(uni_id.eq(1))
                .filter(id.ne(1))
                .load::<Apartment>(&mut pool)
                .unwrap_or_default();

            let msg = serde_json::json!({"universities": results});
            Ok::<_, warp::Rejection>((warp::reply::with_status(
                warp::reply::json(&msg),
                StatusCode::OK,
            ),))
        })
}
