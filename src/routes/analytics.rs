use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{auth::middleware::AuthUser, db::Db, AppError};

const VALID_INTERVALS: &[&str] = &["minute", "hour", "day", "week"];

// ─── Query param structs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VolumeQuery {
    pub source: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub interval: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorRateQuery {
    pub source: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

// ─── Response structs ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct VolumeResponse {
    pub interval: String,
    pub from: chrono::DateTime<Utc>,
    pub to: chrono::DateTime<Utc>,
    pub data: Vec<crate::db::VolumePoint>,
}

#[derive(Debug, Serialize)]
pub struct ErrorRateResponse {
    pub from: chrono::DateTime<Utc>,
    pub to: chrono::DateTime<Utc>,
    pub total: i64,
    pub errors: i64,
    pub rate: f64,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_optional_dt(s: &str, field: &str) -> Result<chrono::DateTime<Utc>, AppError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| AppError::BadRequest(format!("Invalid '{}' date format", field)))
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

pub async fn get_volume(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Query(params): Query<VolumeQuery>,
) -> Result<impl IntoResponse, AppError> {
    let interval = params.interval.clone().unwrap_or_else(|| "hour".to_string());

    if !VALID_INTERVALS.contains(&interval.as_str()) {
        return Err(AppError::BadRequest(format!(
            "invalid interval '{}'; must be one of: minute, hour, day, week",
            interval
        )));
    }

    let now = Utc::now();
    let default_from = now - chrono::Duration::hours(24);

    let from_dt = match &params.from {
        Some(s) => parse_optional_dt(s, "from")?,
        None => default_from,
    };
    let to_dt = match &params.to {
        Some(s) => parse_optional_dt(s, "to")?,
        None => now,
    };

    let data = db
        .analytics_volume(
            params.source.as_deref(),
            Some(from_dt),
            Some(to_dt),
            &interval,
        )
        .map_err(|e| match e {
            crate::db::DbError::InvalidInput(msg) => AppError::BadRequest(msg),
            other => AppError::Internal(other.to_string()),
        })?;

    Ok((
        StatusCode::OK,
        Json(VolumeResponse {
            interval,
            from: from_dt,
            to: to_dt,
            data,
        }),
    ))
}

