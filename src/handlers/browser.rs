use askama::Template;
use axum::{
    Form,
    body::Body,
    extract::{Extension, Multipart, Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::{error::AppError, models::User, s3, state::AppState, templates::BrowserListTemplate};

// Maximum upload size: 100MB
const MAX_UPLOAD_SIZE: usize = 100 * 1024 * 1024;

// Validate directory/file name
fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && name != "."
        && name != ".."
}

// Reject path segments that could escape the user's S3 prefix
fn is_safe_path(path: &str) -> bool {
    path.split('/').all(|seg| seg != "." && seg != "..")
}

pub async fn list<DB: sqlx::Database>(
    State(state): State<AppState<DB>>,
    Extension(user): Extension<User>,
    path: Option<Path<String>>,
) -> Result<Html<String>, AppError> {
    let path_str = path.map(|p| p.0).unwrap_or_default();

    let prefix = if path_str.is_empty() {
        format!("{}/", user.username)
    } else {
        format!("{}/{}/", user.username, path_str)
    };

    let (files, directories) = s3::list_objects(
        &state.s3_client,
        &state.config.s3_bucket,
        &prefix,
        &user.username,
    )
    .await?;

    let template = BrowserListTemplate {
        path: path_str,
        files,
        directories,
    };

    let html = template
        .render()
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct CreateDirectoryForm {
    path: String,
    new_dir: String,
}

pub async fn create_directory<DB: sqlx::Database>(
    State(state): State<AppState<DB>>,
    Extension(user): Extension<User>,
    Form(form): Form<CreateDirectoryForm>,
) -> Result<Redirect, AppError> {
    // Validate directory name
    if !is_valid_name(&form.new_dir) {
        return Err(AppError::Internal(
            "Invalid directory name. Use only alphanumeric characters, hyphens, and underscores."
                .to_string(),
        ));
    }

    // Directory marker objects end with "/" so they are listed as directories
    let key = if form.path.is_empty() {
        format!("{}/{}/", user.username, form.new_dir)
    } else {
        format!("{}/{}/{}/", user.username, form.path, form.new_dir)
    };

    // Create empty object to represent directory
    s3::put_object(&state.s3_client, &state.config.s3_bucket, &key, vec![]).await?;

    let redirect_path = if form.path.is_empty() {
        format!("/{}", form.new_dir)
    } else {
        format!("/{}/{}", form.path, form.new_dir)
    };

    Ok(Redirect::to(&redirect_path))
}

pub async fn delete<DB: sqlx::Database>(
    State(state): State<AppState<DB>>,
    Extension(user): Extension<User>,
    Path(path): Path<String>,
) -> Result<Redirect, AppError> {
    if !is_safe_path(&path) {
        return Err(AppError::NotFound);
    }

    let full_path = format!("{}/{}", user.username, path);

    let is_directory = path.ends_with('/');

    if is_directory {
        s3::delete_directory_recursive(&state.s3_client, &state.config.s3_bucket, &full_path)
            .await?;
    } else {
        s3::delete_object(&state.s3_client, &state.config.s3_bucket, &full_path).await?;
    }

    // Navigate to parent directory
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let redirect_path = if parts.len() > 1 {
        format!("/{}", parts[..parts.len() - 1].join("/"))
    } else {
        "/".to_string()
    };
    Ok(Redirect::to(&redirect_path))
}

pub async fn download<DB: sqlx::Database>(
    State(state): State<AppState<DB>>,
    Extension(user): Extension<User>,
    Path(path): Path<String>,
) -> Result<Response, AppError> {
    if !is_safe_path(&path) {
        return Err(AppError::NotFound);
    }

    let full_path = format!("{}/{}", user.username, path);

    let (body, content_type) =
        s3::get_object(&state.s3_client, &state.config.s3_bucket, &full_path).await?;

    let bytes = body
        .collect()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .into_bytes();

    let filename = path.split('/').next_back().unwrap_or("download");
    let safe_filename: String = filename
        .chars()
        .map(|c| match c {
            '"' | '\r' | '\n' => '_',
            _ => c,
        })
        .collect();

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", safe_filename),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}

pub async fn upload<DB: sqlx::Database>(
    State(state): State<AppState<DB>>,
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<Redirect, AppError> {
    let mut path = String::new();
    let mut file_data: Option<(String, Vec<u8>)> = None;
    let mut total_size = 0usize;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "path" => {
                path = field
                    .text()
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            "file" => {
                let filename = field.file_name().unwrap_or("uploaded_file").to_string();

                // Validate filename
                if !is_valid_name(&filename) {
                    return Err(AppError::Internal(
                        "Invalid filename. Use only alphanumeric characters, hyphens, and underscores.".to_string(),
                    ));
                }

                // Stream the file in chunks, enforcing the size limit as we go
                let mut data = Vec::new();
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?
                {
                    total_size += chunk.len();
                    if total_size > MAX_UPLOAD_SIZE {
                        return Err(AppError::Internal(format!(
                            "File too large. Maximum size is {} MB",
                            MAX_UPLOAD_SIZE / 1024 / 1024
                        )));
                    }
                    data.extend_from_slice(&chunk);
                }

                file_data = Some((filename, data));
            }
            _ => {}
        }
    }

    if let Some((filename, data)) = file_data {
        let key = if path.is_empty() {
            format!("{}/{}", user.username, filename)
        } else {
            format!("{}/{}/{}", user.username, path, filename)
        };

        s3::put_object(&state.s3_client, &state.config.s3_bucket, &key, data).await?;
    }

    let redirect_path = if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path)
    };

    Ok(Redirect::to(&redirect_path))
}
