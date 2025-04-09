use dotenv::dotenv;
use sendgrid::error::SendgridError;
use sendgrid::v3::*;
use std::env;

pub async fn send_email(
    from: Email,
    to: Email,
    subject: &str,
    text: &str,
    reply_to: Option<Email>,
    attachment: Option<Attachment>
) -> Result<(), SendgridError> {
    dotenv().ok();
    let sg_api_key = env::var("SENDGRID_API_KEY").expect("SENDGRID_API_KEY must be set");
    let p = Personalization::new(to);

    let mut m = Message::new(from)
        .set_subject(subject)
        .add_content(Content::new().set_content_type("text/html").set_value(text))
        .add_personalization(p);
    if let Some(reply_to) = reply_to {
        m = m.set_reply_to(reply_to);
    }
    if let Some(attachment) = attachment {
        m = m.add_attachment(attachment);
    }

    let sender = Sender::new(sg_api_key, None);
    let resp = sender.send(&m).await?;
    println!("status: {}", resp.status());

    Ok(())
}

pub fn make_email_obj(addr: &str, name: Option<&str>) -> Email {
    let mut email = Email::new(addr);
    if let Some(name) = name {
        email = email.set_name(name);
    }
    email
}
