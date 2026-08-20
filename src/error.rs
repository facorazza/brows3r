use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::fmt;

use crate::templates::ErrorTemplate;

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    S3Generic(String),
    Authentication,
    NotFound,
    Internal(String),
    #[cfg(feature = "ldap")]
    Ldap(ldap3::LdapError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::S3Generic(e) => write!(f, "S3 error: {}", e),
            AppError::Authentication => write!(f, "Authentication failed"),
            AppError::NotFound => write!(f, "Resource not found"),
            AppError::Internal(e) => write!(f, "Internal error: {}", e),
            #[cfg(feature = "ldap")]
            AppError::Ldap(e) => write!(f, "LDAP error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, title, message) = match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database Error".to_string(),
                    "An error occurred while accessing the database.".to_string(),
                )
            }
            AppError::S3Generic(e) => {
                tracing::error!("S3 error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage Error".to_string(),
                    "An error occurred while accessing storage.".to_string(),
                )
            }
            AppError::Authentication => (
                StatusCode::UNAUTHORIZED,
                "Authentication Failed".to_string(),
                "Invalid credentials or session expired.".to_string(),
            ),
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "Not Found".to_string(),
                "The requested resource was not found.".to_string(),
            ),
            AppError::Internal(e) => {
                tracing::error!("Internal error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "An unexpected error occurred.".to_string(),
                )
            }
            #[cfg(feature = "ldap")]
            AppError::Ldap(e) => {
                tracing::error!("LDAP error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Authentication Error".to_string(),
                    "An error occurred during authentication.".to_string(),
                )
            }
        };

        let template = ErrorTemplate {
            status_code: status.as_u16(),
            title,
            message,
        };

        match template.render() {
            Ok(html) => (status, Html(html)).into_response(),
            Err(_) => (status, "An error occurred").into_response(),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

#[cfg(feature = "ldap")]
impl From<ldap3::LdapError> for AppError {
    fn from(e: ldap3::LdapError) -> Self {
        AppError::Ldap(e)
    }
}
