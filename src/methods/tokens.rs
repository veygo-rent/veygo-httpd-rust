use crate::POOL;
use crate::model::{AccessToken, NewAccessToken, PublishAccessToken};
use crate::schema::access_tokens::dsl::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use hex::FromHexError;
use secrets::Secret;
use std::ops::Add;
use warp::http::StatusCode;
use warp::reply::{Json, WithStatus, with_header};
use warp::{Rejection, Reply};

async fn generate_unique_token() -> Vec<u8> {
    loop {
        // Generate a secure random 32-byte token
        let token_vec = Secret::<[u8; 32]>::random(|s| s.to_vec());

        let token_to_return = token_vec.clone();

        let mut pool = POOL.get().unwrap();
        // Wrap in a block for error handling.
        let token_exists_result = diesel::select(diesel::dsl::exists(
            crate::schema::access_tokens::table.filter(token.eq(token_vec)),
        ))
        .get_result::<bool>(&mut pool);

        let token_exists = token_exists_result.unwrap();

        // If the token does not exist, return it
        if !token_exists {
            // Expose the secret and clone to return owned value
            return token_to_return;
        }
    }
}

pub async fn gen_token_object(_user_id: &i32, user_agent: &String) -> NewAccessToken {
    let mut _exp: DateTime<Utc> = Utc::now().add(chrono::Duration::seconds(600));
    if user_agent.contains("veygo") {
        _exp = Utc::now().add(chrono::Duration::days(28));
    }
    NewAccessToken {
        user_id: *_user_id,
        token: generate_unique_token().await,
        exp: _exp,
    }
}

pub async fn verify_user_token(_user_id: &i32, token_data: &String) -> Result<bool, FromHexError> {
    let binary_token = hex::decode(token_data.clone());
    match binary_token {
        Err(error) => Err(error),
        Ok(binary_token) => {
            let token_clone = binary_token.clone();
            let token_clone_again = binary_token.clone();
            let mut pool = POOL.get().unwrap();
            let token_in_db = diesel::select(diesel::dsl::exists(
                access_tokens
                    .filter(token.eq(token_clone))
                    .filter(user_id.eq(_user_id)),
            ))
            .get_result::<bool>(&mut pool)
            .unwrap();
            if token_in_db {
                let mut pool = POOL.get().unwrap();
                let token_in_db_result = access_tokens
                    .filter(user_id.eq(_user_id))
                    .filter(token.eq(token_clone_again))
                    .first::<AccessToken>(&mut pool)
                    .unwrap();
                let token_exp = token_in_db_result.exp;
                if token_exp >= Utc::now() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        }
    }
}

pub async fn rm_token_by_binary(token_bit: Vec<u8>) {
    let mut pool = POOL.get().unwrap();
    let _ = diesel::delete(access_tokens.filter(token.eq(token_bit)))
        .get_result::<AccessToken>(&mut pool);
}

pub fn token_not_hex_warp_return(
    token_data: &String,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"token": &token_data, "error": "Token not in hex format"});
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&error_msg),
        StatusCode::UNAUTHORIZED,
    )
    .into_response(),))
}

pub fn token_invalid_wrapped_return(
    token_data: &str,
) -> Result<(warp::reply::Response,), Rejection> {
    let error_msg = serde_json::json!({"token": &token_data, "error": "Token not valid"});
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&error_msg),
        StatusCode::UNAUTHORIZED,
    )
    .into_response(),))
}

pub fn wrap_json_reply_with_token(
    token_data: PublishAccessToken,
    reply: WithStatus<Json>,
) -> warp::reply::Response {
    let reply = with_header(reply, "token", token_data.token);
    let reply = with_header(reply, "exp", token_data.exp.timestamp());
    reply.into_response()
}
