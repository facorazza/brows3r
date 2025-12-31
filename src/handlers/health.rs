use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

use crate::state::AppState;

/// Health check endpoint - returns 200 if service is running
pub async fn health_check() -> impl IntoResponse {
    metrics::counter!("health_check_total").increment(1);
    (StatusCode::OK, Json(json!({ "status": "healthy" })))
}

/// Readiness check - returns 200 if service is ready to accept traffic
/// Checks database connectivity
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    metrics::counter!("readiness_check_total").increment(1);

    // Check database connectivity
    match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => {
            metrics::counter!("readiness_check_success").increment(1);
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ready",
                    "database": "connected"
                })),
            )
        }
        Err(e) => {
            metrics::counter!("readiness_check_failure").increment(1);
            tracing::error!("Database health check failed: {:?}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "not ready",
                    "database": "disconnected"
                })),
            )
        }
    }
}

/// Prometheus metrics endpoint
pub async fn metrics(State(state): State<AppState>) -> String {
    state.metrics_handle.render()
}
