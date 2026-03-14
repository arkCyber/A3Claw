//! Inference engine error types (DO-178C inspired).
//!
//! Every error variant carries:
//! - A machine-readable error code for structured logging.
//! - A severity level for triage.
//! - A `is_retryable()` flag so callers can decide whether to retry.

use thiserror::Error;

// ── Severity ───────────────────────────────────────────────────────────────────────

/// Error severity level, analogous to DO-178C DAL categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational — operation can continue normally.
    Info,
    /// Degraded — operation succeeded via fallback.
    Warning,
    /// Hard failure — request could not be completed.
    Error,
    /// Safety-critical — data integrity or security violation.
    Critical,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Info     => write!(f, "INFO"),
            ErrorSeverity::Warning  => write!(f, "WARNING"),
            ErrorSeverity::Error    => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

// ── InferenceError ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum InferenceError {
    /// Circuit breaker is open; the backend is temporarily unavailable.
    #[error("circuit breaker open — backend {backend} unavailable after {failures} consecutive failures")]
    CircuitOpen { backend: String, failures: u32 },

    /// Hard timeout elapsed before the backend responded.
    #[error("inference timeout after {timeout_ms}ms (request_id={request_id})")]
    Timeout { request_id: u64, timeout_ms: u64 },

    /// SHA-256 digest of the model file does not match the expected value.
    #[error("model integrity check failed: expected={expected} actual={actual}")]
    IntegrityFailure { expected: String, actual: String },

    /// The model file does not exist at the given path.
    #[error("model file not found: {path}")]
    ModelNotFound { path: String },

    /// The backend returned a non-2xx HTTP status code.
    #[error("backend {backend} returned HTTP {status}: {body}")]
    HttpError { backend: String, status: u16, body: String },

    /// Failed to parse a streaming token from the backend response.
    #[error("stream parse error: {0}")]
    StreamParse(String),

    /// All backends failed or have open circuit breakers.
    #[error("no backend available — all backends failed or circuit-open")]
    NoBackendAvailable,

    /// The request exceeds the configured context window.
    #[error("context window exceeded: {tokens} tokens > limit {limit}")]
    ContextWindowExceeded { tokens: usize, limit: u32 },

    /// Request failed validation before being sent to a backend.
    #[error("request validation failed: {reason}")]
    ValidationFailed { reason: String },

    /// WasmEdge WASI-NN backend error.
    #[error("WASI-NN error: {0}")]
    WasiNn(String),

    /// JSON serialization / deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Filesystem I/O error (e.g. reading the model file).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP client transport error.
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    /// Catch-all for unexpected internal errors.
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl InferenceError {
    /// Returns `true` if the operation that produced this error may succeed
    /// on a subsequent attempt without any configuration change.
    ///
    /// Callers should consult this before deciding to retry or escalate.
    pub fn is_retryable(&self) -> bool {
        match self {
            // Transient conditions — safe to retry after a delay.
            InferenceError::Timeout { .. }       => true,
            InferenceError::CircuitOpen { .. }   => true,
            InferenceError::NoBackendAvailable   => true,
            InferenceError::Http(_)              => true,
            InferenceError::StreamParse(_)       => true,
            // Permanent conditions — retrying will not help.
            InferenceError::IntegrityFailure { .. }      => false,
            InferenceError::ModelNotFound { .. }         => false,
            InferenceError::ContextWindowExceeded { .. } => false,
            InferenceError::ValidationFailed { .. }      => false,
            InferenceError::WasiNn(_)                    => false,
            InferenceError::Serialization(_)             => false,
            InferenceError::Io(_)                        => false,
            InferenceError::Internal(_)                  => false,
            InferenceError::HttpError { status, .. } => {
                // 429 Too Many Requests and 5xx server errors are retryable.
                *status == 429 || *status >= 500
            }
        }
    }

    /// Returns the severity level of this error.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            InferenceError::IntegrityFailure { .. } => ErrorSeverity::Critical,
            InferenceError::ModelNotFound { .. }    => ErrorSeverity::Critical,
            InferenceError::ValidationFailed { .. } => ErrorSeverity::Error,
            InferenceError::ContextWindowExceeded { .. } => ErrorSeverity::Error,
            InferenceError::NoBackendAvailable      => ErrorSeverity::Error,
            InferenceError::Timeout { .. }          => ErrorSeverity::Warning,
            InferenceError::CircuitOpen { .. }      => ErrorSeverity::Warning,
            InferenceError::HttpError { status, .. } if *status >= 500 => ErrorSeverity::Error,
            InferenceError::HttpError { .. }        => ErrorSeverity::Warning,
            _                                       => ErrorSeverity::Error,
        }
    }

    /// A short machine-readable error code for structured logging and metrics.
    pub fn error_code(&self) -> &'static str {
        match self {
            InferenceError::CircuitOpen { .. }           => "CIRCUIT_OPEN",
            InferenceError::Timeout { .. }               => "TIMEOUT",
            InferenceError::IntegrityFailure { .. }      => "INTEGRITY_FAILURE",
            InferenceError::ModelNotFound { .. }         => "MODEL_NOT_FOUND",
            InferenceError::HttpError { .. }             => "HTTP_ERROR",
            InferenceError::StreamParse(_)               => "STREAM_PARSE_ERROR",
            InferenceError::NoBackendAvailable           => "NO_BACKEND",
            InferenceError::ContextWindowExceeded { .. } => "CONTEXT_WINDOW_EXCEEDED",
            InferenceError::ValidationFailed { .. }      => "VALIDATION_FAILED",
            InferenceError::WasiNn(_)                    => "WASI_NN_ERROR",
            InferenceError::Serialization(_)             => "SERIALIZATION_ERROR",
            InferenceError::Io(_)                        => "IO_ERROR",
            InferenceError::Http(_)                      => "HTTP_CLIENT_ERROR",
            InferenceError::Internal(_)                  => "INTERNAL_ERROR",
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ErrorSeverity ────────────────────────────────────────────────────

    #[test]
    fn severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
    }

    #[test]
    fn severity_display() {
        assert_eq!(ErrorSeverity::Info.to_string(),     "INFO");
        assert_eq!(ErrorSeverity::Warning.to_string(),  "WARNING");
        assert_eq!(ErrorSeverity::Error.to_string(),    "ERROR");
        assert_eq!(ErrorSeverity::Critical.to_string(), "CRITICAL");
    }

    // ── is_retryable() ──────────────────────────────────────────────────

    #[test]
    fn timeout_is_retryable() {
        let e = InferenceError::Timeout { request_id: 1, timeout_ms: 5000 };
        assert!(e.is_retryable());
    }

    #[test]
    fn circuit_open_is_retryable() {
        let e = InferenceError::CircuitOpen { backend: "b".into(), failures: 3 };
        assert!(e.is_retryable());
    }

    #[test]
    fn no_backend_is_retryable() {
        assert!(InferenceError::NoBackendAvailable.is_retryable());
    }

    #[test]
    fn http_5xx_is_retryable() {
        let e = InferenceError::HttpError { backend: "b".into(), status: 503, body: String::new() };
        assert!(e.is_retryable());
    }

    #[test]
    fn http_4xx_not_retryable() {
        let e = InferenceError::HttpError { backend: "b".into(), status: 400, body: String::new() };
        assert!(!e.is_retryable());
    }

    #[test]
    fn http_429_is_retryable() {
        let e = InferenceError::HttpError { backend: "b".into(), status: 429, body: String::new() };
        assert!(e.is_retryable());
    }

    #[test]
    fn integrity_failure_not_retryable() {
        let e = InferenceError::IntegrityFailure { expected: "a".into(), actual: "b".into() };
        assert!(!e.is_retryable());
    }

    #[test]
    fn context_window_exceeded_not_retryable() {
        let e = InferenceError::ContextWindowExceeded { tokens: 9000, limit: 4096 };
        assert!(!e.is_retryable());
    }

    #[test]
    fn validation_failed_not_retryable() {
        let e = InferenceError::ValidationFailed { reason: "empty messages".into() };
        assert!(!e.is_retryable());
    }

    // ── severity() ────────────────────────────────────────────────────────

    #[test]
    fn integrity_failure_is_critical() {
        let e = InferenceError::IntegrityFailure { expected: "a".into(), actual: "b".into() };
        assert_eq!(e.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn timeout_is_warning() {
        let e = InferenceError::Timeout { request_id: 1, timeout_ms: 5000 };
        assert_eq!(e.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn no_backend_is_error_severity() {
        assert_eq!(InferenceError::NoBackendAvailable.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn http_5xx_is_error_severity() {
        let e = InferenceError::HttpError { backend: "b".into(), status: 500, body: String::new() };
        assert_eq!(e.severity(), ErrorSeverity::Error);
    }

    // ── error_code() ───────────────────────────────────────────────────────

    #[test]
    fn error_codes_are_unique_and_non_empty() {
        let errors: Vec<(&str, InferenceError)> = vec![
            ("CIRCUIT_OPEN",            InferenceError::CircuitOpen { backend: "b".into(), failures: 1 }),
            ("TIMEOUT",                 InferenceError::Timeout { request_id: 1, timeout_ms: 1 }),
            ("INTEGRITY_FAILURE",       InferenceError::IntegrityFailure { expected: "a".into(), actual: "b".into() }),
            ("MODEL_NOT_FOUND",         InferenceError::ModelNotFound { path: "/x".into() }),
            ("NO_BACKEND",              InferenceError::NoBackendAvailable),
            ("CONTEXT_WINDOW_EXCEEDED", InferenceError::ContextWindowExceeded { tokens: 1, limit: 1 }),
            ("VALIDATION_FAILED",       InferenceError::ValidationFailed { reason: "r".into() }),
            ("WASI_NN_ERROR",           InferenceError::WasiNn("e".into())),
            ("STREAM_PARSE_ERROR",      InferenceError::StreamParse("e".into())),
        ];
        for (expected_code, err) in &errors {
            assert_eq!(err.error_code(), *expected_code, "wrong code for {:?}", err);
            assert!(!err.error_code().is_empty());
        }
    }

    // ── Display (thiserror) ──────────────────────────────────────────────

    #[test]
    fn display_contains_relevant_fields() {
        let e = InferenceError::Timeout { request_id: 42, timeout_ms: 5000 };
        let s = e.to_string();
        assert!(s.contains("5000"));
        assert!(s.contains("42"));
    }

    #[test]
    fn display_http_error_contains_status() {
        let e = InferenceError::HttpError { backend: "ollama".into(), status: 503, body: "down".into() };
        let s = e.to_string();
        assert!(s.contains("503"));
        assert!(s.contains("ollama"));
    }
}
