//! Action execution for assistant

use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Describes an action the assistant recommends or can trigger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SuggestedAction {
    /// Configure the RAG knowledge base (add/remove/check_status).
    ConfigureRAG {
        operation: String,
        params: Value,
    },
    /// Run a built-in diagnostic test.
    RunDiagnostic {
        test_name: String,
    },
    /// Apply RAG performance tuning parameters.
    OptimizeRAG {
        params: Value,
    },
    /// Apply a security configuration template.
    ApplySecurity {
        template: String,
    },
    /// Open a documentation URL in the browser / docs panel.
    OpenDocument {
        url: String,
    },
    /// Start the WasmEdge sandbox for a given session.
    StartSandbox {
        session_id: Option<String>,
    },
    /// Stop the WasmEdge sandbox gracefully.
    StopSandbox {
        session_id: Option<String>,
    },
    /// Emergency-stop the WasmEdge sandbox (SIGKILL equivalent).
    EmergencyStop {
        session_id: Option<String>,
    },
    /// Clear the security event log displayed in the dashboard.
    ClearEventLog,
}

impl SuggestedAction {
    /// Human-readable label used in the UI action button.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ConfigureRAG { .. }  => "Configure RAG",
            Self::RunDiagnostic { .. } => "Run Diagnostic",
            Self::OptimizeRAG { .. }   => "Optimize RAG",
            Self::ApplySecurity { .. } => "Apply Security Template",
            Self::OpenDocument { .. }  => "Open Document",
            Self::StartSandbox { .. }  => "Start Sandbox",
            Self::StopSandbox { .. }   => "Stop Sandbox",
            Self::EmergencyStop { .. } => "Emergency Stop",
            Self::ClearEventLog        => "Clear Event Log",
        }
    }

    /// Whether this action is potentially destructive / irreversible.
    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::EmergencyStop { .. } | Self::ClearEventLog)
    }
}

/// Result returned after an action is executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Human-readable description of what was done.
    pub message: String,
    /// Whether the action succeeded.
    pub success: bool,
    /// Optional structured payload for the caller.
    pub payload: Option<Value>,
}

impl ActionResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self { message: message.into(), success: true, payload: None }
    }

    pub fn ok_with(message: impl Into<String>, payload: Value) -> Self {
        Self { message: message.into(), success: true, payload: Some(payload) }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self { message: message.into(), success: false, payload: None }
    }
}

/// Executes `SuggestedAction` values produced by the assistant.
///
/// In a full production system the executor would dispatch to the actual
/// sandbox manager, RAG indexer, etc.  For now it validates the action
/// and returns a structured `ActionResult` so the UI / integration layer
/// can present meaningful feedback and hook into real subsystems.
pub struct ActionExecutor;

impl ActionExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, action: &SuggestedAction) -> Result<ActionResult> {
        let result = match action {
            SuggestedAction::ConfigureRAG { operation, params } => {
                ActionResult::ok_with(
                    format!("RAG operation '{}' queued", operation),
                    serde_json::json!({ "operation": operation, "params": params }),
                )
            }
            SuggestedAction::RunDiagnostic { test_name } => {
                ActionResult::ok_with(
                    format!("Diagnostic '{}' started", test_name),
                    serde_json::json!({ "test": test_name, "status": "running" }),
                )
            }
            SuggestedAction::OptimizeRAG { params } => {
                ActionResult::ok_with(
                    "RAG optimization parameters staged — apply in Settings → RAG".to_string(),
                    params.clone(),
                )
            }
            SuggestedAction::ApplySecurity { template } => {
                ActionResult::ok_with(
                    format!("Security template '{}' ready to apply", template),
                    serde_json::json!({ "template": template }),
                )
            }
            SuggestedAction::OpenDocument { url } => {
                ActionResult::ok(format!("Opening: {}", url))
            }
            SuggestedAction::StartSandbox { session_id } => {
                let sid = session_id.as_deref().unwrap_or("default");
                ActionResult::ok_with(
                    format!("Sandbox '{}' start requested", sid),
                    serde_json::json!({ "session_id": sid, "action": "start" }),
                )
            }
            SuggestedAction::StopSandbox { session_id } => {
                let sid = session_id.as_deref().unwrap_or("default");
                ActionResult::ok_with(
                    format!("Sandbox '{}' stop requested", sid),
                    serde_json::json!({ "session_id": sid, "action": "stop" }),
                )
            }
            SuggestedAction::EmergencyStop { session_id } => {
                let sid = session_id.as_deref().unwrap_or("default");
                ActionResult::ok_with(
                    format!("EMERGENCY STOP issued for sandbox '{}'", sid),
                    serde_json::json!({ "session_id": sid, "action": "emergency_stop" }),
                )
            }
            SuggestedAction::ClearEventLog => {
                ActionResult::ok("Event log cleared")
            }
        };
        Ok(result)
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_configure_rag() {
        let ex = ActionExecutor::new();
        let action = SuggestedAction::ConfigureRAG {
            operation: "add_folder".to_string(),
            params: serde_json::json!({ "watch_enabled": true }),
        };
        let result = ex.execute(&action).unwrap();
        assert!(result.success);
        assert!(result.message.contains("add_folder"));
        assert!(result.payload.is_some());
    }

