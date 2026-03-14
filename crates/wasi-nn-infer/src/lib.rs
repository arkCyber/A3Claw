//! Pure-logic library extracted from the WASI-NN inference binary.
//!
//! This crate is compiled as both:
//! - A WASM binary (`src/main.rs`) targeting `wasm32-wasip1` with real
//!   WASI-NN host function bindings.
//! - A native library (`src/lib.rs`) so that unit tests for the pure-logic
//!   helpers (`parse_json_str`, `parse_req`, `json_escape`) can run on
//!   the host without requiring WasmEdge.

// ── JSON helpers ──────────────────────────────────────────────────────────────

/// Minimal JSON string extractor — no external dependencies needed inside WASM.
///
/// Finds `"key": value` in `json` and returns a `&str` slice of the value,
/// stripping surrounding quotes for string values.
pub fn parse_json_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let pos = json.find(needle.as_str())?;
    let after = &json[pos + needle.len()..];
    let colon = after.find(':')? + 1;
    let v = after[colon..].trim_start();
    if let Some(inner) = v.strip_prefix('"') {
        let end = inner.find('"')?;
        Some(&inner[..end])
    } else {
        let end = v
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(v.len());
        Some(&v[..end])
    }
}

// ── Inference request ─────────────────────────────────────────────────────────

/// Decoded inference request parsed from the host-supplied JSON file.
#[derive(Debug)]
pub struct InferReq {
    pub model_alias:  String,
    pub prompt:       String,
    pub n_predict:    u32,
    pub temperature:  f32,
    pub top_p:        f32,
    pub ctx_size:     u32,
    pub n_gpu_layers: i32,
}

/// Parse an `InferReq` from a JSON string.
///
/// Returns `Err` if the required `"prompt"` field is absent.
pub fn parse_req(json: &str) -> Result<InferReq, String> {
    Ok(InferReq {
        model_alias:  parse_json_str(json, "model").unwrap_or("default").to_string(),
        prompt:       parse_json_str(json, "prompt").ok_or("missing prompt")?.to_string(),
        n_predict:    parse_json_str(json, "n_predict").and_then(|v| v.parse().ok()).unwrap_or(512),
        temperature:  parse_json_str(json, "temperature").and_then(|v| v.parse().ok()).unwrap_or(0.7),
        top_p:        parse_json_str(json, "top_p").and_then(|v| v.parse().ok()).unwrap_or(0.9),
        ctx_size:     parse_json_str(json, "ctx_size").and_then(|v| v.parse().ok()).unwrap_or(4096),
        n_gpu_layers: parse_json_str(json, "n_gpu_layers").and_then(|v| v.parse().ok()).unwrap_or(0),
    })
}

// ── JSON output helpers ───────────────────────────────────────────────────────

/// Escape a string for embedding in a JSON value field.
pub fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"',  "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_json_str ────────────────────────────────────────────────────────

    #[test]
    fn parse_string_field() {
        let json = r#"{"model":"llama3","prompt":"hello world"}"#;
        assert_eq!(parse_json_str(json, "model"),  Some("llama3"));
        assert_eq!(parse_json_str(json, "prompt"), Some("hello world"));
    }

    #[test]
    fn parse_numeric_field() {
        let json = r#"{"n_predict":256,"temperature":0.8}"#;
        assert_eq!(parse_json_str(json, "n_predict"),   Some("256"));
        assert_eq!(parse_json_str(json, "temperature"), Some("0.8"));
    }

    #[test]
    fn parse_missing_field_returns_none() {
        let json = r#"{"model":"default"}"#;
        assert_eq!(parse_json_str(json, "prompt"), None);
    }

    #[test]
    fn parse_field_with_spaces() {
        let json = r#"{ "model" : "mistral" }"#;
        assert_eq!(parse_json_str(json, "model"), Some("mistral"));
    }

    // ── parse_req ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_req_minimal() {
        let json = r#"{"prompt":"Tell me a joke"}"#;
        let req = parse_req(json).expect("should parse");
        assert_eq!(req.prompt, "Tell me a joke");
        assert_eq!(req.model_alias, "default");
        assert_eq!(req.n_predict, 512);
        assert!((req.temperature - 0.7).abs() < 1e-5);
        assert!((req.top_p - 0.9).abs() < 1e-5);
        assert_eq!(req.ctx_size, 4096);
        assert_eq!(req.n_gpu_layers, 0);
    }

    #[test]
    fn parse_req_full() {
        let json = r#"{
            "model": "llama3",
            "prompt": "Hello!",
            "n_predict": 128,
            "temperature": 0.5,
            "top_p": 0.95,
            "ctx_size": 2048,
            "n_gpu_layers": 32
        }"#;
        let req = parse_req(json).expect("should parse");
        assert_eq!(req.model_alias, "llama3");
        assert_eq!(req.prompt, "Hello!");
        assert_eq!(req.n_predict, 128);
        assert!((req.temperature - 0.5).abs() < 1e-5);
        assert!((req.top_p - 0.95).abs() < 1e-5);
        assert_eq!(req.ctx_size, 2048);
        assert_eq!(req.n_gpu_layers, 32);
    }

    #[test]
    fn parse_req_missing_prompt_returns_err() {
        let json = r#"{"model":"default","n_predict":256}"#;
        let err = parse_req(json);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("prompt"));
    }

    #[test]
    fn parse_req_empty_json_returns_err() {
        assert!(parse_req("{}").is_err());
    }

    // ── json_escape ───────────────────────────────────────────────────────────

    #[test]
    fn escape_plain_string() {
        assert_eq!(json_escape("hello world"), "hello world");
    }

    #[test]
    fn escape_double_quotes() {
        assert_eq!(json_escape(r#"say "hello""#), r#"say \"hello\""#);
    }

    #[test]
    fn escape_backslash() {
        assert_eq!(json_escape("a\\b"), "a\\\\b");
    }

    #[test]
    fn escape_newline() {
        assert_eq!(json_escape("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn escape_tab() {
        assert_eq!(json_escape("col1\tcol2"), "col1\\tcol2");
    }

    #[test]
    fn escape_carriage_return() {
        assert_eq!(json_escape("a\rb"), "a\\rb");
    }

    #[test]
    fn escape_combined() {
        let input = "He said \"hello\"\nNew line\tTabbed\\slash";
        let expected = "He said \\\"hello\\\"\\nNew line\\tTabbed\\\\slash";
        assert_eq!(json_escape(input), expected);
    }

    // ── Full round-trip: parse → escape ───────────────────────────────────────

    #[test]
    fn roundtrip_prompt_with_special_chars() {
        let json = r#"{"prompt":"What is 2+2?\n\"Four\""}"#;
        let req = parse_req(json).expect("parse succeeded");
        let escaped = json_escape(&req.prompt);
        // Verify escaped output is valid for embedding in JSON
        assert!(!escaped.contains('\n'));
        assert!(!escaped.contains('"') || escaped.contains("\\\""));
    }

    #[test]
    fn response_ok_json_format() {
        let text = "The answer is 42.";
        let escaped = json_escape(text);
        let resp = format!("{{\"ok\":true,\"text\":\"{escaped}\"}}");
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("valid JSON");
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["text"], "The answer is 42.");
    }

    #[test]
    fn response_err_json_format() {
        let err = "load_by_name errno=5";
        let resp = format!("{{\"ok\":false,\"error\":{:?}}}", err);
        let parsed: serde_json::Value = serde_json::from_str(&resp).expect("valid JSON");
        assert_eq!(parsed["ok"], false);
        assert!(parsed["error"].as_str().unwrap().contains("errno=5"));
    }
}
