use axum::{response::IntoResponse, Json, extract::State};
use serde::Serialize;
use std::sync::Arc;
use crate::db::Db;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    setup_required: bool,
}

pub async fn health_handler(State(db): State<Arc<Db>>) -> impl IntoResponse {
    let user_count = db.count_users().unwrap_or(0);
    Json(HealthResponse { 
        status: "ok", 
        setup_required: user_count == 0 
    })
}
