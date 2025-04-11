use crate::model::{PublishAccessToken, Renter};
use crate::POOL;
use chrono::Utc;
use diesel::prelude::*;
use tokio::task;
use warp::http::StatusCode;
use warp::{Rejection};
use crate::methods::tokens::wrap_json_reply_with_token;

pub async fn get_user_by_id(user_id: i32) -> QueryResult<Renter> {
    let mut pool = POOL.clone().get().unwrap();
    task::spawn_blocking(move || {
        use crate::schema::renters::dsl::*;
        renters
            .filter(id.eq(&user_id))
            .get_result::<Renter>(&mut pool)
    })
    .await
    .unwrap()
}

pub async fn check_if_on_do_not_rent(renter: &Renter) -> bool {
    let mut pool = POOL.clone().get().unwrap();
    // Extract values from Renter
    let user_name = renter.name.clone(); // String
    let user_phone = renter.phone.clone(); // String
    let user_email = renter.student_email.clone(); // String
    let today = Utc::now().date_naive(); // e.g. 2025-03-01

    // Run the database check on a blocking thread
    task::spawn_blocking(move || {
        use crate::schema::do_not_rent_lists::dsl::*;
        diesel::select(diesel::dsl::exists(
            do_not_rent_lists
                .filter(
                    name.eq(Some(user_name))
                        .or(phone.eq(Some(user_phone)))
                        .or(email.eq(Some(user_email))),
                )
                .filter(exp.is_null().or(exp.ge(today))),
        ))
        .get_result::<bool>(&mut pool)
    })
    .await
    .unwrap()
    .unwrap()
}

pub fn user_with_admin_access(user: &Renter) -> bool {
    if let Some(email_expiration) = user.student_email_expiration {
        let today = Utc::now().date_naive();
        user.apartment_id == 1 && email_expiration > today
    } else {
        false
    }
}

pub fn user_not_admin_wrapped_return(
    token_data: PublishAccessToken,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"error": "You do not have administrator privileges"});
    Ok::<_, Rejection>((wrap_json_reply_with_token(token_data, warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::UNAUTHORIZED)),))
}
