//! Structured audit log for every inference call (DO-178C inspired).
//!
//! Every request, response, error, and state transition is recorded with:
//! - A monotonically increasing sequence number.
//! - Wall-clock elapsed time since engine start.
//! - Structured tracing fields at the correct severity level.
//! - An in-memory snapshot ring for test inspection.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tracing::{error, info, warn};

// ── Event kinds ───────────────────────────────────────────────────────────────

/// Every distinct event that the inference engine can emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditEventKind {
    /// A new inference request was received by the engine.
    RequestReceived,
    /// A backend was selected to serve the request.
    BackendSelected,
    /// SHA-256 model integrity check passed.
    IntegrityCheckPassed,
    /// SHA-256 model integrity check failed.
    IntegrityCheckFailed,
    /// Inference execution started on the backend.
    InferenceStarted,
    /// Inference completed successfully.
    InferenceCompleted,
    /// Inference failed with an error.
    InferenceFailed,
    /// A circuit breaker transitioned to Open.
    CircuitBreakerOpened,
    /// A circuit breaker recovered to Closed.
    CircuitBreakerClosed,
    /// A hard timeout was triggered.
    TimeoutTriggered,
    /// A streaming response was started.
    StreamStarted,
    /// A streaming response completed.
    StreamCompleted,
    /// A fallback backend was attempted.
    FallbackAttempted,
    /// Request validation failed (e.g. context window exceeded).
    ValidationFailed,
}

impl fmt::Display for AuditEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AuditEventKind::RequestReceived      => "RequestReceived",
            AuditEventKind::BackendSelected      => "BackendSelected",
            AuditEventKind::IntegrityCheckPassed => "IntegrityCheckPassed",
            AuditEventKind::IntegrityCheckFailed => "IntegrityCheckFailed",
            AuditEventKind::InferenceStarted     => "InferenceStarted",
            AuditEventKind::InferenceCompleted   => "InferenceCompleted",
            AuditEventKind::InferenceFailed      => "InferenceFailed",
            AuditEventKind::CircuitBreakerOpened => "CircuitBreakerOpened",
            AuditEventKind::CircuitBreakerClosed => "CircuitBreakerClosed",
            AuditEventKind::TimeoutTriggered     => "TimeoutTriggered",
            AuditEventKind::StreamStarted        => "StreamStarted",
            AuditEventKind::StreamCompleted      => "StreamCompleted",
            AuditEventKind::FallbackAttempted    => "FallbackAttempted",
            AuditEventKind::ValidationFailed     => "ValidationFailed",
        };
        write!(f, "{s}")
    }
}

impl AuditEventKind {
    /// Returns `true` for events that indicate a fault condition.
    pub fn is_fault(&self) -> bool {
        matches!(
            self,
            AuditEventKind::InferenceFailed
                | AuditEventKind::IntegrityCheckFailed
                | AuditEventKind::CircuitBreakerOpened
                | AuditEventKind::TimeoutTriggered
                | AuditEventKind::ValidationFailed
        )
    }

    /// Returns `true` for events that indicate a warning condition.
    pub fn is_warning(&self) -> bool {
        matches!(
            self,
            AuditEventKind::CircuitBreakerClosed | AuditEventKind::FallbackAttempted
        )
    }
}

// ── AuditRecord ───────────────────────────────────────────────────────────────

/// A single immutable audit record.
#[derive(Debug, Clone)]
pub struct AuditRecord {
    /// Monotonically increasing sequence number (engine-global).
    pub seq: u64,
    /// Correlation ID matching the originating `InferenceRequest`.
    pub request_id: u64,
    /// Event type.
    pub kind: AuditEventKind,
    /// Backend that emitted this event.
    pub backend: String,
    /// Human-readable detail string.
    pub detail: String,
    /// Inference latency if applicable.
    pub latency_ms: Option<u64>,
    /// Milliseconds since the engine was started.
    pub elapsed_ms: u128,
}

