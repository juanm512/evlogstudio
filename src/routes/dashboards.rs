use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::AppState;
use crate::db::Db;
use crate::auth::middleware::AuthUser;
use crate::auth::require_admin;
use crate::AppError;

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateDashboardReq {
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct UpdateDashboardReq {
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct CreateWidgetReq {
    title: String,
    #[serde(rename = "type")]
    widget_type: String,
    width: Option<String>,
    position: Option<i32>,
    config: String,
}

#[derive(Deserialize)]
struct UpdateWidgetReq {
    title: String,
    #[serde(rename = "type")]
    widget_type: String,
    width: String,
    config: String,
}

#[derive(Deserialize)]
struct WidgetPositionItem {
    id: String,
    position: i32,
}

// ─── Query types ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct WidgetQuery {
    pub metric: String,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub group_by: Option<GroupBy>,
    #[serde(default)]
    pub filters: Vec<WidgetFilter>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct GroupBy {
    pub field: String,
    pub interval: Option<String>,
}

#[derive(Deserialize)]
pub struct WidgetFilter {
    pub field: String,
    pub op: String,
    pub value: String,
}

#[derive(Serialize)]
struct QueryDataPoint {
    group_key: Option<String>,
    value: f64,
}

// ─── Router ──────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/dashboards", get(list_dashboards).post(create_dashboard))
        // /query before /:id so the literal path takes precedence for POST
        .route("/api/dashboards/query", post(query_widget))
        .route("/api/dashboards/:id", get(get_dashboard).put(update_dashboard).delete(delete_dashboard))
        .route("/api/dashboards/:id/widgets", post(create_widget))
        .route("/api/dashboards/:id/positions", put(update_widget_positions))
        .route("/api/widgets/:id", put(update_widget).delete(delete_widget))
}

// ─── Sanitization helpers ────────────────────────────────────────────────────

const DIRECT_COLS: &[&str] = &[
    "service", "source", "level", "method", "path", "environment",
    "request_id", "error", "message", "status", "duration_ms",
];

fn is_direct_col(f: &str) -> bool {
    DIRECT_COLS.contains(&f)
}

fn contains_sql_keywords(s: &str) -> bool {
    let up = s.to_uppercase();
    let dangerous = [
        "SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "UNION",
        "--", ";", "/*", "*/", "EXEC", "TRUNCATE", "ALTER",
    ];
    dangerous.iter().any(|k| up.contains(k))
}

fn validate_field_name(f: &str) -> Result<(), AppError> {
    if f.is_empty() || f.len() > 64 {
        return Err(AppError::BadRequest("invalid field name length".to_string()));
    }
    let valid = f.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-');
    if !valid {
        return Err(AppError::BadRequest(format!("invalid field name: {}", f)));
    }
    if contains_sql_keywords(f) {
        return Err(AppError::BadRequest(format!("field name contains reserved keywords: {}", f)));
    }
    Ok(())
}

fn validate_value(v: &str) -> Result<(), AppError> {
    if contains_sql_keywords(v) {
        return Err(AppError::BadRequest("filter value contains reserved keywords".to_string()));
    }
    Ok(())
}

fn escape_str(s: &str) -> String {
    s.replace('\'', "''")
}

fn field_text(f: &str) -> String {
    if is_direct_col(f) {
        format!("CAST({} AS VARCHAR)", f)
    } else {
        format!("json_extract_string(fields, '$.{}')", f)
    }
}

fn field_num(f: &str) -> String {
    if is_direct_col(f) {
        format!("CAST({} AS DOUBLE)", f)
    } else {
        format!("CAST(json_extract_string(fields, '$.{}') AS DOUBLE)", f)
    }
}

// ─── Query builder ───────────────────────────────────────────────────────────

fn build_query(q: &WidgetQuery) -> Result<String, AppError> {
    // Validate metric
    let allowed_metrics = ["count", "count_errors", "error_rate", "avg", "p50", "p95", "p99", "sum"];
    if !allowed_metrics.contains(&q.metric.as_str()) {
        return Err(AppError::BadRequest(format!("invalid metric: {}", q.metric)));
    }

    // Validate field for aggregate metrics
    let needs_field = matches!(q.metric.as_str(), "avg" | "p50" | "p95" | "p99" | "sum");
    let agg_field_expr = if needs_field {
        let f = q.field.as_deref()
            .ok_or_else(|| AppError::BadRequest("field is required for this metric".to_string()))?;
        validate_field_name(f)?;
        Some(field_num(f))
    } else {
        None
    };

    // Validate group_by
    if let Some(ref gb) = q.group_by {
        if gb.field != "timestamp" {
            validate_field_name(&gb.field)?;
        }
        if let Some(ref interval) = gb.interval {
            let allowed = ["1m", "5m", "30m", "1h", "1d"];
            if !allowed.contains(&interval.as_str()) {
                return Err(AppError::BadRequest(format!("invalid interval: {}", interval)));
            }
        }
    }

    // Validate filters
    for filter in &q.filters {
        validate_field_name(&filter.field)?;
        let allowed_ops = ["eq", "neq", "gt", "lt", "contains"];
        if !allowed_ops.contains(&filter.op.as_str()) {
            return Err(AppError::BadRequest(format!("invalid filter op: {}", filter.op)));
        }
        validate_value(&filter.value)?;
    }

    // Build metric expression (without alias)
    let metric_expr = match q.metric.as_str() {
        "count" => "COUNT(*)".to_string(),
        "count_errors" => "COUNT(*) FILTER (WHERE level IN ('error','fatal'))".to_string(),
        "error_rate" => {
            "ROUND(COUNT(*) FILTER (WHERE level IN ('error','fatal')) * 100.0 \
             / NULLIF(COUNT(*), 0), 2)".to_string()
        },
        "avg" => format!("ROUND(AVG({}), 2)", agg_field_expr.as_ref().unwrap()),
        "p50" => format!(
            "PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY {})",
            agg_field_expr.as_ref().unwrap()
        ),
        "p95" => format!(
            "PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY {})",
            agg_field_expr.as_ref().unwrap()
        ),
        "p99" => format!(
            "PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY {})",
            agg_field_expr.as_ref().unwrap()
        ),
        "sum" => format!("SUM({})", agg_field_expr.as_ref().unwrap()),
        _ => unreachable!(),
    };

    // Build WHERE clause
    let mut where_parts: Vec<String> = Vec::new();

    // Source filter (IN clause)
    if !q.sources.is_empty() {
        for s in &q.sources {
            validate_field_name(s)?;
        }
        let list: Vec<String> = q.sources.iter()
            .map(|s| format!("'{}'", escape_str(s)))
            .collect();
        where_parts.push(format!("source IN ({})", list.join(", ")));
    }

    if let Some(ref from_str) = q.from {
        validate_value(from_str)?;
        where_parts.push(format!(
            "CAST(timestamp AS TIMESTAMP) >= CAST('{}' AS TIMESTAMP)",
            escape_str(from_str)
        ));
    }
    if let Some(ref to_str) = q.to {
        validate_value(to_str)?;
        where_parts.push(format!(
            "CAST(timestamp AS TIMESTAMP) <= CAST('{}' AS TIMESTAMP)",
            escape_str(to_str)
        ));
    }

    for filter in &q.filters {
        let safe_val = escape_str(&filter.value);
        let cond = match filter.op.as_str() {
            "eq" => {
                if safe_val.is_empty() { continue; }
                format!("{} = '{}'", field_text(&filter.field), safe_val)
            },
            "neq" => {
                if safe_val.is_empty() { continue; }
                format!("{} != '{}'", field_text(&filter.field), safe_val)
            },
            "gt" => {
                if safe_val.is_empty() { continue; }
                safe_val.parse::<f64>().map_err(|_| {
                    AppError::BadRequest(format!("gt requires numeric value, got: {}", filter.value))
                })?;
                format!("{} > {}", field_num(&filter.field), safe_val)
            },
            "lt" => {
                if safe_val.is_empty() { continue; }
                safe_val.parse::<f64>().map_err(|_| {
                    AppError::BadRequest(format!("lt requires numeric value, got: {}", filter.value))
                })?;
                format!("{} < {}", field_num(&filter.field), safe_val)
            },
            "contains" => {
                if safe_val.is_empty() { continue; }
                format!("{} LIKE '%{}%'", field_text(&filter.field), safe_val)
            },
            _ => unreachable!(),
        };
        where_parts.push(cond);
    }

    let where_clause = if where_parts.is_empty() {
        "1=1".to_string()
    } else {
        where_parts.join(" AND ")
    };

    let limit = q.limit.unwrap_or(20).min(1000);

    // Build full query
    let sql = if let Some(ref gb) = q.group_by {
        let (group_key_expr, order_clause) = if gb.field == "timestamp" {
            let interval = gb.interval.as_deref().unwrap_or("1h");
            let trunc_expr = match interval {
                "1m" => "DATE_TRUNC('minute', CAST(timestamp AS TIMESTAMP))".to_string(),
                "5m" => "to_timestamp(floor(extract('epoch' from CAST(timestamp AS TIMESTAMP)) / 300) * 300)".to_string(),
                "30m" => "to_timestamp(floor(extract('epoch' from CAST(timestamp AS TIMESTAMP)) / 1800) * 1800)".to_string(),
                "1h" => "DATE_TRUNC('hour', CAST(timestamp AS TIMESTAMP))".to_string(),
                "1d" => "DATE_TRUNC('day', CAST(timestamp AS TIMESTAMP))".to_string(),
                _ => "DATE_TRUNC('hour', CAST(timestamp AS TIMESTAMP))".to_string(),
            };
            (
                format!("CAST({} AS VARCHAR)", trunc_expr),
                "ORDER BY group_key ASC".to_string(),
            )
        } else if is_direct_col(&gb.field) {
            (
                format!("CAST({} AS VARCHAR)", gb.field),
                "ORDER BY value DESC".to_string(),
            )
        } else {
            (
                format!("json_extract_string(fields, '$.{}')", gb.field),
                "ORDER BY value DESC".to_string(),
            )
        };

        format!(
            "SELECT {} as group_key, {} as value \
             FROM logs WHERE {} \
             GROUP BY group_key {} LIMIT {}",
            group_key_expr, metric_expr, where_clause, order_clause, limit
        )
    } else {
        format!(
            "SELECT NULL as group_key, {} as value FROM logs WHERE {}",
            metric_expr, where_clause
        )
    };

    Ok(sql)
}

// ─── Handlers ────────────────────────────────────────────────────────────────

async fn list_dashboards(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    match db.list_dashboards() {
        Ok(dashboards) => Ok((StatusCode::OK, axum::Json(dashboards))),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn get_dashboard(
    _user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    let dashboard = db.get_dashboard(&id)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;

    let widgets = db.list_widgets(&id)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((StatusCode::OK, axum::Json(json!({
        "id": dashboard.id,
        "name": dashboard.name,
        "description": dashboard.description,
        "created_by": dashboard.created_by,
        "created_at": dashboard.created_at,
        "updated_at": dashboard.updated_at,
        "widgets": widgets,
    }))))
}

async fn create_dashboard(
    user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(payload): Json<CreateDashboardReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    let name = payload.name.trim().to_string();
    if name.is_empty() || name.len() > 128 {
        return Err(AppError::BadRequest("invalid dashboard name".to_string()));
    }
    match db.create_dashboard(&name, payload.description.as_deref(), &user.id) {
        Ok(id) => Ok((StatusCode::CREATED, axum::Json(json!({"id": id, "name": name})))),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn update_dashboard(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<UpdateDashboardReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    let name = payload.name.trim().to_string();
    if name.is_empty() || name.len() > 128 {
        return Err(AppError::BadRequest("invalid dashboard name".to_string()));
    }
    match db.update_dashboard(&id, &name, payload.description.as_deref()) {
        Ok(true) => Ok((StatusCode::OK, axum::Json(json!({"message": "updated"})))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn delete_dashboard(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    match db.delete_dashboard(&id) {
        Ok(true) => Ok((StatusCode::OK, axum::Json(json!({"message": "deleted"})))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn create_widget(
    user: AuthUser,
    Path(dashboard_id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<CreateWidgetReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;

    let allowed_types = ["bar", "line", "number", "table"];
    if !allowed_types.contains(&payload.widget_type.as_str()) {
        return Err(AppError::BadRequest(format!("invalid widget type: {}", payload.widget_type)));
    }
    let allowed_widths = ["half", "full"];
    let width = payload.width.as_deref().unwrap_or("half");
    if !allowed_widths.contains(&width) {
        return Err(AppError::BadRequest(format!("invalid widget width: {}", width)));
    }

    // Verify dashboard exists
    db.get_dashboard(&dashboard_id)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;

    let position = payload.position.unwrap_or(0);

    match db.create_widget(&dashboard_id, &payload.title, &payload.widget_type, width, position, &payload.config) {
        Ok(id) => Ok((StatusCode::CREATED, axum::Json(json!({"id": id})))),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn update_widget(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<UpdateWidgetReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;

    let allowed_types = ["bar", "line", "number", "table"];
    if !allowed_types.contains(&payload.widget_type.as_str()) {
        return Err(AppError::BadRequest(format!("invalid widget type: {}", payload.widget_type)));
    }
    let allowed_widths = ["half", "full"];
    if !allowed_widths.contains(&payload.width.as_str()) {
        return Err(AppError::BadRequest(format!("invalid widget width: {}", payload.width)));
    }

    match db.update_widget(&id, &payload.title, &payload.widget_type, &payload.width, &payload.config) {
        Ok(true) => Ok((StatusCode::OK, axum::Json(json!({"message": "updated"})))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn update_widget_positions(
    user: AuthUser,
    Path(_dashboard_id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<Vec<WidgetPositionItem>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    let positions: Vec<(String, i32)> = payload.into_iter().map(|p| (p.id, p.position)).collect();
    match db.update_widget_positions(&positions) {
        Ok(()) => Ok((StatusCode::OK, axum::Json(json!({"message": "updated"})))),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn delete_widget(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    match db.delete_widget(&id) {
        Ok(true) => Ok((StatusCode::OK, axum::Json(json!({"message": "deleted"})))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

async fn query_widget(
    _user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(query): Json<WidgetQuery>,
) -> Result<impl IntoResponse, AppError> {
    let sql = build_query(&query)?;

    let raw_rows = db.query_raw_flexible(&sql)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let data: Vec<QueryDataPoint> = raw_rows.iter().map(|row| {
        let group_key = row.get("group_key").and_then(|v| match v {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        });
        let value = row.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        QueryDataPoint { group_key, value }
    }).collect();

    let total_rows = data.len();
    let group_by_field = query.group_by.as_ref().map(|g| g.field.clone());

    Ok((StatusCode::OK, axum::Json(json!({
        "data": data,
        "meta": {
            "metric": query.metric,
            "group_by": group_by_field,
            "total_rows": total_rows,
        }
    }))))
}
