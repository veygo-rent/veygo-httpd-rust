mod create;
mod delete;
mod get;

use warp::Filter;

pub fn api_v1_payment_method()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("payment-method")
        .and(get::main().or(create::main()).or(delete::main()))
        .and(warp::path::end())
}
