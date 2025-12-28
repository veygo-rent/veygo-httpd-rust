use warp::Filter;

pub fn webhook() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("webhook")
        .and(warp::path::end())
        .and(warp::post())
        .map(|| warp::reply::json(&"OK"))
}