//! WASM plugin ABI — shared JSON types used by both host and guest.
//!
//! ## Wire protocol
//!
//! All data crosses the WASM boundary as UTF-8 JSON, passed via a
//! `(ptr: i32, len: i32)` pair pointing into the guest's linear memory.
//!
//! ### Guest exports (host calls these)
//! ```text
//! skill_manifest() -> (ptr: i32, len: i32)
//!   Returns a JSON-encoded SkillManifest.
//!
//! skill_execute(req_ptr: i32, req_len: i32) -> (ptr: i32, len: i32)
//!   Accepts a JSON-encoded ExecuteRequest, returns ExecuteResponse.
//!
//! alloc(size: i32) -> ptr: i32
//!   Allocate `size` bytes in guest memory. Host writes request here.
//!
//! dealloc(ptr: i32, size: i32)
//!   Release a previous allocation (optional, for long-lived plugins).
//! ```
//!
//! ### Host imports (guest calls these, optional)
//! ```text
//! host_log(level_ptr: i32, level_len: i32, msg_ptr: i32, msg_len: i32)
//! host_http_fetch(req_ptr: i32, req_len: i32) -> (ptr: i32, len: i32)
//! ```

use serde::{Deserialize, Serialize};

// ── Manifest ──────────────────────────────────────────────────────────────────

/// Returned by `skill_manifest()`. Describes all skills provided by the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Unique plugin identifier, e.g. `"my-org.weather"`.
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version string.
    pub version: String,
    /// Short description of the plugin.
    pub description: String,
    /// Skills provided by this plugin.
    pub skills: Vec<SkillDef>,
}

/// Definition of a single skill provided by a WASM plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    /// Skill name in dot-notation, e.g. `"weather.current"`.
    pub name: String,
    /// Human-readable display name.
    pub display: String,
    /// Short description for LLM tool-call schema.
    pub description: String,
    /// Risk classification: `"safe"`, `"confirm"`, or `"deny"`.
    pub risk: String,
    /// Parameter definitions for LLM tool-call schema generation.
    #[serde(default)]
    pub params: Vec<ParamDef>,
}

/// Single parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub required: bool,
}

// ── Execute request / response ────────────────────────────────────────────────

/// Sent by the host to the guest via `skill_execute`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    /// Skill name to execute (must be listed in the manifest).
    pub skill: String,
    /// JSON arguments for the skill.
    pub args: serde_json::Value,
    /// Opaque request ID for correlation (passed back in response).
    #[serde(default)]
    pub request_id: String,
}

/// Returned by the guest from `skill_execute`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    /// Echoed from request.
    pub request_id: String,
    /// `true` on success, `false` on error.
    pub ok: bool,
    /// Text observation to return to the LLM. Set on success.
    #[serde(default)]
    pub output: String,
    /// Error message. Set when `ok == false`.
    #[serde(default)]
    pub error: String,
}

impl ExecuteResponse {
    pub fn ok(request_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            ok: true,
            output: output.into(),
            error: String::new(),
        }
    }

    pub fn err(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            ok: false,
            output: String::new(),
            error: error.into(),
        }
    }
}

// ── Host-import HTTP types (optional) ────────────────────────────────────────

/// Request passed to the `host_http_fetch` import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHttpRequest {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

