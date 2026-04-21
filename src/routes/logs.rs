use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{db::Db, ingest::normalize::NormalizedLog, auth::middleware::AuthUser, AppError};

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub source: Option<String>,
    pub level: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub search: Option<String>,
    pub service: Option<String>,
    pub environment: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<i32>,
    pub request_id: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<NormalizedLog>,
    pub next_cursor: Option<String>,
}

// Using crate::db::SearchCondition

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub source: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub conditions: Vec<crate::db::SearchCondition>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

pub async fn search_logs(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(req): Json<SearchRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mut from_dt = None;
    if let Some(ref f) = req.from {
        from_dt = chrono::DateTime::parse_from_rfc3339(f).ok().map(|dt| dt.with_timezone(&Utc));
    }
    let mut to_dt = None;
    if let Some(ref t) = req.to {
        to_dt = chrono::DateTime::parse_from_rfc3339(t).ok().map(|dt| dt.with_timezone(&Utc));
    }

    let limit = req.limit.unwrap_or(50).min(200);

    let logs = db.search_logs(
        req.source.as_deref(),
        from_dt,
        to_dt,
        &req.conditions,
        limit,
        req.cursor.as_deref(),
    ).map_err(|e| AppError::Internal(e.to_string()))?;

    let has_more = logs.len() > limit as usize;
    let mut logs = logs;
    if has_more {
        logs.truncate(limit as usize);
    }
    let next_cursor = if has_more {
        logs.last().map(|l| l.id.clone())
    } else {
        None
    };

    Ok((StatusCode::OK, Json(LogsResponse { logs, next_cursor })))
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub source: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub conditions: Vec<crate::db::SearchCondition>,
}

pub async fn export_logs(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(req): Json<ExportRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mut from_dt = None;
    if let Some(ref f) = req.from {
        from_dt = chrono::DateTime::parse_from_rfc3339(f).ok().map(|dt| dt.with_timezone(&Utc));
    }
    let mut to_dt = None;
    if let Some(ref t) = req.to {
        to_dt = chrono::DateTime::parse_from_rfc3339(t).ok().map(|dt| dt.with_timezone(&Utc));
    }

    // Export is limited to 10000 rows for stability
    let logs = db.search_logs(
        req.source.as_deref(),
        from_dt,
        to_dt,
        &req.conditions,
        10000,
        None,
    ).map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((StatusCode::OK, Json(logs)))
}

