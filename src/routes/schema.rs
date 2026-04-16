use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::Db;

#[derive(Debug, Deserialize)]
pub struct SchemaQuery {
    pub source: Option<String>,
}

pub async fn get_schema(
    State(db): State<Arc<Db>>,
    Query(params): Query<SchemaQuery>,
) -> impl IntoResponse {
    let source_ref = params.source.as_deref();
    match db.get_schema(source_ref) {
        Ok(entries) => (StatusCode::OK, Json(entries)).into_response(),
        Err(e) => {
            tracing::error!("Error querying schema: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}
