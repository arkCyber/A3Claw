//! Unified error type for the agent executor.

use std::fmt;

/// Unified error for the agent executor.
#[derive(Debug)]
pub enum ExecutorError {
    AgentNotFound(String),
    NotRunnable(String),
    ContextError(String),
    DispatchFailed { skill: String, reason: String },
    GatewayUnreachable { url: String, source: String },
    SkillDenied { skill: String, reason: String },
    LlmError(String),
    MaxStepsExceeded(u32),
    Timeout(u64),
    BootstrapError(String),
    SessionError(String),
    SerdeError(serde_json::Error),
    IoError(std::io::Error),
    HttpError(String),
    Other(anyhow::Error),
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutorError::AgentNotFound(s)       => write!(f, "Agent not found: {}", s),
            ExecutorError::NotRunnable(s)          => write!(f, "Agent not in a runnable state: {}", s),
            ExecutorError::ContextError(s)         => write!(f, "Task context error: {}", s),
            ExecutorError::DispatchFailed { skill, reason } =>
                write!(f, "Skill dispatch failed: skill={}, reason={}", skill, reason),
            ExecutorError::GatewayUnreachable { url, source } =>
                write!(f, "Gateway unreachable at {}: {}", url, source),
            ExecutorError::SkillDenied { skill, reason } =>
                write!(f, "Gateway denied skill '{}': {}", skill, reason),
            ExecutorError::LlmError(s)             => write!(f, "LLM inference error: {}", s),
            ExecutorError::MaxStepsExceeded(n)     => write!(f, "ReAct loop exceeded max steps ({})", n),
            ExecutorError::Timeout(s)              => write!(f, "ReAct loop timed out after {}s", s),
            ExecutorError::BootstrapError(s)       => write!(f, "Bootstrap failed: {}", s),
            ExecutorError::SessionError(s)         => write!(f, "Session error: {}", s),
            ExecutorError::SerdeError(e)           => write!(f, "Serialization error: {}", e),
            ExecutorError::IoError(e)              => write!(f, "IO error: {}", e),
            ExecutorError::HttpError(s)            => write!(f, "HTTP error: {}", s),
            ExecutorError::Other(e)                => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ExecutorError {}

impl From<serde_json::Error> for ExecutorError {
    fn from(e: serde_json::Error) -> Self { ExecutorError::SerdeError(e) }
}

impl From<std::io::Error> for ExecutorError {
    fn from(e: std::io::Error) -> Self { ExecutorError::IoError(e) }
}

impl From<anyhow::Error> for ExecutorError {
    fn from(e: anyhow::Error) -> Self { ExecutorError::Other(e) }
}

impl ExecutorError {
    /// Returns true if the error is recoverable (retry may succeed).
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            ExecutorError::GatewayUnreachable { .. }
                | ExecutorError::HttpError(_)
                | ExecutorError::LlmError(_)
        )
    }

    /// Short code for logging / UI display.
    pub fn code(&self) -> &'static str {
        match self {
            ExecutorError::AgentNotFound(_)    => "AGENT_NOT_FOUND",
            ExecutorError::NotRunnable(_)      => "NOT_RUNNABLE",
            ExecutorError::ContextError(_)     => "CONTEXT_ERROR",
            ExecutorError::DispatchFailed {..} => "DISPATCH_FAILED",
            ExecutorError::GatewayUnreachable {..} => "GATEWAY_UNREACHABLE",
            ExecutorError::SkillDenied {..}    => "SKILL_DENIED",
            ExecutorError::LlmError(_)         => "LLM_ERROR",
            ExecutorError::MaxStepsExceeded(_) => "MAX_STEPS",
            ExecutorError::Timeout(_)          => "TIMEOUT",
            ExecutorError::BootstrapError(_)   => "BOOTSTRAP_ERROR",
            ExecutorError::SessionError(_)     => "SESSION_ERROR",
            ExecutorError::SerdeError(_)       => "SERDE_ERROR",
            ExecutorError::IoError(_)          => "IO_ERROR",
            ExecutorError::HttpError(_)        => "HTTP_ERROR",
            ExecutorError::Other(_)            => "OTHER",
        }
    }
}
