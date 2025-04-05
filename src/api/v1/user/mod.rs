mod create;
mod login;
mod update_apartment;

use warp::Filter;

pub fn api_v1_user() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("user")
        .and(
            login::user_login()
            .or(create::create_user())
                .or(update_apartment::update())
        )
        .and(warp::path::end())
}
