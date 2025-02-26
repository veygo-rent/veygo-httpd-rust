use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use warp::Filter;
use crate::{db, schema};
use crate::model::{Apartment, PublishApartment};
use tokio::task::{spawn_blocking};
use warp::http::StatusCode;

pub fn get_apartments() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("apartments")
        .and(warp::get())
        .and(warp::path::end())
        .and_then(move || {
            async move {
                use schema::apartments::dsl::*;
                let results = spawn_blocking(move || {
                    apartments.filter(is_operating.eq(true)).load::<Apartment>(&mut db::get_connection_pool().get().unwrap())
                }).await;
                match results {
                    Err(_) => {
                        let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                    }
                    Ok(Ok(apartments_result)) => {
                        let apt_publish: Vec<PublishApartment> = apartments_result.iter().map(|x| x.to_publish_apartment().clone()).collect();
                        let msg = serde_json::json!({"apartments": apt_publish});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),))
                    }
                    Ok(Err(_)) => {
                        let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                    }
                }
            }
        })
}