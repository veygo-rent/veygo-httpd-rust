use a2::{
    Client, DefaultNotificationBuilder, Endpoint, NotificationBuilder, NotificationOptions,
    client::ClientConfig,
};
use std::fs::File;

pub async fn send_notification(
    sandbox: bool,
    device_token: String,
    title: String,
    message: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key_file: String;
    let team_id = String::from("F84843HABV");
    let key_id: String;
    if sandbox {
        key_file = String::from("/app/cert/apple/Sandbox_595B69789J.p8");
        key_id = String::from("595B69789J");
    } else {
        key_file = String::from("/app/cert/apple/Production_3C3L4DRJYN.p8");
        key_id = String::from("3C3L4DRJYN");
    }
    let topic: Option<String> = String::from("com.veygo-rent.veygo-apartment-swift").into();

    // Read the private key from the disk
    let private_key = File::open(key_file).unwrap();

    // Which service to call, test or production?
    let endpoint = if sandbox {
        Endpoint::Sandbox
    } else {
        Endpoint::Production
    };

    // Create config with the given endpoint and default timeouts
    let client_config = ClientConfig::new(endpoint);

    // Connecting to APNs
    let client = Client::token(private_key, key_id, team_id, client_config).unwrap();

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
    let response = client.send(payload).await?;

    println!("Sent: {:?}", response);

    Ok(())
}
