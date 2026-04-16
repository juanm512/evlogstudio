use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::Db;
use crate::auth::middleware::AuthUser;
use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct SchemaQuery {
    pub source: Option<String>,
}

pub async fn get_schema(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Query(params): Query<SchemaQuery>,
) -> Result<impl IntoResponse, AppError> {
    let source_ref = params.source.as_deref();
    match db.get_schema(source_ref) {
        Ok(entries) => Ok((StatusCode::OK, Json(entries))),
        Err(e) => {
            Err(AppError::Internal(e.to_string()))
        }
    }
}
