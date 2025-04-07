pub mod get;
pub mod create;
mod delete;

use warp::Filter;

pub fn api_v1_payment_method() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("payment-method")
        .and(
            get::get_payment_methods()
                .or(create::create_payment_method())
                .or(delete::main())
        )
        .and(warp::path::end())
}
