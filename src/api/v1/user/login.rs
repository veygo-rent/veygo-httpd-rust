use crate::{POOL, methods, model, helper_model};
use bcrypt::verify;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::result::Error;
use serde_derive::{Deserialize, Serialize};
use warp::http::{StatusCode, Method};
use warp::{Filter, Reply};

#[derive(Deserialize, Serialize, Clone)]
struct LoginData {
    email: String,
    password: String,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("login")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, login_data: LoginData, user_agent: String| {
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }
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

                        let pub_token: model::PublishAccessToken = insert_token_result.into();
                        let pub_renter: model::PublishRenter = renter.into();
                        methods::standard_replies::auth_renter_reply(&pub_renter, &pub_token, false)

                    } else {
                        let err_msg = helper_model::ErrorResponse {
                            title: "Credentials Invalid".to_string(),
                            message: "Please check your credentials again. ".to_string(),
                        };
                        methods::standard_replies::response_with_obj(err_msg, StatusCode::UNAUTHORIZED)
                    }
                }
                Err(err) => {
                    match err {
                        Error::NotFound => {
                            let err_msg = helper_model::ErrorResponse {
                                title: "Credentials Invalid".to_string(),
                                message: "Please check your credentials again. ".to_string(),
                            };
                            methods::standard_replies::response_with_obj(err_msg, StatusCode::UNAUTHORIZED)
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response(
                                "user/login: Database error loading renter by student_email",
                            )
                            .await
                        }
                    }
                }
            }
        })
}
