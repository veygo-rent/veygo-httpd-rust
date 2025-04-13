use crate::{POOL, methods, model};
use bytes::Buf;
use diesel::prelude::*;
use futures::TryStreamExt;
use secrets::traits::AsContiguousBytes;
use warp::http::StatusCode;
use warp::multipart::{FormData, Part};
use warp::reply::with_status;
use warp::{Filter, Reply};

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("upload-tolls")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::multipart::form().max_length(5 * 1024 * 1024))
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |form: FormData,
                        token: String,
                        user_id: i32,
                        client_type: Option<String>| {
                let access_token = model::RequestToken { user_id, token };
                let if_token_valid = methods::tokens::verify_user_token(
                    access_token.user_id.clone(),
                    access_token.token.clone(),
                )
                .await;
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                    Ok(token_is_valid) => {
                        if !token_is_valid {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
                        } else {
                            // Token is valid
                            let id_clone = access_token.user_id.clone();
                            let admin = methods::user::get_user_by_id(id_clone).await.unwrap();
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
                            if !methods::user::user_with_admin_access(&admin) {
                                let token_clone = new_token_in_db_publish.clone();
                                return methods::standard_replies::user_not_admin_wrapped_return(
                                    token_clone,
                                );
                            }
                            let parts: Vec<Part> = form.try_collect().await.unwrap();
                            let file_count = parts.len() as i32;
                            if file_count != 1 {
                                let msg = serde_json::json!({
                                    "message": "Please upload exactly one file",
                                });
                                return Ok::<_, warp::Rejection>((
                                    methods::tokens::wrap_json_reply_with_token(
                                        new_token_in_db_publish,
                                        with_status(
                                            warp::reply::json(&msg),
                                            StatusCode::NOT_ACCEPTABLE,
                                        ),
                                    ),
                                ));
                            };
                            let part = parts.into_iter().next().unwrap();
                            let bytes: Vec<u8> = part
                                .stream()
                                .try_fold(Vec::new(), |mut acc, data| async move {
                                    acc.extend_from_slice(data.chunk());
                                    Ok(acc)
                                })
                                .await.unwrap();
                            // Wrap the buffer into a Cursor to implement std::io::Read

                            // Parse CSV and convert to a JSON array
                            let mut rdr = csv::ReaderBuilder::new().has_headers(false).from_reader(bytes.as_bytes());

                            // Try to get headers from the CSV; if this fails, return a BAD_REQUEST response
                            let headers = match rdr.headers() {
                                Ok(h) => h.clone(),
                                Err(e) => return Ok((with_status(
                                    warp::reply::json(&serde_json::json!({ "error": format!("CSV header error: {}", e) })),
                                    StatusCode::BAD_REQUEST
                                ).into_response(), ))
                            };

                            let mut json_records = Vec::new();
                            for result in rdr.records() {
                                match result {
                                    Ok(record) => {
                                        let mut map = serde_json::Map::new();
                                        for (header, field) in headers.iter().zip(record.iter()) {
                                            map.insert(header.to_string(), serde_json::Value::String(field.to_string()));
                                        }
                                        json_records.push(serde_json::Value::Object(map));
                                    },
                                    Err(e) => {
                                        return Ok((with_status(
                                            warp::reply::json(&serde_json::json!({ "error": format!("CSV record error: {}", e) })),
                                            StatusCode::BAD_REQUEST
                                        ).into_response(),));
                                    }
                                }
                            }

                            return Ok((with_status(
                                warp::reply::json(&json_records),
                                StatusCode::OK
                            ).into_response(),));
                        }
                    }
                };
            },
        )
}
