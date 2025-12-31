use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
};

use crate::{config::Config, error::AppError, models::S3Object};

pub async fn create_s3_client(config: &Config) -> Client {
    let creds = Credentials::new(
        &config.s3_access_key_id,
        &config.s3_secret_access_key,
        None,
        None,
        "custom",
    );

    let s3_config = aws_sdk_s3::config::Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(config.s3_region.clone()))
        .endpoint_url(&config.s3_endpoint)
        .credentials_provider(creds)
        .force_path_style(true)
        .build();

    Client::from_conf(s3_config)
}

pub async fn list_objects(
    client: &Client,
    bucket: &str,
    prefix: &str,
    username: &str,
) -> Result<(Vec<S3Object>, Vec<S3Object>), AppError> {
    let result = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(prefix)
        .delimiter("/")
        .send()
        .await
        .map_err(|e| AppError::S3Generic(e.to_string()))?;

    let prefix_len = username.len() + 1;

    // Process files
    let files: Vec<S3Object> = result
        .contents()
        .iter()
        .filter(|obj| obj.key().map(|k| k != prefix).unwrap_or(false))
        .map(|obj| {
            let key = obj.key().unwrap_or("");
            let full_path = key.get(prefix_len..).unwrap_or("");
            let name = key.get(prefix.len()..).unwrap_or("");

            S3Object {
                key: key.to_string(),
                name: name.to_string(),
                full_path: full_path.to_string(),
                size: Some(obj.size().unwrap_or(0)),
                last_modified: obj
                    .last_modified()
                    .and_then(|dt| chrono::DateTime::parse_from_rfc3339(dt.as_ref()).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                is_directory: false,
            }
        })
        .collect();

    // Process directories
    let directories: Vec<S3Object> = result
        .common_prefixes()
        .iter()
        .map(|prefix_obj| {
            let key = prefix_obj.prefix().unwrap_or("");
            let full_path = key
                .get(prefix_len..key.len().saturating_sub(1))
                .unwrap_or("");
            let name = key
                .get(prefix.len()..key.len().saturating_sub(1))
                .unwrap_or("");

            S3Object {
                key: key.to_string(),
                name: name.to_string(),
                full_path: full_path.to_string(),
                size: None,
                last_modified: None,
                is_directory: true,
            }
        })
        .collect();

    Ok((files, directories))
}

pub async fn delete_object(client: &Client, bucket: &str, key: &str) -> Result<(), AppError> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppError::S3Generic(e.to_string()))?;

    Ok(())
}

pub async fn delete_directory_recursive(
    client: &Client,
    bucket: &str,
    prefix: &str,
) -> Result<(), AppError> {
    let mut continuation_token: Option<String> = None;

    loop {
        let mut request = client.list_objects_v2().bucket(bucket).prefix(prefix);

        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }

        let result = request
            .send()
            .await
            .map_err(|e| AppError::S3Generic(e.to_string()))?;

        if let Some(contents) = result.contents() {
            if !contents.is_empty() {
                let objects: Vec<_> = contents
                    .iter()
                    .filter_map(|obj| {
                        obj.key().map(|key| {
                            aws_sdk_s3::types::ObjectIdentifier::builder()
                                .key(key)
                                .build()
                                .ok()
                        })
                    })
                    .flatten()
                    .collect();

                if !objects.is_empty() {
                    client
                        .delete_objects()
                        .bucket(bucket)
                        .delete(
                            aws_sdk_s3::types::Delete::builder()
                                .set_objects(Some(objects))
                                .build()
                                .map_err(|e| AppError::S3Generic(e.to_string()))?,
                        )
                        .send()
                        .await
                        .map_err(|e| AppError::S3Generic(e.to_string()))?;
                }
            }
        }

        if !result.is_truncated().unwrap_or(false) {
            break;
        }

        continuation_token = result.next_continuation_token().map(|s| s.to_string());
    }

    Ok(())
}

pub async fn get_object(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let result = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppError::S3Generic(e.to_string()))?;

    let content_type = result
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let body = result
        .body
        .collect()
        .await
        .map_err(|e| AppError::S3Generic(e.to_string()))?
        .into_bytes()
        .to_vec();

    Ok((body, content_type))
}

pub async fn put_object(
    client: &Client,
    bucket: &str,
    key: &str,
    body: Vec<u8>,
) -> Result<(), AppError> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .send()
        .await
        .map_err(|e| AppError::S3Generic(e.to_string()))?;

    Ok(())
}
