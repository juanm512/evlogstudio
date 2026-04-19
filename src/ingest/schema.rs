use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::ingest::normalize::NormalizedLog;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaEntry {
    pub source: String,
    pub field_path: String,
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seen_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn extract_field_paths(value: &Value, prefix: &str) -> Vec<(String, String)> {
    extract_field_paths_internal(value, prefix, 0)
}

fn extract_field_paths_internal(value: &Value, prefix: &str, depth: usize) -> Vec<(String, String)> {
    if depth >= 5 {
        return vec![];
    }

    let mut paths = Vec::new();

    if let Value::Object(obj) = value {
        for (k, v) in obj {
            if k.is_empty() {
                continue;
            }

            let new_prefix = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{}.{}", prefix, k)
            };

            let p_type = match v {
                Value::Null => "null",
                Value::Bool(_) => "boolean",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
            };

            paths.push((new_prefix.clone(), p_type.to_string()));

            if let Value::Object(_) = v {
                paths.extend(extract_field_paths_internal(v, &new_prefix, depth + 1));
            }
        }
    }

    paths
}

pub fn infer_schema(logs: &[NormalizedLog]) -> Vec<SchemaEntry> {
    let mut map: HashMap<(String, String), String> = HashMap::new();

    for log in logs {
        let paths = extract_field_paths(&log.fields, "");
        for (f_path, f_type) in paths {
            map.entry((log.source.clone(), f_path)).or_insert(f_type);
        }
    }

    map.into_iter()
        .map(|((source, field_path), field_type)| SchemaEntry {
            source,
            field_path,
            field_type,
            seen_count: None,
            last_seen: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_extract_field_paths_flat_object() {
        let val = json!({
            "name": "john",
            "age": 30,
            "active": true,
            "ref": null
        });
        let paths = extract_field_paths(&val, "");
        
        // Output order is not guaranteed because of HashMap/BTreeMap inside serde_json, 
        // though Value uses BTreeMap often, we should sort or just check contains.
        let mut expected = vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "number".to_string()),
            ("active".to_string(), "boolean".to_string()),
            ("ref".to_string(), "null".to_string()),
        ];
        
        let mut actual = paths.clone();
        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_extract_field_paths_nested_dot_notation() {
        let val = json!({
            "user": {
                "id": 1,
                "profile": {
                    "email": "test@test.com"
                }
            }
        });
        let mut paths = extract_field_paths(&val, "");
        paths.sort();
        
        let mut expected = vec![
            ("user".to_string(), "object".to_string()),
            ("user.id".to_string(), "number".to_string()),
            ("user.profile".to_string(), "object".to_string()),
            ("user.profile.email".to_string(), "string".to_string()),
        ];
        expected.sort();
        
        assert_eq!(expected, paths);
    }

    #[test]
    fn test_extract_field_paths_max_depth() {
        let val = json!({
            "a": {
                "b": {
                    "c": {
                        "d": {
                            "e": {
                                "f": 1
                            }
                        }
                    }
                }
            }
        });
        let paths = extract_field_paths(&val, "");
        
        let has_e = paths.iter().any(|(p, _)| p == "a.b.c.d.e");
        assert!(has_e, "Should reach depth 5 object 'e'");
        
        let has_f = paths.iter().any(|(p, _)| p.contains(".f"));
        assert!(!has_f, "Should not exceed depth 5");
    }

    #[test]
    fn test_extract_field_paths_arrays() {
        let val = json!({
            "tags": ["a", "b"],
            "users": [ { "id": 1 } ]
        });
        let mut paths = extract_field_paths(&val, "");
        paths.sort();
        
        let mut expected = vec![
            ("tags".to_string(), "array".to_string()),
            ("users".to_string(), "array".to_string()),
        ];
        expected.sort();
        assert_eq!(expected, paths);
    }

    #[test]
    fn test_infer_schema_deduplicates() {
        let log1 = NormalizedLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            source: "api".to_string(),
            service: None, environment: None, method: None, path: None,
            status: None, duration_ms: None, request_id: None, error: None,
            level: None,
            message: None,
            fields: json!({"status": 200, "user": "john"}),
            ingested_at: Utc::now(),
        };

        let log2 = NormalizedLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            source: "api".to_string(),
            service: None, environment: None, method: None, path: None,
            status: None, duration_ms: None, request_id: None, error: None,
            level: None,
            message: None,
            fields: json!({"status": 500, "duration": 15}),
            ingested_at: Utc::now(),
        };

        let schema = infer_schema(&[log1, log2]);
        assert_eq!(schema.len(), 3); // status, user, duration

        let status_entry = schema.iter().find(|s| s.field_path == "status").unwrap();
        assert_eq!(status_entry.field_type, "number");
        assert_eq!(status_entry.source, "api");
    }
}
