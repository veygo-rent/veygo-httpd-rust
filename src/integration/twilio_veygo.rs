use std::env;
use dotenv::dotenv;
use twilio::{Message, TwilioError, Client, OutboundMessage};

pub async fn send_text(
    to: &str,
    msg: &str,
) -> Result<Message, TwilioError> {
    dotenv().ok();
    let tw_acc_sid = env::var("TWILIO_SID").expect("TWILIO_SID must be set");
    let tw_token = env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN must be set");
    let client = Client::new(&*tw_acc_sid, &*tw_token);
    client.send_message(OutboundMessage::new("+18334683946", to, msg)).await
}