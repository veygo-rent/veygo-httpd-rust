use diesel::prelude::*;
use diesel::dsl::today;
use chrono::NaiveDate;
use diesel::{ExpressionMethods, RunQueryDsl};
use crate::{POOL, model, schema};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Reply, hyper::Method};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
struct GetPolicyData {
    policy_type: model::PolicyType,
    effective_date: Option<NaiveDate>,
}

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[Method::POST, Method::OPTIONS])
        .build();
    warp::path("get")
        .and(warp::path::end())
        .and(
            warp::post()
                .and(warp::body::json())
                .and_then(async move |get_policy_data: GetPolicyData| {
                    let mut pool = POOL.get().unwrap();
                    use schema::policies::dsl as policy_query;
                    let result =
                        if let Some(effective_date) = get_policy_data.effective_date {
                            policy_query::policies
                                .filter(policy_query::policy_type.eq(&get_policy_data.policy_type))
                                .filter(policy_query::policy_effective_date.le(effective_date))
                                .order(policy_query::policy_effective_date.desc())
                                .first::<model::Policy>(&mut pool)
                        } else {
                            policy_query::policies
                                .filter(policy_query::policy_type.eq(&get_policy_data.policy_type))
                                .filter(policy_query::policy_effective_date.le(today))
                                .order(policy_query::policy_effective_date.desc())
                                .first::<model::Policy>(&mut pool)
                        };
                    match result {
                        Ok(policy) => {
                            Ok::<_, warp::Rejection>((with_status(warp::reply::json(&policy), StatusCode::OK),))
                        },
                        Err(_) => {
                            Ok::<_, warp::Rejection>((with_status(warp::reply::json(&"No policy found"), StatusCode::NOT_FOUND),))
                        }
                    }
                })
                .or(warp::options()
                    .map(warp::reply).with(&cors)
                )
        )
}
