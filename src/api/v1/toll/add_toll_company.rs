use crate::{POOL, methods, model};
use diesel::prelude::*;
use warp::Filter;
use warp::http::StatusCode;
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path("add-company")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::optional::<String>("x-client-type"))
        .and_then(
            async move |transponder_company: model::NewTransponderCompany,
                        auth: String,
                        client_type: Option<String>| {
                let token_and_id = auth.split("$").collect::<Vec<&str>>();
                if token_and_id.len() != 2 {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
                }
                let user_id;
                let user_id_parsed_result = token_and_id[1].parse::<i32>();
                user_id = match user_id_parsed_result {
                    Ok(int) => {
                        int
                    }
                    Err(_) => {
                        return methods::tokens::token_invalid_wrapped_return(&auth);
                    }
                };

                let access_token = model::RequestToken { user_id, token: token_and_id[0].parse().unwrap() };
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
                            use crate::schema::transponder_companies::dsl::*;
                            let mut pool = POOL.clone().get().unwrap();
                            let insert_result = diesel::insert_into(transponder_companies)
                                .values(&transponder_company)
                                .get_result::<model::TransponderCompany>(&mut pool);
                            return match insert_result {
                                Err(_) => {
                                    let msg = serde_json::json!({"error": "Transponder company already exists"});
                                    Ok::<_, warp::Rejection>((
                                        methods::tokens::wrap_json_reply_with_token(
                                            new_token_in_db_publish,
                                            with_status(
                                                warp::reply::json(&msg),
                                                StatusCode::NOT_ACCEPTABLE,
                                            ),
                                        ),
                                    ))
                                }
                                Ok(company) => {
                                    let msg = serde_json::json!({"transponder_company": &company});
                                    Ok::<_, warp::Rejection>((
                                        methods::tokens::wrap_json_reply_with_token(
                                            new_token_in_db_publish,
                                            with_status(
                                                warp::reply::json(&msg),
                                                StatusCode::CREATED,
                                            ),
                                        ),
                                    ))
                                }
                            };
                        }
                    }
                };
            },
        )
}
