mod new;

use warp::Filter;

pub fn api_v1_agreement() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path!("agreement")
        .and(
            new::new_agreement()
        )
        .and(warp::path::end())
}