impl fmt::Display for AuditRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[seq={} rid={} +{}ms] {} @ {} — {}",
            self.seq, self.request_id, self.elapsed_ms,
            self.kind, self.backend, self.detail
        )
    }
}

// ── InferenceAuditLog ─────────────────────────────────────────────────────────

/// Thread-safe structured audit log.
///
/// Records are emitted via `tracing` at the correct severity level and
/// optionally retained in an in-memory ring buffer for test inspection.
pub struct InferenceAuditLog {
    seq: AtomicU64,
    start: Instant,
    /// In-memory snapshot — populated only in `#[cfg(test)]` or when
    /// `retain_records` is set to `true` at construction time.
    records: Mutex<Vec<AuditRecord>>,
    retain_records: bool,
}

impl InferenceAuditLog {
    /// Create a new audit log. Records are NOT retained in memory by default.
    pub fn new() -> Self {
        Self {
            seq: AtomicU64::new(0),
            start: Instant::now(),
            records: Mutex::new(Vec::new()),
            retain_records: false,
        }
    }

    /// Create a log that retains all records in memory (useful for testing).
    pub fn new_with_retention() -> Self {
        Self {
            seq: AtomicU64::new(0),
            start: Instant::now(),
            records: Mutex::new(Vec::new()),
            retain_records: true,
        }
    }

    /// Emit a single audit event.
    ///
    /// The event is always written to `tracing` at the appropriate severity.
    /// If `retain_records` is set, it is also appended to the in-memory log.
    pub fn record(
        &self,
        request_id: u64,
        kind: AuditEventKind,
        backend: &str,
        detail: &str,
        latency_ms: Option<u64>,
    ) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let elapsed_ms = self.start.elapsed().as_millis();

        let rec = AuditRecord {
            seq,
            request_id,
            kind: kind.clone(),
            backend: backend.to_string(),
            detail: detail.to_string(),
            latency_ms,
            elapsed_ms,
        };

        // Emit to tracing at the correct severity.
        if kind.is_fault() {
            error!(
                seq, request_id, backend, detail,
                elapsed_ms = elapsed_ms,
                kind = %kind,
                "inference audit [FAULT]"
            );
        } else if kind.is_warning() {
            warn!(
                seq, request_id, backend, detail,
                elapsed_ms = elapsed_ms,
                kind = %kind,
                "inference audit [WARN]"
            );
        } else {
            info!(
                seq, request_id, backend, detail,
                elapsed_ms = elapsed_ms,
                latency_ms = latency_ms.unwrap_or(0),
                kind = %kind,
                "inference audit"
            );
        }

        if self.retain_records {
            if let Ok(mut guard) = self.records.lock() {
                guard.push(rec);
            }
        }
    }

    /// Return a snapshot of all retained records.
    ///
    /// Returns an empty `Vec` if `retain_records` was not set.
    pub fn records(&self) -> Vec<AuditRecord> {
        self.records
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Total number of events emitted since construction.
    pub fn event_count(&self) -> u64 {
        self.seq.load(Ordering::Relaxed)
    }

    /// Elapsed milliseconds since the log was created.
    pub fn uptime_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}

impl Default for InferenceAuditLog {
    fn default() -> Self { Self::new() }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AuditEventKind ────────────────────────────────────────────────────

    #[test]
    fn event_kind_display() {
        assert_eq!(AuditEventKind::RequestReceived.to_string(),      "RequestReceived");
        assert_eq!(AuditEventKind::InferenceCompleted.to_string(),   "InferenceCompleted");
        assert_eq!(AuditEventKind::CircuitBreakerOpened.to_string(), "CircuitBreakerOpened");
        assert_eq!(AuditEventKind::ValidationFailed.to_string(),     "ValidationFailed");
    }

