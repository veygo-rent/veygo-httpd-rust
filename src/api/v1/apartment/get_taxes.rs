use crate::{POOL, methods, model};
use diesel::RunQueryDsl;
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("get-taxes")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |auth: String, user_agent: String| {
            let token_and_id = auth.split("$").collect::<Vec<&str>>();
            if token_and_id.len() != 2 {
                return methods::tokens::token_invalid_wrapped_return(&auth);
            }
            let user_id;
            let user_id_parsed_result = token_and_id[1].parse::<i32>();
            user_id = match user_id_parsed_result {
                Ok(int) => int,
                Err(_) => {
                    return methods::tokens::token_invalid_wrapped_return(&auth);
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
                Err(_) => methods::tokens::token_not_hex_warp_return(&access_token.token),
                Ok(token_bool) => {
                    if !token_bool {
                        methods::tokens::token_invalid_wrapped_return(&access_token.token)
                    } else {
                        // Token is valid
                        let admin = methods::user::get_user_by_id(&access_token.user_id)
                            .await
                            .unwrap();
                        let token_clone = access_token.clone();
                        methods::tokens::rm_token_by_binary(
                            hex::decode(token_clone.token).unwrap(),
                        )
                        .await;
                        let new_token =
                            methods::tokens::gen_token_object(&access_token.user_id, &user_agent)
                                .await;
                        use crate::schema::access_tokens::dsl::*;
                        let mut pool = POOL.get().unwrap();
                        let new_token_in_db_publish = diesel::insert_into(access_tokens)
                            .values(&new_token)
                            .get_result::<model::AccessToken>(&mut pool)
                            .unwrap()
                            .to_publish_access_token();
                        if !methods::user::user_is_operational_admin(&admin) {
                            let token_clone = new_token_in_db_publish.clone();
                            return methods::standard_replies::user_not_admin_wrapped_return(
                                token_clone,
                            );
                        }
                        let publish_taxes: Vec<model::Tax>;
                        use crate::schema::taxes::dsl::*;
                        // get ALL taxes
                        publish_taxes = taxes.load::<model::Tax>(&mut pool).unwrap();
                        let msg = serde_json::json!({
                            "taxes": publish_taxes,
                        });
                        Ok::<_, warp::Rejection>((methods::tokens::wrap_json_reply_with_token(
                            new_token_in_db_publish,
                            with_status(warp::reply::json(&msg), StatusCode::OK),
                        ),))
                    }
                }
            };
        })
}
