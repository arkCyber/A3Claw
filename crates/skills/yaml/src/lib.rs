//! skill-yaml — flat YAML-style key-value parsing and generation (pure-Rust, no I/O).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.yaml",
  "name": "YAML Skills",
  "version": "0.1.0",
  "description": "Flat YAML key-value parsing, stringification, and merging",
  "skills": [
    {
      "name": "yaml.parse",
      "display": "Parse YAML",
      "description": "Parse a flat YAML string (key: value lines) into a JSON object.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "required": true }]
    },
    {
      "name": "yaml.stringify",
      "display": "Stringify YAML",
      "description": "Convert a flat JSON object into a YAML key: value string.",
      "risk": "safe",
      "params": [{ "name": "data", "type": "object", "required": true }]
    },
    {
      "name": "yaml.get",
      "display": "Get YAML Key",
      "description": "Parse YAML and return the value for a given key.",
      "risk": "safe",
      "params": [
        { "name": "text", "type": "string", "required": true },
        { "name": "key",  "type": "string", "required": true }
      ]
    },
    {
      "name": "yaml.set",
      "display": "Set YAML Key",
      "description": "Parse YAML, set or update a key, and return the modified YAML string.",
      "risk": "safe",
      "params": [
        { "name": "text",  "type": "string", "required": true },
        { "name": "key",   "type": "string", "required": true },
        { "name": "value", "type": "string", "required": true }
      ]
    },
    {
      "name": "yaml.keys",
      "display": "YAML Keys",
      "description": "Return all top-level keys from a flat YAML string.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "required": true }]
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
        "yaml.parse" => {
            let t = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            let obj = yaml_parse(t);
            sdk_respond_ok(rid, &serde_json::to_string(&obj).unwrap())
        }
        "yaml.stringify" => {
            let data = match req.args["data"].as_object() { Some(o) => o, None => return sdk_respond_err(rid, "missing object 'data'") };
            sdk_respond_ok(rid, &yaml_stringify(data))
        }
        "yaml.get" => {
            let t   = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            let key = match req.args["key"].as_str()  { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'")  };
            let obj = yaml_parse(t);
            match obj.get(key) {
                Some(v) => sdk_respond_ok(rid, &v.to_string()),
                None    => sdk_respond_ok(rid, "null"),
            }
        }
        "yaml.set" => {
            let t   = match req.args["text"].as_str()  { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            let key = match req.args["key"].as_str()   { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'")  };
            let val = match req.args["value"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'value'")};
            let mut obj = yaml_parse(t);
            obj.insert(key.to_string(), serde_json::Value::String(val.to_string()));
            sdk_respond_ok(rid, &yaml_stringify(&obj))
        }
        "yaml.keys" => {
            let t = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            let obj = yaml_parse(t);
            let keys: Vec<serde_json::Value> = obj.keys().map(|k| serde_json::Value::String(k.clone())).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&keys).unwrap())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── YAML logic ────────────────────────────────────────────────────────────────

fn yaml_parse(text: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_string();
            let val = line[colon+1..].trim().trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() {
                map.insert(key, serde_json::Value::String(val));
            }
        }
    }
    map
}

fn yaml_stringify(data: &serde_json::Map<String, serde_json::Value>) -> String {
    data.iter().map(|(k, v)| {
        let val = match v {
            serde_json::Value::String(s) => {
                if s.contains(':') || s.contains('#') || s.is_empty() {
                    format!("\"{}\"", s)
                } else { s.clone() }
            }
            other => other.to_string(),
        };
        format!("{}: {}", k, val)
    }).collect::<Vec<_>>().join("\n")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let obj = yaml_parse("name: Alice\nage: 30");
        assert_eq!(obj["name"], "Alice");
        assert_eq!(obj["age"], "30");
    }
    #[test]
    fn parse_skips_comments() {
        let obj = yaml_parse("# comment\nkey: val");
        assert!(!obj.contains_key("# comment"));
        assert_eq!(obj["key"], "val");
    }
    #[test]
    fn parse_skips_blank_lines() {
        let obj = yaml_parse("\nkey: val\n");
        assert_eq!(obj.len(), 1);
    }
    #[test]
    fn parse_quoted_value() {
        let obj = yaml_parse("greeting: \"hello world\"");
        assert_eq!(obj["greeting"], "hello world");
    }
    #[test]
    fn parse_empty() {
        let obj = yaml_parse("");
        assert!(obj.is_empty());
    }
    #[test]
    fn stringify_basic() {
        let mut m = serde_json::Map::new();
        m.insert("x".to_string(), serde_json::Value::String("1".to_string()));
        let s = yaml_stringify(&m);
        assert!(s.contains("x: 1"));
    }
    #[test]
    fn stringify_empty() {
        assert_eq!(yaml_stringify(&serde_json::Map::new()), "");
    }
    #[test]
    fn get_key() {
        let obj = yaml_parse("a: foo\nb: bar");
        assert_eq!(obj.get("a"), Some(&serde_json::Value::String("foo".to_string())));
    }
    #[test]
    fn get_missing_key() {
        let obj = yaml_parse("a: foo");
        assert!(obj.get("z").is_none());
    }
    #[test]
    fn set_key() {
        let mut obj = yaml_parse("x: 1");
        obj.insert("y".to_string(), serde_json::Value::String("2".to_string()));
        assert_eq!(obj["y"], "2");
    }
    #[test]
    fn roundtrip() {
        let mut m = serde_json::Map::new();
        m.insert("name".to_string(), serde_json::Value::String("Bob".to_string()));
        m.insert("city".to_string(), serde_json::Value::String("NYC".to_string()));
        let s = yaml_stringify(&m);
        let back = yaml_parse(&s);
        assert_eq!(back["name"], "Bob");
    }
    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.yaml");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("yaml."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
