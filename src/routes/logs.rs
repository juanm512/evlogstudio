use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{db::Db, ingest::normalize::NormalizedLog};

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub source: Option<String>,
    pub level: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub search: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<NormalizedLog>,
    pub next_cursor: Option<String>,
}

pub async fn get_logs(
    State(db): State<Arc<Db>>,
    Query(params): Query<LogsQuery>,
) -> impl IntoResponse {
    let mut from_dt = None;
    if let Some(ref f) = params.from {
        match chrono::DateTime::parse_from_rfc3339(f) {
            Ok(dt) => from_dt = Some(dt.with_timezone(&Utc)),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid 'from' date format").into_response(),
        }
    }

    let mut to_dt = None;
    if let Some(ref t) = params.to {
        match chrono::DateTime::parse_from_rfc3339(t) {
            Ok(dt) => to_dt = Some(dt.with_timezone(&Utc)),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid 'to' date format").into_response(),
        }
    }

    let mut limit = params.limit.unwrap_or(50);
    if limit > 200 {
        limit = 200;
    }

    let db_params = crate::db::LogQueryParams {
        source: params.source,
        level: params.level,
        from: from_dt,
        to: to_dt,
        search: params.search,
        limit,
        cursor: params.cursor,
    };

    match db.query_logs(&db_params) {
        Ok(logs) => {
            let next_cursor = if logs.len() == limit as usize {
                logs.last().map(|l| l.id.clone())
            } else {
                None
            };

            let resp = LogsResponse { logs, next_cursor };
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => {
            tracing::error!("Error querying logs: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use crate::config::Config;
    use serde_json::json;
    use axum_test::TestServer;

    async fn setup_app() -> Router {
        let config = Config {
            data_path: ":memory:".to_string(),
            storage_mode: "local".to_string(),
            ..Default::default()
        };
        let db = Db::open(&config).unwrap();
        
        let dummy_logs = vec![
            NormalizedLog {
                id: "1".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-01-01T10:00:00Z").unwrap().with_timezone(&Utc),
                source: "source1".to_string(),
                level: Some("info".to_string()),
                message: Some("log 1 info".to_string()),
                fields: json!({"extra": "a"}),
                ingested_at: Utc::now(),
            },
            NormalizedLog {
                id: "2".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-01-01T11:00:00Z").unwrap().with_timezone(&Utc),
                source: "source2".to_string(),
                level: Some("error".to_string()),
                message: Some("log 2 error".to_string()),
                fields: json!({"extra": "b"}),
                ingested_at: Utc::now(),
            },
            NormalizedLog {
                id: "3".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z").unwrap().with_timezone(&Utc),
                source: "source1".to_string(),
                level: Some("warn".to_string()),
                message: Some("log 3 warn".to_string()),
                fields: json!({"extra": "c"}),
                ingested_at: Utc::now(),
            },
        ];
        db.insert_logs(&dummy_logs).unwrap();

        Router::new()
            .route("/api/logs", get(get_logs))
            .with_state(Arc::new(db))
    }

    #[tokio::test]
    async fn test_get_logs_no_params() {
        let app = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        
        assert!(body_json.get("logs").unwrap().is_array());
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 3);
    }

    #[tokio::test]
    async fn test_get_logs_with_limit() {
        let app = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("limit", 2).await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn test_get_logs_with_level() {
        let app = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("level", "error").await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0]["level"], "error");
    }

    #[tokio::test]
    async fn test_get_logs_future_date() {
        let app = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("from", "2026-05-14T00:00:00Z").await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 0);
    }

    #[tokio::test]
    async fn test_get_logs_invalid_date() {
        let app = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("from", "no-es-fecha").await;
        response.assert_status(StatusCode::BAD_REQUEST);
    }
}
