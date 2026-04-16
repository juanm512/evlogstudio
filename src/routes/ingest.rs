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
    headers: axum::http::HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({"error": "missing token"}))
            ).into_response();
        }
    };

    let source = match db.verify_ingest_token(token) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({"error": "invalid token"}))
            ).into_response();
        }
        Err(e) => {
            tracing::error!("Token verification error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"error": "internal server error"}))
            ).into_response();
        }
    };

    let logs = normalize_batch(payload, &source);

    if logs.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": "invalid payload"}))
        ).into_response();
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
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Error inserting logs: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"error": "internal server error"}))
            ).into_response()
        }
    }
}
