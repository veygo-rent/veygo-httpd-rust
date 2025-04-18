use crate::{POOL, methods, model};
use bytes::BufMut;
use diesel::dsl::exists;
use diesel::prelude::*;
use futures::TryStreamExt;
use std::collections::HashSet;
use warp::Filter;
use warp::http::StatusCode;
use warp::multipart::FormData;
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("upload-tolls")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::multipart::form().max_length(5 * 1024 * 1024))
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::<i32>("toll_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |form: FormData,
                        token: String,
                        user_id: i32,
                        toll_id: i32,
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
                            let field_names: Vec<_> = form
                                .and_then(|mut field| async move {
                                    let mut bytes: Vec<u8> = Vec::new();

                                    // field.data() only returns a piece of the content, you should call over it until it replies None
                                    while let Some(content) = field.data().await {
                                        let content = content.unwrap();
                                        bytes.put(content);
                                    }
                                    Ok((
                                        bytes,
                                    ))
                                })
                                .try_collect()
                                .await
                                .unwrap();
                            let file_count = field_names.len() as i32;
                            if file_count != 1 {
                                let msg = serde_json::json!({
                                      "message": "Please upload exactly one file",
                                  });
                                return Ok::<_, warp::Rejection>((
                                    methods::tokens::wrap_json_reply_with_token(
                                        new_token_in_db_publish,
                                        with_status(
                                            warp::reply::json(&msg),
                                            StatusCode::BAD_REQUEST,
                                        ),
                                    ),
                                ));
                            };
                            // Parse CSV and convert to a JSON array
                            let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(field_names[0].0.as_slice());

                            // Try to get headers from the CSV; if this fails, return a BAD_REQUEST response
                            let headers = match rdr.headers() {
                                Ok(h) => h.clone(),
                                Err(e) => return Ok((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(
                                    warp::reply::json(&serde_json::json!({ "error": format!("CSV header error: {}", e) })),
                                    StatusCode::NOT_ACCEPTABLE
                                )),))
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
                                        return Ok((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(
                                            warp::reply::json(&serde_json::json!({ "message": format!("CSV record error: {}", e) })),
                                            StatusCode::NOT_ACCEPTABLE
                                        )),));
                                    }
                                }
                            }
                            let toll_company = {
                                use crate::schema::transponder_companies::dsl::*;
                                let if_exists = diesel::select(exists(transponder_companies.filter(id.eq(toll_id)))).get_result::<bool>(&mut pool).unwrap();
                                if !if_exists {
                                    return Ok((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(
                                        warp::reply::json(&serde_json::json!({ "error": "Toll company not found" })),
                                        StatusCode::NOT_ACCEPTABLE
                                    )),));
                                }
                                transponder_companies.filter(id.eq(toll_id)).get_result::<model::TransponderCompany>(&mut pool).unwrap()
                            };
                            let required: HashSet<&str> = [
                                toll_company.corresponding_key_for_transaction_amount.as_str(),
                                toll_company.corresponding_key_for_transaction_name.as_str(),
                                toll_company.corresponding_key_for_transaction_time.as_str(),
                                toll_company.corresponding_key_for_vehicle_id.as_str(),
                            ]
                                .into_iter()
                                .collect();
                            let csv_cols: HashSet<&str> = headers.iter().collect();
                            let missing: Vec<&str> = required
                                .difference(&csv_cols)
                                .copied()
                                .collect();
                            if !missing.is_empty() {
                                let msg = serde_json::json!({
                                    "error": "CSV is missing required columns",
                                    "missing_columns": missing,
                                });
                                return Ok((methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    with_status(warp::reply::json(&msg), StatusCode::NOT_ACCEPTABLE),
                                ),));
                            }
                            // TODO
                            return Ok((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(
                                warp::reply::json(&json_records),
                                StatusCode::OK
                            )),));
                        }
                    }
                };
            },
        )
}
