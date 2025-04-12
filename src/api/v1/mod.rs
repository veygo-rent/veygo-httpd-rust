mod agreement;
mod apartment;
mod payment_method;
mod toll;
mod user;
mod vehicle;
mod verification;

use warp::Filter;

pub fn api_v1() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("v1")
        .and(
            user::api_v1_user()
                .or(payment_method::api_v1_payment_method())
                .or(apartment::api_v1_apartment())
                .or(vehicle::api_v1_vehicle())
                .or(agreement::api_v1_agreement())
                .or(toll::api_v1_toll())
                .or(verification::api_v1_verification()),
        )
        .and(warp::path::end())
}
