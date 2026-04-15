use axum::{routing::get, Router};

pub mod health;

pub fn create_router() -> Router {
    Router::new().route("/health", get(health::health_handler))
}
