use crate::model::{AccessToken, Renter};
use crate::schema::access_tokens::dsl::access_tokens;
use crate::POOL;
use bcrypt::verify;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use tokio::task;
use warp::http::StatusCode;
use warp::Filter;

#[derive(Deserialize, Serialize, Clone)]
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
                let mut pool = POOL.clone().get().unwrap();
                let input_email = login_data.email.clone();
                let input_password = login_data.password.clone();
                let result = task::spawn_blocking(move || {
                    renters.filter(student_email.eq(&login_data.email)).get_result::<Renter>(&mut pool)
                }).await.unwrap();

                match result {
                    Ok(renter) => {
                        if verify(&input_password, &renter.password).unwrap_or(false) {
                            // user and password is verified
                            let user_id_data = renter.id;
                            let new_access_token = crate::methods::tokens::gen_token_object(user_id_data, client_type).await;
                            let mut pool = POOL.clone().get().unwrap();
                            let insert_token_result = task::spawn_blocking(move || {
                                diesel::insert_into(access_tokens)
                                    .values(&new_access_token)
                                    .get_result::<AccessToken>(&mut pool) // Get the inserted Renter
                            }).await.unwrap().unwrap();

                            let pub_token = insert_token_result.to_publish_access_token();
                            let pub_renter = renter.to_publish_renter();
                            let renter_msg = serde_json::json!({
                                        "renter": pub_renter,
                                        "access_token": pub_token,
                                    });
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&renter_msg), StatusCode::OK),))

                        } else {
                            let error_msg = serde_json::json!({"email": &input_email, "password": &input_password, "error": "Credentials invalid"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN),))
                        }
                    }
                    Err(_) => {
                        let error_msg = serde_json::json!({"email": &input_email, "password": &input_password, "error": "Credentials invalid"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN),))
                    }
                }
            }
        })
}
