use crate::{model, POOL};
use diesel::prelude::*;
use http::StatusCode;
use warp::Filter;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-mileage-packages")
        .and(warp::path::end())
        .and(warp::get())
        .and_then(async move || {
            use crate::schema::mileage_packages::dsl as mileage_package_query;
            let mut pool = POOL.get().unwrap();
            let mps: Vec<model::MileagePackage> = mileage_package_query::mileage_packages
                .filter(mileage_package_query::is_active)
                .get_results(&mut pool)
                .unwrap_or_default();
            let msg = serde_json::json!({"mileage-packages": mps});
            Ok::<_, warp::Rejection>((warp::reply::with_status(
                warp::reply::json(&msg),
                StatusCode::OK,
            ),))
        })
}