use axum::{routing::get, Router};
use std::sync::Arc;
use crate::db::Db;

pub mod health;
pub mod ingest;
pub mod logs;
pub mod schema;

pub fn create_router(db: Arc<Db>) -> Router {
    Router::new()
        .route("/health", get(health::health_handler))
        .route("/ingest", axum::routing::post(ingest::ingest_handler))
        .route("/api/logs", get(logs::get_logs))
        .route("/api/schema", get(schema::get_schema))
        .with_state(db)
}
