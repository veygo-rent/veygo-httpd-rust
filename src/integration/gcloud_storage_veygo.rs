use gcloud_storage::client::google_cloud_auth::credentials::CredentialsFile;
use gcloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use gcloud_storage::http::objects::delete::DeleteObjectRequest;
use gcloud_storage::http::objects::list::ListObjectsRequest;
use gcloud_storage::http::objects::get::GetObjectRequest;
use gcloud_storage::sign;
use gcloud_storage::sign::SignedURLOptions;
use std::borrow::Cow;
use std::path::Path;
use uuid;

use gcloud_storage::client::{Client, ClientConfig};
use std::sync::Arc;
use tokio::sync::OnceCell;

const BUCKET: &str = "veygo-store-progressive";
const CREDS_PATH: &str = "/app/cert/gcloud/veygo-server-8d64193d983c.json";
const GOOGLE_ACCESS_ID: &str = "veygo-server@veygo-server.iam.gserviceaccount.com";

static GCS_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

async fn gcs_client() -> Arc<Client> {
    GCS_CLIENT
        .get_or_init(|| async {
            let config = ClientConfig::default()
                .with_credentials(
                    CredentialsFile::new_from_file(CREDS_PATH.to_string())
                        .await
                        .expect("Failed to load GCS credentials"),
                )
                .await
                .expect("Failed to build GCS client config");
            Arc::new(Client::new(config))
        })
        .await
        .clone()
}

#[allow(dead_code)]
pub async fn get_signed_url(object_path: &str) -> String {
    let client = gcs_client().await;
    client
        .signed_url(
            BUCKET,
            object_path,
            Some(GOOGLE_ACCESS_ID.to_string()),
            Some(sign::SignBy::SignBytes),
            SignedURLOptions::default(),
        )
        .await
        .unwrap()
}

#[allow(dead_code)]
pub async fn upload_file(object_path: String, file_name: String, data_clone: Vec<u8>) -> String {
    let path = Path::new(&file_name);
    let ext = path.extension().unwrap_or("".as_ref()).to_str().unwrap_or("").to_uppercase();
    let content_type = match ext.as_str() {
        "PDF" => "application/pdf",
        "JPG" | "JPEG" => "image/jpeg",
        "PNG" => "image/png",
        "CSV" => "text/csv",
        "HEIC" => "image/heic",
        _ => "application/octet-stream",
    };
    let u = uuid::Uuid::new_v4().to_string().to_uppercase();
    let file_name_with_uuid = u + "." + ext.as_str();
    let client = gcs_client().await;
    let stored_file_abs_path = format!("{}{}", object_path, file_name_with_uuid);
    let upload_type = UploadType::Simple(Media {
        name: Cow::from(stored_file_abs_path.clone()),
        content_type: Cow::from(content_type),
        content_length: None,
    });
    let _ = client
        .upload_object(
            &UploadObjectRequest {
                bucket: BUCKET.to_string(),
                ..Default::default()
            },
            data_clone,
            &upload_type,
        )
        .await;
    file_name_with_uuid
}

#[allow(dead_code)]
pub async fn delete_object(stored_file_abs_path: String) {
    let client = gcs_client().await;
    let _ = client.delete_object(&DeleteObjectRequest {
        bucket: BUCKET.to_string(),
        object: stored_file_abs_path,
        ..Default::default()
    }).await;
}

#[allow(dead_code)]
pub async fn check_exists(stored_file_abs_path: String) -> bool {
    let client = gcs_client().await;
    let result = client.get_object(
        &GetObjectRequest {
            bucket: BUCKET.to_string(),
            object: stored_file_abs_path,
            ..Default::default()
        },
    ).await;
    match result {
        Ok(_) => true,
        Err(_) => false
    }
}

#[allow(dead_code)]
pub async fn delete_all_objects() -> Result<(), Box<dyn std::error::Error>> {
    let client = gcs_client().await;

    // List all objects in the bucket
    let list_req = ListObjectsRequest {
        bucket: BUCKET.to_string(),
        ..Default::default()
    };
    let objects = client.list_objects(&list_req).await?;

    // Delete each object
    if let Some(items) = objects.items {
        for obj in items {
            let name = obj.name;
            client
                .delete_object(&DeleteObjectRequest {
                    bucket: BUCKET.to_string(),
                    object: name,
                    ..Default::default()
                })
                .await?;
        }
    }

    Ok(())
}
