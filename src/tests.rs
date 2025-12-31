#[cfg(test)]
mod tests {
    use crate::{config::Config, state::AppState};
    use axum_test::TestServer;
    use sqlx::SqlitePool;

    async fn create_test_app() -> TestServer {
        // Setup test database
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Create test config (you'll need to mock or use test S3)
        let config = Config {
            database_url: ":memory:".to_string(),
            secret_key: "test-secret-key".to_string(),
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

        let state = AppState {
            pool,
            s3_client,
            config,
        };

        // Build the router (same as main.rs)
        let app = crate::build_router(state);

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
