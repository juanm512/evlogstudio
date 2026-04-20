use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, delete},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use serde_json::json;

use crate::AppState;
use crate::db::Db;
use crate::auth::middleware::AuthUser;
use crate::auth::require_admin;
use crate::AppError;

#[derive(Deserialize)]
pub struct CreateSourceReq {
    name: String,
    description: Option<String>,
    retention: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSourceReq {
    retention: Option<String>,
    sampling_enabled: Option<bool>,
    sampling_debug_rate: Option<i32>,
    sampling_info_rate: Option<i32>,
    sampling_warn_rate: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateTokenReq {
    name: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/sources", get(list_sources).post(create_source))
        .route("/api/sources/:id", delete(delete_source).patch(update_source))
        .route("/api/sources/:id/logs", delete(delete_source_logs))
        .route("/api/sources/:id/tokens", get(list_tokens).post(create_token))
        .route("/api/tokens/:id", delete(revoke_token))
}

async fn list_sources(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    match db.list_sources() {
        Ok(sources) => Ok((StatusCode::OK, axum::Json(sources))),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn create_source(
    user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(payload): Json<CreateSourceReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    let name = payload.name.trim();
    if name.is_empty() || name.len() > 64 || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(AppError::BadRequest("invalid source name".to_string()));
    }

    if let Some(ref r) = payload.retention {
        crate::db::Db::parse_retention(r)
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
    }

    match db.create_source(name, payload.description.as_deref(), payload.retention) {
        Ok(id) => Ok((StatusCode::CREATED, axum::Json(json!({"id": id, "name": name})))),
        Err(e) => {
            let err_str = e.to_string();
            // DuckDB: "Duplicate key ... violates unique constraint"
            // SQLite: "UNIQUE constraint failed"
            let is_conflict = err_str.contains("UNIQUE")
                || err_str.contains("violates unique constraint")
                || err_str.contains("Duplicate key");
            if is_conflict {
                Err(AppError::Conflict("source already exists".to_string()))
            } else {
                Err(AppError::Internal(err_str))
            }
        }
    }
}

async fn update_source(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<UpdateSourceReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    let params = crate::db::UpdateSourceParams {
        retention: payload.retention,
        sampling_enabled: payload.sampling_enabled,
        sampling_debug_rate: payload.sampling_debug_rate,
        sampling_info_rate: payload.sampling_info_rate,
        sampling_warn_rate: payload.sampling_warn_rate,
    };
    match db.update_source(&id, &params) {
        Ok(true) => Ok((StatusCode::OK, axum::Json(json!({"message": "updated"})))),
        Ok(false) => Err(AppError::NotFound),
        Err(crate::db::DbError::InvalidInput(msg)) => Err(AppError::BadRequest(msg)),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn delete_source_logs(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    match db.delete_source_logs(&id) {
        Ok(deleted) => Ok((StatusCode::OK, axum::Json(json!({"deleted": deleted})))),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn delete_source(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    match db.delete_source(&id) {
        Ok(true) => Ok((StatusCode::OK, axum::Json(json!({"message": "deleted"})))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn list_tokens(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    match db.list_ingest_tokens(Some(&id)) {
        Ok(tokens) => Ok((StatusCode::OK, axum::Json(tokens))),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn create_token(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<CreateTokenReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    let sources = match db.list_sources() {
        Ok(s) => s,
        Err(e) => return Err(AppError::Internal(e.to_string())),
    };
    let source = match sources.into_iter().find(|s| s.id == id) {
        Some(s) => s,
        None => return Err(AppError::NotFound),
    };

    match db.create_ingest_token(&payload.name, &source.name, &user.id) {
        Ok((token_id, token)) => {
            Ok((StatusCode::CREATED, axum::Json(json!({
                "id": token_id,
                "token": token,
                "name": payload.name,
                "message": "This is the only time the token will be shown."
            }))))
        },
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn revoke_token(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    match db.revoke_ingest_token(&id) {
        Ok(true) => Ok((StatusCode::OK, axum::Json(json!({"message": "revoked"})))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}
