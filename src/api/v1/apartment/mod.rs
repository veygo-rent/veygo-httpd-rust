mod add_apartment;
mod get_all_apartments;
mod get_taxes;
mod get_universities;

use warp::Filter;

pub fn api_v1_apartment()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("apartment")
        .and(
            get_universities::main()
                .or(get_taxes::main())
                .or(get_all_apartments::main())
                .or(add_apartment::main()),
        )
        .and(warp::path::end())
}
