use crate::{methods, model, POOL};
use diesel::prelude::*;
use http::{Method, StatusCode};
use warp::{Filter, Reply};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-mileage-packages")
        .and(warp::path::end())
        .and(warp::method())
        .and_then(async move |method: Method| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response();
            }
            use crate::schema::mileage_packages::dsl as mileage_package_query;
            let mut pool = POOL.get().unwrap();
            let mps: Vec<model::MileagePackage> = mileage_package_query::mileage_packages
                .filter(mileage_package_query::is_active)
                .order(mileage_package_query::miles)
                .get_results(&mut pool)
                .unwrap_or_default();
            let msg = serde_json::json!({"mileage-packages": mps});
            Ok::<_, warp::Rejection>((warp::reply::with_status(
                warp::reply::json(&msg),
                StatusCode::OK,
            ).into_response(),))
        })
}