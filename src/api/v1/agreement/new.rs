use chrono::{DateTime, Utc, Duration};
use diesel::RunQueryDsl;
use serde_derive::{Deserialize, Serialize};
use warp::Filter;
use warp::http::StatusCode;
use crate::{model, POOL};
use crate::methods::{tokens, user};
use crate::model::AccessToken;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct NewAgreementRequestBodyData {
    access_token: model::RequestBodyToken, // contains 'user_id' and 'token'
    vehicle_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

pub fn new_agreement(
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("new")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |body: NewAgreementRequestBodyData, client_type: Option<String>| async move {
            let if_token_valid = tokens::verify_user_token(body.access_token.user_id.clone(), body.access_token.token.clone()).await;
            match if_token_valid {
                Err(_) => {
                    tokens::token_not_hex_warp_return(&body.access_token.token)
                }
                Ok(token_bool) => {
                    if !token_bool {
                        tokens::token_invalid_warp_return(&body.access_token.token)
                    } else {
                        // Token is valid, generate new publish token, user_id valid
                        tokens::rm_token_by_binary(hex::decode(body.access_token.token).unwrap()).await;
                        let new_token = tokens::gen_token_object(body.access_token.user_id.clone(), client_type.clone()).await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.clone().get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens).values(&new_token).get_result::<AccessToken>(&mut pool).unwrap().to_publish_access_token();
                        let user_in_request = user::get_user_by_id(body.access_token.user_id).await.unwrap();
                        // Check if renter in DNR
                        let if_in_dnr = user::check_if_on_do_not_rent(&user_in_request).await;
                        if !if_in_dnr {
                            if body.start_time + Duration::hours(1) > body.end_time {
                                let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "The reservation has to be at least one hour"});
                                return Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN),))
                            }
                            // TODO
                            let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "TODO"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_IMPLEMENTED),))

                        } else {
                            let error_msg = serde_json::json!({"access_token": &new_token_in_db_publish, "error": "User on do not rent list"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::FORBIDDEN),))
                        }
                    }
                }
            }
        })
}