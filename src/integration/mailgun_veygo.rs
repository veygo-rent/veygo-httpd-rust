use reqwest::{multipart, Client};
use std::env;
use tokio::sync::OnceCell;

#[derive(Debug)]
#[allow(dead_code)]
pub enum MailgunError {
    MissingEnvVar ( env::VarError ),
    Http ( reqwest::Error ),
    Api { status: reqwest::StatusCode, body: String },
}

impl From<env::VarError> for MailgunError {
    fn from(err: env::VarError) -> Self {
        Self::MissingEnvVar(err)
    }
}

impl From<reqwest::Error> for MailgunError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err)
    }
}

#[derive(Clone, Debug)]
pub struct MailgunClient {
    client: Client,
    api_key: String,
    domain: String,
}

#[derive(Clone, Debug)]
pub struct EmailAddress {
    pub address: String,
    pub name: Option<String>,
}

impl EmailAddress {
    #[allow(dead_code)]
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            name: None,
        }
    }

    pub fn name_address(name: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            name: Some(name.into()),
        }
    }

    fn as_rfc822(&self) -> String {
        match &self.name {
            Some(name) if !name.is_empty() => format!("{} <{}>", name, self.address),
            _ => self.address.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Attachment {
    pub filename: String,
    pub content_type: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct MailgunResponse {
    pub id: String,
    pub message: String,
}

static MAILGUN_CLIENT: OnceCell<MailgunClient> = OnceCell::const_new();

async fn mailgun_client() -> &'static MailgunClient {
    MAILGUN_CLIENT
        .get_or_init(|| async {
            let api_key = env::var("MAILGUN_SENDING_KEY").expect("MAILGUN_SENDING_KEY must be set");
            let domain = String::from("mailer.veygo.rent");

            MailgunClient {
                client: Client::new(),
                api_key,
                domain,
            }
        })
        .await
}

pub async fn send_email(
    from_name: Option<&str>,
    to: Vec<EmailAddress>,
    subject: &str,
    html: &str,
    attachments: Option<Vec<Attachment>>,
) -> Result<MailgunResponse, MailgunError> {
    let client = mailgun_client().await;

    let from = EmailAddress::name_address(
        from_name.unwrap_or("Team Veygo"),
        "no-reply@mailer.veygo.rent",
    );

    let url = format!("https://api.mailgun.net/v3/{}/messages", client.domain);

    let mut form = multipart::Form::new()
        .text("from", from.as_rfc822())
        .text(
            "to",
            to.into_iter()
                .map(|email| email.as_rfc822())
                .collect::<Vec<_>>()
                .join(", "),
        )
        .text("subject", subject.to_string())
        .text("html", html.to_string());

    if let Some(attachments) = attachments {
        for attachment in attachments {
            let mut part = multipart::Part::bytes(attachment.bytes)
                .file_name(attachment.filename);

            if let Some(content_type) = attachment.content_type {
                part = part.mime_str(&content_type)?;
            }

            form = form.part("attachment", part);
        }
    }

    let response = client
        .client
        .post(url)
        .basic_auth("api", Some(&client.api_key))
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        return Err(MailgunError::Api { status, body });
    }

    let parsed = serde_json::from_str::<MailgunResponseBody>(&body)
        .map_err(|err| MailgunError::Api {
            status,
            body: format!("failed to parse Mailgun response: {err}; raw body: {body}"),
        })?;

    Ok(MailgunResponse {
        id: parsed.id,
        message: parsed.message,
    })
}

#[derive(serde::Deserialize)]
struct MailgunResponseBody {
    id: String,
    message: String,
}

pub fn make_email_obj(addr: &str, name: &str) -> EmailAddress {
    EmailAddress::name_address(name, addr)
}