use warp::{Filter, Reply};
use bytes;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use http::StatusCode;
use stripe_webhook::{Webhook, EventObject};
use warp::reply::with_status;
use crate::{methods, POOL, model, integration};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("stripe")
        .and(warp::path::end())
        .and(warp::header::<String>("stripe-signature"))
        .and(warp::body::bytes())
        .and_then(
            |stripe_signature: String, body: bytes::Bytes| async move {
                let secret = std::env::var("STRIPE_WEBHOOK_SECRET").expect("No webhook signing secret found");

                let sig = stripe_signature;
                let payload = std::str::from_utf8(&body).unwrap();

                let event = Webhook::construct_event(payload, &sig, &secret);

                match event {
                    Ok(event) => {
                        let obj = event.clone().data.object;
                        match obj {
                            EventObject::PaymentIntentCanceled(pmi) => {
                                let payment_intent = pmi.clone();
                                tokio::spawn(async move {
                                    let mut pool = POOL.get().unwrap();

                                    use crate::schema::payments::dsl as p_q;
                                    let agreement_id_result = p_q::payments
                                        .filter(p_q::reference_number.eq(&payment_intent.id.as_str()))
                                        .select(p_q::agreement_id)
                                        .get_result::<i32>(&mut pool);

                                    match agreement_id_result {
                                        Ok(ag_id) => {
                                            use crate::schema::agreements::dsl as a_q;

                                            let ag_that_payment_being_canceled = a_q::agreements
                                                .find(ag_id)
                                                .get_result::<model::Agreement>(&mut pool)
                                                .unwrap();

                                            match ag_that_payment_being_canceled.status {
                                                model::AgreementStatus::Rental => {}
                                                _ => {
                                                    // TODO: Do nothing
                                                }
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                });
                            }
                            _ => {}
                        }
                        let empty_msg = serde_json::json!({});
                        Ok::<_, warp::Rejection>((with_status(warp::reply::json(&empty_msg), StatusCode::OK).into_response(),))
                    }
                    Err(_err) => {
                        methods::standard_replies::internal_server_error_response("webhook: Unauthenticated webhook request detected").await
                    }
                }
            }
        )
}