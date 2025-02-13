use crate::db;
use crate::model::Renter;
use bcrypt::verify;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::Filter;
use tokio::task;

#[derive(Deserialize, Serialize)]
struct LoginData {
    email: String,
    password: String,
}

pub fn user_login() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("login")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |login_data: LoginData| {
            async move {
                use crate::schema::renters::dsl::*;
                let pool = db::get_connection_pool();
                let input_email = login_data.email.clone();
                let input_password = login_data.password.clone();
                let result = task::spawn_blocking(move || {
                    let conn = &mut pool.get().unwrap();
                    renters.filter(student_email.eq(&login_data.email)).first::<Renter>(conn)
                }).await;

                match result {
                    Ok(Ok(user)) => {
                        if verify(&input_password, &user.password).unwrap_or(false) {
                            let success_msg = serde_json::json!({"status": "success", "message": "Logged in"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&success_msg), StatusCode::ACCEPTED),))
                        } else {
                            let error_msg = serde_json::json!({"email": &input_email, "password": &input_password});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                    }
                    Ok(Err(_)) => {
                        let error_msg = serde_json::json!({"email": &input_email, "password": &input_password});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                    }
                    Err(_) => {
                        let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                    }
                }
            }
        })
}
