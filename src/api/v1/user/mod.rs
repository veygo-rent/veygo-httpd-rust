mod create;
mod login;
mod get_payment_methods;

use warp::Filter;

pub fn api_v1_user() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("user")
        .and(
            login::user_login()
            .or(create::create_user())
            .or(get_payment_methods::get_payment_methods())
        )
        .and(warp::path::end())
}
