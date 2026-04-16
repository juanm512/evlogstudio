use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::Db;
use crate::auth::middleware::AuthUser;
use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct SchemaQuery {
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SchemaResponse {
    pub fields: Vec<crate::ingest::schema::SchemaEntry>,
    pub sources: Vec<String>,
}

pub async fn get_schema(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Query(params): Query<SchemaQuery>,
) -> Result<impl IntoResponse, AppError> {
    let source_ref = params.source.as_deref();

    let fields = db.get_schema(source_ref).map_err(|e| AppError::Internal(e.to_string()))?;
    let sources = db.list_schema_sources().map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((StatusCode::OK, Json(SchemaResponse { fields, sources })))
}
