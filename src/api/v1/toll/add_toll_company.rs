use crate::{POOL, integration, methods};
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use warp::Filter;
use warp::http::StatusCode;

#[derive(Deserialize, Serialize, Clone)]
struct TollCompanyData {
    email: String,
    password: String,
}

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("add-company")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(async move |body: TollCompanyData, token: String, user_id: i32, client_type: Option<String>| { 
            methods::standard_replies::not_implemented()
        })
}
