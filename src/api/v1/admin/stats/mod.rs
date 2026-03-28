use warp::Filter;
mod renters;

pub fn api_v1_admin_stats() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("stats")
        .and(
            renters::main()
        )
        .and(warp::path::end())
}