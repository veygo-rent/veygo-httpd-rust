use crate::methods::tokens::wrap_json_reply_with_token;
use crate::model::{PublishAccessToken, PublishRenter};
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

pub fn method_not_allowed_response() -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Method Not Allowed"});
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&error_msg),
        StatusCode::METHOD_NOT_ALLOWED,
    )
    .into_response(),))
}

pub fn card_declined_wrapped(
    token_data: PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Credit card declined"});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::PAYMENT_REQUIRED),
    ),))
}

pub fn card_invalid_wrapped(
    token_data: PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Credit card invalid"});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),
    ),))
}

pub fn apartment_not_operational_wrapped(
    token_data: PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "Apartment is not operational"});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),
    ),))
}

pub fn user_not_admin_wrapped_return(
    token_data: PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "You do not have administrator privileges"});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN),
    ),))
}

pub fn renter_wrapped(
    token_data: PublishAccessToken,
    renter: &PublishRenter,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg = serde_json::json!({"renter": renter});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),
    ),))
}

pub fn admin_wrapped(
    token_data: PublishAccessToken,
    renter: &PublishRenter,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg = serde_json::json!({"admin": renter});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),
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
