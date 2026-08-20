#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::{config::Config, state::AppState};
    use axum_test::TestServer;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::OnceLock;
    use tower_sessions_sqlx_store::SqliteStore;

    fn metrics_handle() -> metrics_exporter_prometheus::PrometheusHandle {
        static HANDLE: OnceLock<metrics_exporter_prometheus::PrometheusHandle> = OnceLock::new();
        HANDLE
            .get_or_init(|| {
                metrics_exporter_prometheus::PrometheusBuilder::new()
                    .install_recorder()
                    .expect("failed to install prometheus recorder")
            })
            .clone()
    }

    async fn create_test_app() -> TestServer {
        // A single connection is required so the in-memory database is shared
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .unwrap();

        // SQLite-compatible schema (the Postgres migration is not portable)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT,
                email TEXT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let config = Config {
            database_url: ":memory:".to_string(),
            secret_key: "test-secret-key".to_string(),
            session_secure: false,
            s3_endpoint: "http://localhost:9000".to_string(),
            s3_region: "us-east-1".to_string(),
            s3_bucket: "test-bucket".to_string(),
            s3_access_key_id: "test".to_string(),
            s3_secret_access_key: "test".to_string(),
            #[cfg(feature = "ldap")]
            ldap_server_uri: None,
            #[cfg(feature = "ldap")]
            ldap_bind_dn: None,
            #[cfg(feature = "ldap")]
            ldap_bind_password: None,
            #[cfg(feature = "ldap")]
            ldap_user_dn_template: None,
            #[cfg(feature = "ldap")]
            ldap_group_search: None,
        };

        let s3_client = crate::s3::create_s3_client(&config).await;

        let session_store = SqliteStore::new(pool.clone());
        session_store.migrate().await.unwrap();

        let state = AppState {
            pool,
            s3_client,
            config,
            metrics_handle: metrics_handle(),
        };

        // Build the router (same as main.rs)
        let app = crate::build_router(state, session_store);

        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_login_page_loads() {
        let server = create_test_app().await;

        let response = server.get("/users/login/").await;

        assert_eq!(response.status_code(), 200);
        response.assert_text_contains("Login");
    }

    #[tokio::test]
    async fn test_protected_route_redirects() {
        let server = create_test_app().await;

        let response = server.get("/").await;

        // Should redirect to login
        assert_eq!(response.status_code(), 303);
    }

    #[tokio::test]
    async fn test_invalid_login() {
        let server = create_test_app().await;

        let response = server
            .post("/users/login/")
            .form(&[("username", "invalid"), ("password", "wrong")])
            .await;

        response.assert_text_contains("Invalid username or password");
    }
}
