use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NormalizedLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub level: Option<String>,
    pub message: Option<String>,
    pub fields: Value,
    pub ingested_at: DateTime<Utc>,
}

pub fn normalize(mut payload: Value) -> NormalizedLog {
    let mut timestamp = None;
    let mut level = None;
    let mut message = None;

    if let Some(obj) = payload.as_object_mut() {
        for key in ["timestamp", "time", "ts", "@timestamp"] {
            if let Some(val) = obj.get(key) {
                if let Some(s) = val.as_str() {
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(s) {
                        timestamp = Some(ts.with_timezone(&Utc));
                        break;
                    }
                }
            }
        }

        for key in ["level", "severity", "lvl"] {
            if let Some(val) = obj.get(key) {
                if let Some(s) = val.as_str() {
                    level = Some(s.to_string());
                    break;
                }
            }
        }

        for key in ["message", "msg", "body"] {
            if let Some(val) = obj.get(key) {
                if let Some(s) = val.as_str() {
                    message = Some(s.to_string());
                    break;
                }
            }
        }
    }

    NormalizedLog {
        id: Uuid::new_v4().to_string(),
        timestamp: timestamp.unwrap_or_else(Utc::now),
        source: "default".to_string(), // Source from token in the future
        level,
        message,
        fields: payload,
        ingested_at: Utc::now(),
    }
}

pub fn normalize_batch(payload: Value) -> Vec<NormalizedLog> {
    match payload {
        Value::Array(arr) => arr.into_iter().map(normalize).collect(),
        Value::Object(_) => vec![normalize(payload)],
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
        let logs = normalize_batch(payload);
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
        let logs = normalize_batch(payload);
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].message.as_deref(), Some("ev 1"));
        assert_eq!(logs[2].message.as_deref(), Some("ev 3"));
    }

    #[test]
    fn test_normalize_batch_with_string_returns_empty_vec() {
        let payload = json!("a simple string without json structure");
        let logs = normalize_batch(payload);
        assert!(logs.is_empty());
    }

    #[test]
    fn test_normalize_extracts_msg_if_no_message() {
        let payload = json!({"msg": "fallback via msg", "level": "info"});
        let log = normalize(payload);
        assert_eq!(log.message.as_deref(), Some("fallback via msg"));
    }

    #[test]
    fn test_normalize_uses_now_if_no_valid_timestamp() {
        let payload = json!({"msg": "no time here"});
        
        let before = Utc::now();
        let log = normalize(payload);
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
        let log = normalize(payload);
        assert_eq!(log.message, None);
        assert_eq!(log.level, None);
    }

    #[test]
    fn test_normalize_ignores_invalid_date_format() {
        // 'timestamp' is present but cannot be parsed as RFC3339
        let payload = json!({"timestamp": "yesterday"});
        
        let before = Utc::now();
        let log = normalize(payload);
        let after = Utc::now();

        assert!(log.timestamp >= before && log.timestamp <= after);
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
        
        let log = normalize(payload);
        let expected_time = chrono::DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
        
        assert_eq!(log.timestamp, expected_time);
        assert_eq!(log.level.as_deref(), Some("info"));
        assert_eq!(log.message.as_deref(), Some("primary message"));
    }

    #[test]
    fn test_normalize_preserves_original_fields() {
        let payload = json!({
            "timestamp": "2023-01-01T00:00:00Z",
            "level": "info",
            "message": "hello",
            "extra_field": 123
        });
        
        let log = normalize(payload.clone());
        // The embedded 'fields' should be exactly the unchanged original payload
        assert_eq!(log.fields, payload);
    }

    #[test]
    fn test_normalize_batch_with_primitives_returns_empty_vec() {
        let null_payload = Value::Null;
        let bool_payload = json!(true);
        let number_payload = json!(100);
        
        assert!(normalize_batch(null_payload).is_empty());
        assert!(normalize_batch(bool_payload).is_empty());
        assert!(normalize_batch(number_payload).is_empty());
    }
}
