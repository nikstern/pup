//! Token-efficient JSON compression for LLM agent mode.
//!
//! Schema extraction ported from rtk-ai/rtk (MIT License).
//! Source: https://github.com/rtk-ai/rtk/blob/main/src/json_cmd.rs
//! Copyright: Patrick Szymkowiak
//!
//! `compress_value` is a new addition: keeps real values but strips the fat
//! (nulls, long strings truncated, large arrays sampled) so the LLM sees
//! actionable data rather than type descriptors.

use serde_json::Value;

/// Tunable parameters for JSON compression.
#[derive(Clone)]
pub struct CompressConfig {
    /// Truncate strings longer than this many bytes (default: 200).
    pub string_trunc: usize,
    /// Max items to show from a top-level array, e.g. a list response (default: 20).
    pub array_items_top: usize,
    /// Max items to show from nested arrays, e.g. tags (default: 10).
    pub array_items_nested: usize,
    /// Optional projection applied to each top-level item before compression.
    /// For arrays, called on every element; for single objects, called on the object itself.
    /// Use this to keep only relevant fields for a specific command (e.g. monitors, logs).
    pub project: Option<fn(&Value) -> Value>,
}

impl Default for CompressConfig {
    fn default() -> Self {
        Self {
            string_trunc: 200,
            array_items_top: 20,
            array_items_nested: 10,
            project: None,
        }
    }
}

/// Compress a JSON string: project to relevant fields, strip nulls, truncate long strings,
/// sample large arrays. Returns compact (non-pretty) JSON so the caller controls formatting.
pub fn compress_json_string(json_str: &str, cfg: &CompressConfig) -> anyhow::Result<String> {
    let value: Value = serde_json::from_str(json_str)?;
    let projected = project_value(&value, cfg);
    let compressed = compress_value(&projected, 0, cfg);
    Ok(serde_json::to_string(&compressed)?)
}

/// Apply the projection function to each top-level item (or the item itself if not an array).
fn project_value(value: &Value, cfg: &CompressConfig) -> Value {
    let f = match cfg.project {
        Some(f) => f,
        None => return value.clone(),
    };
    match value {
        Value::Array(arr) => Value::Array(arr.iter().map(f).collect()),
        other => f(other),
    }
}

/// Keep only the specified top-level keys from an object. Passes non-objects through unchanged.
fn keep_fields(v: &Value, fields: &[&str]) -> Value {
    match v {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for &field in fields {
                if let Some(val) = map.get(field) {
                    out.insert(field.to_owned(), val.clone());
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Per-command projectors
// ---------------------------------------------------------------------------

/// Monitor: keep actionable fields, drop the bulky options object.
pub fn project_monitor(v: &Value) -> Value {
    keep_fields(
        v,
        &[
            "id",
            "name",
            "overall_state",
            "type",
            "query",
            "message",
            "tags",
            "creator",
            "modified",
        ],
    )
}

/// Log: flatten the nested attributes wrapper to a flat, scannable object.
pub fn project_log(v: &Value) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(id) = v.get("id") {
        out.insert("id".into(), id.clone());
    }
    if let Some(attrs) = v.get("attributes") {
        for &field in &["timestamp", "message", "service", "status", "host", "tags"] {
            if let Some(val) = attrs.get(field) {
                out.insert(field.into(), val.clone());
            }
        }
    }
    Value::Object(out)
}

/// Span: flatten attributes to a flat object, dropping the verbose `custom` bag.
pub fn project_span(v: &Value) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(id) = v.get("id") {
        out.insert("id".into(), id.clone());
    }
    if let Some(attrs) = v.get("attributes") {
        for &field in &[
            "trace_id",
            "span_id",
            "service",
            "operation_name",
            "resource_name",
            "status",
            "start_timestamp",
            "end_timestamp",
            "env",
            "host",
        ] {
            if let Some(val) = attrs.get(field) {
                out.insert(field.into(), val.clone());
            }
        }
        // Include error type if present, without the full stack trace.
        if let Some(err) = attrs.get("error") {
            if let Some(t) = err.get("type") {
                out.insert("error_type".into(), t.clone());
            }
        }
    }
    Value::Object(out)
}

/// Return the projection function for a given pup command string, if one exists.
pub fn projection_for_command(command: &str) -> Option<fn(&Value) -> Value> {
    match command {
        "monitors list" | "monitors get" => Some(project_monitor),
        "logs search" => Some(project_log),
        "traces search" | "traces aggregate" => Some(project_span),
        _ => None,
    }
}

fn compress_value(value: &Value, depth: u8, cfg: &CompressConfig) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(*b),
        Value::Number(n) => Value::Number(n.clone()),
        Value::String(s) => {
            if s.len() > cfg.string_trunc {
                Value::String(format!("{}...[{} chars]", &s[..cfg.string_trunc], s.len()))
            } else {
                Value::String(s.clone())
            }
        }
        Value::Array(arr) => {
            let limit = if depth == 0 {
                cfg.array_items_top
            } else {
                cfg.array_items_nested
            };
            let mut items: Vec<Value> = arr
                .iter()
                .take(limit)
                .map(|v| compress_value(v, depth + 1, cfg))
                .filter(|v| !v.is_null())
                .collect();
            if arr.len() > limit {
                items.push(Value::String(format!("... +{} more", arr.len() - limit)));
            }
            Value::Array(items)
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if v.is_null() {
                    continue;
                }
                let c = compress_value(v, depth + 1, cfg);
                if !c.is_null() {
                    out.insert(k.clone(), c);
                }
            }
            Value::Object(out)
        }
    }
}

