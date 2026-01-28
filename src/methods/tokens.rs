use crate::{model, helper_model, POOL};
use crate::schema::access_tokens::dsl as at_q;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use secrets::Secret;
use std::ops::Add;
use diesel::result::Error;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

async fn generate_unique_token() -> Vec<u8> {
    // Generate a secure random 32-byte token
    let token_vec = Secret::<[u8; 32]>::random(|s| s.to_vec());
    token_vec
}

pub async fn gen_token_object(user_id: &i32, user_agent: &String) -> model::NewAccessToken {
    let exp: DateTime<Utc> = if user_agent.contains("veygo") {
        Utc::now().add(chrono::Duration::days(28))
    } else {
        Utc::now().add(chrono::Duration::seconds(600))
    };
    model::NewAccessToken {
        user_id: *user_id,
        token: generate_unique_token().await,
        exp,
    }
}

pub fn extend_token(verified_token_id: i32, user_agent: &String) -> Result<bool, helper_model::VeygoError> {
    let mut pool = POOL.get().unwrap();
    let exp: DateTime<Utc> = if user_agent.contains("veygo") {
        Utc::now().add(chrono::Duration::days(28))
    } else {
        Utc::now().add(chrono::Duration::seconds(600))
    };
    let result = diesel::update
        (
            at_q::access_tokens
                .find(verified_token_id)
        )
        .set(at_q::exp.eq(exp))
        .execute(&mut pool);
    match result {
        Ok(count) => {
            if count == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(_) => {
            Err(helper_model::VeygoError::InternalServerError)
        }
    }
}

pub async fn verify_user_token(user_id: &i32, token_data: &String) -> Result<(Vec<u8>, i32), helper_model::VeygoError> {
    let binary_token = hex::decode(token_data.clone());
    match binary_token {
        Err(_) => Err(helper_model::VeygoError::TokenFormatError),
        Ok(binary_token) => {
            let mut pool = POOL.get().unwrap();

            let token_db_result = at_q::access_tokens
                .filter(at_q::user_id.eq(&user_id))
                .filter(at_q::token.eq(&binary_token))
                .select((at_q::exp, at_q::id))
                .get_result::<(DateTime<Utc>, i32)>(&mut pool);

            match token_db_result {
                Err(e) => {
                    match e {
                        Error::NotFound => {
                            Err(helper_model::VeygoError::InvalidToken)
                        }
                        _ => {
                            Err(helper_model::VeygoError::InternalServerError)
                        }
                    }
                },
                Ok((token_exp, id)) => {
                    if token_exp >= Utc::now() {
                        Ok((binary_token, id))
                    } else {
                        Err(helper_model::VeygoError::InvalidToken)
                    }
                },
            }
        }
    }
}

pub fn token_not_hex_warp_return() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model::ErrorResponse {
        title: String::from("Corrupted Token"),
        message: String::from("Please login again."),   
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::UNAUTHORIZED,
    )
    .into_response(),))
}

pub fn token_invalid_return() -> Result<(warp::reply::Response,), Rejection> {
    let msg: helper_model::ErrorResponse = helper_model ::ErrorResponse {
        title: String::from("Invalid Token"),
        message: String::from("Please login again."),
    };
    Ok::<_, Rejection>((warp::reply::with_status(
        warp::reply::json(&msg),
        StatusCode::UNAUTHORIZED,
    )
    .into_response(),))
}

pub fn rm_token(token: Vec<u8>, user_id: i32) {
    let mut pool = POOL.get().unwrap();
    let _ = diesel::delete(
        at_q::access_tokens
            .filter(at_q::token.eq(&token))
            .filter(at_q::user_id.eq(&user_id))
    ).execute(&mut pool);
}