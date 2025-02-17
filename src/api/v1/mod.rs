mod user;
mod get_apartments;

use warp::Filter;

pub fn api_v1() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("v1")
        .and(
            user::api_v1_user()
                .or(get_apartments::get_apartments())
        )
        .and(warp::path::end())
}
