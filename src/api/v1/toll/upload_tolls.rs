use crate::{POOL, methods, model, helper_model};
use blake3;
use bytes::BufMut;
use currency_rs::Currency;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Timestamptz};
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
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("toll-id"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |form: FormData,
                        auth: String,
                        toll_id: String,
                        user_agent: String| {
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_wrapped_return();
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_wrapped_return();
                    }
                };

                let field_names: Vec<_> = form
                    .and_then(|mut field| async move {
                        let mut bytes: Vec<u8> = Vec::new();

                        // field.data() only returns a piece of the content, you should call over it until it replies to None
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
                    return methods::standard_replies::bad_request("Please upload exactly one file");
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid = methods::tokens::verify_user_token(
                    &access_token.user_id,
                    &access_token.token,
                )
                .await;
                return match if_token_valid {
                    Err(_) => methods::tokens::token_not_hex_warp_return(),
                    Ok(token_is_valid) => {
                        if !token_is_valid {
                            methods::tokens::token_invalid_wrapped_return()
                        } else {
                            // Token is valid
                            let admin = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();
                            let token_clone = access_token.clone();
                            methods::tokens::rm_token_by_binary(
                                hex::decode(token_clone.token).unwrap(),
                            )
                            .await;
                            let new_token = methods::tokens::gen_token_object(
                                &access_token.user_id,
                                &user_agent,
                            )
                            .await;
                            use crate::schema::access_tokens::dsl::*;
                            let mut pool = POOL.get().unwrap();
                            let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .into();
                            let toll_id_int: i32 = match toll_id.parse() {
                                Ok(int) => int,
                                Err(_) => {
                                    return Ok((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(
                                        warp::reply::json(&serde_json::json!({ "error": "Toll company invalid" })),
                                        StatusCode::NOT_ACCEPTABLE
                                    )),));
                                }
                            };
                            let toll_company = {
                                use crate::schema::transponder_companies::dsl::*;
                                let if_exists = diesel::select(exists(transponder_companies
                                    .into_boxed().filter(id.eq(toll_id_int)))).get_result::<bool>(&mut pool).unwrap();
                                if !if_exists {
                                    return Ok((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish, with_status(
                                        warp::reply::json(&serde_json::json!({ "error": "Toll company not found" })),
                                        StatusCode::NOT_ACCEPTABLE
                                    )),));
                                }
                                transponder_companies
                                    .into_boxed().filter(id.eq(toll_id_int))
                                    .get_result::<model::TransponderCompany>(&mut pool).unwrap()
                            };
                            if !methods::user::user_is_operational_admin(&admin) {
                                let token_clone = new_token_in_db_publish.clone();
                                return methods::standard_replies::user_not_admin_wrapped_return(
                                    token_clone,
                                );
                            }
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
                            let toll_company_clone = toll_company.clone();
                            let required: HashSet<&str> = [
                                toll_company_clone.corresponding_key_for_transaction_amount.as_str(),
                                toll_company_clone.corresponding_key_for_transaction_name.as_str(),
                                toll_company_clone.corresponding_key_for_transaction_time.as_str(),
                                toll_company_clone.corresponding_key_for_vehicle_id.as_str(),
                            ]
                                .into_iter()
                                .collect();
                            let csv_cols: HashSet<&str> = headers.iter().collect();
                            let missing: Vec<&str> = required
                                .difference(&csv_cols)
                                .copied()
                                .collect();
                            if !missing.is_empty() {
                                let err_msg = helper_model::ErrorResponse {
                                    title: "Missing Columns".to_string(),
                                    message: "CSV is missing required columns: ".to_string() + missing.join(", ").as_str(),
                                };
                                return Ok((methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    with_status(warp::reply::json(&err_msg), StatusCode::NOT_ACCEPTABLE),
                                ),));
                            }

                            // Clone what we need into the background task
                            let records = json_records.clone();
                            let tc = toll_company.clone();

                            // Spawn background processing of each record
                            tokio::spawn(async move {
                                for record in records.into_iter() {
                                    let toll_company_clone = tc.clone();
                                    let toll_company_id = toll_company_clone.id;
                                    let vehicle_identifier_str = record[toll_company_clone.corresponding_key_for_vehicle_id].to_string();
                                    let transaction_time_string = record[toll_company_clone.corresponding_key_for_transaction_time].to_string();
                                    let transaction_amount_string = record[toll_company_clone.corresponding_key_for_transaction_amount].to_string();
                                    let transaction_time = methods::timestamps::to_utc(&transaction_time_string, &toll_company_clone.timestamp_format, toll_company_clone.timezone).unwrap();
                                    let transaction_amount = Currency::new_string(&transaction_amount_string, None).unwrap().value().abs();
                                    let transaction_name = record[toll_company_clone.corresponding_key_for_transaction_name].to_string();
                                    let to_be_hashed = transaction_name.clone() + &transaction_amount.to_string() + &transaction_time.to_string();
                                    let hashed = blake3::hash(to_be_hashed.as_bytes()).to_hex().to_string();
                                    use crate::schema::charges::dsl::*;
                                    let mut pool = POOL.get().unwrap();
                                    let if_exist = diesel::select(diesel::dsl::exists(
                                        charges
                                            .filter(checksum.eq(&hashed)),
                                    )).get_result::<bool>(&mut pool).unwrap();
                                    if !if_exist {
                                        let mut charge_record = crate::model::NewCharge {
                                            name: toll_company_clone.custom_prefix_for_transaction_name + &*" ".to_string() + &*transaction_name,
                                            time: transaction_time,
                                            amount: transaction_amount,
                                            note: None,
                                            agreement_id: None,
                                            vehicle_id: 0,
                                            checksum: hashed.to_string(),
                                            transponder_company_id: Option::from(toll_company_id),
                                            vehicle_identifier: Option::from(vehicle_identifier_str.clone()),
                                        };
                                        use crate::schema::vehicles::dsl::*;
                                        let vehicle_result: QueryResult<crate::model::Vehicle> = vehicles
                                            .into_boxed().filter(
                                            first_transponder_company_id.eq(toll_company_id).and(first_transponder_number.eq(&vehicle_identifier_str))
                                                .or(second_transponder_company_id.eq(toll_company_id).and(second_transponder_number.eq(&vehicle_identifier_str)))
                                                .or(third_transponder_company_id.eq(toll_company_id).and(third_transponder_number.eq(&vehicle_identifier_str)))
                                                .or(fourth_transponder_company_id.eq(toll_company_id).and(fourth_transponder_number.eq(&vehicle_identifier_str)))
                                        ).get_result::<crate::model::Vehicle>(&mut pool);
                                        if let Ok(vehicle_result) = vehicle_result {
                                            // found vehicle
                                            charge_record.vehicle_id = vehicle_result.id;
                                            // try to find agreement
                                            use crate::schema::agreements::dsl::*;
                                            use diesel::dsl::sql;
                                            let agreement_result = agreements
                                                .into_boxed().filter(vehicle_id.eq(vehicle_result.id))
                                                .filter(sql::<Bool>("COALESCE(actual_pickup_time, rsvp_pickup_time) <= ")
                                                    .bind::<Timestamptz, _>(transaction_time)
                                                    .sql(" AND COALESCE(actual_drop_off_time, rsvp_drop_off_time) >= ")
                                                    .bind::<Timestamptz, _>(transaction_time)
                                                )
                                                .get_result::<crate::model::Agreement>(&mut pool);
                                            if let Ok(agreement) = agreement_result {
                                                charge_record.agreement_id = Some(agreement.id);
                                            }
                                            let _ = diesel::insert_into(charges).values(&charge_record).execute(&mut pool);
                                        }
                                    }
                                }
                            });

                            // Immediately respond OK; records are processing in the background
                            return Ok((
                                methods::tokens::wrap_json_reply_with_token(
                                    new_token_in_db_publish,
                                    with_status(warp::reply::json(&json_records), StatusCode::OK),
                                ),
                            ));
                            // note: original synchronous insertion code has been moved into the spawn above
                        }
                    }
                };
            },
        )
}
