mod login;
mod retrieve;
mod update_apns;

use warp::Filter;

pub fn api_v1_admin() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("admin")
        .and(
            login::main()
                .or(retrieve::main())
                .or(update_apns::main())
        )
        .and(warp::path::end())
}
