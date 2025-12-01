use crate::methods::tokens::wrap_json_reply_with_token;
use crate::{model, helper_model};
use warp::http::StatusCode;
use warp::{Rejection, Reply};

pub fn bad_request(err_msg: &str) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Bad Request"),
        message: err_msg.to_string(),
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::BAD_REQUEST,
    ).into_response(),))
}
pub fn internal_server_error_response_without_token() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Internal Server Error"),
        message: String::from("Please try again later."),
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::INTERNAL_SERVER_ERROR,
    ).into_response(),))
}

pub fn internal_server_error_response(
    token_data: model::PublishAccessToken
) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Internal Server Error"),
        message: String::from("Please try again later."),
    };
    let with_status = warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::INTERNAL_SERVER_ERROR,
    );
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        with_status,
    ),))
}

pub fn method_not_allowed_response() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Method Not Allowed"),
        message: String::from("Using third party applications is not encouraged. And Veygo will not guarantee the product. "),
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::METHOD_NOT_ALLOWED,
    )
    .into_response(),))
}

pub fn card_declined_wrapped(
    token_data: model::PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Credit Card Declined"),
        message: String::from("Please check your card details and try again."),
    };
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::PAYMENT_REQUIRED),
    ),))
}

pub fn card_invalid_wrapped(
    token_data: model::PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Credit Card Invalid"),
        message: String::from("Please check your card details and try again."),
    };
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::PAYMENT_REQUIRED),
    ),))
}

pub fn apartment_not_operational_wrapped(
    token_data: model::PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Booking Not Allowed"),
        message: String::from("This location is not currently available for booking."),
    };
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN),
    ),))
}

pub fn double_booking_not_allowed_wrapped(
    token_data: model::PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Booking Not Allowed"),
        message: String::from("This booking overlaps with your other booking. Please try a different time."),
    };
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN),
    ),))
}

pub fn user_not_admin_wrapped_return(
    token_data: model::PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Permission Denied"),
        message: String::from("You are not an admin."),
    };
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN),
    ),))
}

pub fn renter_wrapped(
    token_data: model::PublishAccessToken,
    renter: &model::PublishRenter,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg = serde_json::json!({"renter": renter});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),
    ),))
}

pub fn admin_wrapped(
    token_data: model::PublishAccessToken,
    renter: &model::PublishRenter,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg = serde_json::json!({"admin": renter});
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK),
    ),))
}

#[allow(dead_code)]
pub fn not_implemented_response() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Not Implemented"),
        message: String::from("Don't get too excited. We are working on it."),
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::NOT_IMPLEMENTED,
    )
    .into_response(),))
}

pub fn apartment_not_allowed_response(
    token_data: model::PublishAccessToken,
    apartment: i32,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg_txt = "Apartment ".to_owned() + &apartment.to_string() + " is not allowed.";
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Booking Not Allowed"),
        message: msg_txt,
    };
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN),
    ),))
}

pub fn promo_code_not_allowed_response(
    token_data: model::PublishAccessToken,
    code: &str,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg_txt = "Promo code ".to_owned() + code + " is not allowed. Please try another one.";
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Promo Code Not Allowed"),
        message: msg_txt,
    };
    Ok::<_, Rejection>((wrap_json_reply_with_token(
        token_data,
        warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN),
    ),))
}