pub async fn get_logs(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Query(params): Query<LogsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let mut from_dt = None;
    if let Some(ref f) = params.from {
        match chrono::DateTime::parse_from_rfc3339(f) {
            Ok(dt) => from_dt = Some(dt.with_timezone(&Utc)),
            Err(_) => return Err(AppError::BadRequest("Invalid 'from' date format".to_string())),
        }
    }

    let mut to_dt = None;
    if let Some(ref t) = params.to {
        match chrono::DateTime::parse_from_rfc3339(t) {
            Ok(dt) => to_dt = Some(dt.with_timezone(&Utc)),
            Err(_) => return Err(AppError::BadRequest("Invalid 'to' date format".to_string())),
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
        service: params.service,
        environment: params.environment,
        method: params.method,
        path: params.path,
        status: params.status,
        request_id: params.request_id,
        limit,
        cursor: params.cursor,
    };

    match db.query_logs(&db_params) {
        Ok(mut rows) => {
            // DB returned up to limit+1 rows. If we got limit+1, there is a next page.
            let has_more = rows.len() > limit as usize;
            if has_more {
                rows.truncate(limit as usize);
            }

            let next_cursor = if has_more {
                rows.last().map(|l| l.id.clone())
            } else {
                None
            };

            let resp = LogsResponse { logs: rows, next_cursor };
            Ok((StatusCode::OK, Json(resp)))
        }
        Err(e) => {
            Err(AppError::Internal(e.to_string()))
        }
    }
}

// ----- Polling endpoint -----

#[derive(Debug, Deserialize)]
pub struct PollQuery {
    pub source: Option<String>,
    pub level: Option<String>,
    pub search: Option<String>,
    /// ID of the last log seen by the client. Used to derive the `ingested_at` cutoff.
    pub since_id: Option<String>,
    /// ISO-8601 fallback cutoff when `since_id` is absent or unknown.
    pub since_timestamp: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PollResponse {
    pub logs: Vec<crate::db::LogRecord>,
    pub last_id: Option<String>,
    pub last_timestamp: Option<chrono::DateTime<Utc>>,
    pub count: usize,
}

pub async fn poll_logs(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Query(params): Query<PollQuery>,
) -> Result<impl IntoResponse, AppError> {
    let since_ts = if let Some(ref s) = params.since_timestamp {
        match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(_) => return Err(AppError::BadRequest("Invalid 'since_timestamp' date format".to_string())),
        }
    } else {
        None
    };

    let mut limit = params.limit.unwrap_or(100);
    if limit > 500 {
        limit = 500;
    }

    let records = db
        .poll_logs(
            params.source.as_deref(),
            params.level.as_deref(),
            params.search.as_deref(),
            params.since_id.as_deref(),
            since_ts,
            limit,
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let count = records.len();
    let last_id = records.last().map(|r| r.id.clone());
    let last_timestamp = records.last().map(|r| r.ingested_at);

    Ok((StatusCode::OK, Json(PollResponse { logs: records, last_id, last_timestamp, count })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use crate::config::Config;
    use serde_json::json;
    use axum_test::TestServer;

    async fn setup_app() -> (Router, String) {
        let config = Config {
            data_path: ":memory:".to_string(),
            storage_mode: "local".to_string(),
            ..Default::default()
        };
        let db = Db::open(&config).unwrap();
        
        // Creamos un usuario de prueba para poder generar un token
        db.create_user("test@example.com", "password", "viewer").unwrap();
        let user = db.find_user_by_email("test@example.com").unwrap().unwrap();
        
        let now = Utc::now();
        let dummy_logs = vec![
            NormalizedLog {
                id: "1".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-01-01T10:00:00Z").unwrap().with_timezone(&Utc),
                source: "source1".to_string(),
                service: None, environment: None, method: None, path: None,
                status: None, duration_ms: None, request_id: None, error: None,
                level: Some("info".to_string()),
                message: Some("log 1 info".to_string()),
                fields: json!({"extra": "a"}),
                ingested_at: now - chrono::Duration::hours(3),
            },
            NormalizedLog {
                id: "2".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-01-01T11:00:00Z").unwrap().with_timezone(&Utc),
                source: "source2".to_string(),
                service: None, environment: None, method: None, path: None,
                status: None, duration_ms: None, request_id: None, error: None,
                level: Some("error".to_string()),
                message: Some("log 2 error".to_string()),
                fields: json!({"extra": "b"}),
                ingested_at: now - chrono::Duration::hours(2),
            },
            NormalizedLog {
                id: "3".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z").unwrap().with_timezone(&Utc),
                source: "source1".to_string(),
                service: None, environment: None, method: None, path: None,
                status: None, duration_ms: None, request_id: None, error: None,
                level: Some("warn".to_string()),
                message: Some("log 3 warn".to_string()),
                fields: json!({"extra": "c"}),
                ingested_at: now - chrono::Duration::hours(1),
            },
        ];
        db.insert_logs(&dummy_logs).unwrap();

        let jwt_secret = "test-secret".to_string();
        let token = crate::auth::create_jwt(&user, &jwt_secret).unwrap();

        let app_state = crate::AppState {
            db: Arc::new(db),
            jwt_secret,
            sampling_config: Arc::new(tokio::sync::RwLock::new(crate::SamplingConfig {
                enabled: false,
                debug_rate: 10,
                info_rate: 100,
                warn_rate: 100,
            })),
            source_sampling: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            login_limiter: Arc::new(governor::RateLimiter::dashmap(
                governor::Quota::per_minute(std::num::NonZeroU32::new(100).unwrap())
            )),
        };

        let app = Router::new()
            .route("/api/logs/poll", get(poll_logs))
            .route("/api/logs", get(get_logs))
            .with_state(app_state);
            
        (app, token)
    }

    #[tokio::test]
    async fn test_get_logs_no_params() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        
        assert!(body_json.get("logs").unwrap().is_array());
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 3);
    }

    #[tokio::test]
    async fn test_get_logs_with_limit() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("limit", 2)
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn test_get_logs_with_level() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("level", "error")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0]["level"], "error");
    }

    #[tokio::test]
    async fn test_get_logs_future_date() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("from", "2026-05-14T00:00:00Z")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body_json = response.json::<serde_json::Value>();
        let logs = body_json["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 0);
    }

    #[tokio::test]
    async fn test_get_logs_invalid_date() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/logs").add_query_param("from", "no-es-fecha")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::BAD_REQUEST);
    }

    // ----- poll_logs tests -----

    #[tokio::test]
    async fn test_poll_logs_no_params_returns_200_with_correct_shape() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        // No params: default window is now()-30s. The 3 test logs were just inserted,
        // so they should all appear.
        let response = server
            .get("/api/logs/poll")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert!(body["logs"].is_array(), "'logs' must be an array");
        assert!(body["count"].is_number(), "'count' must be a number");
        // last_id and last_timestamp may be non-null because we have logs
        let count = body["count"].as_u64().unwrap();
        if count == 0 {
            assert!(body["last_id"].is_null());
            assert!(body["last_timestamp"].is_null());
        } else {
            assert!(!body["last_id"].is_null());
        }
    }

    #[tokio::test]
    async fn test_poll_logs_with_since_id_returns_only_newer_logs() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        // Logs were inserted with ingested_at staggered: id=1 at -3h, id=2 at -2h, id=3 at -1h.
        // Polling with since_id=2 (cutoff = -2h) should return only log id=3.
        let response = server
            .get("/api/logs/poll")
            .add_query_param("since_id", "2")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        // Exactly one log (id=3) has ingested_at > (ingested_at of id=2)
        assert_eq!(body["count"].as_u64().unwrap(), 1);
        assert_eq!(body["last_id"].as_str().unwrap(), "3");
    }

    #[tokio::test]
    async fn test_poll_logs_count_zero_when_no_new_logs() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        // Use a future since_timestamp so nothing can possibly be newer
        let response = server
            .get("/api/logs/poll")
            .add_query_param("since_timestamp", "2099-01-01T00:00:00Z")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert_eq!(body["count"].as_u64().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_poll_logs_last_id_null_when_empty() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/logs/poll")
            .add_query_param("since_timestamp", "2099-01-01T00:00:00Z")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
            .await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert!(body["last_id"].is_null(), "last_id must be null when no logs returned");
        assert!(body["last_timestamp"].is_null(), "last_timestamp must be null when no logs returned");
    }
}
