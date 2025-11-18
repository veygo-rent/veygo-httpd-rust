use crate::schema::renters::dsl::renters;
use crate::{POOL, methods, model};
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Reply};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UpdateApnsBody {
    apns: String,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("update-apns")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(
            async move |body: UpdateApnsBody, auth: String, user_agent: String| {
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
                            // gen new token
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
                            let mut usr_in_question =
                                methods::user::get_user_by_id(&access_token.user_id)
                                    .await
                                    .unwrap();
                            usr_in_question.apple_apns = Option::from(body.apns.clone());
                            let renter_updated: model::PublishRenter =
                                diesel::update(renters.find(&access_token.user_id))
                                    .set(&usr_in_question)
                                    .get_result::<model::Renter>(&mut pool)
                                    .unwrap()
                                    .into();
                            return methods::standard_replies::renter_wrapped(
                                new_token_in_db_publish,
                                &renter_updated,
                            );
                        }
                    }
                };
            },
        )
}
