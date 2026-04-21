use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub service: Option<String>,
    pub environment: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<i32>,
    pub duration: Option<i32>,
    pub request_id: Option<String>,
    pub error: Option<String>,
    pub level: Option<String>,
    pub message: Option<String>,
    pub fields: Value,
    pub ingested_at: DateTime<Utc>,
}

fn extract_str(obj: &mut serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(val) = obj.remove(*key) {
            if let Some(s) = val.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn parse_duration(value: Value) -> Option<i32> {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() { Some(i as i32) }
            else { n.as_f64().map(|f| f as i32) }
        }
        Value::String(s) => {
            let s = s.trim().to_lowercase();
            if s.ends_with("ms") {
                s.strip_suffix("ms")?.trim().parse::<f64>().ok().map(|f| f as i32)
            } else if s.ends_with('s') {
                s.strip_suffix('s')?.trim().parse::<f64>().ok().map(|f| (f * 1000.0) as i32)
            } else if s.ends_with('m') {
                s.strip_suffix('m')?.trim().parse::<f64>().ok().map(|f| (f * 60000.0) as i32)
            } else {
                s.parse::<f64>().ok().map(|f| f as i32)
            }
        }
        _ => None,
    }
}

fn extract_i32(obj: &mut serde_json::Map<String, Value>, keys: &[&str]) -> Option<i32> {
    for key in keys {
        if let Some(val) = obj.remove(*key) {
            if let Some(n) = val.as_i64() { return Some(n as i32); }
            if let Some(f) = val.as_f64() { return Some(f as i32); }
            if let Some(s) = val.as_str() {
                if let Ok(ms) = s.parse::<i32>() { return Some(ms); }
            }
        }
    }
    None
}

pub fn normalize(mut payload: Value, source: &str) -> NormalizedLog {
    let mut timestamp = None;
    let mut level = None;
    let mut message = None;
    let mut service = None;
    let mut environment = None;
    let mut method = None;
    let mut path = None;
    let mut status: Option<i32> = None;
    let mut duration: Option<i32> = None;
    let mut request_id = None;
    let mut error = None;

    if let Some(obj) = payload.as_object_mut() {
        for key in ["timestamp", "time", "ts", "@timestamp"] {
            if let Some(val) = obj.remove(key) {
                if let Some(s) = val.as_str() {
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(s) {
                        timestamp = Some(ts.with_timezone(&Utc));
                        break;
                    }
                }
            }
        }

        for key in ["level", "severity", "lvl"] {
            if let Some(val) = obj.remove(key) {
                if let Some(s) = val.as_str() {
                    level = Some(s.to_string());
                    break;
                }
            }
        }

        for key in ["message", "msg", "body"] {
            if let Some(val) = obj.remove(key) {
                if let Some(s) = val.as_str() {
                    message = Some(s.to_string());
                    break;
                }
            }
        }

        service     = extract_str(obj, &["service", "serviceName", "service_name"]);
        environment = extract_str(obj, &["environment", "env"]);
        method      = extract_str(obj, &["method", "httpMethod", "http_method"]);
        path        = extract_str(obj, &["path", "url", "pathname", "route"]);
        status      = extract_i32(obj, &["status", "statusCode", "status_code"]);
        duration    = obj.remove("duration")
            .or_else(|| obj.remove("duration_ms"))
            .or_else(|| obj.remove("durationMs"))
            .and_then(parse_duration);
        request_id  = extract_str(obj, &["request_id", "requestId", "req_id", "traceId"]);

        if let Some(err_val) = obj.remove("error") {
            error = match err_val {
                Value::String(s) => Some(s.clone()),
                Value::Null => None,
                other => Some(other.to_string()),
            };
        }
    }

    NormalizedLog {
        id: Uuid::new_v4().to_string(),
        timestamp: timestamp.unwrap_or_else(Utc::now),
        source: source.to_string(),
        service,
        environment,
        method,
        path,
        status,
        duration,
        request_id,
        error,
        level,
        message,
        fields: payload,
        ingested_at: Utc::now(),
    }
}

