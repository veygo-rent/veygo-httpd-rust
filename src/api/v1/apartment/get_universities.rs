use crate::model::Apartment;
use crate::{connection_pool, schema, methods};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use warp::{Filter, Reply, http::Method, http::StatusCode};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-universities")
        .and(warp::method())
        .and(warp::path::end())
        .and_then(async move |method: Method| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response_405();
            }
            use schema::apartments::dsl::*;
            let mut pool = connection_pool().await.get().unwrap();
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
                    methods::standard_replies::internal_server_error_response_500(
                        String::from("apartment/get-universities: Database error loading universities"),
                    )
                }
            }
        })
}
