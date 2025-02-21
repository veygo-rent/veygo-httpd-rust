mod create;
mod login;
mod get_payment_methods;
mod create_payment_method;

use warp::Filter;

pub fn api_v1_user() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("user")
        .and(
            login::user_login()
            .or(create::create_user())
            .or(get_payment_methods::get_payment_methods())
            .or(create_payment_method::create_payment_method())
        )
        .and(warp::path::end())
}
