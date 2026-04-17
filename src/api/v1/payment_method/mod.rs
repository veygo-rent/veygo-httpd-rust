mod create;
mod delete;
mod get;

use warp::Filter;

pub fn api_v1_payment_method()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let routes = get::main()
        .or(create::main())
        .or(delete::main())
        .boxed();

    warp::path("payment-method")
        .and(routes)
        .and(warp::path::end())
        .boxed()
}
