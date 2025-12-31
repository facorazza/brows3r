use axum::extract::FromRef;
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub s3_client: aws_sdk_s3::Client,
    pub config: Config,
    pub metrics_handle: PrometheusHandle,
}

// Implement FromRef for each field so they can be extracted independently
impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for aws_sdk_s3::Client {
    fn from_ref(state: &AppState) -> Self {
        state.s3_client.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
