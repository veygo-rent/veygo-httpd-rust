use crate::{POOL, methods, model, helper_model};
use diesel::prelude::*;
use futures::TryFutureExt;
use warp::{Filter, Rejection, Reply};
use warp::http::{Method, StatusCode};
use warp::reply::with_status;

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("check-in")
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::body::json())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |method: Method, body: helper_model::CheckOutRequest, auth: String, user_agent: String| {

            // Checking method is POST
            if method != Method::POST {
                return methods::standard_replies::method_not_allowed_response();
            }

            if body.agreement_id <= 0 || body.hours_using_reward < 0 || body.vehicle_snapshot_id <= 0 {
                return methods::standard_replies::bad_request("Bad request: wrong parameters. ")
            }

            // Pool connection
            let mut pool = POOL.get().unwrap();

            use crate::schema::agreements::dsl as agreement_q;
            use crate::schema::vehicle_snapshots::dsl as v_s_q;
            let ag_v_s_result = v_s_q::vehicle_snapshots
                .inner_join(agreement_q::agreements.on(v_s_q::vehicle_id.eq(agreement_q::vehicle_id)))
                .filter(v_s_q::id.eq(&body.vehicle_snapshot_id))
                .filter(agreement_q::id.eq(&body.agreement_id))
                .filter(v_s_q::time.ge(agreement_q::rsvp_pickup_time))
                .filter(v_s_q::time.lt(agreement_q::rsvp_drop_off_time))
                .select((agreement_q::agreements::all_columns(), v_s_q::vehicle_snapshots::all_columns()))
                .get_result::<(model::Agreement, model::VehicleSnapshot)>(&mut pool);
            methods::standard_replies::not_implemented_response()
        })
}