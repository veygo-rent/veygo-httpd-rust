use warp::{reply, Filter, Reply};
use warp::http::StatusCode;
use warp::reply::with_status;
use std::collections::HashMap;

pub fn main() -> impl Filter<Extract=(impl Reply,), Error=warp::Rejection> + Clone {
    warp::path("header-check")
        .and(warp::path::end())
        .and(warp::header::headers_cloned())
        .and_then(async move |headers: warp::http::HeaderMap| {
            let mut header_map = HashMap::new();
            for (key, value) in headers.iter() {
                if let Ok(val_str) = value.to_str() {
                    header_map.insert(key.to_string(), val_str.to_string());
                }
            }
            Ok::<_, warp::Rejection>((with_status(reply::json(&header_map), StatusCode::OK).into_response(),))
        })
}