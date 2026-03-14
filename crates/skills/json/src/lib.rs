//! skill-json — JSON manipulation skills using serde_json.
//!
//! Skills exposed:
//!   json.validate  { text: string }                       → "valid" | "invalid: <reason>"
//!   json.format    { text: string, indent?: u32 }         → pretty-printed string
//!   json.minify    { text: string }                       → minified string
//!   json.get       { text: string, path: string }         → JSON value at dot-path
//!   json.set       { text: string, path: string, value: string } → updated JSON
//!   json.merge     { base: string, patch: string }        → merged JSON object
//!   json.keys      { text: string }                       → JSON array of top-level keys
//!   json.to_csv    { text: string }                       → CSV string (array of objects)

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.json",
  "name": "JSON Skills",
  "version": "0.1.0",
  "description": "JSON validation, formatting, path access, merging, and CSV conversion",
  "skills": [
    {
      "name": "json.validate",
      "display": "Validate JSON",
      "description": "Check if a string is valid JSON. Returns 'valid' or 'invalid: <reason>'.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "description": "JSON text to validate", "required": true }]
    },
    {
      "name": "json.format",
      "display": "Format JSON",
      "description": "Pretty-print JSON with configurable indent (default 2 spaces).",
      "risk": "safe",
      "params": [
        { "name": "text",   "type": "string",  "description": "JSON text",            "required": true },
        { "name": "indent", "type": "integer", "description": "Indent spaces (1-8)",  "required": false }
      ]
    },
    {
      "name": "json.minify",
      "display": "Minify JSON",
      "description": "Remove all insignificant whitespace from JSON.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "description": "JSON text", "required": true }]
    },
    {
      "name": "json.get",
      "display": "Get JSON Value",
      "description": "Extract a value at a dot-notation path (e.g. 'user.address.city').",
      "risk": "safe",
      "params": [
        { "name": "text", "type": "string", "description": "JSON text",   "required": true },
        { "name": "path", "type": "string", "description": "Dot path",    "required": true }
      ]
    },
    {
      "name": "json.set",
      "display": "Set JSON Value",
      "description": "Set a value at a dot-notation path. Value must be valid JSON.",
      "risk": "safe",
      "params": [
        { "name": "text",  "type": "string", "description": "JSON object text",         "required": true },
        { "name": "path",  "type": "string", "description": "Dot path to set",          "required": true },
        { "name": "value", "type": "string", "description": "JSON value to set",        "required": true }
      ]
    },
    {
      "name": "json.merge",
      "display": "Merge JSON Objects",
      "description": "Shallow-merge a JSON patch object into a base JSON object.",
      "risk": "safe",
      "params": [
        { "name": "base",  "type": "string", "description": "Base JSON object",  "required": true },
        { "name": "patch", "type": "string", "description": "Patch JSON object", "required": true }
      ]
    },
    {
      "name": "json.keys",
      "display": "Get JSON Keys",
      "description": "Return a JSON array of the top-level keys of a JSON object.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "description": "JSON object text", "required": true }]
    },
    {
      "name": "json.to_csv",
      "display": "JSON Array to CSV",
      "description": "Convert a JSON array of objects to CSV format.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "description": "JSON array of objects", "required": true }]
    }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 {
    sdk_export_str(MANIFEST)
}

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r)  => r,
        Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();

    match req.skill.as_str() {
        "json.validate" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(_)  => sdk_respond_ok(rid, "valid"),
                Err(e) => sdk_respond_ok(rid, &format!("invalid: {}", e)),
            }
        }
        "json.format" => {
            let text   = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            let indent = req.args["indent"].as_u64().unwrap_or(2).max(1).min(8) as usize;
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(v)  => sdk_respond_ok(rid, &format_json(&v, indent)),
                Err(e) => sdk_respond_err(rid, &format!("invalid JSON: {}", e)),
            }
        }
        "json.minify" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(v)  => sdk_respond_ok(rid, &v.to_string()),
                Err(e) => sdk_respond_err(rid, &format!("invalid JSON: {}", e)),
            }
        }
        "json.get" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            let path = match req.args["path"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'"),
            };
            let v: serde_json::Value = match serde_json::from_str(text) {
                Ok(v)  => v,
                Err(e) => return sdk_respond_err(rid, &format!("invalid JSON: {}", e)),
            };
            match json_get(&v, path) {
                Some(val) => sdk_respond_ok(rid, &val.to_string()),
                None      => sdk_respond_ok(rid, "null"),
            }
        }
        "json.set" => {
            let text  = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            let path  = match req.args["path"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'"),
            };
            let value = match req.args["value"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'value'"),
            };
            let mut v: serde_json::Value = match serde_json::from_str(text) {
                Ok(v)  => v,
                Err(e) => return sdk_respond_err(rid, &format!("invalid JSON: {}", e)),
            };
            let new_val: serde_json::Value = match serde_json::from_str(value) {
                Ok(v)  => v,
                Err(e) => return sdk_respond_err(rid, &format!("invalid value JSON: {}", e)),
            };
            json_set(&mut v, path, new_val);
            sdk_respond_ok(rid, &v.to_string())
        }
        "json.merge" => {
            let base  = match req.args["base"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'base'"),
            };
            let patch = match req.args["patch"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'patch'"),
            };
            let mut bv: serde_json::Value = match serde_json::from_str(base) {
                Ok(v)  => v,
                Err(e) => return sdk_respond_err(rid, &format!("invalid base JSON: {}", e)),
            };
            let pv: serde_json::Value = match serde_json::from_str(patch) {
                Ok(v)  => v,
                Err(e) => return sdk_respond_err(rid, &format!("invalid patch JSON: {}", e)),
            };
            match (bv.as_object_mut(), pv.as_object()) {
                (Some(bm), Some(pm)) => {
                    for (k, v) in pm { bm.insert(k.clone(), v.clone()); }
                    sdk_respond_ok(rid, &serde_json::Value::Object(bm.clone()).to_string())
                }
                _ => sdk_respond_err(rid, "both base and patch must be JSON objects"),
            }
        }
        "json.keys" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            let v: serde_json::Value = match serde_json::from_str(text) {
                Ok(v)  => v,
                Err(e) => return sdk_respond_err(rid, &format!("invalid JSON: {}", e)),
            };
            match v.as_object() {
                Some(m) => {
                    let keys: Vec<serde_json::Value> = m.keys().map(|k| serde_json::Value::String(k.clone())).collect();
                    sdk_respond_ok(rid, &serde_json::Value::Array(keys).to_string())
                }
                None => sdk_respond_err(rid, "input must be a JSON object"),
            }
        }
        "json.to_csv" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            let v: serde_json::Value = match serde_json::from_str(text) {
                Ok(v)  => v,
                Err(e) => return sdk_respond_err(rid, &format!("invalid JSON: {}", e)),
            };
            match json_to_csv(&v) {
                Ok(csv) => sdk_respond_ok(rid, &csv),
                Err(e)  => sdk_respond_err(rid, &e),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── JSON helpers ──────────────────────────────────────────────────────────────

fn format_json(v: &serde_json::Value, indent: usize) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
        .lines()
        .map(|line| {
            let spaces = line.len() - line.trim_start().len();
            let new_indent = (spaces / 2) * indent;
            format!("{}{}", " ".repeat(new_indent), line.trim_start())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn json_get<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for key in path.split('.') {
        cur = match cur {
            serde_json::Value::Object(m) => m.get(key)?,
            serde_json::Value::Array(a)  => {
                let idx: usize = key.parse().ok()?;
                a.get(idx)?
            }
            _ => return None,
        };
    }
    Some(cur)
}

fn json_set(v: &mut serde_json::Value, path: &str, new_val: serde_json::Value) {
    let parts: Vec<&str> = path.splitn(2, '.').collect();
    match parts.as_slice() {
        [key] => {
            if let serde_json::Value::Object(m) = v {
                m.insert(key.to_string(), new_val);
            }
        }
        [key, rest] => {
            if let serde_json::Value::Object(m) = v {
                let entry = m.entry(key.to_string()).or_insert(serde_json::Value::Object(Default::default()));
                json_set(entry, rest, new_val);
            }
        }
        _ => {}
    }
}

fn json_to_csv(v: &serde_json::Value) -> Result<String, String> {
    let arr = v.as_array().ok_or("input must be a JSON array")?;
    if arr.is_empty() { return Ok(String::new()); }
    let headers: Vec<String> = arr[0].as_object()
        .ok_or("array elements must be objects")?
        .keys().cloned().collect();
    let mut out = csv_row(&headers.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    for item in arr {
        let obj = item.as_object().ok_or("array elements must be objects")?;
        let row: Vec<String> = headers.iter().map(|h| {
            match obj.get(h) {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(v) => v.to_string(),
                None    => String::new(),
            }
        }).collect();
        out.push('\n');
        out.push_str(&csv_row(&row.iter().map(|s| s.as_str()).collect::<Vec<_>>()));
    }
    Ok(out)
}

fn csv_row(fields: &[&str]) -> String {
    fields.iter().map(|f| {
        if f.contains(',') || f.contains('"') || f.contains('\n') {
            format!("\"{}\"", f.replace('"', "\"\""))
        } else {
            f.to_string()
        }
    }).collect::<Vec<_>>().join(",")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let text = v.to_string();
        let v2: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(v2.is_object());
    }

    #[test]
    fn format_produces_pretty() {
        let v: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let pretty = format_json(&v, 2);
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn minify_removes_whitespace() {
        let v: serde_json::Value = serde_json::from_str("{\n  \"a\": 1\n}").unwrap();
        assert_eq!(v.to_string(), r#"{"a":1}"#);
    }

    #[test]
    fn get_nested() {
        let v: serde_json::Value = serde_json::from_str(r#"{"a":{"b":{"c":42}}}"#).unwrap();
        let result = json_get(&v, "a.b.c");
        assert_eq!(result, Some(&serde_json::Value::Number(42.into())));
    }

    #[test]
    fn get_missing_returns_none() {
        let v: serde_json::Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
        assert!(json_get(&v, "b").is_none());
    }

    #[test]
    fn set_top_level() {
        let mut v: serde_json::Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
        json_set(&mut v, "b", serde_json::Value::Number(2.into()));
        assert_eq!(v["b"], 2);
    }

    #[test]
    fn merge_shallow() {
        let mut base: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let patch: serde_json::Value = serde_json::from_str(r#"{"b":99,"c":3}"#).unwrap();
        if let (Some(bm), Some(pm)) = (base.as_object_mut(), patch.as_object()) {
            for (k, v) in pm { bm.insert(k.clone(), v.clone()); }
        }
        assert_eq!(base["b"], 99);
        assert_eq!(base["c"], 3);
        assert_eq!(base["a"], 1);
    }

    #[test]
    fn keys_returns_all() {
        let v: serde_json::Value = serde_json::from_str(r#"{"x":1,"y":2,"z":3}"#).unwrap();
        let mut keys: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec!["x", "y", "z"]);
    }

    #[test]
    fn to_csv_basic() {
        let v: serde_json::Value = serde_json::from_str(r#"[{"name":"Alice","age":30},{"name":"Bob","age":25}]"#).unwrap();
        let csv = json_to_csv(&v).unwrap();
        assert!(csv.contains("name,age") || csv.contains("age,name"));
        assert!(csv.contains("Alice"));
        assert!(csv.contains("Bob"));
    }

    #[test]
    fn to_csv_empty() {
        let v: serde_json::Value = serde_json::from_str("[]").unwrap();
        assert_eq!(json_to_csv(&v).unwrap(), "");
    }

    // ── validate ───────────────────────────────────────────────────────────────
    #[test]
    fn is_valid_json_object()   { assert!(serde_json::from_str::<serde_json::Value>(r#"{"a":1}"#).is_ok()); }
    #[test]
    fn is_valid_json_array()    { assert!(serde_json::from_str::<serde_json::Value>("[1,2,3]").is_ok()); }
    #[test]
    fn is_valid_json_string()   { assert!(serde_json::from_str::<serde_json::Value>("\"hello\"").is_ok()); }
    #[test]
    fn is_invalid_json_bare_word() { assert!(serde_json::from_str::<serde_json::Value>("notjson").is_err()); }
    #[test]
    fn is_invalid_json_trailing_comma() { assert!(serde_json::from_str::<serde_json::Value>("{\"a\":1,}").is_err()); }

    // ── get nested ─────────────────────────────────────────────────────────────
    #[test]
    fn get_nested_path() {
        let v: serde_json::Value = serde_json::from_str(r#"{"a":{"b":42}}"#).unwrap();
        let got = json_get(&v, "a.b");
        assert!(got.is_some());
        assert_eq!(got.unwrap(), &serde_json::Value::Number(42.into()));
    }
    #[test]
    fn get_array_index() {
        let v: serde_json::Value = serde_json::from_str(r#"{"arr":[10,20,30]}"#).unwrap();
        let got = json_get(&v, "arr");
        assert!(got.is_some());
        assert_eq!(got.unwrap()[1], 20);
    }

    // ── set / merge ────────────────────────────────────────────────────────────
    #[test]
    fn set_overwrites_existing() {
        let mut v: serde_json::Value = serde_json::from_str(r#"{"x":1}"#).unwrap();
        json_set(&mut v, "x", serde_json::json!(99));
        assert_eq!(v["x"], 99);
    }
    #[test]
    fn merge_does_not_lose_original_keys() {
        let mut base: serde_json::Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
        let patch: serde_json::Value = serde_json::from_str(r#"{"b":2}"#).unwrap();
        if let (Some(bm), Some(pm)) = (base.as_object_mut(), patch.as_object()) {
            for (k,v) in pm { bm.insert(k.clone(), v.clone()); }
        }
        assert_eq!(base["a"], 1);
        assert_eq!(base["b"], 2);
    }

    // ── keys ─────────────────────────────────────────────────────────────────────
    #[test]
    fn keys_empty_object() {
        let v: serde_json::Value = serde_json::from_str("{}").unwrap();
        let keys: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        assert!(keys.is_empty());
    }
    #[test]
    fn keys_single() {
        let v: serde_json::Value = serde_json::from_str(r#"{"only":true}"#).unwrap();
        let keys: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, vec!["only"]);
    }

    // ── to_csv ────────────────────────────────────────────────────────────────
    #[test]
    fn to_csv_single_row() {
        let v = serde_json::json!([{"id":1,"val":"x"}]);
        let csv = json_to_csv(&v).unwrap();
        assert!(csv.contains('\n'));
    }
    #[test]
    fn to_csv_non_array_returns_err() {
        let v = serde_json::json!({"a":1});
        assert!(json_to_csv(&v).is_err());
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.json");
        assert_eq!(v["skills"].as_array().unwrap().len(), 8);
    }
    #[test]
    fn manifest_skill_names_start_with_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("json."));
        }
    }
}
