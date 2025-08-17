mod new;
mod get;

use warp::Filter;

pub fn api_v1_agreement()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("agreement")
        .and(
            new::main()
                .or(get::main())
        )
        .and(warp::path::end())
}
