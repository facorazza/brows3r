use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: Option<String>,
    pub email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    pub fn verify_password(&self, password: &str) -> bool {
        if let Some(hash) = &self.password_hash {
            use argon2::{Argon2, PasswordHash, PasswordVerifier};

            if let Ok(parsed_hash) = PasswordHash::new(hash) {
                return Argon2::default()
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .is_ok();
            }
        }
        false
    }

    pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(password_hash.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Object {
    pub key: String,
    pub name: String,
    pub full_path: String,
    pub size: Option<i64>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub is_directory: bool,
}
