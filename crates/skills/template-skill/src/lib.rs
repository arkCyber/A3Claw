//! skill-template-skill — mustache-style string templating (pure-Rust, no I/O).
//!
//! Skills:
//!   template.render   { template: string, vars: object }  → rendered string
//!   template.extract  { template: string, text: string }  → extracted vars object
//!   template.list_vars { template: string }               → [varname, ...]

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.template",
  "name": "Template Skills",
  "version": "0.1.0",
  "description": "Mustache-style string templating: render, extract variables, list placeholders",
  "skills": [
    {
      "name": "template.render",
      "display": "Render Template",
      "description": "Replace {{var}} placeholders with values from the vars object.",
      "risk": "safe",
      "params": [
        { "name": "template", "type": "string", "required": true },
        { "name": "vars",     "type": "object", "required": true }
      ]
    },
    {
      "name": "template.list_vars",
      "display": "List Template Variables",
      "description": "Return a list of all {{var}} placeholder names in the template.",
      "risk": "safe",
      "params": [
        { "name": "template", "type": "string", "required": true }
      ]
    },
    {
      "name": "template.extract",
      "display": "Extract Template Variables",
      "description": "Given a template with {{var}} and a rendered text, extract the variable values.",
      "risk": "safe",
      "params": [
        { "name": "template", "type": "string", "required": true },
        { "name": "text",     "type": "string", "required": true }
      ]
    }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r)  => r,
        Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();

    match req.skill.as_str() {
        "template.render" => {
            let tmpl = match req.args["template"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'template'") };
            let vars = match req.args["vars"].as_object() { Some(o) => o, None => return sdk_respond_err(rid, "missing object 'vars'") };
            let rendered = render_template(tmpl, vars);
            sdk_respond_ok(rid, &rendered)
        }
        "template.list_vars" => {
            let tmpl = match req.args["template"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'template'") };
            let vars = list_vars(tmpl);
            sdk_respond_ok(rid, &serde_json::to_string(&vars).unwrap())
        }
        "template.extract" => {
            let tmpl = match req.args["template"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'template'") };
            let text = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            let extracted = extract_vars(tmpl, text);
            sdk_respond_ok(rid, &serde_json::to_string(&extracted).unwrap())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Template logic ────────────────────────────────────────────────────────────

fn render_template(tmpl: &str, vars: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut result = tmpl.to_string();
    for (k, v) in vars {
        let placeholder = format!("{{{{{}}}}}", k);
        let replacement = match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

fn list_vars(tmpl: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut rest = tmpl;
    while let Some(start) = rest.find("{{") {
        rest = &rest[start + 2..];
        if let Some(end) = rest.find("}}") {
            let name = rest[..end].trim().to_string();
            if !name.is_empty() && !vars.contains(&name) {
                vars.push(name);
            }
            rest = &rest[end + 2..];
        } else {
            break;
        }
    }
    vars
}

fn extract_vars(tmpl: &str, text: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    let vars = list_vars(tmpl);
    if vars.is_empty() { return map; }

    let mut segments: Vec<&str> = vec![tmpl];
    for var in &vars {
        let ph = format!("{{{{{}}}}}", var);
        let mut new_segs = Vec::new();
        for seg in &segments {
            let parts: Vec<&str> = seg.splitn(2, ph.as_str()).collect();
            new_segs.extend(parts);
        }
        segments = new_segs;
    }

    let mut pos = 0;
    let tbytes = text.as_bytes();
    for (i, var) in vars.iter().enumerate() {
        let prefix = segments[i];
        if !text[pos..].starts_with(prefix) { break; }
        pos += prefix.len();
        let next_prefix = if i + 1 < segments.len() { segments[i + 1] } else { "" };
        let end = if next_prefix.is_empty() {
            text.len()
        } else {
            match text[pos..].find(next_prefix) {
                Some(p) => pos + p,
                None    => { break; }
            }
        };
        if end <= text.len() && pos <= end {
            let val = std::str::from_utf8(&tbytes[pos..end]).unwrap_or("").to_string();
            map.insert(var.clone(), serde_json::Value::String(val));
            pos = end;
        }
    }
    map
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vars(pairs: &[(&str, &str)]) -> serde_json::Map<String, serde_json::Value> {
        pairs.iter().map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string()))).collect()
    }

    #[test]
    fn render_basic() {
        let vars = make_vars(&[("name", "World")]);
        assert_eq!(render_template("Hello, {{name}}!", &vars), "Hello, World!");
    }
    #[test]
    fn render_multiple_vars() {
        let vars = make_vars(&[("a", "foo"), ("b", "bar")]);
        assert_eq!(render_template("{{a}} and {{b}}", &vars), "foo and bar");
    }
    #[test]
    fn render_missing_var() {
        let vars = make_vars(&[]);
        assert_eq!(render_template("Hello, {{name}}!", &vars), "Hello, {{name}}!");
    }
    #[test]
    fn render_empty_template() {
        let vars = make_vars(&[("x", "1")]);
        assert_eq!(render_template("", &vars), "");
    }
    #[test]
    fn render_no_placeholders() {
        let vars = make_vars(&[("x", "1")]);
        assert_eq!(render_template("plain text", &vars), "plain text");
    }
    #[test]
    fn list_vars_basic() {
        let v = list_vars("Hello, {{name}}! You are {{age}} years old.");
        assert_eq!(v, vec!["name", "age"]);
    }
    #[test]
    fn list_vars_empty() {
        assert!(list_vars("no placeholders").is_empty());
    }
    #[test]
    fn list_vars_dedup() {
        let v = list_vars("{{x}} and {{x}}");
        assert_eq!(v.len(), 1);
    }
    #[test]
    fn list_vars_empty_template() {
        assert!(list_vars("").is_empty());
    }
    #[test]
    fn extract_basic() {
        let m = extract_vars("Hello, {{name}}!", "Hello, Alice!");
        assert_eq!(m["name"], "Alice");
    }
    #[test]
    fn extract_no_vars() {
        let m = extract_vars("plain", "plain");
        assert!(m.is_empty());
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.template");
        assert_eq!(v["skills"].as_array().unwrap().len(), 3);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("template."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
