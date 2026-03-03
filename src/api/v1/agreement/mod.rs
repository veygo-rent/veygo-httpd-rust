mod new;
mod current;
mod check_out;

mod check_in;
mod get_upcoming;
mod get_past;

use warp::Filter;

pub fn api_v1_agreement()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("agreement")
        .and(
            new::main()
                .or(get_upcoming::main())
                .or(get_past::main())
                .or(current::main())
                .or(check_out::main())
                .or(check_in::main())
        )
        .and(warp::path::end())
}
