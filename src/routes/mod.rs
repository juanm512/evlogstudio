use axum::{routing::get, Router};
use std::sync::Arc;
use crate::db::Db;

pub mod health;
pub mod ingest;

pub fn create_router(db: Arc<Db>) -> Router {
    Router::new()
        .route("/health", get(health::health_handler))
        .route("/ingest", axum::routing::post(ingest::ingest_handler))
        .with_state(db)
}
