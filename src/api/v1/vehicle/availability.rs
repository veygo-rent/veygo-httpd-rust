use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use warp::Filter;
use warp::http::StatusCode;
use crate::model;
use crate::methods::tokens;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct AvailabilityData {
    access_token: model::RequestBodyToken, // contains 'user_id' and 'token'
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

pub fn vehicle_availability() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("availability")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(move |body: AvailabilityData, client_type: Option<String>| {
            async move {
                let if_token_valid = tokens::verify_user_token(body.access_token.user_id.clone(), body.access_token.token.clone()).await;
                match if_token_valid {
                    Ok(token_bool) => {
                        if !token_bool {
                            let error_msg = serde_json::json!({"token": &body.access_token.token, "error": "Token not valid"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        } else {
                            // Token is validated
                            // TODO: procedure after token valid, using err message for placeholder for now
                            let error_msg = serde_json::json!({"token": &body.access_token.token, "error": "Token not valid"});
                            Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                        }
                    }
                    Err(_) => {
                        let error_msg = serde_json::json!({"token": &body.access_token.token, "error": "Token not in hex format"});
                        Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::NOT_ACCEPTABLE),))
                    }
                }
            }
        })
}