pub async fn get_error_rate(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Query(params): Query<ErrorRateQuery>,
) -> Result<impl IntoResponse, AppError> {
    let now = Utc::now();
    let default_from = now - chrono::Duration::hours(24);

    let from_dt = match &params.from {
        Some(s) => parse_optional_dt(s, "from")?,
        None => default_from,
    };
    let to_dt = match &params.to {
        Some(s) => parse_optional_dt(s, "to")?,
        None => now,
    };

    let result = db
        .analytics_error_rate(params.source.as_deref(), Some(from_dt), Some(to_dt))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(ErrorRateResponse {
            from: from_dt,
            to: to_dt,
            total: result.total,
            errors: result.errors,
            rate: result.rate,
        }),
    ))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use axum_test::TestServer;
    use crate::config::Config;
    use crate::ingest::normalize::NormalizedLog;
    use serde_json::json;

    async fn setup_app() -> (Router, String) {
        let config = Config {
            data_path: ":memory:".to_string(),
            storage_mode: "local".to_string(),
            ..Default::default()
        };
        let db = Db::open(&config).expect("db open");

        db.create_user("analytics@example.com", "password", "viewer")
            .expect("create user");
        let user = db
            .find_user_by_email("analytics@example.com")
            .expect("find user")
            .expect("user exists");

        let jwt_secret = "test-secret-analytics".to_string();
        let token = crate::auth::create_jwt(&user, &jwt_secret).expect("create jwt");

        let app_state = crate::AppState {
            db: Arc::new(db),
            jwt_secret,
        };

        let app = Router::new()
            .route("/api/analytics/volume", get(get_volume))
            .route("/api/analytics/errors", get(get_error_rate))
            .with_state(app_state);

        (app, token)
    }

    async fn setup_app_with_logs() -> (Router, String) {
        let config = Config {
            data_path: ":memory:".to_string(),
            storage_mode: "local".to_string(),
            ..Default::default()
        };
        let db = Db::open(&config).expect("db open");

        db.create_user("analytics2@example.com", "password", "viewer")
            .expect("create user");
        let user = db
            .find_user_by_email("analytics2@example.com")
            .expect("find user")
            .expect("user exists");

        // Insert some logs: 3 info, 2 error, 1 fatal → total=6, errors=3, rate=0.5
        let logs = vec![
            NormalizedLog {
                id: "a1".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-06-01T10:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                source: "svc".to_string(),
                level: Some("info".to_string()),
                message: Some("msg".to_string()),
                fields: json!({}),
                ingested_at: Utc::now(),
            },
            NormalizedLog {
                id: "a2".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-06-01T11:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                source: "svc".to_string(),
                level: Some("error".to_string()),
                message: Some("msg".to_string()),
                fields: json!({}),
                ingested_at: Utc::now(),
            },
            NormalizedLog {
                id: "a3".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-06-01T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                source: "svc".to_string(),
                level: Some("info".to_string()),
                message: Some("msg".to_string()),
                fields: json!({}),
                ingested_at: Utc::now(),
            },
            NormalizedLog {
                id: "a4".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-06-01T13:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                source: "svc".to_string(),
                level: Some("fatal".to_string()),
                message: Some("msg".to_string()),
                fields: json!({}),
                ingested_at: Utc::now(),
            },
            NormalizedLog {
                id: "a5".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-06-01T14:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                source: "svc".to_string(),
                level: Some("error".to_string()),
                message: Some("msg".to_string()),
                fields: json!({}),
                ingested_at: Utc::now(),
            },
            NormalizedLog {
                id: "a6".to_string(),
                timestamp: chrono::DateTime::parse_from_rfc3339("2025-06-01T15:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                source: "svc".to_string(),
                level: Some("info".to_string()),
                message: Some("msg".to_string()),
                fields: json!({}),
                ingested_at: Utc::now(),
            },
        ];
        db.insert_logs(&logs).expect("insert logs");

        let jwt_secret = "test-secret-analytics2".to_string();
        let token = crate::auth::create_jwt(&user, &jwt_secret).expect("create jwt");

        let app_state = crate::AppState {
            db: Arc::new(db),
            jwt_secret,
        };

        let app = Router::new()
            .route("/api/analytics/volume", get(get_volume))
            .route("/api/analytics/errors", get(get_error_rate))
            .with_state(app_state);

        (app, token)
    }

    // ── volume tests ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_volume_no_params_returns_200_with_data() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).expect("server");

        let resp = server
            .get("/api/analytics/volume")
            .add_header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .await;

        resp.assert_status(StatusCode::OK);
        let body = resp.json::<serde_json::Value>();
        assert!(body.get("data").is_some(), "response must have 'data' field");
        assert!(body["data"].is_array(), "'data' must be an array");
        assert_eq!(body["interval"], "hour");
        assert!(body.get("from").is_some());
        assert!(body.get("to").is_some());
    }

    #[tokio::test]
    async fn test_volume_invalid_interval_returns_400() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).expect("server");

        let resp = server
            .get("/api/analytics/volume")
            .add_query_param("interval", "second")
            .add_header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .await;

        resp.assert_status(StatusCode::BAD_REQUEST);
    }

    // ── error-rate tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_error_rate_no_params_returns_200_with_fields() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).expect("server");

        let resp = server
            .get("/api/analytics/errors")
            .add_header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .await;

        resp.assert_status(StatusCode::OK);
        let body = resp.json::<serde_json::Value>();
        assert!(body.get("total").is_some(), "must have 'total'");
        assert!(body.get("errors").is_some(), "must have 'errors'");
        assert!(body.get("rate").is_some(), "must have 'rate'");
    }

    #[tokio::test]
    async fn test_error_rate_is_zero_when_no_logs() {
        let (app, token) = setup_app().await;
        let server = TestServer::new(app).expect("server");

        let resp = server
            .get("/api/analytics/errors")
            .add_header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .await;

        resp.assert_status(StatusCode::OK);
        let body = resp.json::<serde_json::Value>();
        assert_eq!(body["total"].as_i64().unwrap_or(-1), 0);
        assert_eq!(body["errors"].as_i64().unwrap_or(-1), 0);
        assert_eq!(body["rate"].as_f64().unwrap_or(-1.0), 0.0);
    }

    #[tokio::test]
    async fn test_error_rate_rounded_to_4_decimals() {
        let (app, token) = setup_app_with_logs().await;
        let server = TestServer::new(app).expect("server");

        let resp = server
            .get("/api/analytics/errors")
            .add_query_param("from", "2025-01-01T00:00:00Z")
            .add_query_param("to", "2025-12-31T23:59:59Z")
            .add_header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .await;

        resp.assert_status(StatusCode::OK);
        let body = resp.json::<serde_json::Value>();
        let rate = body["rate"].as_f64().expect("rate is f64");

        // Check at most 4 decimal places by comparing with rounded value
        let rounded = (rate * 10_000.0).round() / 10_000.0;
        assert!(
            (rate - rounded).abs() < f64::EPSILON,
            "rate {} is not rounded to 4 decimals",
            rate
        );
    }
}
