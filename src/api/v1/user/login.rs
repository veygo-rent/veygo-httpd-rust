use crate::model::{AccessToken, Renter};
use crate::schema::access_tokens::dsl::access_tokens;
use crate::db;
use bcrypt::verify;
use diesel::{ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use tokio::task;
use warp::http::StatusCode;
use warp::Filter;

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
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |login_data: LoginData, client_type: Option<String>| {
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
                    Ok(Ok(renter)) => {
                        if verify(&input_password, &renter.password).unwrap_or(false) {
                            let _user_id = renter.id;
                            let new_access_token = crate::gen_token::gen_token_object(_user_id, client_type).await;
                            let _result: Result<QueryResult<AccessToken>, tokio::task::JoinError> = task::spawn_blocking(move || {
                                // Diesel operations are synchronous, so we use spawn_blocking
                                diesel::insert_into(access_tokens)
                                    .values(&new_access_token)
                                    .get_result::<AccessToken>(&mut db::get_connection_pool().get().unwrap()) // Get the inserted Renter
                            }).await;
                            match _result {
                                Ok(Ok(access_token)) => {
                                    let pub_token = access_token.to_publish_access_token();
                                    let pub_renter = renter.to_publish_renter();
                                    let renter_msg = serde_json::json!({
                                        "renter": pub_renter,
                                        "access_token": pub_token,
                                    });
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&renter_msg), StatusCode::ACCEPTED),))
                                }
                                _ => {
                                    let error_msg = serde_json::json!({"status": "error", "message": "Internal server error"});
                                    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::INTERNAL_SERVER_ERROR),))
                                }
                            }
                        } else {
                            let error_msg = serde_json::json!({"email": &input_email, "password": &input_password, "error": "Credentials invalid. "});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                    }
                    Ok(Err(_)) => {
                        let error_msg = serde_json::json!({"email": &input_email, "password": &input_password, "error": "Credentials invalid. "});
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