pub fn normalize_batch(payload: Value, source: &str) -> Vec<NormalizedLog> {
    match payload {
        Value::Array(arr) => arr.into_iter().map(|p| normalize(p, source)).collect(),
        Value::Object(_) => vec![normalize(payload, source)],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_batch_with_object_returns_1_log() {
        let payload = json!({"message": "test event"});
        let logs = normalize_batch(payload, "default");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message.as_deref(), Some("test event"));
    }

    #[test]
    fn test_normalize_batch_with_array_of_3_returns_3_logs() {
        let payload = json!([
            {"message": "ev 1"},
            {"message": "ev 2"},
            {"message": "ev 3"}
        ]);
        let logs = normalize_batch(payload, "default");
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].message.as_deref(), Some("ev 1"));
        assert_eq!(logs[2].message.as_deref(), Some("ev 3"));
    }

    #[test]
    fn test_normalize_batch_with_string_returns_empty_vec() {
        let payload = json!("a simple string without json structure");
        let logs = normalize_batch(payload, "default");
        assert!(logs.is_empty());
    }

    #[test]
    fn test_normalize_extracts_msg_if_no_message() {
        let payload = json!({"msg": "fallback via msg", "level": "info"});
        let log = normalize(payload, "default");
        assert_eq!(log.message.as_deref(), Some("fallback via msg"));
    }

    #[test]
    fn test_normalize_uses_now_if_no_valid_timestamp() {
        let payload = json!({"msg": "no time here"});
        
        let before = Utc::now();
        let log = normalize(payload, "default");
        let after = Utc::now();

        // The timestamp must be between the start and end of the test execution
        assert!(log.timestamp >= before && log.timestamp <= after);
    }

    #[test]
    fn test_normalize_ignores_invalid_data_types() {
        // 'message' is an object instead of a string, 'level' is a number
        let payload = json!({
            "message": {"error": "timeout"}, 
            "level": 50
        });
        let log = normalize(payload, "default");
        assert_eq!(log.message, None);
        assert_eq!(log.level, None);
    }

    #[test]
    fn test_normalize_ignores_invalid_date_format() {
        // 'timestamp' is present but cannot be parsed as RFC3339
        let payload = json!({"timestamp": "yesterday"});
        
        let before = Utc::now();
        let log = normalize(payload, "default");
        let after = Utc::now();

        assert!(log.timestamp >= before && log.timestamp <= after);
    }

    #[test]
    fn test_normalize_extracts_timestamp_iso8601_correctly() {
        let payload = json!({"timestamp": "2023-10-15T12:00:00Z"});
        let log = normalize(payload, "default");
        let expected = chrono::DateTime::parse_from_rfc3339("2023-10-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(log.timestamp, expected);
    }

    #[test]
    fn test_normalize_respects_key_precedence() {
        // It should pick 'timestamp' over 'time', 'level' over 'severity', 'message' over 'msg'
        let payload = json!({
            "timestamp": "2023-01-01T00:00:00Z",
            "time": "2024-01-01T00:00:00Z",
            "level": "info",
            "severity": "debug",
            "message": "primary message",
            "msg": "secondary message"
        });
        
        let log = normalize(payload, "default");
        let expected_time = chrono::DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
        
        assert_eq!(log.timestamp, expected_time);
        assert_eq!(log.level.as_deref(), Some("info"));
        assert_eq!(log.message.as_deref(), Some("primary message"));
    }

    #[test]
    fn test_normalize_removes_extracted_fields_from_json() {
        let payload = json!({
            "timestamp": "2023-01-01T00:00:00Z",
            "level": "info",
            "message": "hello",
            "extra_field": 123
        });
        
        let log = normalize(payload.clone(), "default");
        // Extracted fields should be removed from the 'fields' JSON blob
        assert_eq!(log.fields, json!({"extra_field": 123}));
        assert_eq!(log.level.as_deref(), Some("info"));
        assert_eq!(log.message.as_deref(), Some("hello"));
    }

    #[test]
    fn test_normalize_batch_with_primitives_returns_empty_vec() {
        let null_payload = Value::Null;
        let bool_payload = json!(true);
        let number_payload = json!(100);
        
        assert!(normalize_batch(null_payload, "default").is_empty());
        assert!(normalize_batch(bool_payload, "default").is_empty());
        assert!(normalize_batch(number_payload, "default").is_empty());
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration(json!(1250)), Some(1250));
        assert_eq!(parse_duration(json!("1250")), Some(1250));
        assert_eq!(parse_duration(json!("1250ms")), Some(1250));
        assert_eq!(parse_duration(json!("1.25s")), Some(1250));
        assert_eq!(parse_duration(json!("1.5m")), Some(90000));
        assert_eq!(parse_duration(json!("abc")), None);
        assert_eq!(parse_duration(json!(null)), None);
    }
}
