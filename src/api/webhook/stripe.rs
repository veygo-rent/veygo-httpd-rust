use warp::{Filter, Reply};
use bytes;
use diesel::prelude::*;
use http::StatusCode;
use stripe_webhook::{Webhook, EventObject};
use warp::reply::with_status;
use crate::{methods, POOL, model};

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
                            EventObject::PaymentMethodAutomaticallyUpdated(pm) => {
                                let payment_method = pm.clone();
                                let pm_id = pm.id;
                                if let Some(payment_method_card) = payment_method.card {
                                    tokio::spawn(async move {
                                        let mut pool = POOL.get().unwrap();
                                        use crate::schema::payment_methods::dsl as pm_q;

                                        let said_payment = pm_q::payment_methods
                                            .filter(pm_q::token.eq(pm_id.as_str()))
                                            .get_result::<model::PaymentMethod>(&mut pool);

                                        if let Ok(mut said_payment) = said_payment {
                                            let mut masked_card_number = format!("**** **** **** {}", payment_method_card.last4);
                                            if payment_method_card.brand == "amex" {
                                                masked_card_number = format!("**** ****** *{}", payment_method_card.last4);
                                            }
                                            let expiration = format!("{:02}/{}", payment_method_card.exp_month, payment_method_card.exp_year);

                                            said_payment.masked_card_number = masked_card_number;
                                            said_payment.expiration = expiration;
                                            said_payment.fingerprint = payment_method_card.fingerprint.unwrap();

                                            let _ = said_payment.save_changes::<model::PaymentMethod>(&mut pool);
                                        };
                                    });
                                };
                            }
                            EventObject::PaymentMethodDetached(pm_d) => {
                                tokio::spawn(async move {
                                    let pmi_id = pm_d.id.to_string();
                                    let mut pool = POOL.get().unwrap();
                                    use crate::schema::payment_methods::dsl as pm_q;
                                    let _ = diesel::update
                                        (
                                            pm_q::payment_methods
                                                .filter(pm_q::token.eq(&pmi_id))
                                        )
                                        .set(pm_q::is_enabled.eq(false))
                                        .execute(&mut pool);
                                });
                            }
                            EventObject::SetupIntentSucceeded(set_i) => {
                                if let Some(pmi) = set_i.payment_method {
                                    let pmi_id = pmi.id().to_string();
                                    tokio::spawn(async move {
                                        let mut pool = POOL.get().unwrap();
                                        use crate::schema::payment_methods::dsl as pm_q;
                                        let _ = diesel::update
                                            (
                                                pm_q::payment_methods
                                                    .filter(pm_q::token.eq(&pmi_id))
                                            )
                                            .set(pm_q::is_enabled.eq(true))
                                            .execute(&mut pool);
                                    });
                                }
                            }
                            EventObject::SetupIntentSetupFailed(set_i) => {
                                if let Some(pmi) = set_i.payment_method {
                                    let pmi_id = pmi.id().to_string();
                                    tokio::spawn(async move {
                                        let mut pool = POOL.get().unwrap();
                                        use crate::schema::payment_methods::dsl as pm_q;
                                        let _ = diesel::delete
                                            (
                                                pm_q::payment_methods
                                                    .filter(pm_q::token.eq(&pmi_id))
                                            )
                                            .execute(&mut pool);
                                    });
                                }
                            }
                            _ => {}
                        }
                        let empty_msg = serde_json::json!({});
                        Ok::<_, warp::Rejection>((with_status(warp::reply::json(&empty_msg), StatusCode::OK).into_response(),))
                    }
                    Err(_err) => {
                        methods::standard_replies::internal_server_error_response(String::from("webhook: Unauthenticated webhook request detected"))
                    }
                }
            }
        )
}