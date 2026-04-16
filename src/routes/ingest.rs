use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::sync::Arc;
use crate::db::Db;
use crate::ingest::normalize::normalize_batch;

pub async fn ingest_handler(
    State(db): State<Arc<Db>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let logs = normalize_batch(payload);

    if logs.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": "invalid payload"}))
        );
    }

    match db.insert_logs(&logs) {
        Ok(inserted) => {
            let bg_db = db.clone();
            tokio::spawn(async move {
                let entries = crate::ingest::schema::infer_schema(&logs);
                if let Err(e) = bg_db.upsert_schema(&entries) {
                    tracing::error!("Error upserting schema: {}", e);
                }
            });

            (
                StatusCode::OK,
                axum::Json(json!({"inserted": inserted}))
            )
        }
        Err(e) => {
            tracing::error!("Error inserting logs: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"error": "internal server error"}))
            )
        }
    }
}
