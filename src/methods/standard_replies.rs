use warp::http::StatusCode;
use warp::{Rejection, Reply};
use crate::methods::tokens::wrap_json_reply_with_token;
use crate::model::PublishAccessToken;


pub fn internal_server_error_response() -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Internal Server Error"});
    Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR).into_response(),))
}
pub fn card_declined (
    token_data: PublishAccessToken
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"access_token": &token_data, "error": "Credit card declined"});
    Ok::<_, Rejection>((wrap_json_reply_with_token(token_data, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::PAYMENT_REQUIRED)),))
}

pub fn not_implemented () -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Not Implemented"});
    Ok::<_, Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_IMPLEMENTED).into_response(),))
}
