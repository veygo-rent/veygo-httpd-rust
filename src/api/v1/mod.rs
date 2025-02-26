mod user;
mod payment_method;
mod apartment;

use warp::Filter;

pub fn api_v1() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("v1")
        .and(
            user::api_v1_user()
                .or(payment_method::api_v1_payment_method())
                .or(apartment::api_v1_apartment())
        )
        .and(warp::path::end())
}
