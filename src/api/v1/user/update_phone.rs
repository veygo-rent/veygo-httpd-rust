use crate::model::Renter;
use crate::schema::renters::dsl::renters;
use crate::{POOL, methods, model};
use diesel::prelude::*;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UpdatePhoneBody {
    phone_number: String,
}

fn is_valid_phone_number(phone: &str) -> bool {
    lazy_static::lazy_static! {
        static ref PHONE_REGEX: Regex = Regex::new(
            r"^\d{10}$"  // Exactly 10 digits
        ).expect("Invalid phone number regex");
    }
    PHONE_REGEX.is_match(phone)
}

pub fn main() -> impl Filter<Extract=(impl Reply,), Error=warp::Rejection> + Clone {
    warp::path("update-phone")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |body: UpdatePhoneBody,
                        token: String,
                        user_id: i32,
                        client_type: Option<String>| {
                let access_token = model::RequestToken { user_id, token };
                if !is_valid_phone_number(&body.phone_number) {
                    // invalid email or phone number format
                    let error_msg = serde_json::json!({"phone": &body.phone_number, "error": "Please check your phone number format"});
                    return Ok::<_, warp::Rejection>((with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST).into_response(),));
                };
                let if_token_valid = methods::tokens::verify_user_token(
                    access_token.user_id.clone(),
                    access_token.token.clone(),
                )
                    .await;
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
                        } else {
                            // gen new token
                            let token_clone = access_token.clone();
                            methods::tokens::rm_token_by_binary(
                                hex::decode(token_clone.token).unwrap(),
                            )
                                .await;
                            let new_token = methods::tokens::gen_token_object(
                                access_token.user_id.clone(),
                                client_type.clone(),
                            )
                                .await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .to_publish_access_token();
                            let usr_id_clone = access_token.user_id.clone();
                            let mut usr_in_question = methods::user::get_user_by_id(usr_id_clone).await.unwrap();
                            usr_in_question.phone = body.phone_number.clone();
                            usr_in_question.phone_is_verified = false;
                            let renter_updated = diesel::update(renters.find(usr_id_clone))
                                .set(&usr_in_question).get_result::<Renter>(&mut pool).unwrap().to_publish_renter();
                            let renter_msg = serde_json::json!({
                                        "renter": renter_updated,
                                    });
                            return Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(warp::reply::json(&renter_msg), StatusCode::OK)),));
                        }
                    }
                };
            },
        )
}
