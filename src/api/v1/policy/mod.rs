use warp::Filter;

pub fn api_v1_policy()
    -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("policy")
        .and(warp::get().map(|| "Privacy Policy and Terms of Service"))
        .and(warp::path::end())
}