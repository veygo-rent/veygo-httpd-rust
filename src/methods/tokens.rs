use crate::POOL;
use crate::model;
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
    // Generate a secure random 32-byte token
    let token_vec = Secret::<[u8; 32]>::random(|s| s.to_vec());
    token_vec
}

pub async fn gen_token_object(_user_id: &i32, user_agent: &String) -> model::NewAccessToken {
    let mut _exp: DateTime<Utc> = Utc::now().add(chrono::Duration::seconds(600));
    if user_agent.contains("veygo") {
        _exp = Utc::now().add(chrono::Duration::days(28));
    }
    model::NewAccessToken {
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
                    .first::<model::AccessToken>(&mut pool)
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
        .execute(&mut pool);
}

pub fn token_not_hex_warp_return() -> Result<(warp::reply::Response,), Rejection> {
    let msg: model::ErrorResponse = model::ErrorResponse {
        title: String::from("Corrupted Token"),
        message: String::from("Please login again."),   
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::UNAUTHORIZED,
    )
    .into_response(),))
}

pub fn token_invalid_wrapped_return() -> Result<(warp::reply::Response,), Rejection> {
    let msg: model::ErrorResponse = model::ErrorResponse {
        title: String::from("Invalid Token"),
        message: String::from("Please login again."),
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::UNAUTHORIZED,
    )
    .into_response(),))
}

pub fn wrap_json_reply_with_token(
    token_data: model::PublishAccessToken,
    reply: WithStatus<Json>,
) -> warp::reply::Response {
    let reply = with_header(reply, "token", token_data.token);
    let reply = with_header(reply, "exp", token_data.exp.timestamp());
    reply.into_response()
}
