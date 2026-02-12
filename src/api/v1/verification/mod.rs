mod request_token;
mod verify_token;
mod reset_password;

use warp::Filter;

pub fn api_v1_verification()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("verification")
        .and(
            request_token::main()
                .or(verify_token::main())
                .or(reset_password::main())
        )
        .and(warp::path::end())
}
