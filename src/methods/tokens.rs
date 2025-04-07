use std::ops::Add;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use hex::FromHexError;
use secrets::Secret;
use tokio::task;
use tokio::task::spawn_blocking;
use warp::http::StatusCode;
use warp::Rejection;
use warp::reply::{Json, WithStatus};
use crate::{POOL};
use crate::model::{AccessToken, NewAccessToken};
use crate::schema::access_tokens::dsl::*;

async fn generate_unique_token() -> Vec<u8> {
    loop {
        // Generate a secure random 32-byte token
        let token_vec = Secret::<[u8; 32]>::random(|s| s.to_vec());

        let token_to_return = token_vec.clone();

        // Wrap in a block for error handling.
        let token_exists_result = task::spawn_blocking(move || {
            //get connection.
            let mut pool = POOL.clone().get().unwrap();
            // Perform the Diesel query *synchronously* within the closure.
            // Use the token_vec.expose_secret() here.
            diesel::select(diesel::dsl::exists(
                crate::schema::access_tokens::table.filter(crate::schema::access_tokens::token.eq(token_vec))
            ))
                .get_result::<bool>(&mut pool)

        }).await;

        let token_exists = match token_exists_result {
            Ok(result) => {
                match result{
                    Ok(v) => {v},
                    Err(e) => {
                        // Handle database query errors.  In a real application, you'd probably
                        // want to log this error and possibly retry.  For this example,
                        // we'll just treat any database error as if the token *does* exist,
                        // to force a retry. This avoids leaking information about internal errors.
                        eprintln!("Database error: {:?}", e);
                        true // Treat a DB error as if the token exists, to force a retry.
                    }
                }
            },
            Err(join_err) => { //For any tokio error.
                eprintln!("Error joining blocking task: {:?}", join_err);
                true // Treat a join error as if the token exists.
            }
        };

        // If the token does not exist, return it
        if !token_exists {
            // Expose the secret and clone to return owned value
            return token_to_return;
        }
    }
}

pub async fn gen_token_object(
    _user_id: i32,
    client_type: Option<String>,
) -> NewAccessToken {
    let mut _exp: DateTime<Utc> = Utc::now().add(chrono::Duration::seconds(600));
    if let Some(client_type) = client_type {
        if client_type == "veygo-app" {
            _exp = Utc::now().add(chrono::Duration::days(28));
        }
    }
    NewAccessToken {
        user_id: _user_id,
        token: generate_unique_token().await,
        exp: _exp,
    }
}

pub async fn verify_user_token(
    _user_id: i32,
    token_data: String,
) -> Result<bool, FromHexError> {
    let binary_token = hex::decode(token_data);
    match binary_token {
        Err(error) => {
            Err(error)
        }
        Ok(binary_token) => {
            let token_clone = binary_token.clone();
            let token_clone_again = binary_token.clone();
            let mut pool = POOL.clone().get().unwrap();
            let token_in_db = spawn_blocking(move || {
                diesel::select(diesel::dsl::exists(access_tokens.filter(token.eq(token_clone)).filter(user_id.eq(_user_id)))).get_result::<bool>(&mut pool)
            }).await.unwrap().unwrap();
            if token_in_db {
                let mut pool = POOL.clone().get().unwrap();
                let token_in_db_result = spawn_blocking(move || {
                    access_tokens.filter(user_id.eq(_user_id)).filter(token.eq(token_clone_again)).first::<AccessToken>(&mut pool)
                }).await.unwrap().unwrap();
                let token_exp = token_in_db_result.exp;
                if token_exp >= Utc::now() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            } else {Ok(false)}
        }
    }
}

pub async fn rm_token_by_binary(
    token_bit: Vec<u8>
) -> AccessToken {
    let mut pool = POOL.clone().get().unwrap();
    diesel::delete(access_tokens.filter(token.eq(token_bit))).get_result::<AccessToken>(&mut pool).unwrap()
}

pub fn token_not_hex_warp_return(
    token_data: &String
) -> Result<(WithStatus<Json>,), Rejection> {
    let error_msg = serde_json::json!({"token": &token_data, "error": "Token not in hex format"});
    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::BAD_REQUEST),))
}

pub fn token_invalid_warp_return(
    token_data: &String
) -> Result<(WithStatus<Json>,), Rejection> {
    let error_msg = serde_json::json!({"token": &token_data, "error": "Token not valid"});
    Ok::<_, warp::Rejection>((warp::reply::with_status(warp::reply::json(&error_msg), StatusCode::UNAUTHORIZED),))
}
