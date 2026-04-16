use axum::{routing::{get, post}, Router};
use crate::AppState;

pub mod health;
pub mod ingest;
pub mod logs;
pub mod schema;
pub mod auth;
pub mod sources;
pub mod users;
pub mod analytics;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_handler))
        .route("/ingest", post(ingest::ingest_handler))
        .route("/api/logs", get(logs::get_logs))
        .route("/api/schema", get(schema::get_schema))
        .route("/auth/login", post(auth::login_handler))
        .route("/setup", get(auth::setup_get_handler).post(auth::setup_post_handler))
        .route("/api/analytics/volume", get(analytics::get_volume))
        .route("/api/analytics/errors", get(analytics::get_error_rate))
        .merge(sources::router())
        .merge(users::router())
        .with_state(state)
}
