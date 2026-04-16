use axum::{routing::get, Router};
use std::sync::Arc;
use crate::db::Db;

pub mod health;

pub fn create_router(db: Arc<Db>) -> Router {
    Router::new()
        .route("/health", get(health::health_handler))
        .with_state(db)
}
