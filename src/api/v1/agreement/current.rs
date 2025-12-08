use crate::{POOL, methods, model, helper_model};
use diesel::prelude::*;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("current")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method, auth: String, user_agent: String| {

                // Checking method is GET
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response();
                }

                // Pool connection
                let mut pool = POOL.get().unwrap();

                // Checking token
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_wrapped_return();
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_wrapped_return();
                    }
                };
                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                match if_token_valid_result {
                    Err(_) => methods::tokens::token_not_hex_warp_return(),
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_not_hex_warp_return()
                        } else {
                            // Generate new token
                            let token_clone = access_token.clone();
                            methods::tokens::rm_token_by_binary(
                                hex::decode(token_clone.token).unwrap(),
                            ).await;
                            let new_token = methods::tokens::gen_token_object(
                                &access_token.user_id,
                                &user_agent,
                            ).await;
                            use crate::schema::access_tokens::dsl as access_token_query;
                            let new_token_in_db_publish: model::PublishAccessToken = diesel::insert_into(access_token_query::access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .into();

                            // Get current user
                            let user_in_request = methods::user::get_user_by_id(&access_token.user_id).await.unwrap();

                            use crate::schema::agreements::dsl as agreement_query;
                            use crate::schema::apartments::dsl as apartment_query;
                            use crate::schema::vehicles::dsl as vehicle_query;
                            use crate::schema::locations::dsl as location_query;
                            use crate::schema::payment_methods::dsl as payment_method_query;
                            let now = chrono::Utc::now();
                            let now_plus_buffer = now + chrono::Duration::hours(1);
                            let current_agreement_result = agreement_query::agreements
                                .inner_join(location_query::locations
                                    .inner_join(apartment_query::apartments)
                                )
                                .inner_join(vehicle_query::vehicles)
                                .inner_join(payment_method_query::payment_methods)
                                .filter(agreement_query::renter_id.eq(&user_in_request.id))
                                .filter(agreement_query::status.eq(model::AgreementStatus::Rental))
                                .filter(agreement_query::actual_drop_off_time.is_null())
                                .filter(
                                    agreement_query::actual_pickup_time.is_not_null()
                                        .or(agreement_query::rsvp_drop_off_time.ge(now))
                                )
                                .filter(agreement_query::rsvp_pickup_time.le(&now_plus_buffer))
                                .order_by(agreement_query::rsvp_pickup_time.asc())
                                .select(
                                    (
                                        agreement_query::agreements::all_columns(),
                                        vehicle_query::vehicles::all_columns(),
                                        apartment_query::apartments::all_columns(),
                                        location_query::locations::all_columns(),
                                        payment_method_query::payment_methods::all_columns()
                                    )
                                )
                                .first::<(model::Agreement, model::Vehicle, model::Apartment, model::Location, model::PaymentMethod)>(&mut pool);
                            return match current_agreement_result {
                                Err(_) => {
                                    let err_msg = helper_model::ErrorResponse {
                                        title: "No Current Reservation".to_string(),
                                        message: "Cannot find current reservation. ".to_string(),
                                    };
                                    let reply = with_status(warp::reply::json(&err_msg), StatusCode::NOT_FOUND);
                                    Ok::<_, Rejection>((methods::tokens::wrap_json_reply_with_token(new_token_in_db_publish.clone(), reply).into_response(),))
                                },
                                Ok(current) => {
                                    let mut current_trip = helper_model::CurrentTrip {
                                        agreement: current.0,
                                        vehicle: current.1.into(),
                                        apartment: current.2,
                                        location: current.3,
                                        vehicle_snapshot_before: None,
                                        payment_method: current.4.into(),
                                        promo: None,
                                        mileage_package: None,
                                    };
                                    if let Some(snapshot_id) = current_trip.agreement.vehicle_snapshot_before {
                                        use crate::schema::vehicle_snapshots::dsl as vehicle_snapshot_query;
                                        let snapshot = vehicle_snapshot_query::vehicle_snapshots
                                            .filter(vehicle_snapshot_query::id.eq(&snapshot_id))
                                            .get_result::<model::VehicleSnapshot>(&mut pool);
                                        if snapshot.is_ok() {
                                            current_trip.vehicle_snapshot_before = Some(snapshot.unwrap());
                                        }
                                    }
                                    if let Some(promo_code) = current_trip.agreement.promo_id.clone() {
                                        use crate::schema::promos::dsl as promo_query;
                                        let promo = promo_query::promos
                                            .filter(promo_query::code.eq(&promo_code))
                                            .get_result::<model::Promo>(&mut pool);
                                        if promo.is_ok() {
                                            current_trip.promo = Some(promo.unwrap().into());
                                        }
                                    }
                                    if let Some(mileage_package_id) = current_trip.agreement.mileage_package_id {
                                        use crate::schema::mileage_packages::dsl as mp_query;
                                        let mp = mp_query::mileage_packages
                                            .filter(mp_query::id.eq(&mileage_package_id))
                                            .get_result::<model::MileagePackage>(&mut pool);
                                        if mp.is_ok() {
                                            current_trip.mileage_package = Some(mp.unwrap());
                                        }
                                    }
                                    Ok::<_, Rejection>((methods::tokens::wrap_json_reply_with_token(
                                        new_token_in_db_publish,
                                        with_status(warp::reply::json(&current_trip), StatusCode::OK),
                                    ),))
                                }
                            }
                        }
                    }
                }

            }
        )
}