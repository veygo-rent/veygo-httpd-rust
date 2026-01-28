use crate::model::Apartment;
use crate::{POOL, schema, methods};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use http::Method;
use warp::{Filter, Reply};
use warp::http::StatusCode;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-universities")
        .and(warp::method())
        .and(warp::path::end())
        .and_then(async move |method: Method| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response();
            }
            use schema::apartments::dsl::*;
            let mut pool = POOL.get().unwrap();
            let results = apartments
                .into_boxed()
                .filter(is_operating.eq(true))
                .filter(uni_id.eq(1))
                .filter(id.ne(1))
                .load::<Apartment>(&mut pool);

            match results {
                Ok(apt) => {
                    methods::standard_replies::response_with_obj(apt, StatusCode::OK)
                }
                Err(_) => {
                    methods::standard_replies::internal_server_error_response(
                        "apartment/get-universities: Database error loading universities",
                    )
                    .await
                }
            }
        })
}
