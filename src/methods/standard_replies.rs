use crate::{model, helper_model, integration};
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
pub fn internal_server_error_response(msg: String) -> Result<(warp::reply::Response,), Rejection> {
    let _ = tokio::task::spawn_blocking(move || {
        let dev = integration::sendgrid_veygo::make_email_obj("dev@veygo.rent", "Veygo Dev Team");
        let _ = integration::sendgrid_veygo::send_email(
            Option::from("Veygo Server"),
            dev,
            "Internal Server Error",
            &*msg,
            None,
            None,
        );
    });
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Internal Server Error"),
        message: String::from("Please try again later. If issue present, contact us at dev@veygo.rent "),
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::INTERNAL_SERVER_ERROR,
    ).into_response(),))
}

pub fn method_not_allowed_response() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Method Not Allowed"),
        message: String::from("Using third party applications is not encouraged. And Veygo will not guarantee the product. "),
    };
    Ok((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::METHOD_NOT_ALLOWED,
    )
    .into_response(),))
}

pub fn card_declined(
) -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Credit Card Declined"),
        message: String::from("Please check your card details and try again."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::PAYMENT_REQUIRED).into_response(),))
}

pub fn card_invalid() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Credit Card Invalid"),
        message: String::from("Please check your card details and try again."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::PAYMENT_REQUIRED).into_response(),))
}

pub fn apartment_not_operational() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Booking Not Allowed"),
        message: String::from("This location is not currently available for booking."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn double_booking_not_allowed() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Booking Not Allowed"),
        message: String::from("This booking overlaps with your other booking. Please try a different time."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn user_not_admin() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Permission Denied"),
        message: String::from("You are not an admin."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn user_email_not_verified() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Permission Denied"),
        message: String::from("Your email is not verified."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn admin_not_verified() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Permission Denied"),
        message: String::from("Please verify your email address before proceeding."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn admin_not_allowed() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Permission Denied"),
        message: String::from("You do not have permission to access this property."),
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn response_with_obj<T>(obj: T, status_code: StatusCode)
    -> Result<(warp::reply::Response,), Rejection> where T: serde::Serialize {
    Ok((warp::reply::with_status(warp::reply::json(&obj), status_code).into_response(),))
}

pub fn auth_renter_reply(
    renter: &model::PublishRenter,
    token_data: &model::PublishAccessToken,
    is_created: bool
) -> Result<(warp::reply::Response,), Rejection> {
    let reply = warp::reply::json(&renter);
    let reply = warp::reply::with_header(reply, "token", token_data.clone().token);
    let status_code = if is_created { StatusCode::CREATED } else { StatusCode::OK };
    Ok((warp::reply::with_status(reply, status_code).into_response(),))
}

#[allow(dead_code)]
pub fn not_implemented_response() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Not Implemented"),
        message: String::from("Don't get too excited. We are working on it."),
    };
    Ok((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::NOT_IMPLEMENTED,
    )
    .into_response(),))
}

pub fn apartment_not_allowed_response(
    apartment: i32,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg_txt = "Apartment ".to_owned() + &apartment.to_string() + " is not allowed.";
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Booking Not Allowed"),
        message: msg_txt,
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn promo_code_not_allowed_response(
    code: &str,
) -> Result<(warp::reply::Response,), Rejection> {
    let msg_txt = "Promo code ".to_owned() + code + " is not allowed. Please try another one.";
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Promo Code Not Allowed"),
        message: msg_txt,
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}

pub fn agreement_not_allowed_response() -> Result<(warp::reply::Response,), Rejection> {
    let msg_txt = String::from("Accessing this agreement is not allowed. Please try another one.");
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Access Agreement Not Allowed"),
        message: msg_txt,
    };
    Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::FORBIDDEN).into_response(),))
}
