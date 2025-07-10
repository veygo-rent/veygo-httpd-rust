mod get_taxes;
pub mod get_universities;

use warp::Filter;

pub fn api_v1_apartment()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("apartment")
        .and(get_universities::main().or(get_taxes::main()))
        .and(warp::path::end())
}
