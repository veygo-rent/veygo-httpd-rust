mod get;

use warp::Filter;

pub fn api_v1_policy()
    -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("policy")
        .and(get::main())
        .and(warp::path::end())
}