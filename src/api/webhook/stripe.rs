use warp::{Filter, Reply};
use bytes;
use http::StatusCode;
use stripe::EventObject;
use warp::reply::with_status;
use crate::{methods};

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

                let event = stripe::Webhook::construct_event(payload, &sig, &secret);

                match event {
                    Ok(event) => {
                        let obj = event.clone().data.object;
                        match obj {
                            EventObject::PaymentIntent(pmi) => {
                                println!("{:?}", pmi);
                            }
                            _ => {}
                        }
                        let event_msg = serde_json::json!({
                            "event": event,
                        });
                        Ok::<_, warp::Rejection>((with_status(warp::reply::json(&event_msg), StatusCode::OK).into_response(),))
                    }
                    Err(_err) => {
                        methods::standard_replies::internal_server_error_response_without_token()
                    }
                }
            }
        )
}