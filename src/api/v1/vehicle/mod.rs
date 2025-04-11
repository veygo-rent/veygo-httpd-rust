mod availability;

use warp::Filter;

pub fn api_v1_vehicle()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("vehicle")
        .and(availability::main())
        .and(warp::path::end())
}
