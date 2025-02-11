use warp::Filter;
use warp::http::StatusCode;
use tokio::time::{sleep, Duration}; // Import for asynchronous sleep

pub fn user_login() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("login")
        .and(warp::path::end())
        .and(warp::get()) // Good practice to specify the method
        .and_then(|| async {  // Use and_then for async
            // Simulate an async operation (e.g., database query)
            sleep(Duration::from_millis(2000)).await;

            // Return a Result, as required by and_then
            Ok::<_, warp::Rejection>((warp::reply::with_status("Everything is OK", StatusCode::OK),))
        })
}