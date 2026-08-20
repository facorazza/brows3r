mod auth;
mod config;
mod error;
mod handlers;
mod models;
mod s3;
mod state;
mod templates;

#[cfg(test)]
mod tests;

use axum::{
    Router, middleware,
    routing::{get, post},
};
use sqlx::postgres::PgPool;
use std::net::SocketAddr;
use time::Duration;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tower_sessions::{Expiry, SessionManagerLayer, session_store::SessionStore};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    auth::require_auth,
    config::Config,
    handlers::{browser, health, users},
    state::AppState,
};

fn build_router<DB, S>(state: AppState<DB>, session_store: S) -> Router
where
    DB: sqlx::Database,
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
    for<'q> <DB as sqlx::Database>::Arguments<'q>: sqlx::IntoArguments<'q, DB>,
    for<'r> crate::models::User: sqlx::FromRow<'r, DB::Row>,
    for<'q> uuid::Uuid: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    for<'q> &'q str: sqlx::Encode<'q, DB>,
    for<'q> String: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    str: sqlx::Type<DB>,
    S: SessionStore + Clone,
{
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(state.config.session_secure)
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    let protected_routes = Router::new()
        .route("/", get(browser::list::<DB>))
        .route("/{*path}", get(browser::list::<DB>))
        .route("/create-directory/", post(browser::create_directory::<DB>))
        .route(
            "/delete/{*path}",
            get(browser::delete::<DB>).delete(browser::delete::<DB>),
        )
        .route("/download/{*path}", get(browser::download::<DB>))
        .route("/upload/", post(browser::upload::<DB>))
        .route("/users/", get(users::user_list::<DB>))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth::<DB>,
        ));

    Router::new()
        .merge(protected_routes)
        .route(
            "/users/login/",
            get(users::login_form).post(users::login::<DB>),
        )
        .route("/users/logout/", get(users::logout))
        .route("/health", get(health::health_check))
        .route("/ready", get(health::readiness_check::<DB>))
        .route("/metrics", get(health::metrics::<DB>))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(session_layer)
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize metrics
    let prometheus_handle =
        metrics_exporter_prometheus::PrometheusBuilder::new().install_recorder()?;

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "brows3r=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded successfully");

    // Setup PostgreSQL database
    let pool = PgPool::connect(&config.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Setup session store
    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await?;

    // Create S3 client
    let s3_client = crate::s3::create_s3_client(&config).await;

    // Build application state
    let state = AppState {
        pool,
        s3_client,
        config,
        metrics_handle: prometheus_handle,
    };

    // Build router
    let app = build_router(state, session_store);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
