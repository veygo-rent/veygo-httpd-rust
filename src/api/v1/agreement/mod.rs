mod new;
mod get;
mod current;

use warp::Filter;

pub fn api_v1_agreement()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("agreement")
        .and(
            new::main()
                .or(get::main())
                .or(current::main())
        )
        .and(warp::path::end())
}
