use crate::{methods, model, connection_pool};
use diesel::prelude::*;
use warp::{Filter, Reply, http::{Method, StatusCode}};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("mileage-packages")
        .and(warp::path::end())
        .and(warp::method())
        .and_then(async move |method: Method| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response_405();
            }
            use crate::schema::mileage_packages::dsl as mileage_package_query;
            let mut pool = connection_pool().await.get().unwrap();
            let mps = mileage_package_query::mileage_packages
                .filter(mileage_package_query::is_active)
                .order(mileage_package_query::miles)
                .get_results::<model::MileagePackage>(&mut pool);

            let Ok(mps) = mps else {
                return methods::standard_replies::internal_server_error_response_500(String::from("vehicle/get-mileage-packages: Database error loading mileage packages"));
            };

            methods::standard_replies::response_with_obj(mps, StatusCode::OK)
        })
}