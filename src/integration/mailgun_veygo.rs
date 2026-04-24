use std::env;
use tokio::sync::OnceCell;
use mailgun_rs::{Mailgun, Message, EmailAddress, SendResult, SendResponse, MailgunRegion, Attachment};

static MAILGUN_CLIENT: OnceCell<Mailgun> = OnceCell::const_new();

async fn mailgun_client() -> &'static Mailgun {
    MAILGUN_CLIENT
        .get_or_init(|| async {
            let mg_api_key =
                env::var("MAILGUN_SENDING_KEY").expect("MAILGUN_SENDING_KEY must be set");

            Mailgun {
                api_key: mg_api_key,
                domain: "mail.veygo.rent".to_string(),
            }
        })
        .await
}

pub async fn send_email(
    from_name: Option<&str>,
    to: Vec<EmailAddress>,
    subject: &str,
    html: &str,
    attachments: Option<Vec<Attachment>>
) -> SendResult<SendResponse> {
    let client = mailgun_client().await;

    let from = EmailAddress::name_address(
        from_name.unwrap_or("Team Veygo"), "postmaster@mailer.veygo.rent"
    );

    let msg = Message {
        to,
        subject: String::from(subject),
        html: String::from(html),
        ..Default::default()
    };

    client.async_send(MailgunRegion::US, &from, msg, attachments).await
}

pub fn make_email_obj(addr: &str, name: &str) -> EmailAddress {
    EmailAddress::name_address(name, addr)
}
