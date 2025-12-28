mod stripe;

use warp::Filter;

pub fn webhook() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("webhook")
        .and(warp::post())
        .and(stripe::main())
        .and(warp::path::end())
}