    #[test]
    fn event_kind_is_fault() {
        assert!(AuditEventKind::InferenceFailed.is_fault());
        assert!(AuditEventKind::IntegrityCheckFailed.is_fault());
        assert!(AuditEventKind::CircuitBreakerOpened.is_fault());
        assert!(AuditEventKind::TimeoutTriggered.is_fault());
        assert!(AuditEventKind::ValidationFailed.is_fault());
        assert!(!AuditEventKind::InferenceCompleted.is_fault());
        assert!(!AuditEventKind::RequestReceived.is_fault());
    }

    #[test]
    fn event_kind_is_warning() {
        assert!(AuditEventKind::CircuitBreakerClosed.is_warning());
        assert!(AuditEventKind::FallbackAttempted.is_warning());
        assert!(!AuditEventKind::InferenceCompleted.is_warning());
        assert!(!AuditEventKind::InferenceFailed.is_warning());
    }

    // ── InferenceAuditLog::new() ───────────────────────────────────────────

    #[test]
    fn new_starts_with_zero_events() {
        let log = InferenceAuditLog::new();
        assert_eq!(log.event_count(), 0);
        assert!(log.records().is_empty());
    }

    #[test]
    fn new_with_retention_retains_records() {
        let log = InferenceAuditLog::new_with_retention();
        log.record(1, AuditEventKind::RequestReceived, "http", "test", None);
        assert_eq!(log.records().len(), 1);
    }

    // ── record() ──────────────────────────────────────────────────────────

    #[test]
    fn record_increments_event_count() {
        let log = InferenceAuditLog::new();
        log.record(1, AuditEventKind::RequestReceived, "http", "d1", None);
        log.record(2, AuditEventKind::InferenceCompleted, "http", "d2", Some(42));
        assert_eq!(log.event_count(), 2);
    }

    #[test]
    fn record_fields_correct() {
        let log = InferenceAuditLog::new_with_retention();
        log.record(99, AuditEventKind::InferenceCompleted, "ollama", "ok", Some(123));
        let recs = log.records();
        assert_eq!(recs.len(), 1);
        let r = &recs[0];
        assert_eq!(r.seq, 0);
        assert_eq!(r.request_id, 99);
        assert_eq!(r.kind, AuditEventKind::InferenceCompleted);
        assert_eq!(r.backend, "ollama");
        assert_eq!(r.detail, "ok");
        assert_eq!(r.latency_ms, Some(123));
    }

    #[test]
    fn record_seq_monotonically_increases() {
        let log = InferenceAuditLog::new_with_retention();
        for i in 0..5 {
            log.record(i, AuditEventKind::RequestReceived, "b", "d", None);
        }
        let recs = log.records();
        for (i, r) in recs.iter().enumerate() {
            assert_eq!(r.seq, i as u64);
        }
    }

    #[test]
    fn record_without_retention_does_not_store() {
        let log = InferenceAuditLog::new();
        log.record(1, AuditEventKind::InferenceFailed, "b", "err", None);
        assert!(log.records().is_empty());
    }

    // ── records() / event_count() / uptime_ms() ───────────────────────────

    #[test]
    fn event_count_matches_records_len() {
        let log = InferenceAuditLog::new_with_retention();
        log.record(1, AuditEventKind::StreamStarted, "b", "d", None);
        log.record(1, AuditEventKind::StreamCompleted, "b", "d", None);
        assert_eq!(log.event_count(), log.records().len() as u64);
    }

    #[test]
    fn uptime_ms_increases_over_time() {
        let log = InferenceAuditLog::new();
        let t0 = log.uptime_ms();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(log.uptime_ms() >= t0);
    }

    // ── AuditRecord Display ───────────────────────────────────────────────

    #[test]
    fn audit_record_display_contains_key_fields() {
        let log = InferenceAuditLog::new_with_retention();
        log.record(42, AuditEventKind::InferenceCompleted, "llama", "done", Some(77));
        let s = log.records()[0].to_string();
        assert!(s.contains("seq=0"));
        assert!(s.contains("rid=42"));
        assert!(s.contains("InferenceCompleted"));
        assert!(s.contains("llama"));
        assert!(s.contains("done"));
    }
}
