mod availability;
mod new;
mod get;
mod get_mileage_packages;
mod user_identify;
mod upload_image;
mod generate_snapshot;

use warp::Filter;

pub fn api_v1_vehicle()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("vehicle")
        .and(
            availability::main()
                .or(new::main())
                .or(get::main())
                .or(get_mileage_packages::main())
                .or(user_identify::main())
                .or(upload_image::main())
                .or(generate_snapshot::main())
        )
        .and(warp::path::end())
}
