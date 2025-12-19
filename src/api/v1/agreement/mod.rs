mod new;
mod get;
mod current;
mod check_out;

mod check_in;

use warp::Filter;

pub fn api_v1_agreement()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("agreement")
        .and(
            new::main()
                .or(get::main())
                .or(current::main())
                .or(check_out::main())
                .or(check_in::main())
        )
        .and(warp::path::end())
}
