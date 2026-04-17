mod new;
mod current;
mod check_out;

mod check_in;
mod get_upcoming;
mod get_past;
mod get;
mod lock;
mod unlock;

use warp::Filter;

pub fn api_v1_agreement()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let routes = new::main()
        .or(current::main())
        .or(get_upcoming::main())
        .or(get_past::main())
        .or(check_out::main())
        .or(check_in::main())
        .or(get::main())
        .or(lock::main())
        .or(unlock::main())
        .boxed();

    warp::path("agreement")
        .and(routes)
        .and(warp::path::end())
        .boxed()
}
