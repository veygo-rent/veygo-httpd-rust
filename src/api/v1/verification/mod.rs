mod request_token;
mod verify_token;
mod reset_password;
mod request_password_token;

use warp::Filter;

pub fn api_v1_verification()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let routes = request_token::main()
        .or(verify_token::main())
        .or(reset_password::main())
        .or(request_password_token::main())
        .boxed();

    warp::path("verification")
        .and(routes)
        .and(warp::path::end())
        .boxed()
}
