mod availability;
mod new;
mod get;
mod get_mileage_packages;
mod user_identify;
mod generate_snapshot;
mod request_upload_link;

use warp::Filter;

pub fn api_v1_vehicle()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let routes = availability::main()
        .or(new::main())
        .or(get::main())
        .or(get_mileage_packages::main())
        .or(user_identify::main())
        .or(generate_snapshot::main())
        .or(request_upload_link::main())
        .boxed();

    warp::path("vehicle")
        .and(routes)
        .and(warp::path::end())
        .boxed()
}
