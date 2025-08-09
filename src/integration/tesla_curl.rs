use std::env;
use std::sync::RwLock;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
const DEFAULT_AUDIENCE: &str = "https://fleet-api.prd.na.vn.cloud.tesla.com";
#[allow(dead_code)]
const DEFAULT_AUTH_URL: &str = "https://fleet-auth.prd.vn.cloud.tesla.com/oauth2/v3/token";
#[allow(dead_code)]
const BASE_URL: &str = "https://tesla-commend.veygo.rent";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeslaToken {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: String,
    #[serde(skip)]
    #[allow(dead_code)]
    pub obtained_at_unix: i64,
}

impl TeslaToken {
    #[allow(dead_code)]
    pub fn new(mut token: TeslaToken) -> TeslaToken {
        token.obtained_at_unix = Utc::now().timestamp();
        token
    }

    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        // Add a small safety margin of 60 seconds
        let expires_at = self.obtained_at_unix + self.expires_in - 60;
        Utc::now().timestamp() >= expires_at
    }

    pub fn bearer(&self) -> String {
        format!("{} {}", self.token_type, self.access_token)
    }
}

// Thread-safe, process-wide storage for the latest Tesla token
pub static TESLA_TOKEN: Lazy<RwLock<Option<TeslaToken>>> = Lazy::new(|| RwLock::new(None));

/// Set the current token from a parsed TeslaToken (will stamp obtained_at)
#[allow(dead_code)]
pub fn set_tesla_token(token: TeslaToken) {
    let token = TeslaToken::new(token);
    if let Ok(mut guard) = TESLA_TOKEN.write() {
        *guard = Some(token);
    }
}

/// Parse and set the token from a JSON string that looks like Tesla's response
#[allow(dead_code)]
pub fn set_tesla_token_from_json(json: &str) -> Result<()> {
    let token: TeslaToken = serde_json::from_str(json)?;
    set_tesla_token(token);
    Ok(())
}

/// Get a copy of the current token (if any)
#[allow(dead_code)]
pub fn get_tesla_token() -> Option<TeslaToken> {
    TESLA_TOKEN.read().ok().and_then(|g| g.as_ref().cloned())
}

/// Get a bearer string if token exists and is not expired
#[allow(dead_code)]
pub fn get_valid_bearer() -> Option<String> {
    get_tesla_token().and_then(|t| {
        if !t.is_expired() {
            Some(t.bearer())
        } else {
            None
        }
    })
}

#[allow(dead_code)]
fn env_or_err(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("Environment variable {} is not set", key))
}

#[allow(dead_code)]
fn env_or_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Fetch a new Tesla OAuth token using client credentials and store it globally
#[allow(dead_code)]
pub async fn fetch_and_store_tesla_token() -> Result<TeslaToken> {
    let client_id = env_or_err("TESLA_CLIENT_ID")?;
    let client_secret = env_or_err("TESLA_CLIENT_SECRET")?;

    let form = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        (
            "scope",
            "vehicle_device_data vehicle_cmds vehicle_charging_cmds",
        ),
        ("audience", DEFAULT_AUDIENCE),
    ];

    let client = reqwest::Client::new();
    let resp = client
        .post(DEFAULT_AUTH_URL)
        .form(&form)
        .send()
        .await
        .with_context(|| "Failed to send token request to Tesla AUTH_URL")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Tesla auth failed: status={} body={}",
            status,
            body
        ));
    }

    let token: TeslaToken = resp
        .json()
        .await
        .with_context(|| "Failed to parse Tesla auth response JSON")?;

    set_tesla_token(token.clone());
    Ok(token)
}

#[allow(dead_code)]
pub async fn tesla_make_request(
    method: reqwest::Method,
    path: &str,
    body: Option<String>,
) -> Result<reqwest::Response> {
    let client = reqwest::Client::new();

    let url = format!("{}{}", BASE_URL, path);

    let mut req = client.request(method, &url);

    // Ensure we have a valid token; fetch if missing/expired
    let bearer = match get_valid_bearer() {
        Some(b) => b,
        None => {
            fetch_and_store_tesla_token().await?;
            get_valid_bearer().ok_or_else(|| anyhow!("Failed to obtain Tesla access token"))?
        }
    };

    req = req.header(reqwest::header::AUTHORIZATION, bearer);

    if let Some(b) = body {
        req = req.header(reqwest::header::CONTENT_TYPE, "application/json");
        req = req.body(b);
    }

    let resp = req.send().await?;
    Ok(resp)
}
