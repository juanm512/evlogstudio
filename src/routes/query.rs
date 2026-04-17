use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{db::Db, ingest::normalize::NormalizedLog, auth::middleware::AuthUser, AppError};

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub sql: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub logs: Vec<NormalizedLog>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JsonQueryResponse {
    pub rows: Vec<serde_json::Value>,
}

pub async fn post_query(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(body): Json<QueryRequest>,
) -> Result<impl IntoResponse, AppError> {
    let trimmed = body.sql.trim().to_uppercase();
    if !trimmed.starts_with("SELECT") {
        return Err(AppError::BadRequest("Only SELECT queries are allowed".to_string()));
    }

    match db.query_raw(&body.sql) {
        Ok(logs) => {
            let resp = QueryResponse { logs, next_cursor: None };
            Ok((StatusCode::OK, Json(resp)))
        }
        Err(e) => Err(AppError::Internal(e.to_string()))
    }
}

pub async fn post_query_json(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(body): Json<QueryRequest>,
) -> Result<impl IntoResponse, AppError> {
    let trimmed = body.sql.trim().to_uppercase();
    if !trimmed.starts_with("SELECT") {
        return Err(AppError::BadRequest("Only SELECT queries are allowed".to_string()));
    }

    match db.query_raw_flexible(&body.sql) {
        Ok(rows) => Ok((StatusCode::OK, Json(JsonQueryResponse { rows }))),
        Err(e) => Err(AppError::Internal(e.to_string()))
    }
}