/// Response returned from the `host_http_fetch` import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHttpResponse {
    pub status: u16,
    pub body: String,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manifest() -> SkillManifest {
        SkillManifest {
            id: "my-org.weather".into(),
            name: "Weather Plugin".into(),
            version: "1.2.3".into(),
            description: "Provides weather data".into(),
            skills: vec![
                SkillDef {
                    name: "weather.current".into(),
                    display: "Current Weather".into(),
                    description: "Get current weather".into(),
                    risk: "safe".into(),
                    params: vec![
                        ParamDef {
                            name: "city".into(),
                            description: "City name".into(),
                            param_type: "string".into(),
                            required: true,
                        },
                    ],
                },
                SkillDef {
                    name: "weather.forecast".into(),
                    display: "Weather Forecast".into(),
                    description: "Get 5-day forecast".into(),
                    risk: "safe".into(),
                    params: vec![],
                },
            ],
        }
    }

    // ── SkillManifest ─────────────────────────────────────────────────────────

    #[test]
    fn manifest_json_roundtrip() {
        let m = make_manifest();
        let json = serde_json::to_string(&m).unwrap();
        let decoded: SkillManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, m.id);
        assert_eq!(decoded.name, m.name);
        assert_eq!(decoded.version, m.version);
        assert_eq!(decoded.skills.len(), 2);
    }

    #[test]
    fn manifest_skills_count() {
        let m = make_manifest();
        assert_eq!(m.skills.len(), 2);
        assert_eq!(m.skills[0].name, "weather.current");
        assert_eq!(m.skills[1].name, "weather.forecast");
    }

    #[test]
    fn manifest_version_string_preserved() {
        let m = make_manifest();
        assert_eq!(m.version, "1.2.3");
    }

    #[test]
    fn manifest_empty_skills_roundtrip() {
        let m = SkillManifest {
            id: "empty.plugin".into(),
            name: "Empty".into(),
            version: "0.0.1".into(),
            description: "No skills".into(),
            skills: vec![],
        };
        let json = serde_json::to_string(&m).unwrap();
        let d: SkillManifest = serde_json::from_str(&json).unwrap();
        assert!(d.skills.is_empty());
    }

    // ── SkillDef ──────────────────────────────────────────────────────────────

    #[test]
    fn skill_def_risk_levels_preserved() {
        for risk in &["safe", "confirm", "deny"] {
            let def = SkillDef {
                name: "test.skill".into(),
                display: "Test".into(),
                description: "desc".into(),
                risk: risk.to_string(),
                params: vec![],
            };
            let json = serde_json::to_string(&def).unwrap();
            let d: SkillDef = serde_json::from_str(&json).unwrap();
            assert_eq!(&d.risk, risk);
        }
    }

    #[test]
    fn skill_def_params_default_empty() {
        let json = r#"{"name":"x","display":"X","description":"d","risk":"safe"}"#;
        let d: SkillDef = serde_json::from_str(json).unwrap();
        assert!(d.params.is_empty(), "params should default to empty vec");
    }

    // ── ParamDef ──────────────────────────────────────────────────────────────

    #[test]
    fn param_def_required_flag_preserved() {
        let p = ParamDef {
            name: "city".into(),
            description: "City name".into(),
            param_type: "string".into(),
            required: true,
        };
        let json = serde_json::to_string(&p).unwrap();
        let d: ParamDef = serde_json::from_str(&json).unwrap();
        assert!(d.required);
    }

    #[test]
    fn param_def_required_defaults_false() {
        let json = r#"{"name":"x","description":"d","type":"string"}"#;
        let d: ParamDef = serde_json::from_str(json).unwrap();
        assert!(!d.required, "required should default to false");
    }

    // ── ExecuteRequest ────────────────────────────────────────────────────────

    #[test]
    fn execute_request_json_roundtrip() {
        let req = ExecuteRequest {
            skill: "weather.current".into(),
            args: serde_json::json!({"city": "Tokyo"}),
            request_id: "req-123".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: ExecuteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.skill, req.skill);
        assert_eq!(d.request_id, "req-123");
        assert_eq!(d.args["city"], "Tokyo");
    }

    #[test]
    fn execute_request_id_defaults_empty() {
        let json = r#"{"skill":"x","args":{}}"#;
        let d: ExecuteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(d.request_id, "");
    }

    // ── ExecuteResponse ───────────────────────────────────────────────────────

    #[test]
    fn execute_response_ok_constructor() {
        let r = ExecuteResponse::ok("r1", "weather is sunny");
        assert!(r.ok);
        assert_eq!(r.request_id, "r1");
        assert_eq!(r.output, "weather is sunny");
        assert!(r.error.is_empty());
    }

    #[test]
    fn execute_response_err_constructor() {
        let r = ExecuteResponse::err("r2", "API rate limited");
        assert!(!r.ok);
        assert_eq!(r.request_id, "r2");
        assert!(r.output.is_empty());
        assert!(r.error.contains("rate limited"));
    }

    #[test]
    fn execute_response_json_roundtrip_ok() {
        let r = ExecuteResponse::ok("req-1", "42°C");
        let json = serde_json::to_string(&r).unwrap();
        let d: ExecuteResponse = serde_json::from_str(&json).unwrap();
        assert!(d.ok);
        assert_eq!(d.output, "42°C");
    }

    #[test]
    fn execute_response_json_roundtrip_err() {
        let r = ExecuteResponse::err("req-2", "timeout");
        let json = serde_json::to_string(&r).unwrap();
        let d: ExecuteResponse = serde_json::from_str(&json).unwrap();
        assert!(!d.ok);
        assert_eq!(d.error, "timeout");
    }

    #[test]
    fn execute_response_output_defaults_empty() {
        let json = r#"{"request_id":"r","ok":false,"error":"oops"}"#;
        let d: ExecuteResponse = serde_json::from_str(json).unwrap();
        assert!(d.output.is_empty());
    }

    #[test]
    fn execute_response_error_defaults_empty() {
        let json = r#"{"request_id":"r","ok":true,"output":"done"}"#;
        let d: ExecuteResponse = serde_json::from_str(json).unwrap();
        assert!(d.error.is_empty());
    }

    // ── HostHttpRequest ───────────────────────────────────────────────────────

    #[test]
    fn host_http_request_default_method_get() {
        let json = r#"{"url":"https://api.example.com"}"#;
        let d: HostHttpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(d.method, "GET");
    }

    #[test]
    fn host_http_request_post_method_preserved() {
        let req = HostHttpRequest {
            url: "https://api.example.com".into(),
            method: "POST".into(),
            headers: std::collections::HashMap::new(),
            body: Some(r#"{"key":"value"}"#.into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: HostHttpRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.method, "POST");
        assert_eq!(d.body.unwrap(), r#"{"key":"value"}"#);
    }

    // ── HostHttpResponse ──────────────────────────────────────────────────────

    #[test]
    fn host_http_response_json_roundtrip() {
        let resp = HostHttpResponse {
            status: 200,
            body: r#"{"temperature":22}"#.into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: HostHttpResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(d.status, 200);
        assert!(d.body.contains("temperature"));
    }

    #[test]
    fn host_http_response_error_status() {
        let resp = HostHttpResponse { status: 404, body: "Not Found".into() };
        assert_eq!(resp.status, 404);
    }

    // ── SHA-256 integrity (via WasmPluginMeta concept) ────────────────────────

    #[test]
    fn sha256_hex_is_64_chars() {
        use sha2::{Digest, Sha256};
        let digest = hex::encode(Sha256::digest(b"\x00asm\x01\x00\x00\x00"));
        assert_eq!(digest.len(), 64, "SHA-256 hex should be 64 characters");
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sha256_different_content_gives_different_hash() {
        use sha2::{Digest, Sha256};
        let h1 = hex::encode(Sha256::digest(b"plugin_v1"));
        let h2 = hex::encode(Sha256::digest(b"plugin_v2"));
        assert_ne!(h1, h2, "different content must produce different SHA-256");
    }

    #[test]
    fn sha256_same_content_gives_same_hash() {
        use sha2::{Digest, Sha256};
        let h1 = hex::encode(Sha256::digest(b"stable_content"));
        let h2 = hex::encode(Sha256::digest(b"stable_content"));
        assert_eq!(h1, h2, "same content must produce identical SHA-256");
    }
}
