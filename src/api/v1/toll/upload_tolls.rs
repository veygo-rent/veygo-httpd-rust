use crate::{POOL, methods, model, helper_model, methods::diesel_fn};
use currency_rs::Currency;
use diesel::prelude::*;
use std::collections::HashSet;
use warp::{Filter, Reply};
use warp::http::{StatusCode, Method};
use bytes::{Bytes};
use rust_decimal::Decimal;
use crate::helper_model::VeygoError;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path!("upload-tolls" / i32)
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::bytes())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |toll_id: i32,
                        method: Method,
                        body: Bytes,
                        auth: String,
                        user_agent: String| {
                if method != Method::POST {
                    return methods::standard_replies::method_not_allowed_response();
                }
                
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_return();
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_return();
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid = methods::tokens::verify_user_token(
                    &access_token.user_id,
                    &access_token.token,
                )
                .await;

                return match if_token_valid {
                    Err(err) => {
                        match err {
                            VeygoError::TokenFormatError => {
                                methods::tokens::token_not_hex_warp_return()
                            }
                            VeygoError::InvalidToken => {
                                methods::tokens::token_invalid_return()
                            }
                            _ => {
                                methods::standard_replies::internal_server_error_response(
                                    "toll/upload-tolls: Token verification unexpected error",
                                )
                                .await
                            }
                        }
                    }
                    Ok(valid_token) => {
                        // token is valid
                        let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent);

                        match ext_result {
                            Ok(bool) => {
                                if !bool {
                                    return methods::standard_replies::internal_server_error_response(
                                        "toll/upload-tolls: Token extension failed (returned false)",
                                    )
                                    .await;
                                }
                            }
                            Err(_) => {
                                return methods::standard_replies::internal_server_error_response(
                                    "toll/upload-tolls: Token extension error",
                                )
                                .await;
                            }
                        }

                        let admin = methods::user::get_user_by_id(&access_token.user_id)
                            .await;

                        let Ok(admin) = admin else {
                            return methods::standard_replies::internal_server_error_response(
                                "toll/upload-tolls: Database error loading admin user",
                            )
                            .await
                        };

                        if !admin.is_operational_admin() {
                            return methods::standard_replies::admin_not_verified()
                        }
                        let mut pool = POOL.get().unwrap();

                        let toll_company = {
                            use crate::schema::transponder_companies::dsl::*;

                            let selected_tc_result = transponder_companies
                                .find(&toll_id)
                                .get_result::<model::TransponderCompany>(&mut pool);

                            match selected_tc_result {
                                Ok(tc) => { tc }
                                Err(_) => {
                                    return methods::standard_replies::internal_server_error_response(
                                        "toll/upload-tolls: Database error loading transponder company",
                                    )
                                    .await
                                }
                            }
                        };

                        // Parse CSV and convert to a JSON array
                        let file_bytes = body.to_vec();
                        let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(file_bytes.as_slice());

                        let headers = match (&mut rdr).headers() {
                            Ok(h) => { h.clone() }
                            Err(_) => {
                                let msg = helper_model::ErrorResponse { title: "CSV File Error".to_string(), message: "No headers found. Please check the file again. ".to_string() };
                                return methods::standard_replies::response_with_obj(msg, StatusCode::NOT_ACCEPTABLE)
                            }
                        };

                        let required: HashSet<&str> =
                            [
                                toll_company.corresponding_key_for_transaction_amount.as_str(),
                                toll_company.corresponding_key_for_transaction_name.as_str(),
                                toll_company.corresponding_key_for_transaction_time.as_str(),
                                toll_company.corresponding_key_for_vehicle_id.as_str(),
                            ].into_iter().collect();
                        let csv_cols: HashSet<&str> = headers.iter().collect();

                        let missing: Vec<&str> = required
                            .difference(&csv_cols)
                            .copied()
                            .collect();

                        if !missing.is_empty() {
                            let err_msg = helper_model::ErrorResponse {
                                title: "CSV File Error".to_string(),
                                message: "CSV is missing required columns: ".to_string() + missing.join(", ").as_str(),
                            };
                            return methods::standard_replies::response_with_obj(err_msg, StatusCode::NOT_ACCEPTABLE)
                        }

                        let transaction_amount_index = headers
                            .iter()
                            .position(|h| h == toll_company.corresponding_key_for_transaction_amount.as_str());
                        let Some(transaction_amount_index) = transaction_amount_index else {
                            return methods::standard_replies::internal_server_error_response(
                                "toll/upload-tolls: CSV header missing transaction amount column",
                            )
                            .await
                        };

                        let transaction_name_index = headers
                            .iter()
                            .position(|h| h == toll_company.corresponding_key_for_transaction_name.as_str());
                        let Some(transaction_name_index) = transaction_name_index else {
                            return methods::standard_replies::internal_server_error_response(
                                "toll/upload-tolls: CSV header missing transaction name column",
                            )
                            .await
                        };

                        let transaction_time_index = headers
                            .iter()
                            .position(|h| h == toll_company.corresponding_key_for_transaction_time.as_str());
                        let Some(transaction_time_index) = transaction_time_index else {
                            return methods::standard_replies::internal_server_error_response(
                                "toll/upload-tolls: CSV header missing transaction time column",
                            )
                            .await
                        };

                        let vehicle_id_index = headers
                            .iter()
                            .position(|h| h == toll_company.corresponding_key_for_vehicle_id.as_str());
                        let Some(vehicle_id_index) = vehicle_id_index else {
                            return methods::standard_replies::internal_server_error_response(
                                "toll/upload-tolls: CSV header missing vehicle id column",
                            )
                            .await
                        };

                        let file_bytes_to_move = file_bytes.clone();
                        let toll_company_to_move = toll_company.clone();
                        tokio::spawn(async move {
                            let mut pool = POOL.get().unwrap();
                            let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(file_bytes_to_move.as_slice());
                            let _ = (&mut rdr).headers();

                            for record_res in (&mut rdr).records() {
                                match record_res {
                                    Ok(result) => {
                                        let transaction_amount = match result.get(transaction_amount_index) {
                                            None => { continue }
                                            Some(str) => {
                                                let temp = Currency::new_string(str, None);
                                                let Ok(transaction_amount_in_currency) = temp else {
                                                    continue
                                                };
                                                let cents = transaction_amount_in_currency.cents();
                                                Decimal::new(cents as i64, 2)
                                            }
                                        };
                                        let transaction_name = match result.get(transaction_name_index) {
                                            None => { continue }
                                            Some(str) => { str }
                                        };
                                        let transaction_time = match result.get(transaction_time_index) {
                                            None => { continue }
                                            Some(str) => { str }
                                        };
                                        let vehicle_id_str = match result.get(vehicle_id_index) {
                                            None => { continue }
                                            Some(str) => { str }
                                        };

                                        use crate::schema::vehicles::dsl as v_q;
                                        let vehicle_result = v_q::vehicles
                                            .into_boxed()
                                            .filter(
                                                v_q::first_transponder_company_id.eq(&toll_company_to_move.id).and(v_q::first_transponder_number.eq(&vehicle_id_str))
                                                    .or(v_q::second_transponder_company_id.eq(&toll_company_to_move.id).and(v_q::second_transponder_number.eq(&vehicle_id_str)))
                                                    .or(v_q::third_transponder_company_id.eq(&toll_company_to_move.id).and(v_q::third_transponder_number.eq(&vehicle_id_str)))
                                                    .or(v_q::fourth_transponder_company_id.eq(&toll_company_to_move.id).and(v_q::fourth_transponder_number.eq(&vehicle_id_str)))
                                            )
                                            .select(v_q::id)
                                            .get_result::<i32>(&mut pool);
                                        let Ok(vehicle_id) = vehicle_result else { continue };


                                        let transaction_time = methods::timestamps::to_utc(
                                            &transaction_time, &toll_company_to_move.timestamp_format, toll_company_to_move.timezone.clone()
                                        );
                                        let Ok(transaction_time) = transaction_time else { continue };

                                        let charge_record = model::NewCharge {
                                            name: toll_company_to_move.custom_prefix_for_transaction_name.clone() + &*" ".to_string() + &*transaction_name,
                                            time: transaction_time,
                                            amount: transaction_amount,
                                            note: None,
                                            agreement_id: None,
                                            vehicle_id,
                                            transponder_company_id: Option::from(toll_company_to_move.id),
                                            vehicle_identifier: Option::from(String::from(vehicle_id_str)),
                                        };

                                        use crate::schema::charges::dsl as c_q;
                                        let insert_result = diesel::insert_into(c_q::charges)
                                            .values(&charge_record)
                                            .get_result::<model::Charge>(&mut pool);

                                        match insert_result {
                                            Ok(chg) => {
                                                use crate::schema::agreements::dsl as ag_q;
                                                let affected_agreement = ag_q::agreements
                                                    .filter(ag_q::actual_pickup_time.le(&chg.time))
                                                    .filter(diesel_fn::coalesce(ag_q::actual_drop_off_time, diesel::dsl::now).ge(&chg.time))
                                                    .filter(ag_q::vehicle_id.eq(&chg.vehicle_id))
                                                    .select(ag_q::id)
                                                    .get_result::<i32>(&mut pool);

                                                if let Ok(ag) = affected_agreement {
                                                    let _ = diesel::update
                                                        (
                                                            c_q::charges
                                                                .find(&chg.id)
                                                        )
                                                        .set(c_q::agreement_id.eq(ag))
                                                        .execute(&mut pool);
                                                }

                                            }
                                            Err(_err) => {}
                                        }
                                    }
                                    Err(_) => continue
                                }
                            }
                        });

                        let msg = serde_json::json!({});
                        Ok((warp::reply::with_status(warp::reply::json(&msg), StatusCode::OK).into_response(),))
                    }
                };
            },
        )
}
