//! skill-csv — CSV parsing and generation skills (pure-Rust, no I/O).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.csv",
  "name": "CSV Skills",
  "version": "0.1.0",
  "description": "CSV parsing, generation, filtering, and column extraction",
  "skills": [
    {
      "name": "csv.parse",
      "display": "Parse CSV",
      "description": "Parse a CSV string into an array of row objects using the first row as headers.",
      "risk": "safe",
      "params": [
        { "name": "text",      "type": "string", "required": true  },
        { "name": "delimiter", "type": "string", "required": false }
      ]
    },
    {
      "name": "csv.stringify",
      "display": "Stringify CSV",
      "description": "Convert an array of objects to a CSV string.",
      "risk": "safe",
      "params": [
        { "name": "rows",      "type": "array",  "required": true  },
        { "name": "delimiter", "type": "string", "required": false }
      ]
    },
    {
      "name": "csv.column",
      "display": "Extract Column",
      "description": "Extract a single column from parsed CSV data as an array.",
      "risk": "safe",
      "params": [
        { "name": "rows", "type": "array",  "required": true },
        { "name": "name", "type": "string", "required": true }
      ]
    },
    {
      "name": "csv.filter",
      "display": "Filter CSV Rows",
      "description": "Filter rows where a given column equals a value.",
      "risk": "safe",
      "params": [
        { "name": "rows",  "type": "array",  "required": true },
        { "name": "key",   "type": "string", "required": true },
        { "name": "value", "type": "string", "required": true }
      ]
    },
    {
      "name": "csv.headers",
      "display": "Get CSV Headers",
      "description": "Return the list of column headers from an array of row objects.",
      "risk": "safe",
      "params": [{ "name": "rows", "type": "array", "required": true }]
    }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r) => r, Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();

    match req.skill.as_str() {
        "csv.parse" => {
            let text = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            let delim = req.args["delimiter"].as_str().unwrap_or(",");
            match csv_parse(text, delim) {
                Ok(rows) => sdk_respond_ok(rid, &serde_json::to_string(&rows).unwrap()),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "csv.stringify" => {
            let rows = match req.args["rows"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'rows'") };
            let delim = req.args["delimiter"].as_str().unwrap_or(",");
            sdk_respond_ok(rid, &csv_stringify(rows, delim))
        }
        "csv.column" => {
            let rows = match req.args["rows"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'rows'") };
            let name = match req.args["name"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'name'") };
            let col: Vec<serde_json::Value> = rows.iter().map(|r| r.get(name).cloned().unwrap_or(serde_json::Value::Null)).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&col).unwrap())
        }
        "csv.filter" => {
            let rows = match req.args["rows"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'rows'") };
            let key = match req.args["key"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'") };
            let val = match req.args["value"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'value'") };
            let filtered: Vec<&serde_json::Value> = rows.iter()
                .filter(|r| r.get(key).and_then(|v| v.as_str()) == Some(val))
                .collect();
            sdk_respond_ok(rid, &serde_json::to_string(&filtered).unwrap())
        }
        "csv.headers" => {
            let rows = match req.args["rows"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'rows'") };
            let headers: Vec<serde_json::Value> = rows.first()
                .and_then(|r| r.as_object())
                .map(|obj| obj.keys().map(|k| serde_json::Value::String(k.clone())).collect())
                .unwrap_or_default();
            sdk_respond_ok(rid, &serde_json::to_string(&headers).unwrap())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── CSV logic ─────────────────────────────────────────────────────────────────

fn csv_parse(text: &str, delim: &str) -> Result<Vec<serde_json::Value>, String> {
    let mut lines = text.lines();
    let header_line = lines.next().ok_or("empty CSV")?;
    let headers: Vec<&str> = header_line.split(delim).map(|s| s.trim_matches('"')).collect();
    let rows: Vec<serde_json::Value> = lines.map(|line| {
        let cells: Vec<&str> = line.split(delim).collect();
        let mut map = serde_json::Map::new();
        for (i, h) in headers.iter().enumerate() {
            let val = cells.get(i).map(|s| s.trim_matches('"')).unwrap_or("");
            map.insert(h.to_string(), serde_json::Value::String(val.to_string()));
        }
        serde_json::Value::Object(map)
    }).collect();
    Ok(rows)
}

fn csv_stringify(rows: &[serde_json::Value], delim: &str) -> String {
    if rows.is_empty() { return String::new(); }
    let headers: Vec<String> = rows[0].as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    let mut out = headers.join(delim);
    out.push('\n');
    for row in rows {
        let cells: Vec<String> = headers.iter().map(|h| {
            let v = row.get(h).map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }).unwrap_or_default();
            if v.contains(delim) || v.contains('"') || v.contains('\n') {
                format!("\"{}\"", v.replace('"', "\"\""))
            } else { v }
        }).collect();
        out.push_str(&cells.join(delim));
        out.push('\n');
    }
    out.trim_end_matches('\n').to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let rows = csv_parse("name,age\nAlice,30\nBob,25", ",").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["age"], "25");
    }
    #[test]
    fn parse_empty() { assert!(csv_parse("", ",").is_err()); }
    #[test]
    fn parse_headers_only() {
        let rows = csv_parse("a,b,c", ",").unwrap();
        assert!(rows.is_empty());
    }
    #[test]
    fn parse_tab_delimited() {
        let rows = csv_parse("x\ty\n1\t2", "\t").unwrap();
        assert_eq!(rows[0]["x"], "1");
    }
    #[test]
    fn stringify_basic() {
        let rows = vec![serde_json::json!({"name": "Alice", "age": "30"})];
        let s = csv_stringify(&rows, ",");
        assert!(s.contains("Alice"));
        assert!(s.contains("age") || s.contains("name"));
    }
    #[test]
    fn stringify_empty() { assert_eq!(csv_stringify(&[], ","), ""); }
    #[test]
    fn column_extraction() {
        let rows = vec![
            serde_json::json!({"a": "1", "b": "x"}),
            serde_json::json!({"a": "2", "b": "y"}),
        ];
        let col: Vec<serde_json::Value> = rows.iter().map(|r| r.get("a").cloned().unwrap()).collect();
        assert_eq!(col, vec![serde_json::Value::String("1".into()), serde_json::Value::String("2".into())]);
    }
    #[test]
    fn filter_rows() {
        let rows = vec![
            serde_json::json!({"type": "A", "v": "1"}),
            serde_json::json!({"type": "B", "v": "2"}),
            serde_json::json!({"type": "A", "v": "3"}),
        ];
        let filtered: Vec<_> = rows.iter().filter(|r| r.get("type").and_then(|v| v.as_str()) == Some("A")).collect();
        assert_eq!(filtered.len(), 2);
    }
    #[test]
    fn headers_from_rows() {
        let rows = vec![serde_json::json!({"x": 1, "y": 2})];
        let hdrs: Vec<String> = rows[0].as_object().unwrap().keys().cloned().collect();
        assert!(hdrs.contains(&"x".to_string()));
    }
    #[test]
    fn roundtrip() {
        let text = "name,score\nAlice,95\nBob,80";
        let rows = csv_parse(text, ",").unwrap();
        let out  = csv_stringify(&rows, ",");
        assert!(out.contains("Alice"));
        assert!(out.contains("95"));
    }
    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.csv");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("csv."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
