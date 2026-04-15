use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
}

pub async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}
