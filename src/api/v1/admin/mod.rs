mod login;
mod retrieve;

use warp::Filter;

pub fn api_v1_admin() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("admin")
        .and(login::main()
        .or(retrieve::main()))
        .and(warp::path::end())
}
