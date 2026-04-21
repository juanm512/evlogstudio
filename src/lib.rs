pub mod config;
pub mod db;
pub mod ingest;
pub mod routes;
pub mod auth;
pub mod error;

pub use error::AppError;
use std::sync::Arc;
use std::collections::HashMap;
use std::net::IpAddr;
use governor::{RateLimiter, state::keyed::DashMapStateStore};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SamplingConfig {
    pub enabled: bool,
    pub debug_rate: u8,
    pub info_rate: u8,
    pub warn_rate: u8,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::Db>,
    pub jwt_secret: String,
    pub sampling_config: Arc<tokio::sync::RwLock<SamplingConfig>>,
    pub source_sampling: Arc<tokio::sync::RwLock<HashMap<String, SamplingConfig>>>,
    pub login_limiter: Arc<RateLimiter<IpAddr, DashMapStateStore<IpAddr>, governor::clock::DefaultClock>>,
}

impl axum::extract::FromRef<AppState> for Arc<db::Db> {
    fn from_ref(state: &AppState) -> Arc<db::Db> {
        state.db.clone()
    }
}
