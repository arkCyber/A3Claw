//! Error types and execution result for the WASM skill runtime.

use thiserror::Error;
use std::time::Duration;
use serde_json::Value;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Memory error: {0}")]
    MemoryError(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Output parse error: {0}")]
    OutputParseError(String),

    #[error("Registry error: {0}")]
    RegistryError(String),
}

/// Result of a single WASM skill execution.
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Success {
        result: Value,
        execution_time: Duration,
    },
    Error {
        error: String,
    },
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionResult::Success { .. })
    }

    pub fn into_value(self) -> Value {
        match self {
            ExecutionResult::Success { result, .. } => result,
            ExecutionResult::Error { error } => serde_json::json!({ "error": error }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_result_success_is_success() {
        let r = ExecutionResult::Success {
            result: serde_json::json!({"ok": true}),
            execution_time: Duration::from_millis(10),
        };
        assert!(r.is_success());
    }

    #[test]
    fn execution_result_error_is_not_success() {
        let r = ExecutionResult::Error { error: "boom".into() };
        assert!(!r.is_success());
    }

    #[test]
    fn execution_result_success_into_value() {
        let val = serde_json::json!({"answer": 42});
        let r = ExecutionResult::Success {
            result: val.clone(),
            execution_time: Duration::ZERO,
        };
        assert_eq!(r.into_value(), val);
    }

    #[test]
    fn execution_result_error_into_value_contains_error_key() {
        let r = ExecutionResult::Error { error: "something failed".into() };
        let v = r.into_value();
        assert_eq!(v["error"], "something failed");
    }

    #[test]
    fn runtime_error_display_skill_not_found() {
        let e = RuntimeError::SkillNotFound("math.add".into());
        assert!(e.to_string().contains("math.add"));
    }

    #[test]
    fn runtime_error_display_execution_failed() {
        let e = RuntimeError::ExecutionFailed("WASM trap".into());
        assert!(e.to_string().contains("WASM trap"));
    }

    #[test]
    fn runtime_error_display_memory_error() {
        let e = RuntimeError::MemoryError("ptr out of range".into());
        assert!(e.to_string().contains("ptr out of range"));
    }

    #[test]
    fn runtime_error_display_validation_failed() {
        let e = RuntimeError::ValidationFailed("bad magic".into());
        assert!(e.to_string().contains("bad magic"));
    }

    #[test]
    fn runtime_error_display_output_parse() {
        let e = RuntimeError::OutputParseError("invalid json".into());
        assert!(e.to_string().contains("invalid json"));
    }

    #[test]
    fn runtime_error_display_registry() {
        let e = RuntimeError::RegistryError("not found".into());
        assert!(e.to_string().contains("not found"));
    }
}
