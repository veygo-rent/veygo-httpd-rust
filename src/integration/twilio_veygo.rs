use std::env;
use twilio::{Call, Client, Message, OutboundCall, OutboundMessage, TwilioError};

#[allow(dead_code)]
pub async fn send_text(to: &str, msg: &str) -> Result<Message, TwilioError> {
    let tw_acc_sid = env::var("TWILIO_SID").expect("TWILIO_SID must be set");
    let tw_token = env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN must be set");
    let client = Client::new(&*tw_acc_sid, &*tw_token);
    let result = client
        .send_message(OutboundMessage::new("+18334683946", to, msg))
        .await;
    result
}

#[allow(dead_code)]
pub async fn call_otp(to: &str, otp: &str) -> Result<Call, TwilioError> {
    let tw_acc_sid = env::var("TWILIO_SID").expect("TWILIO_SID must be set");
    let tw_token = env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN must be set");
    let client = Client::new(&*tw_acc_sid, &*tw_token);
    let arr = otp.chars().collect::<Vec<char>>();
    let url = format!(
        "https://handler.twilio.com/twiml/EHa070238c9880235bc03d060b9f915e1d?first_digit={}&second_digit={}&third_digit={}&fourth_digit={}&fifth_digit={}&sixth_digit={}",
        arr[0], arr[1], arr[2], arr[3], arr[4], arr[5]
    );
    let call = OutboundCall::new("+18334683946", to, &*url);
    client.make_call(call).await
}
