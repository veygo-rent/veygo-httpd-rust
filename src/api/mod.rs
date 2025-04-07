mod v1;

use warp::Filter;

pub fn api() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("api").and(v1::api_v1()).and(warp::path::end())
}
