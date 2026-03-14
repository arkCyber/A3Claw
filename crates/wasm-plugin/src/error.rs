//! Error types for the WASM plugin subsystem.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("WASM instantiation failed for {path}: {reason}")]
    Instantiation { path: PathBuf, reason: String },

    #[error("Plugin {id} missing required export: {export}")]
    MissingExport { id: String, export: String },

    #[error("Plugin {id} manifest parse error: {reason}")]
    ManifestParse { id: String, reason: String },

    #[error("Plugin {id} execution error for skill '{skill}': {reason}")]
    Execution { id: String, skill: String, reason: String },

    #[error("Plugin {id} memory error: {reason}")]
    Memory { id: String, reason: String },

    #[error("Plugin {id} skill '{skill}' not found in manifest")]
    SkillNotFound { id: String, skill: String },

    #[error("Plugin {id} timed out after {timeout_ms}ms")]
    Timeout { id: String, timeout_ms: u64 },

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("WasmEdge error: {0}")]
    WasmEdge(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let e = PluginError::Io {
            path: PathBuf::from("/plugins/foo.wasm"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let s = e.to_string();
        assert!(s.contains("/plugins/foo.wasm"), "expected path in: {s}");
        assert!(s.contains("not found"), "expected source in: {s}");
    }

    #[test]
    fn instantiation_error_display() {
        let e = PluginError::Instantiation {
            path: PathBuf::from("/plugins/bad.wasm"),
            reason: "invalid magic bytes".into(),
        };
        let s = e.to_string();
        assert!(s.contains("bad.wasm"));
        assert!(s.contains("invalid magic bytes"));
    }

    #[test]
    fn missing_export_display() {
        let e = PluginError::MissingExport {
            id: "com.test.plugin".into(),
            export: "_manifest".into(),
        };
        let s = e.to_string();
        assert!(s.contains("com.test.plugin"));
        assert!(s.contains("_manifest"));
    }

    #[test]
    fn manifest_parse_display() {
        let e = PluginError::ManifestParse {
            id: "com.test.plugin".into(),
            reason: "unexpected end of input".into(),
        };
        let s = e.to_string();
        assert!(s.contains("unexpected end of input"));
    }

    #[test]
    fn execution_error_display() {
        let e = PluginError::Execution {
            id: "com.test.plugin".into(),
            skill: "fs.readFile".into(),
            reason: "WASM trap".into(),
        };
        let s = e.to_string();
        assert!(s.contains("fs.readFile"));
        assert!(s.contains("WASM trap"));
    }

    #[test]
    fn memory_error_display() {
        let e = PluginError::Memory {
            id: "com.test.plugin".into(),
            reason: "ptr out of bounds".into(),
        };
        let s = e.to_string();
        assert!(s.contains("ptr out of bounds"));
    }

    #[test]
    fn skill_not_found_display() {
        let e = PluginError::SkillNotFound {
            id: "com.test.plugin".into(),
            skill: "unknown.skill".into(),
        };
        let s = e.to_string();
        assert!(s.contains("unknown.skill"));
        assert!(s.contains("not found in manifest"));
    }

    #[test]
    fn timeout_display() {
        let e = PluginError::Timeout {
            id: "com.test.plugin".into(),
            timeout_ms: 5000,
        };
        let s = e.to_string();
        assert!(s.contains("5000ms"));
    }

    #[test]
    fn serde_error_from() {
        let serde_err: serde_json::Error = serde_json::from_str::<i32>("bad").unwrap_err();
        let e: PluginError = serde_err.into();
        assert!(e.to_string().contains("Serialization error"));
    }

    #[test]
    fn wasmedge_error_display() {
        let e = PluginError::WasmEdge("module not found".into());
        assert!(e.to_string().contains("module not found"));
    }
}
