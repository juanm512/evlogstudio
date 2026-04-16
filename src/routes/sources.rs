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

#[derive(Deserialize)]
pub struct CreateSourceReq {
    name: String,
    description: Option<String>,
    retention_days: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateTokenReq {
    name: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/sources", get(list_sources).post(create_source))
        .route("/api/sources/:id", delete(delete_source))
        .route("/api/sources/:id/tokens", get(list_tokens).post(create_token))
        .route("/api/tokens/:id", delete(revoke_token))
}

async fn list_sources(
    user: AuthUser,
    State(db): State<Arc<Db>>,
) -> impl IntoResponse {
    if user.role != "admin" {
        return (StatusCode::UNAUTHORIZED, axum::Json(json!({"error": "forbidden"}))).into_response();
    }
    match db.list_sources() {
        Ok(sources) => (StatusCode::OK, axum::Json(sources)).into_response(),
        Err(e) => {
            tracing::error!("list_sources error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "internal server error"}))).into_response()
        }
    }
}

async fn create_source(
    user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(payload): Json<CreateSourceReq>,
) -> impl IntoResponse {
    if user.role != "admin" {
        return (StatusCode::UNAUTHORIZED, axum::Json(json!({"error": "forbidden"}))).into_response();
    }
    
    let name = payload.name.trim();
    if name.is_empty() || name.len() > 64 || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return (StatusCode::BAD_REQUEST, axum::Json(json!({"error": "invalid source name"}))).into_response();
    }

    match db.create_source(name, payload.description.as_deref(), payload.retention_days) {
        Ok(id) => (StatusCode::CREATED, axum::Json(json!({"id": id, "name": name}))).into_response(),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("UNIQUE") || err_str.contains("UNIQUE constraint failed") {
                (StatusCode::CONFLICT, axum::Json(json!({"error": "source already exists"}))).into_response()
            } else {
                tracing::error!("create_source error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "internal server error"}))).into_response()
            }
        }
    }
}

async fn delete_source(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> impl IntoResponse {
    if user.role != "admin" {
        return (StatusCode::UNAUTHORIZED, axum::Json(json!({"error": "forbidden"}))).into_response();
    }
    match db.delete_source(&id) {
        Ok(true) => (StatusCode::OK, axum::Json(json!({"message": "deleted"}))).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, axum::Json(json!({"error": "not found"}))).into_response(),
        Err(e) => {
            tracing::error!("delete_source error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "internal server error"}))).into_response()
        }
    }
}

async fn list_tokens(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> impl IntoResponse {
    if user.role != "admin" {
        return (StatusCode::UNAUTHORIZED, axum::Json(json!({"error": "forbidden"}))).into_response();
    }
    match db.list_ingest_tokens(Some(&id)) {
        Ok(tokens) => (StatusCode::OK, axum::Json(tokens)).into_response(),
        Err(e) => {
            tracing::error!("list_tokens error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "internal server error"}))).into_response()
        }
    }
}

async fn create_token(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<CreateTokenReq>,
) -> impl IntoResponse {
    if user.role != "admin" {
        return (StatusCode::UNAUTHORIZED, axum::Json(json!({"error": "forbidden"}))).into_response();
    }
    
    let sources = match db.list_sources() {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "db error"}))).into_response(),
    };
    let source = match sources.into_iter().find(|s| s.id == id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, axum::Json(json!({"error": "source not found"}))).into_response(),
    };

    match db.create_ingest_token(&payload.name, &source.name, &user.id) {
        Ok((token_id, token)) => {
            (StatusCode::CREATED, axum::Json(json!({
                "id": token_id,
                "token": token,
                "name": payload.name,
                "message": "This is the only time the token will be shown."
            }))).into_response()
        },
        Err(e) => {
            tracing::error!("create_token error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "internal server error"}))).into_response()
        }
    }
}

async fn revoke_token(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> impl IntoResponse {
    if user.role != "admin" {
        return (StatusCode::UNAUTHORIZED, axum::Json(json!({"error": "forbidden"}))).into_response();
    }
    match db.revoke_ingest_token(&id) {
        Ok(true) => (StatusCode::OK, axum::Json(json!({"message": "revoked"}))).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, axum::Json(json!({"error": "not found"}))).into_response(),
        Err(e) => {
            tracing::error!("revoke_token error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "internal server error"}))).into_response()
        }
    }
}
