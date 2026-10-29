use a2::{
    Client, DefaultNotificationBuilder, Endpoint, NotificationBuilder, NotificationOptions,
    client::ClientConfig,
};
use std::fs::File;

pub async fn send_notification(
    device_token: &str,
    title: &str,
    message: &str,
    is_admin_app: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (sandbox, device_token) = if let Some(stripped) = device_token.strip_prefix('!') {
        (true, stripped)
    } else {
        (false, device_token)
    };
    let team_id = String::from("F84843HABV");
    let key_id: String;
    let key_file: String;
    let endpoint: Endpoint;
    if sandbox {
        key_id = String::from("595B69789J");
        key_file = String::from("/app/cert/apple/Sandbox_595B69789J.p8");
        endpoint = Endpoint::Sandbox;
    } else {
        key_id = String::from("3C3L4DRJYN");
        key_file = String::from("/app/cert/apple/Production_3C3L4DRJYN.p8");
        endpoint = Endpoint::Production;
    }
    let topic: Option<String>;
    if is_admin_app {
        topic = String::from("com.veygo-rent.veygo-apartment-admin-swift").into();
    } else {
        topic = String::from("com.veygo-rent.veygo-apartment-swift").into();
    }

    // Read the private key from the disk
    let private_key = File::open(key_file)?;

    // Create config with the given endpoint and default timeouts
    let client_config = ClientConfig::new(endpoint);

    // Connecting to APNs
    let client = Client::token(private_key, key_id, team_id, client_config)?;

    let options = NotificationOptions {
        apns_topic: topic.as_deref(),
        ..Default::default()
    };

    // Notification payload
    let builder = DefaultNotificationBuilder::new()
        .set_title(title.as_ref())
        .set_body(message.as_ref())
        .set_sound("default");

    let payload = builder.build(device_token.as_ref(), options);
    let _ = client.send(payload).await?;

    Ok(())
}
