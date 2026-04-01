mod login;
mod retrieve;
mod update_apns;
mod stats;
mod verify_dl;
mod renter_need_verify;
mod verify_lease;

use warp::Filter;

pub fn api_v1_admin() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("admin")
        .and(
            stats::api_v1_admin_stats()
                .or(login::main())
                .or(retrieve::main())
                .or(update_apns::main())
                .or(renter_need_verify::main())
                .or(verify_dl::main())
                .or(verify_lease::main())
        )
        .and(warp::path::end())
}
