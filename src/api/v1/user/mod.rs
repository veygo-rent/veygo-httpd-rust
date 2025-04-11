mod create;
mod login;
mod update_apartment;
mod update_phone;

use warp::Filter;

pub fn api_v1_user() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("user")
        .and(
            login::main()
                .or(create::main())
                .or(update_apartment::main()),
        )
        .and(warp::path::end())
}
