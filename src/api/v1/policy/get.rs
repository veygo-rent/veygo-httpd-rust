use std::collections::HashMap;
use diesel::prelude::*;
use chrono::prelude::*;
use diesel::{ExpressionMethods, RunQueryDsl};
use diesel::result::Error;
use crate::{POOL, model, schema, methods};
use warp::http::{StatusCode, Method};
use warp::reply::with_status;
use warp::{Filter, Reply};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path::end()
        .and(warp::method())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(async move |method: Method, request: HashMap<String, String>| {
            if method != Method::GET {
                return methods::standard_replies::method_not_allowed_response();
            }

            let policy_type: model::PolicyType = {
                let raw_policy_str = request.get("policy");
                let Some(raw_policy_str) = raw_policy_str else {
                    return methods::standard_replies::bad_request("unknown policy type");
                };
                let raw_policy_str = raw_policy_str.clone();
                if raw_policy_str.eq(&String::from("Rental")) || raw_policy_str.eq(&String::from("rental")) {
                    model::PolicyType::Rental
                } else if raw_policy_str.eq(&String::from("Privacy")) || raw_policy_str.eq(&String::from("privacy")) {
                    model::PolicyType::Privacy
                } else if raw_policy_str.eq(&String::from("Membership")) || raw_policy_str.eq(&String::from("membership")) {
                    model::PolicyType::Membership
                } else {
                    return methods::standard_replies::bad_request("unknown policy type");
                }
            };

            let effective_date: NaiveDate = {
                let raw_date_str = request.get("date");
                if let Some(date_str) = raw_date_str {
                    let date_str = date_str.clone();
                    let try_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d");
                    if let Ok(date_input) = try_date {
                        date_input
                    } else {
                        return methods::standard_replies::bad_request("invalid date");
                    }
                } else {
                    let today = Utc::now();
                    today.date_naive()
                }
            };

            let mut pool = POOL.get().unwrap();
            use schema::policies::dsl as policy_query;
            let result = policy_query::policies
                    .filter(policy_query::policy_type.eq(&policy_type))
                    .filter(policy_query::policy_effective_date.le(effective_date))
                    .order(policy_query::policy_effective_date.desc())
                    .first::<model::Policy>(&mut pool);
            match result {
                Ok(policy) => {
                    methods::standard_replies::response_with_obj(policy, StatusCode::OK)
                },
                Err(e) => {
                    match e {
                        Error::NotFound => {
                            Ok::<_, warp::Rejection>((with_status(warp::reply::json(&"No policy found"), StatusCode::NOT_FOUND).into_response(),))
                        }
                        _ => {
                            methods::standard_replies::internal_server_error_response(String::from("policy/get: Failed to retrieve policy"))
                        }
                    }
                }
            }
        })
}
