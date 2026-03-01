use crate::{POOL, methods, model, helper_model};
use bcrypt::verify;
use diesel::RunQueryDsl;
use diesel::prelude::*;
use diesel::result::Error;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::{Filter, Reply, http::Method};

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
            let mut pool = POOL.get().unwrap();
            use crate::schema::renters::dsl::*;
            let result = renters.filter(student_email.eq(&login_data.email)).get_result::<model::Renter>(&mut pool);
            return match result {
                Ok(admin) => {
                    if !admin.is_manager() {
                        let error_msg = helper_model::ErrorResponse {
                            title: "Credentials Invalid".to_string(),
                            message: "Please check your credentials again. ".to_string(),
                        };
                        return methods::standard_replies::response_with_obj(error_msg, StatusCode::UNAUTHORIZED);
                    }
                    return if verify(&login_data.password, &admin.password).unwrap_or(false) {
                        // user and password are verified
                        let user_id_data = admin.id;
                        let new_access_token = methods::tokens::gen_token_object(&user_id_data, &user_agent).await;
                        let mut pool = POOL.get().unwrap();
                        use crate::schema::access_tokens::dsl::*;
                        let insert_token_result = diesel::insert_into(access_tokens)
                            .values(&new_access_token)
                            .get_result::<model::AccessToken>(&mut pool) // Get the inserted Renter 
                            .unwrap();

                        let pub_token = insert_token_result.into();
                        let pub_renter: model::PublishRenter = admin.into();
                        methods::standard_replies::auth_renter_reply(&pub_renter, &pub_token, false)
                    } else {
                        let err_msg = helper_model::ErrorResponse {
                            title: "Credentials Invalid".to_string(),
                            message: "Please check your credentials again. ".to_string(),
                        };
                        return methods::standard_replies::response_with_obj(err_msg, StatusCode::UNAUTHORIZED);
                    }
                },
                Err(err) => {
                    match err {
                        Error::NotFound => {
                            let err_msg = helper_model::ErrorResponse {
                                title: "Credentials Invalid".to_string(),
                                message: "Please check your credentials again. ".to_string(),
                            };
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::UNAUTHORIZED);
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response(
                                String::from("admin/login: Database error loading renter by student email"),
                            )
                        }
                    }
                }
            };
        })
}
