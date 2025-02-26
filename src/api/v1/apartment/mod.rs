pub mod get;

use warp::Filter;

pub fn api_v1_apartment() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("apartment")
        .and(
            get::get_apartments()
        )
        .and(warp::path::end())
}
