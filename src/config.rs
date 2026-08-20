use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub secret_key: String,
    pub session_secure: bool,

    // S3 Configuration
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key_id: String,
    pub s3_secret_access_key: String,

    // LDAP Configuration (optional)
    #[cfg(feature = "ldap")]
    pub ldap_server_uri: Option<String>,
    #[cfg(feature = "ldap")]
    pub ldap_bind_dn: Option<String>,
    #[cfg(feature = "ldap")]
    pub ldap_bind_password: Option<String>,
    #[cfg(feature = "ldap")]
    pub ldap_user_dn_template: Option<String>,
    #[cfg(feature = "ldap")]
    pub ldap_group_search: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            secret_key: env::var("SECRET_KEY").expect("SECRET_KEY must be set"),
            session_secure: env::var("SESSION_SECURE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),

            s3_endpoint: env::var("S3_URL").expect("S3_URL must be set"),
            s3_region: env::var("S3_REGION").expect("S3_REGION must be set"),
            s3_bucket: env::var("S3_BUCKET").expect("S3_BUCKET must be set"),
            s3_access_key_id: env::var("S3_ACCESS_KEY_ID").expect("S3_ACCESS_KEY_ID must be set"),
            s3_secret_access_key: env::var("S3_ACCESS_KEY_SECRET")
                .expect("S3_ACCESS_KEY_SECRET must be set"),

            #[cfg(feature = "ldap")]
            ldap_server_uri: env::var("AUTH_LDAP_SERVER_URI").ok(),
            #[cfg(feature = "ldap")]
            ldap_bind_dn: env::var("AUTH_LDAP_BIND_DN").ok(),
            #[cfg(feature = "ldap")]
            ldap_bind_password: env::var("AUTH_LDAP_BIND_PASSWORD").ok(),
            #[cfg(feature = "ldap")]
            ldap_user_dn_template: env::var("AUTH_LDAP_USER_DN_TEMPLATE").ok(),
            #[cfg(feature = "ldap")]
            ldap_group_search: env::var("AUTH_LDAP_GROUP_SEARCH").ok(),
        })
    }
}
