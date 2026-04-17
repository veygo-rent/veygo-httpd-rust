mod v1;
mod header_check;
mod webhook;

use warp::Filter;

pub fn api() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let routes = v1::api_v1()
        .or(header_check::main())
        .or(webhook::webhook())
        .boxed();

    warp::path("api")
        .and(routes)
        .and(warp::path::end())
        .boxed()
}
