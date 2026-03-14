//! Shared request / response types for the OpenClaw plugin hook API.
//!
//! OpenClaw calls our plugin gateway over HTTP before and after every Skill
//! execution. These types model the JSON payloads exchanged on those calls.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Inbound hook payloads (OpenClaw → Gateway) ────────────────────────────────

/// Payload sent by OpenClaw to `/hooks/before-skill` before executing a Skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeSkillPayload {
    /// Unique identifier for this invocation (used to correlate before/after).
    pub invocation_id: String,
    /// The name of the Skill being invoked (e.g. `"shell.exec"`, `"fs.writeFile"`).
    pub skill_name: String,
    /// The agent session that triggered this Skill call.
    pub session_id: String,
    /// Skill-specific input arguments.
    pub args: HashMap<String, serde_json::Value>,
    /// Wall-clock timestamp (ISO 8601) when the call was initiated.
    pub timestamp: String,
}

/// Payload sent by OpenClaw to `/hooks/after-skill` after a Skill completes.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterSkillPayload {
    pub invocation_id: String,
    pub skill_name: String,
    pub session_id: String,
    /// Whether the Skill completed successfully.
    pub success: bool,
    /// Optional error message when `success` is false.
    pub error: Option<String>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    pub timestamp: String,
}

/// Payload sent by OpenClaw to `/hooks/agent-start`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStartPayload {
    pub session_id: String,
    pub agent_name: Option<String>,
    pub timestamp: String,
}

/// Payload sent by OpenClaw to `/hooks/agent-stop`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStopPayload {
    pub session_id: String,
    pub reason: Option<String>,
    pub timestamp: String,
}

// ── Outbound hook responses (Gateway → OpenClaw) ──────────────────────────────

/// The gateway's verdict on a `before-skill` hook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HookVerdict {
    /// Allow the Skill to proceed.
    Allow,
    /// Block the Skill and surface the reason to the agent.
    Deny,
    /// Block the Skill and wait for human confirmation (async; not yet supported
    /// by all OpenClaw versions — falls back to `deny` if unsupported).
    Confirm,
}

/// Response body returned from `/hooks/before-skill`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeSkillResponse {
    pub verdict: HookVerdict,
    /// Human-readable reason shown to the agent when `verdict != allow`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Confirmation prompt shown in the UI when `verdict == confirm`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_prompt: Option<String>,
}

impl BeforeSkillResponse {
    pub fn allow() -> Self {
        Self { verdict: HookVerdict::Allow, reason: None, confirm_prompt: None }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self { verdict: HookVerdict::Deny, reason: Some(reason.into()), confirm_prompt: None }
    }

    pub fn confirm(prompt: impl Into<String>) -> Self {
        Self {
            verdict: HookVerdict::Confirm,
            reason: None,
            confirm_prompt: Some(prompt.into()),
        }
    }
}

/// Generic acknowledgement returned from lifecycle hooks.
#[derive(Debug, Clone, Serialize)]
pub struct AckResponse {
    pub ok: bool,
}

impl AckResponse {
    pub fn ok() -> Self { Self { ok: true } }
}

// ── Skill API response types ──────────────────────────────────────────────────

/// Response from `/skills/status`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub sandbox_running: bool,
    pub breaker_tripped: bool,
    pub total_events: u64,
    pub allowed_events: u64,
    pub denied_events: u64,
    pub pending_events: u64,
    pub dangerous_commands_blocked: u64,
}

/// Response from `/skills/policy` PATCH.
#[derive(Debug, Clone, Serialize)]
pub struct PolicyUpdateResponse {
    pub ok: bool,
    pub applied: Vec<String>,
}

/// Request body for `/skills/policy` PATCH.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyUpdateRequest {
    pub network_allowlist:  Option<Vec<String>>,
    pub confirm_shell_exec: Option<bool>,
    pub confirm_file_delete: Option<bool>,
    pub confirm_network:    Option<bool>,
    pub intercept_shell:    Option<bool>,
}
