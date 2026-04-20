use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use crate::ingest::normalize::normalize_batch;
use crate::AppError;

use crate::AppState;
use rand::Rng;

pub async fn ingest_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
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

    // Apply sampling — per-source config takes precedence over global
    let source_map = state.source_sampling.read().await;
    let global_sampling;
    let sampling: &crate::SamplingConfig = if let Some(s) = source_map.get(&source) {
        s
    } else {
        global_sampling = state.sampling_config.read().await;
        &global_sampling
    };

    let filtered_logs = if sampling.enabled {
        let mut rng = rand::thread_rng();
        logs.into_iter().filter(|log| {
            let rate = match log.level.as_deref() {
                Some("debug") => sampling.debug_rate,
                Some("info") => sampling.info_rate,
                Some("warn") => sampling.warn_rate,
                _ => 100, // error, fatal, or unknown always 100%
            };
            if rate >= 100 {
                true
            } else if rate == 0 {
                false
            } else {
                rng.gen_range(0..100) < rate
            }
        }).collect::<Vec<_>>()
    } else {
        logs
    };

    if filtered_logs.is_empty() {
        return Ok((
            StatusCode::OK,
            axum::Json(json!({"inserted": 0}))
        ));
    }

    match db.insert_logs(&filtered_logs) {
        Ok(inserted) => {
            let bg_db = db.clone();
            let logs_for_inference = filtered_logs.clone();
            tokio::spawn(async move {
                let entries = crate::ingest::schema::infer_schema(&logs_for_inference);
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
