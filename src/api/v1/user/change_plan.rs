use crate::{POOL, integration, methods, model, schema};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use warp::{Filter, Reply};

#[derive(Deserialize, Serialize)]
struct ChangePlanRequest {
    plan: model::PlanTier,
    is_plan_annual: bool,
    payment_method_id: Option<i32>,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("change-plan")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("token"))
        .and(warp::header::<i32>("user_id"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |request_body: ChangePlanRequest,
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
                    Ok(token_bool) => {
                        if !token_bool {
                            methods::tokens::token_invalid_wrapped_return(&access_token.token)
                        } else {
                            // gen new token
                            methods::tokens::rm_token_by_binary(
                                hex::decode(access_token.token).unwrap(),
                            )
                            .await;
                            let new_token = methods::tokens::gen_token_object(
                                access_token.user_id.clone(),
                                client_type.clone(),
                            )
                            .await;
                            use schema::access_tokens::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let new_token_in_db_publish = diesel::insert_into(access_tokens)
                                .values(&new_token)
                                .get_result::<model::AccessToken>(&mut pool)
                                .unwrap()
                                .to_publish_access_token();
                            if request_body.plan == model::PlanTier::Free {
                            } else {
                                if let Some(pm_id) = request_body.payment_method_id {
                                    use schema::payment_methods::dsl::*;
                                    let payment_method_result = payment_methods
                                        .into_boxed()
                                        .filter(id.eq(pm_id))
                                        .filter(is_enabled.eq(true))
                                        .filter(renter_id.eq(access_token.user_id))
                                        .get_result::<model::PaymentMethod>(&mut pool);
                                    return match payment_method_result { 
                                        Err(_) => {
                                            methods::standard_replies::card_invalid_wrapped(new_token_in_db_publish)
                                        },
                                        Ok(payment_method) => {
                                            use schema::apartments::dsl::*;
                                            methods::standard_replies::not_implemented_response()
                                        }
                                    }
                                }
                            }
                            methods::standard_replies::not_implemented_response()
                        }
                    }
                };
            },
        )
}
