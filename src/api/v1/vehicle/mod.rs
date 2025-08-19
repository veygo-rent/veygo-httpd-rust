mod availability;
mod new;
mod get;
mod generate_smartcar_token;

use warp::Filter;

pub fn api_v1_vehicle()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("vehicle")
        .and(
            availability::main()
                .or(new::main())
                .or(get::main())
                .or(generate_smartcar_token::main())
        )
        .and(warp::path::end())
}
