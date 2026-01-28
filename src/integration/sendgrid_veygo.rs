use reqwest::Response;
use sendgrid::error::SendgridError;
use sendgrid::v3::*;
use std::env;
use tokio::sync::OnceCell;

static SENDGRID_CLIENT: OnceCell<Sender> = OnceCell::const_new();

async fn sendgrid_client() -> &'static Sender<'static> {
    SENDGRID_CLIENT
        .get_or_init(|| async {
            let sg_api_key = env::var("SENDGRID_API_KEY")
                .expect("SENDGRID_API_KEY must be set");
            Sender::new(&sg_api_key, None)
        })
        .await
}

pub async fn send_email<'a>(
    from_name: Option<&str>,
    to: Email<'a>,
    subject: &str,
    text: &str,
    reply_to: Option<Email<'a>>,
    attachment: Option<Attachment<'a>>,
) -> Result<Response, SendgridError> {
    let client = sendgrid_client().await;
    
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

    let resp = client.send(&m).await;
    match resp {
        Ok(resp) => {Ok(resp)}
        Err(err) => {Err(err)}
    }
}

pub fn make_email_obj<'a>(addr: &'a str, name: &'a str) -> Email<'a> {
    let mut email = Email::new(addr);
    email = email.set_name(name);
    email
}