// Schema extraction — ported verbatim from rtk-ai/rtk json_cmd.rs.
// Kept for potential future use (e.g. `pup --schema` flag).
#[allow(dead_code)]
pub fn filter_json_string(json_str: &str) -> anyhow::Result<String> {
    let value: Value = serde_json::from_str(json_str)?;
    Ok(extract_schema(&value, 0, 5))
}

#[allow(dead_code)]
fn extract_schema(value: &Value, depth: usize, max_depth: usize) -> String {
    let indent = "  ".repeat(depth);

    if depth > max_depth {
        return format!("{}...", indent);
    }

    match value {
        Value::Null => format!("{}null", indent),
        Value::Bool(_) => format!("{}bool", indent),
        Value::Number(n) => {
            if n.is_i64() {
                format!("{}int", indent)
            } else {
                format!("{}float", indent)
            }
        }
        Value::String(s) => {
            if s.len() > 50 {
                format!("{}string[{}]", indent, s.len())
            } else if s.is_empty() {
                format!("{}string", indent)
            } else if s.starts_with("http") {
                format!("{}url", indent)
            } else if s.contains('-') && s.len() == 10 {
                format!("{}date?", indent)
            } else {
                format!("{}string", indent)
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                format!("{}[]", indent)
            } else {
                let first_schema = extract_schema(&arr[0], depth + 1, max_depth);
                let trimmed = first_schema.trim();
                if arr.len() == 1 {
                    format!("{}[\n{}\n{}]", indent, first_schema, indent)
                } else {
                    format!("{}[{}] ({})", indent, trimmed, arr.len())
                }
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                return format!("{}{{}}", indent);
            }
            let mut lines = vec![format!("{}{{", indent)];
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();

            for (i, key) in keys.iter().enumerate() {
                let val = &map[*key];
                let val_schema = extract_schema(val, depth + 1, max_depth);
                let val_trimmed = val_schema.trim();

                let is_simple = matches!(
                    val,
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
                );

                if is_simple {
                    if i < keys.len() - 1 {
                        lines.push(format!("{}  {}: {},", indent, key, val_trimmed));
                    } else {
                        lines.push(format!("{}  {}: {}", indent, key, val_trimmed));
                    }
                } else {
                    lines.push(format!("{}  {}:", indent, key));
                    lines.push(val_schema);
                }

                if i >= 15 {
                    lines.push(format!("{}  ... +{} more keys", indent, keys.len() - i - 1));
                    break;
                }
            }
            lines.push(format!("{}}}", indent));
            lines.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- compress_value tests ---

    #[test]
    fn test_compress_drops_nulls() {
        let obj = serde_json::json!({"id": 1, "deleted": null, "name": "foo"});
        let c = compress_value(&obj, 0, &CompressConfig::default());
        let m = c.as_object().unwrap();
        assert!(m.contains_key("id"));
        assert!(m.contains_key("name"));
        assert!(!m.contains_key("deleted"));
    }

    #[test]
    fn test_compress_truncates_long_string() {
        let long = "x".repeat(300);
        let c = compress_value(&Value::String(long), 0, &CompressConfig::default());
        let s = c.as_str().unwrap();
        assert!(s.contains("...[300 chars]"));
        assert!(s.len() < 300);
    }

    #[test]
    fn test_compress_keeps_short_string() {
        let c = compress_value(&serde_json::json!("hello"), 0, &CompressConfig::default());
        assert_eq!(c.as_str().unwrap(), "hello");
    }

    #[test]
    fn test_compress_array_top_level_sampled() {
        let arr: Vec<Value> = (0..30).map(|i| serde_json::json!(i)).collect();
        let c = compress_value(&Value::Array(arr), 0, &CompressConfig::default());
        let items = c.as_array().unwrap();
        // 20 real items + 1 "+10 more" sentinel
        assert_eq!(items.len(), 21);
        assert_eq!(items.last().unwrap().as_str().unwrap(), "... +10 more");
    }

    #[test]
    fn test_compress_array_nested_sampled() {
        let arr: Vec<Value> = (0..15).map(|i| serde_json::json!(i)).collect();
        let c = compress_value(&Value::Array(arr), 1, &CompressConfig::default()); // depth=1 → nested limit
        let items = c.as_array().unwrap();
        assert_eq!(items.len(), 11); // 10 + sentinel
    }

    #[test]
    fn test_compress_array_within_limit() {
        let arr = serde_json::json!(["env:prod", "team:api"]);
        let c = compress_value(&arr, 1, &CompressConfig::default());
        assert_eq!(c.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_compress_preserves_real_values() {
        let obj = serde_json::json!({
            "id": 123,
            "name": "High CPU",
            "status": "Alert",
            "tags": ["env:prod", "team:api"],
        });
        let c = compress_value(&obj, 0, &CompressConfig::default());
        let m = c.as_object().unwrap();
        assert_eq!(m["id"], serde_json::json!(123));
        assert_eq!(m["name"].as_str().unwrap(), "High CPU");
        assert_eq!(m["status"].as_str().unwrap(), "Alert");
        assert_eq!(m["tags"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_compress_smaller_than_original() {
        // Lots of null fields + a long message → should compress well
        let json = r#"[{"id":1,"name":"High CPU","message":"CPU above 90% on host for 5 minutes. Check runaway processes or traffic spikes immediately. Alert will resolve when CPU drops below threshold for 3 consecutive minutes.","tags":["env:prod","team:api"],"status":"Alert","deleted":null,"draft":null,"restricted_roles":null,"creator":null,"priority":null,"org_id":null}]"#;
        let compressed = compress_json_string(json, &CompressConfig::default()).unwrap();
        assert!(
            compressed.len() < json.len(),
            "compressed ({}) should be smaller than original ({})",
            compressed.len(),
            json.len()
        );
    }

    #[test]
    fn test_compress_config_custom_values() {
        let cfg = CompressConfig {
            string_trunc: 10,
            array_items_top: 2,
            array_items_nested: 1,
            project: None,
        };
        // String truncation at 10 chars
        let c = compress_value(&Value::String("hello world!".into()), 0, &cfg);
        assert_eq!(c.as_str().unwrap(), "hello worl...[12 chars]");

        // Top-level array capped at 2
        let arr: Vec<Value> = (0..5).map(|i| serde_json::json!(i)).collect();
        let c = compress_value(&Value::Array(arr), 0, &cfg);
        assert_eq!(c.as_array().unwrap().len(), 3); // 2 items + sentinel

        // Nested array capped at 1
        let arr: Vec<Value> = (0..5).map(|i| serde_json::json!(i)).collect();
        let c = compress_value(&Value::Array(arr), 1, &cfg);
        assert_eq!(c.as_array().unwrap().len(), 2); // 1 item + sentinel
    }

    // --- projector tests ---

    #[test]
    fn test_project_monitor_keeps_key_fields() {
        let monitor = serde_json::json!({
            "id": 123,
            "name": "High CPU",
            "overall_state": "Alert",
            "type": "metric alert",
            "query": "avg(last_5m):avg:system.cpu.user{*} > 90",
            "message": "CPU is high",
            "tags": ["env:prod"],
            "creator": {"email": "a@b.com", "handle": "alice", "id": 9876},
            "modified": "2024-03-11",
            "options": {"avalanche_window": 10, "include_tags": true, "thresholds": {"critical": 90.0}},
            "org_id": 2,
            "multi": false,
            "matching_downtimes": [],
        });
        let projected = project_monitor(&monitor);
        let m = projected.as_object().unwrap();
        assert!(m.contains_key("id"));
        assert!(m.contains_key("name"));
        assert!(m.contains_key("overall_state"));
        assert!(m.contains_key("query"));
        assert!(m.contains_key("creator"));
        assert!(!m.contains_key("options"), "options should be stripped");
        assert!(!m.contains_key("org_id"), "org_id should be stripped");
        assert!(!m.contains_key("multi"), "multi should be stripped");
        assert!(
            !m.contains_key("matching_downtimes"),
            "matching_downtimes should be stripped"
        );
    }

    #[test]
    fn test_project_log_flattens_attributes() {
        let log = serde_json::json!({
            "id": "abc123",
            "type": "log",
            "attributes": {
                "timestamp": "2026-03-11T17:00:00Z",
                "message": "something failed",
                "service": "my-service",
                "status": "error",
                "host": "host-1",
                "tags": ["env:prod"],
                "attributes": {"caller": "main.go:42", "level": "ERROR"}
            }
        });
        let projected = project_log(&log);
        let m = projected.as_object().unwrap();
        assert_eq!(m["id"].as_str().unwrap(), "abc123");
        assert_eq!(m["message"].as_str().unwrap(), "something failed");
        assert_eq!(m["service"].as_str().unwrap(), "my-service");
        assert_eq!(m["status"].as_str().unwrap(), "error");
        assert_eq!(m["host"].as_str().unwrap(), "host-1");
        assert!(
            !m.contains_key("attributes"),
            "nested custom attributes should be stripped"
        );
        assert!(!m.contains_key("type"), "type wrapper should be stripped");
    }

    #[test]
    fn test_project_span_flattens_attributes() {
        let span = serde_json::json!({
            "id": "span1",
            "type": "spans",
            "attributes": {
                "trace_id": "abc",
                "span_id": "def",
                "service": "api",
                "operation_name": "http.request",
                "resource_name": "GET /health",
                "status": "error",
                "start_timestamp": "2026-03-11T17:00:00Z",
                "end_timestamp": "2026-03-11T17:00:01Z",
                "env": "prod",
                "host": "host-1",
                "error": {"type": "RuntimeError", "message": "oops", "stack": "long stack..."},
                "custom": {"lots": "of", "verbose": "fields"}
            }
        });
        let projected = project_span(&span);
        let m = projected.as_object().unwrap();
        assert_eq!(m["service"].as_str().unwrap(), "api");
        assert_eq!(m["status"].as_str().unwrap(), "error");
        assert_eq!(m["error_type"].as_str().unwrap(), "RuntimeError");
        assert!(!m.contains_key("custom"), "custom bag should be stripped");
    }

    #[test]
    fn test_projection_for_command_routing() {
        assert!(projection_for_command("monitors list").is_some());
        assert!(projection_for_command("monitors get").is_some());
        assert!(projection_for_command("logs search").is_some());
        assert!(projection_for_command("traces search").is_some());
        assert!(projection_for_command("traces aggregate").is_some());
        assert!(projection_for_command("dashboards list").is_none());
        assert!(projection_for_command("unknown").is_none());
    }

    #[test]
    fn test_compress_with_projection_via_config() {
        let monitor = serde_json::json!([{
            "id": 1,
            "name": "test",
            "overall_state": "OK",
            "type": "metric alert",
            "query": "avg(last_5m):avg:system.cpu.user{*} > 90",
            "message": "ok",
            "options": {"avalanche_window": 10, "thresholds": {"critical": 90.0}},
            "org_id": 2,
        }]);
        let cfg = CompressConfig {
            project: Some(project_monitor),
            ..Default::default()
        };
        let compressed =
            compress_json_string(&serde_json::to_string(&monitor).unwrap(), &cfg).unwrap();
        let result: Value = serde_json::from_str(&compressed).unwrap();
        let item = &result[0];
        assert!(item.get("id").is_some());
        assert!(
            item.get("options").is_none(),
            "options stripped by projector"
        );
        assert!(item.get("org_id").is_none(), "org_id stripped by projector");
    }

    // --- extract_schema tests (RTK port) ---

    #[test]
    fn test_extract_schema_primitives() {
        assert_eq!(extract_schema(&serde_json::json!(null), 0, 5), "null");
        assert_eq!(extract_schema(&serde_json::json!(true), 0, 5), "bool");
        assert_eq!(extract_schema(&serde_json::json!(42), 0, 5), "int");
        assert_eq!(extract_schema(&serde_json::json!(3.7), 0, 5), "float");
    }

    #[test]
    fn test_extract_schema_string_short() {
        assert_eq!(extract_schema(&serde_json::json!("hello"), 0, 5), "string");
    }

    #[test]
    fn test_extract_schema_string_long() {
        let long = "a".repeat(60);
        assert_eq!(extract_schema(&serde_json::json!(long), 0, 5), "string[60]");
    }

    #[test]
    fn test_extract_schema_string_url() {
        assert_eq!(
            extract_schema(&serde_json::json!("https://app.datadoghq.com"), 0, 5),
            "url"
        );
    }

    #[test]
    fn test_extract_schema_string_date() {
        assert_eq!(
            extract_schema(&serde_json::json!("2024-03-11"), 0, 5),
            "date?"
        );
    }

    #[test]
    fn test_extract_schema_empty_array() {
        assert_eq!(extract_schema(&serde_json::json!([]), 0, 5), "[]");
    }

    #[test]
    fn test_extract_schema_array_single() {
        let arr = serde_json::json!([42]);
        let result = extract_schema(&arr, 0, 5);
        assert!(result.contains("int"));
        assert!(result.starts_with('['));
    }

    #[test]
    fn test_extract_schema_array_multi() {
        let arr = serde_json::json!(["env:prod", "team:api", "service:web"]);
        assert_eq!(extract_schema(&arr, 0, 5), "[string] (3)");
    }

    #[test]
    fn test_extract_schema_object() {
        let obj = serde_json::json!({"id": 42, "name": "monitor"});
        let result = extract_schema(&obj, 0, 5);
        assert!(result.contains("id: int"));
        assert!(result.contains("name: string"));
    }

    #[test]
    fn test_extract_schema_object_nested() {
        let obj = serde_json::json!({
            "id": 1,
            "creator": {"email": "a@b.com", "handle": "alice"}
        });
        let result = extract_schema(&obj, 0, 5);
        assert!(result.contains("creator:"));
        assert!(result.contains("email: string"));
    }

    #[test]
    fn test_extract_schema_depth_limit() {
        let deep = serde_json::json!({"a": 1});
        let result = extract_schema(&deep, 0, 0);
        assert!(result.contains("a: ..."), "got: {result}");
    }

    #[test]
    fn test_filter_json_string_roundtrip() {
        let json = r#"{"name": "test", "count": 42, "tags": ["a", "b"]}"#;
        let result = filter_json_string(json).unwrap();
        assert!(result.contains("name: string"));
        assert!(result.contains("count: int"));
        assert!(result.contains("[string] (2)"));
    }

    #[test]
    fn test_filter_json_string_invalid() {
        assert!(filter_json_string("not json").is_err());
    }

    #[test]
    fn test_schema_smaller_than_original() {
        let json = r#"[{"id":123456,"name":"High CPU on prod","message":"CPU above 90% on host for 5 minutes. Please check for runaway processes or traffic spikes immediately.","tags":["env:production","team:platform","severity:high","service:web"],"status":"Alert","created":"2024-01-15","type":"metric alert"}]"#;
        let schema = filter_json_string(json).unwrap();
        assert!(
            schema.len() < json.len(),
            "schema ({}) should be smaller than json ({})",
            schema.len(),
            json.len()
        );
    }
}
