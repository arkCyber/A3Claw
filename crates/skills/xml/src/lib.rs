//! skill-xml — XML escaping, simple generation, and attribute parsing (pure-Rust).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.xml",
  "name": "XML Skills",
  "version": "0.1.0",
  "description": "XML escape/unescape, tag building, attribute parsing, and text extraction",
  "skills": [
    {
      "name": "xml.escape",
      "display": "XML Escape",
      "description": "Escape special characters for safe embedding in XML/HTML.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "required": true }]
    },
    {
      "name": "xml.unescape",
      "display": "XML Unescape",
      "description": "Decode XML/HTML entities back to plain text.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "required": true }]
    },
    {
      "name": "xml.build_tag",
      "display": "Build XML Tag",
      "description": "Build an XML element string from tag name, attributes, and inner text.",
      "risk": "safe",
      "params": [
        { "name": "tag",   "type": "string", "required": true  },
        { "name": "attrs", "type": "object", "required": false },
        { "name": "text",  "type": "string", "required": false }
      ]
    },
    {
      "name": "xml.strip_tags",
      "display": "Strip Tags",
      "description": "Remove all XML/HTML tags from a string, returning plain text.",
      "risk": "safe",
      "params": [{ "name": "text", "type": "string", "required": true }]
    },
    {
      "name": "xml.extract_attr",
      "display": "Extract Attribute",
      "description": "Extract the value of a named attribute from an XML tag string.",
      "risk": "safe",
      "params": [
        { "name": "tag",  "type": "string", "required": true },
        { "name": "attr", "type": "string", "required": true }
      ]
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
        "xml.escape" => {
            let t = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            sdk_respond_ok(rid, &xml_escape(t))
        }
        "xml.unescape" => {
            let t = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            sdk_respond_ok(rid, &xml_unescape(t))
        }
        "xml.build_tag" => {
            let tag = match req.args["tag"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'tag'") };
            let attrs = req.args["attrs"].as_object();
            let text = req.args["text"].as_str().unwrap_or("");
            sdk_respond_ok(rid, &build_tag(tag, attrs, text))
        }
        "xml.strip_tags" => {
            let t = match req.args["text"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
            sdk_respond_ok(rid, &strip_tags(t))
        }
        "xml.extract_attr" => {
            let tag  = match req.args["tag"].as_str()  { Some(s) => s, None => return sdk_respond_err(rid, "missing 'tag'")  };
            let attr = match req.args["attr"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'attr'") };
            sdk_respond_ok(rid, &extract_attr(tag, attr).unwrap_or_default())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── XML logic ─────────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

fn xml_unescape(s: &str) -> String {
    s.replace("&amp;",  "&")
     .replace("&lt;",   "<")
     .replace("&gt;",   ">")
     .replace("&quot;", "\"")
     .replace("&apos;", "'")
     .replace("&#39;",  "'")
}

fn build_tag(tag: &str, attrs: Option<&serde_json::Map<String, serde_json::Value>>, text: &str) -> String {
    let mut out = format!("<{}", tag);
    if let Some(map) = attrs {
        for (k, v) in map {
            let val = match v { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
            out.push_str(&format!(" {}=\"{}\"", k, xml_escape(&val)));
        }
    }
    if text.is_empty() {
        out.push_str(" />");
    } else {
        out.push('>');
        out.push_str(&xml_escape(text));
        out.push_str(&format!("</{}>", tag));
    }
    out
}

fn strip_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c   => if !in_tag { result.push(c); }
        }
    }
    result
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn escape_ampersand()  { assert_eq!(xml_escape("a&b"), "a&amp;b"); }
    #[test] fn escape_lt_gt()      { assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;"); }
    #[test] fn escape_quote()      { assert_eq!(xml_escape("say \"hi\""), "say &quot;hi&quot;"); }
    #[test] fn escape_apostrophe() { assert_eq!(xml_escape("it's"), "it&apos;s"); }
    #[test] fn escape_noop()       { assert_eq!(xml_escape("hello"), "hello"); }

    #[test] fn unescape_amp()   { assert_eq!(xml_unescape("a&amp;b"), "a&b"); }
    #[test] fn unescape_lt_gt() { assert_eq!(xml_unescape("&lt;tag&gt;"), "<tag>"); }
    #[test] fn unescape_quote() { assert_eq!(xml_unescape("&quot;hi&quot;"), "\"hi\""); }
    #[test] fn unescape_noop()  { assert_eq!(xml_unescape("hello"), "hello"); }

    #[test] fn build_self_closing() { assert_eq!(build_tag("br", None, ""), "<br />"); }
    #[test]
    fn build_with_text() {
        let s = build_tag("p", None, "hello");
        assert_eq!(s, "<p>hello</p>");
    }
    #[test]
    fn build_with_attr() {
        let mut m = serde_json::Map::new();
        m.insert("class".to_string(), serde_json::Value::String("foo".to_string()));
        let s = build_tag("div", Some(&m), "bar");
        assert!(s.contains("class=\"foo\""));
        assert!(s.contains("bar"));
    }

    #[test] fn strip_basic()  { assert_eq!(strip_tags("<b>bold</b>"), "bold"); }
    #[test] fn strip_nested() { assert_eq!(strip_tags("<div><p>hi</p></div>"), "hi"); }
    #[test] fn strip_noop()   { assert_eq!(strip_tags("plain"), "plain"); }

    #[test]
    fn extract_attr_basic() {
        assert_eq!(extract_attr("<a href=\"http://x.com\">", "href"), Some("http://x.com".to_string()));
    }
    #[test]
    fn extract_attr_missing() {
        assert_eq!(extract_attr("<a href=\"x\">", "class"), None);
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.xml");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("xml."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
