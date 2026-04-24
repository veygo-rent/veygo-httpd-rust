use rust_decimal::prelude::*;
use diesel::prelude::*;
use diesel::result::Error;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use crate::{helper_model, methods, model, schema, connection_pool};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!(String)
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |conf_id: String, method: Method, auth: String, user_agent: String| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response_405();
            }
            let mut pool = connection_pool().await.get().unwrap();
            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                // RETURN: UNAUTHORIZED
                return methods::tokens::token_invalid_return();
            }
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            let user_id = match user_id_parsed_result {
                Ok(int) => {
                    int
                }
                Err(_) => {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_return();
                }
            };

            let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
            let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;

            match if_token_valid_result {
                Err(e) => {
                    match e {
                        helper_model::VeygoError::TokenFormatError => {
                            methods::tokens::token_not_hex_warp_return()
                        }
                        helper_model::VeygoError::InvalidToken => {
                            methods::tokens::token_invalid_return()
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/get: Token verification unexpected error"),
                            )
                        }
                    }
                }
                Ok(valid_token) => {
                    // token is valid
                    let ext_result = methods::tokens::extend_token(valid_token.1, &user_agent).await;

                    match ext_result {
                        Ok(bool) => {
                            if !bool {
                                return methods::standard_replies::internal_server_error_response_500(
                                    String::from("agreement/get: Token extension failed (returned false)"),
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/get: Token extension error"),
                            );
                        }
                    }

                    use schema::agreements::dsl as ag_q;
                    let agreement = ag_q::agreements
                        .filter(ag_q::confirmation.eq(conf_id.to_uppercase()))
                        .get_result::<model::Agreement>(&mut pool);

                    let agreement = match agreement {
                        Ok(agreement) => agreement,
                        Err(e) => {
                            return match e {
                                Error::NotFound => {
                                    methods::standard_replies::agreement_not_allowed_response()
                                }
                                _ => {
                                    methods::standard_replies::internal_server_error_response_500(
                                        String::from("agreement/get: Database error loading agreement"),
                                    )
                                }
                            };
                        }
                    };

                    if agreement.renter_id == user_id {
                        let now = chrono::Utc::now();
                        let six_months_from_now = now - chrono::Duration::days(180);
                        if agreement.rsvp_pickup_time < six_months_from_now {
                            return methods::standard_replies::agreement_not_allowed_response()
                        }
                    } else {
                        let admin = methods::user::get_user_by_id(&user_id).await;
                        if admin.is_err() {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/get: Database error loading admin"),
                            );
                        }
                        use schema::locations::dsl as loc_q;
                        let apt_id = loc_q::locations
                            .find(agreement.location_id)
                            .select(loc_q::apartment_id)
                            .get_result::<i32>(&mut pool);
                        let Ok(apt_id) = apt_id else {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/get: Database error loading apartment id"),
                            );
                        };
                        let admin = admin.unwrap();
                        if !(admin.is_operational_admin() || admin.is_operational_manager() && admin.apartment_id == apt_id) {
                            return methods::standard_replies::agreement_not_allowed_response()
                        }
                    }

                    // user has access to the agreement
                    // load details for TripDetailedInfo
                    use schema::vehicles::dsl as veh_q;
                    use schema::apartments::dsl as apt_q;
                    use schema::locations::dsl as loc_q;
                    use schema::payment_methods::dsl as pm_q;
                    use schema::promos::dsl as prom_q;
                    use schema::mileage_packages::dsl as mp_q;

                    let ag_detailed_tup = ag_q::agreements
                        .find(agreement.id)
                        .inner_join(veh_q::vehicles)
                        .inner_join(
                            loc_q::locations
                                .inner_join(apt_q::apartments)
                        )
                        .inner_join(pm_q::payment_methods)
                        .left_join(prom_q::promos)
                        .left_join(mp_q::mileage_packages)
                        .select((
                            veh_q::vehicles::all_columns(),
                            loc_q::locations::all_columns(),
                            apt_q::apartments::all_columns(),
                            pm_q::payment_methods::all_columns(),
                            prom_q::promos::all_columns().nullable(),
                            mp_q::mileage_packages::all_columns().nullable(),
                        ))
                        .get_result::<(
                            model::Vehicle,
                            model::Location,
                            model::Apartment,
                            model::PaymentMethod,
                            Option<model::Promo>,
                            Option<model::MileagePackage>
                        )>(&mut pool);

                    let Ok(ag_detailed_tup) = ag_detailed_tup else {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("agreement/get: Database error loading apartment details"),
                        );
                    };

                    let vs_before = if let Some(snapshot_id) = agreement.vehicle_snapshot_before {
                        use schema::vehicle_snapshots::dsl as vehicle_snapshot_query;
                        let result = vehicle_snapshot_query::vehicle_snapshots
                            .find(&snapshot_id)
                            .get_result::<model::VehicleSnapshot>(&mut pool);
                        let Ok(vs_before) = result else {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/get: Database error loading before vehicle snapshot"),
                            );
                        };
                        Some(vs_before)
                    } else {
                        None
                    };

                    let vs_after = if let Some(snapshot_id) = agreement.vehicle_snapshot_after {
                        use schema::vehicle_snapshots::dsl as vehicle_snapshot_query;
                        let result = vehicle_snapshot_query::vehicle_snapshots
                            .find(&snapshot_id)
                            .get_result::<model::VehicleSnapshot>(&mut pool);
                        let Ok(vs_after) = result else {
                            return methods::standard_replies::internal_server_error_response_500(
                                String::from("agreement/get: Database error loading after vehicle snapshot"),
                            );
                        };
                        Some(vs_after)
                    } else {
                        None
                    };

                    use schema::taxes::dsl as tax_query;
                    use schema::agreements_taxes::dsl as ag_tax_query;
                    let taxes = ag_tax_query::agreements_taxes
                        .inner_join(tax_query::taxes)
                        .filter(ag_tax_query::agreement_id.eq(&agreement.id))
                        .select(tax_query::taxes::all_columns())
                        .get_results::<model::Tax>(&mut pool);
                    let Ok(taxes) = taxes else {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("agreement/get: Database error loading taxes"),
                        );
                    };

                    use schema::reward_transactions::dsl as rt_query;
                    let reward_transactions = rt_query::reward_transactions
                        .filter(rt_query::agreement_id.eq(&agreement.id))
                        .filter(rt_query::duration.gt(Decimal::ZERO))
                        .get_results::<model::RewardTransaction>(&mut pool);
                    let Ok(reward_transactions) = reward_transactions else {
                        return methods::standard_replies::internal_server_error_response_500(
                            String::from("agreement/get: Database error loading reward transactions"),
                        );
                    };

                    let detailed_trip = helper_model::TripDetailedInfo {
                        agreement,
                        vehicle: ag_detailed_tup.0.into(),
                        apartment: ag_detailed_tup.2,
                        location: ag_detailed_tup.1,
                        vehicle_snapshot_before: vs_before,
                        payment_method: ag_detailed_tup.3.into(),
                        promo: ag_detailed_tup.4.map(Into::into),
                        mileage_package: ag_detailed_tup.5,
                        taxes,
                        vehicle_snapshot_after: vs_after,
                        reward_transactions
                    };

                    methods::standard_replies::response_with_obj(detailed_trip, StatusCode::OK)
                }
            }
        })
}