    #[test]
    fn test_execute_run_diagnostic() {
        let ex = ActionExecutor::new();
        let action = SuggestedAction::RunDiagnostic {
            test_name: "wasi_permissions".to_string(),
        };
        let result = ex.execute(&action).unwrap();
        assert!(result.success);
        assert!(result.message.contains("wasi_permissions"));
        let payload = result.payload.unwrap();
        assert_eq!(payload["status"], "running");
    }

    #[test]
    fn test_execute_optimize_rag() {
        let ex = ActionExecutor::new();
        let action = SuggestedAction::OptimizeRAG {
            params: serde_json::json!({ "chunk_size": 800 }),
        };
        let result = ex.execute(&action).unwrap();
        assert!(result.success);
        assert!(result.payload.is_some());
    }

    #[test]
    fn test_execute_apply_security() {
        let ex = ActionExecutor::new();
        let action = SuggestedAction::ApplySecurity {
            template: "production".to_string(),
        };
        let result = ex.execute(&action).unwrap();
        assert!(result.success);
        assert!(result.message.contains("production"));
    }

    #[test]
    fn test_execute_start_sandbox() {
        let ex = ActionExecutor::new();
        let result = ex.execute(&SuggestedAction::StartSandbox {
            session_id: Some("sess-123".to_string()),
        }).unwrap();
        assert!(result.success);
        assert!(result.message.contains("sess-123"));
        assert_eq!(result.payload.unwrap()["action"], "start");
    }

    #[test]
    fn test_execute_stop_sandbox() {
        let ex = ActionExecutor::new();
        let result = ex.execute(&SuggestedAction::StopSandbox {
            session_id: None,
        }).unwrap();
        assert!(result.success);
        assert_eq!(result.payload.unwrap()["action"], "stop");
    }

    #[test]
    fn test_execute_emergency_stop() {
        let ex = ActionExecutor::new();
        let result = ex.execute(&SuggestedAction::EmergencyStop {
            session_id: None,
        }).unwrap();
        assert!(result.success);
        assert!(result.message.contains("EMERGENCY"));
    }

    #[test]
    fn test_execute_clear_event_log() {
        let ex = ActionExecutor::new();
        let result = ex.execute(&SuggestedAction::ClearEventLog).unwrap();
        assert!(result.success);
        assert!(result.message.contains("cleared"));
    }

    #[test]
    fn test_action_label() {
        assert_eq!(SuggestedAction::ClearEventLog.label(), "Clear Event Log");
        assert_eq!(
            SuggestedAction::EmergencyStop { session_id: None }.label(),
            "Emergency Stop"
        );
    }

    #[test]
    fn test_action_is_destructive() {
        assert!(SuggestedAction::EmergencyStop { session_id: None }.is_destructive());
        assert!(SuggestedAction::ClearEventLog.is_destructive());
        assert!(!SuggestedAction::StartSandbox { session_id: None }.is_destructive());
        assert!(!SuggestedAction::RunDiagnostic { test_name: "x".into() }.is_destructive());
    }

    #[test]
    fn test_action_result_constructors() {
        let r = ActionResult::ok("done");
        assert!(r.success);
        assert!(r.payload.is_none());

        let r2 = ActionResult::err("fail");
        assert!(!r2.success);

        let r3 = ActionResult::ok_with("ok", serde_json::json!({ "k": 1 }));
        assert!(r3.success);
        assert!(r3.payload.is_some());
    }
}
