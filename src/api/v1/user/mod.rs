mod create;
mod login;

use warp::Filter;

pub fn api_v1_user() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("user")
        .and(
            login::user_login()
            .or(create::create_user())
        )
        .and(warp::path::end())
}
