use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;
use crate::db::Db;
use crate::auth::middleware::AuthUser;
use crate::auth::require_admin;
use crate::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct FullConfig {
    pub retention_default: String,
    pub sampling_enabled: bool,
    pub sampling_debug_rate: u8,
    pub sampling_info_rate: u8,
    pub sampling_warn_rate: u8,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigReq {
    pub retention_default: Option<String>,
    pub sampling_enabled: Option<bool>,
    pub sampling_debug_rate: Option<u8>,
    pub sampling_info_rate: Option<u8>,
    pub sampling_warn_rate: Option<u8>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/config", get(get_config).put(update_config))
        .route("/api/logs/all", delete(delete_all_logs))
}

async fn get_config(
    user: AuthUser,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    let config_map = db.get_all_config().map_err(|e| AppError::Internal(e.to_string()))?;
    
    let config = FullConfig {
        retention_default: config_map.get("retention.default")
            .cloned()
            .unwrap_or_else(|| "30d".to_string()),
        sampling_enabled: config_map.get("sampling.enabled")
            .map(|v| v == "true")
            .unwrap_or(false),
        sampling_debug_rate: config_map.get("sampling.debug_rate")
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),
        sampling_info_rate: config_map.get("sampling.info_rate")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
        sampling_warn_rate: config_map.get("sampling.warn_rate")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
    };
    
    Ok((StatusCode::OK, Json(config)))
}

async fn update_config(
    user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(payload): Json<UpdateConfigReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    if let Some(ref retention) = payload.retention_default {
        Db::parse_retention(retention)
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        db.set_config_value("retention.default", retention)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    
    if let Some(enabled) = payload.sampling_enabled {
        db.set_config_value("sampling.enabled", &enabled.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    
    if let Some(rate) = payload.sampling_debug_rate {
        if rate > 100 {
            return Err(AppError::BadRequest("debug rate must be between 0 and 100".to_string()));
        }
        db.set_config_value("sampling.debug_rate", &rate.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    
    if let Some(rate) = payload.sampling_info_rate {
        if rate > 100 {
            return Err(AppError::BadRequest("info rate must be between 0 and 100".to_string()));
        }
        db.set_config_value("sampling.info_rate", &rate.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    
    if let Some(rate) = payload.sampling_warn_rate {
        if rate > 100 {
            return Err(AppError::BadRequest("warn rate must be between 0 and 100".to_string()));
        }
        db.set_config_value("sampling.warn_rate", &rate.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    
    Ok((StatusCode::OK, Json(serde_json::json!({"message": "updated"}))))
}

async fn delete_all_logs(
    user: AuthUser,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    let deleted_count = db.delete_all_logs().map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": deleted_count}))))
}
