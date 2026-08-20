use axum::extract::FromRef;
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::{Database, Pool};

use crate::config::Config;

pub struct AppState<DB: Database> {
    pub pool: Pool<DB>,
    pub s3_client: aws_sdk_s3::Client,
    pub config: Config,
    pub metrics_handle: PrometheusHandle,
}

// Manual Clone impl: `sqlx::Sqlite` is not Clone, so the derive would over-constrain.
impl<DB: Database> Clone for AppState<DB> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            s3_client: self.s3_client.clone(),
            config: self.config.clone(),
            metrics_handle: self.metrics_handle.clone(),
        }
    }
}

// Implement FromRef for each field so they can be extracted independently
impl<DB: Database> FromRef<AppState<DB>> for Pool<DB> {
    fn from_ref(state: &AppState<DB>) -> Self {
        state.pool.clone()
    }
}

impl<DB: Database> FromRef<AppState<DB>> for aws_sdk_s3::Client {
    fn from_ref(state: &AppState<DB>) -> Self {
        state.s3_client.clone()
    }
}

impl<DB: Database> FromRef<AppState<DB>> for Config {
    fn from_ref(state: &AppState<DB>) -> Self {
        state.config.clone()
    }
}
