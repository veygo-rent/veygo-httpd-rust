use warp::http::StatusCode;
use warp::Rejection;
use warp::reply::{Json, WithStatus};
use crate::model::PublishAccessToken;

pub fn internal_server_error_response (
    token_data: &PublishAccessToken
) -> Result<(WithStatus<Json>,), Rejection> {
    let error_msg = serde_json::json!({"access_token": &token_data, "error": "Internal Server Error"});
    Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
}
pub fn internal_server_error_response_without_access_token () -> Result<(WithStatus<Json>,), Rejection> {
    let error_msg = serde_json::json!({"error": "Internal Server Error"});
    Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
}
pub fn card_declined (
    token_data: &PublishAccessToken
) -> Result<(WithStatus<Json>,), Rejection> {
    let error_msg = serde_json::json!({"access_token": &token_data, "error": "Credit card declined"});
    Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::PAYMENT_REQUIRED),))
}

pub fn not_implemented () -> Result<(WithStatus<Json>,), Rejection> {
    let error_msg = serde_json::json!({"error": "Not Implemented"});
    Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_IMPLEMENTED),))
}
