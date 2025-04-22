use gcloud_storage::client::google_cloud_auth::credentials::CredentialsFile;
use gcloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use gcloud_storage::http::objects::delete::DeleteObjectRequest;
use gcloud_storage::sign;
use gcloud_storage::sign::SignedURLOptions;
use std::borrow::Cow;
use std::path::Path;
use uuid;

pub async fn get_signed_url(object_path: &str) -> String {
    use gcloud_storage::client::{Client, ClientConfig};
    let config = ClientConfig::default()
        .with_credentials(
            CredentialsFile::new_from_file(String::from(
                "/app/cert/gcloud/veygo-server-8d64193d983c.json",
            ))
            .await
            .unwrap(),
        )
        .await
        .unwrap();
    let client = Client::new(config);
    let google_access_id = "veygo-server@veygo-server.iam.gserviceaccount.com".to_string();
    let url = client
        .signed_url(
            "veygo-store",
            object_path,
            Some(google_access_id),
            Some(sign::SignBy::SignBytes),
            SignedURLOptions::default(),
        )
        .await
        .unwrap();
    url
}

pub async fn upload_file(object_path: String, file_name: String, data_clone: Vec<u8>) -> String {
    let path = Path::new(&file_name);
    let ext = path.extension().unwrap_or("".as_ref()).to_str().unwrap_or("").to_uppercase();
    let content_type = match ext.as_str() {
        "PDF" => Some("application/pdf"),
        "JPG" => Some("image/jpeg"),
        "JPEG" => Some("image/jpeg"),
        "PNG" => Some("image/png"),
        "CSV" => Some("text/csv"),
        _ => None,
    }.unwrap();
    let u = uuid::Uuid::new_v4().to_string().to_uppercase();
    let file_name_with_uuid = u + "." + ext.as_str();
    use gcloud_storage::client::{Client, ClientConfig};
    let config = ClientConfig::default()
        .with_credentials(
            CredentialsFile::new_from_file(String::from(
                "/app/cert/gcloud/veygo-server-8d64193d983c.json",
            ))
            .await
            .unwrap(),
        )
        .await
        .unwrap();
    let client = Client::new(config);
    let stored_file_abs_path = format!("{}{}", object_path, file_name_with_uuid);
    let upload_type = UploadType::Simple(Media {
        name: Cow::from(stored_file_abs_path.clone()),
        content_type: Cow::from(content_type),
        content_length: None,
    });
    let _ = client
        .upload_object(
            &UploadObjectRequest {
                bucket: "veygo-store".to_string(),
                ..Default::default()
            },
            data_clone,
            &upload_type,
        )
        .await;
    stored_file_abs_path
}

pub async fn delete_object(stored_file_abs_path: String) {
    use gcloud_storage::client::{Client, ClientConfig};
    let config = ClientConfig::default()
        .with_credentials(
            CredentialsFile::new_from_file(String::from(
                "/app/cert/gcloud/veygo-server-8d64193d983c.json",
            ))
                .await
                .unwrap(),
        )
        .await
        .unwrap();
    let client = Client::new(config);
    let _ = client.delete_object(&DeleteObjectRequest {
        bucket: "veygo-store".to_string(),
        object: stored_file_abs_path,
        ..Default::default()
    }).await;
}
