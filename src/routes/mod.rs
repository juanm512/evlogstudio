use axum::{routing::{get, post}, Router};
use crate::AppState;

pub mod health;
pub mod ingest;
pub mod logs;
pub mod schema;
pub mod auth;
pub mod sources;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_handler))
        .route("/ingest", post(ingest::ingest_handler))
        .route("/api/logs", get(logs::get_logs))
        .route("/api/schema", get(schema::get_schema))
        .route("/auth/login", post(auth::login_handler))
        .route("/setup", get(auth::setup_get_handler).post(auth::setup_post_handler))
        .merge(sources::router())
        .with_state(state)
}
