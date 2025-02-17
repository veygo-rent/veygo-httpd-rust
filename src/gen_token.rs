use std::ops::Add;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use secrets::Secret;
use tokio::task;
use crate::db;
use crate::model::NewAccessToken;

pub async fn generate_unique_token() -> Vec<u8> {
    loop {
        // Generate a secure random 32-byte token
        let token_vec = Secret::<[u8; 32]>::random(|s| s.to_vec());

        let token_to_return = token_vec.clone();

        // Wrap in a block for error handling.
        let token_exists_result: Result<QueryResult<bool>, task::JoinError> = task::spawn_blocking(move || {
            //get connection.
            let conn = &mut db::get_connection_pool().get().unwrap();
            // Perform the Diesel query *synchronously* within the closure.
            // Use the token_vec.expose_secret() here.
            diesel::select(diesel::dsl::exists(
                crate::schema::access_tokens::table.filter(crate::schema::access_tokens::token.eq(token_vec))
            ))
                .get_result::<bool>(conn)

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
    user_id: i32,
    client_type: Option<String>,
) -> NewAccessToken {
    let mut _exp: DateTime<Utc> = Utc::now().add(chrono::Duration::seconds(600));
    if let Some(client_type) = client_type {
        if client_type == "veygo-app" {
            _exp = Utc::now().add(chrono::Duration::days(28));
        }
    }
    NewAccessToken {
        user_id,
        token: generate_unique_token().await,
        exp: _exp,
    }
}
