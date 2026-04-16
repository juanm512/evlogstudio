use axum::{response::IntoResponse, Json, extract::State};
use serde::Serialize;
use std::sync::Arc;
use crate::db::Db;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    count: usize,
}

pub async fn health_handler(State(db): State<Arc<Db>>) -> impl IntoResponse {
    tracing::info!("Health check");
    let count = db.count_logs().unwrap_or(0);
    Json(HealthResponse { status: "ok", count })
}
