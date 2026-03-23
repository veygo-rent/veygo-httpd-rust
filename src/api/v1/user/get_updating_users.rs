use diesel::prelude::*;
use diesel::sql_types::Numeric;
use http::{Method, StatusCode};
use warp::{Filter, Reply};
use crate::{methods, model, POOL};
use crate::helper_model::VeygoError;
use diesel::dsl::{IntervalDsl, today as current_date};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("updating-user")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method:Method, auth: String, user_agent: String| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response();
            }
            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_return();
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => int,
                Err(_) => {
                    return methods::tokens::token_invalid_return();
                }
            };

            let access_token = model::RequestToken {
                user_id,
                token: token_and_id[0].parse().unwrap(),
            };
            let if_token_valid =
                methods::tokens::verify_user_token(&access_token.user_id, &access_token.token)
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
                                String::from("user/get: Token verification unexpected error")
                            )
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
                                    String::from("user/get: Token extension failed (returned false)")
                                );
                            }
                        }
                        Err(_) => {
                            return methods::standard_replies::internal_server_error_response(
                                String::from("user/get: Token extension error")
                            );
                        }
                    }

                    let admin = methods::user::get_user_by_id(&access_token.user_id)
                        .await;

                    let Ok(admin) = admin else {
                        return methods::standard_replies::internal_server_error_response(
                            String::from("user/get: Database error loading admin user")
                        );
                    };

                    if !admin.is_operational_admin() {
                        return methods::standard_replies::admin_not_verified()
                    }


                    use diesel::dsl::sql;
                    use crate::schema::renters::dsl as rt_q;

                    let mut pool = POOL.get().unwrap();

                    let one_day = 1.day();

                    let renewal_day_as_number = sql::<Numeric>("plan_renewal_day::numeric");

                    let user_needs_to_renew_cmd = rt_q::renters
                        .filter(rt_q::plan_expire_month_year.eq(methods::diesel_fn::to_char_tstz(methods::diesel_fn::now(), "MMYYYY")))
                        .filter(
                            renewal_day_as_number.clone().eq(methods::diesel_fn::extract_date("DAY", current_date))
                                .or(
                                    renewal_day_as_number.gt(methods::diesel_fn::extract_date("DAY", current_date))
                                        .and(
                                            methods::diesel_fn::extract_ts("MONTH", current_date + one_day)
                                                .ne(methods::diesel_fn::extract_date("MONTH", current_date))
                                        )
                                )
                        );

                    let user_needs_to_renew = user_needs_to_renew_cmd
                        .load::<model::Renter>(&mut pool);

                    if let Ok(user_needs_to_renew) = user_needs_to_renew {
                        let pub_user: Vec<model::PublishRenter> = user_needs_to_renew
                            .into_iter()
                            .map(|x| x.into())
                            .collect();
                        methods::standard_replies::response_with_obj(pub_user, StatusCode::OK)
                    } else {
                        methods::standard_replies::internal_server_error_response(String::from("user/updating-user: DB error loading renewing renters"))
                    }
                }
            }
        })
}