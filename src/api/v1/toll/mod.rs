mod add_toll_company;
mod get_toll_company;
mod upload_tolls;

use warp::Filter;

pub fn api_v1_toll() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("toll")
        .and(
            add_toll_company::main()
                .or(get_toll_company::main())
                .or(upload_tolls::main()),
        )
        .and(warp::path::end())
}
