use crate::methods::tokens::wrap_json_reply_with_token;
use crate::model::PublishAccessToken;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

pub fn internal_server_error_response() -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Internal Server Error"});
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&error_msg),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .into_response(),))
}
pub fn card_declined_wrapped(
    token_data: PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg =
        serde_json::json!({"access_token": &token_data, "error": "Credit card declined"});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::PAYMENT_REQUIRED),
    ),))
}

#[allow(dead_code)]
pub fn not_implemented_response() -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Not Implemented"});
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&error_msg),
        StatusCode::NOT_IMPLEMENTED,
    )
    .into_response(),))
}
