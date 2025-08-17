use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use warp::{Filter, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;
use crate::{methods, model, POOL};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("get")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |method: Method,
                        auth: String,
                        user_agent: String| {
                if method != Method::GET {
                    return methods::standard_replies::method_not_allowed_response();
                }
                let mut pool = POOL.get().unwrap();
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    // RETURN: UNAUTHORIZED
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                let user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        // RETURN: UNAUTHORIZED
                        return methods::tokens::token_invalid_wrapped_return(&auth);
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
                let if_token_valid_result = methods::tokens::verify_user_token(&access_token.user_id, &access_token.token).await;
                if if_token_valid_result.is_err() {
                    return methods::tokens::token_not_hex_warp_return(&access_token.token);
                }
                let token_bool = if_token_valid_result.unwrap();

                if !token_bool {
                    // RETURN: UNAUTHORIZED
                    methods::tokens::token_invalid_wrapped_return(&access_token.token)
                } else {
                    // gen new token
                    let token_clone = access_token.clone();
                    methods::tokens::rm_token_by_binary(
                        hex::decode(token_clone.token).unwrap(),
                    ).await;
                    let new_token = methods::tokens::gen_token_object(
                        &access_token.user_id,
                        &user_agent,
                    ).await;
                    use crate::schema::access_tokens::dsl::*;
                    let new_token_in_db_publish = diesel::insert_into(access_tokens)
                        .values(&new_token)
                        .get_result::<model::AccessToken>(&mut pool)
                        .unwrap()
                        .to_publish_access_token();

                    use crate::schema::agreements::dsl as agreement_query;
                    let agreements = agreement_query::agreements.filter(agreement_query::renter_id.eq(&access_token.user_id)).get_results::<model::Agreement>(&mut pool).unwrap_or_default();

                    let msg = serde_json::json!({
                            "agreements": agreements,
                        });
                    Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(
                        new_token_in_db_publish,
                        with_status(warp::reply::json(&msg), StatusCode::OK),
                    ),))
                }
            }
        )
}