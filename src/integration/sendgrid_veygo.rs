use dotenv::dotenv;
use sendgrid::error::SendgridError;
use sendgrid::v3::*;
use std::env;

pub async fn send_email(
    from_name: Option<&str>,
    to: Email,
    subject: &str,
    text: &str,
    reply_to: Option<Email>,
    attachment: Option<Attachment>
) -> Result<(), SendgridError> {
    dotenv().ok();
    let sg_api_key = env::var("SENDGRID_API_KEY").expect("SENDGRID_API_KEY must be set");
    let p = Personalization::new(to);

    let from = make_email_obj("no-reply@veygo.rent", from_name.unwrap_or("Team Veygo"));
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
    if !resp.status().is_success() {
        println!("status: {}", resp.status());
    }

    Ok(())
}

pub fn make_email_obj(addr: &str, name: &str) -> Email {
    let mut email = Email::new(addr);
    email = email.set_name(name);
    email
}
