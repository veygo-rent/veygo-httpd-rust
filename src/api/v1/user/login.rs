use crate::{POOL, methods, model};
use bcrypt::verify;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

#[derive(Deserialize, Serialize, Clone)]
struct LoginData {
    email: String,
    password: String,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("login")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |login_data: LoginData, user_agent: String| {
            use crate::schema::renters::dsl::*;
            let mut pool = POOL.get().unwrap();
            let result = renters.filter(student_email.eq(&login_data.email)).get_result::<model::Renter>(&mut pool);

            match result {
                Ok(renter) => {
                    if verify(&login_data.password, &renter.password).unwrap_or(false) {
                        // user and password are verified
                        let user_id_data = renter.id;
                        let new_access_token = methods::tokens::gen_token_object(&user_id_data, &user_agent).await;
                        let mut pool = POOL.get().unwrap();
                        use crate::schema::access_tokens::dsl::*;
                        let insert_token_result = diesel::insert_into(access_tokens)
                            .values(&new_access_token)
                            .get_result::<model::AccessToken>(&mut pool) // Get the inserted Renter 
                            .unwrap();

                        let pub_token = insert_token_result.to_publish_access_token();
                        let pub_renter = renter.to_publish_renter();
                        let renter_msg = serde_json::json!({
                            "renter": pub_renter,
                        });
                        Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(pub_token, with_status(warp::reply::json(&renter_msg), StatusCode::OK)),))

                    } else {
                        let error_msg = serde_json::json!({"email": &login_data.email, "password": &login_data.password, "error": "Credentials invalid"});
                        Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::UNAUTHORIZED).into_response(),))
                    }
                }
                Err(_) => {
                    let error_msg = serde_json::json!({"email": &login_data.email, "password": &login_data.password, "error": "Credentials invalid"});
                    Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::UNAUTHORIZED).into_response(),))
                }
            }
        })
}
