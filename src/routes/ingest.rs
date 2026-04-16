use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::sync::Arc;
use crate::db::Db;
use crate::ingest::normalize::normalize_batch;
use crate::AppError;

pub async fn ingest_handler(
    State(db): State<Arc<Db>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err(AppError::Unauthorized);
        }
    };

    let source = match db.verify_ingest_token(token) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Err(AppError::Unauthorized);
        }
        Err(e) => {
            return Err(AppError::Internal(e.to_string()));
        }
    };

    let logs = normalize_batch(payload, &source);

    if logs.is_empty() {
        return Err(AppError::BadRequest("invalid payload".to_string()));
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

            Ok((
                StatusCode::OK,
                axum::Json(json!({"inserted": inserted}))
            ))
        }
        Err(e) => {
            Err(AppError::Internal(e.to_string()))
        }
    }
}
