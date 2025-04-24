use dotenv::dotenv;
use std::env;
use twilio_openapi::apis::Error;
use twilio_openapi::apis::configuration::Configuration;
use twilio_openapi::apis::default_api::{CreateMessageError, create_message};
use twilio_openapi::models::ApiPeriodV2010PeriodAccountPeriodMessage;

pub async fn send_text(
    to: &str,
    msg: &str,
) -> Result<ApiPeriodV2010PeriodAccountPeriodMessage, Error<CreateMessageError>> {
    dotenv().ok();
    let tw_acc_sid = env::var("TWILIO_SID").expect("TWILIO_SID must be set");
    let tw_token = env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN must be set");
    let twilio_config = Configuration {
        basic_auth: Some((tw_acc_sid.clone(), Some(tw_token))),
        ..Default::default()
    };
    let message = create_message(
        &twilio_config,
        &tw_acc_sid,
        to,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Option::from("+18334683946"),
        None,
        Option::from(msg),
        None,
    )
    .await;
    message
}
