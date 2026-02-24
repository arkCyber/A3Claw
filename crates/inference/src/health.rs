//! Backend health monitor — periodic liveness probes (DO-178C inspired).
//!
//! `BackendHealth` tracks the liveness of a single backend by recording
//! the outcomes of probe calls. The status transitions are:
//!
//! ```text
//! Unknown → Healthy  (first successful probe)
//! Healthy → Degraded (1-2 consecutive failures)
//! Degraded → Unhealthy (3+ consecutive failures)
//! Any → Healthy (any successful probe)
//! ```

use crate::types::BackendKind;
use std::fmt;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

// ── HealthStatus ────────────────────────────────────────────────────────────────

/// Liveness status of a single backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// No probe has been run yet.
    Unknown,
    /// Last probe succeeded; backend is operating normally.
    Healthy,
    /// 1-2 consecutive probe failures; backend may be recovering.
    Degraded,
    /// 3+ consecutive probe failures; backend is considered down.
    Unhealthy,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Unknown   => write!(f, "Unknown"),
            HealthStatus::Healthy   => write!(f, "Healthy"),
            HealthStatus::Degraded  => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
        }
    }
}

impl HealthStatus {
    /// Returns `true` if the backend should be considered usable.
    ///
    /// `Unknown` and `Degraded` are treated as usable (best-effort);
    /// `Unhealthy` is not.
    pub fn is_usable(&self) -> bool {
        !matches!(self, HealthStatus::Unhealthy)
    }
}

// ── BackendHealth ──────────────────────────────────────────────────────────────

/// Tracks the health of a single inference backend.
pub struct BackendHealth {
    /// Which backend this monitor tracks.
    pub backend: BackendKind,
    /// Current health status.
    pub status: HealthStatus,
    /// Timestamp of the last probe (success or failure).
    pub last_probe: Option<Instant>,
    /// Latency of the last successful probe.
    pub last_latency_ms: Option<u64>,
    /// Number of consecutive probe failures since the last success.
    pub consecutive_failures: u32,
    /// Total successful probes since construction.
    pub total_ok: u64,
    /// Total failed probes since construction.
    pub total_fail: u64,
    /// Threshold of consecutive failures before transitioning to Unhealthy.
    pub unhealthy_threshold: u32,
}

impl BackendHealth {
    /// Create a new health monitor in the `Unknown` state.
    pub fn new(backend: BackendKind) -> Self {
        Self::with_threshold(backend, 3)
    }

    /// Create a health monitor with a custom unhealthy threshold.
    pub fn with_threshold(backend: BackendKind, unhealthy_threshold: u32) -> Self {
        debug_assert!(unhealthy_threshold >= 1, "unhealthy_threshold must be >= 1");
        Self {
            backend,
            status: HealthStatus::Unknown,
            last_probe: None,
            last_latency_ms: None,
            consecutive_failures: 0,
            total_ok: 0,
            total_fail: 0,
            unhealthy_threshold,
        }
    }

    /// Record a successful probe with the given round-trip latency.
    pub fn record_probe_ok(&mut self, latency_ms: u64) {
        self.last_probe = Some(Instant::now());
        self.last_latency_ms = Some(latency_ms);
        self.consecutive_failures = 0;
        self.total_ok = self.total_ok.saturating_add(1);
        let prev = std::mem::replace(&mut self.status, HealthStatus::Healthy);
        if prev != HealthStatus::Healthy {
            info!(
                backend = %self.backend,
                latency_ms,
                prev_status = %prev,
                total_ok = self.total_ok,
                "backend health: → Healthy"
            );
        } else {
            debug!(
                backend = %self.backend,
                latency_ms,
                total_ok = self.total_ok,
                "backend health: probe ok (Healthy)"
            );
        }
    }

    /// Record a failed probe with a human-readable reason.
    pub fn record_probe_fail(&mut self, reason: &str) {
        self.last_probe = Some(Instant::now());
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.total_fail = self.total_fail.saturating_add(1);
        let new_status = if self.consecutive_failures >= self.unhealthy_threshold {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        };
        let prev = std::mem::replace(&mut self.status, new_status);
        warn!(
            backend = %self.backend,
            consecutive_failures = self.consecutive_failures,
            threshold = self.unhealthy_threshold,
            total_fail = self.total_fail,
            prev_status = %prev,
            status = %self.status,
            reason,
            "backend health: probe failed"
        );
    }

    /// Returns `true` if the last probe was within `max_age`.
    pub fn is_fresh(&self, max_age: Duration) -> bool {
        self.last_probe
            .map(|t| t.elapsed() < max_age)
            .unwrap_or(false)
    }

    /// Returns `true` if the backend should be considered usable.
    pub fn is_usable(&self) -> bool {
        self.status.is_usable()
    }

    /// Total number of probes run (ok + fail).
    pub fn total_probes(&self) -> u64 {
        self.total_ok.saturating_add(self.total_fail)
    }
}

impl fmt::Display for BackendHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BackendHealth[{}] status={} consecutive_failures={} total=({}/{})",
            self.backend, self.status,
            self.consecutive_failures,
            self.total_ok, self.total_fail
        )
    }
}

// ── HTTP health probe ─────────────────────────────────────────────────────────────

/// Probe an HTTP inference backend's health endpoint.
///
/// Tries the following paths in order, returning on the first 2xx response:
/// 1. `{endpoint}/health`    — llama.cpp
/// 2. `{endpoint}/api/tags`  — Ollama
/// 3. `{endpoint}/v1/models` — OpenAI-compat
///
/// Returns the round-trip latency in milliseconds, or an error string.
pub async fn probe_http(endpoint: &str, timeout: Duration) -> Result<u64, String> {
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    let urls = [
        format!("{endpoint}/health"),
        format!("{endpoint}/api/tags"),
        format!("{endpoint}/v1/models"),
    ];

    for url in &urls {
        debug!(url = %url, "health probe attempt");
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency_ms = start.elapsed().as_millis() as u64;
                debug!(url = %url, latency_ms, "health probe succeeded");
                return Ok(latency_ms);
            }
            Ok(resp) => {
                debug!(url = %url, status = %resp.status(), "health probe: non-2xx response");
            }
            Err(e) => {
                debug!(url = %url, error = %e, "health probe: request failed");
            }
        }
    }

    Err(format!("no health endpoint responded at {endpoint}"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BackendKind;

    fn health() -> BackendHealth {
        BackendHealth::new(BackendKind::LlamaCppHttp)
    }

    // ── HealthStatus ────────────────────────────────────────────────────

    #[test]
    fn health_status_display() {
        assert_eq!(HealthStatus::Unknown.to_string(),   "Unknown");
        assert_eq!(HealthStatus::Healthy.to_string(),   "Healthy");
        assert_eq!(HealthStatus::Degraded.to_string(),  "Degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "Unhealthy");
    }

    #[test]
    fn health_status_is_usable() {
        assert!(HealthStatus::Unknown.is_usable());
        assert!(HealthStatus::Healthy.is_usable());
        assert!(HealthStatus::Degraded.is_usable());
        assert!(!HealthStatus::Unhealthy.is_usable());
    }

    // ── BackendHealth::new() ───────────────────────────────────────────────

    #[test]
    fn new_starts_unknown() {
        let h = health();
        assert_eq!(h.status, HealthStatus::Unknown);
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.total_ok, 0);
        assert_eq!(h.total_fail, 0);
        assert!(h.last_probe.is_none());
        assert!(h.is_usable());
    }

    #[test]
    fn with_threshold_sets_threshold() {
        let h = BackendHealth::with_threshold(BackendKind::Ollama, 5);
        assert_eq!(h.unhealthy_threshold, 5);
    }

    // ── record_probe_ok() ─────────────────────────────────────────────────

    #[test]
    fn probe_ok_transitions_to_healthy() {
        let mut h = health();
        h.record_probe_ok(10);
        assert_eq!(h.status, HealthStatus::Healthy);
        assert_eq!(h.last_latency_ms, Some(10));
        assert_eq!(h.total_ok, 1);
        assert_eq!(h.consecutive_failures, 0);
    }

    #[test]
    fn probe_ok_resets_consecutive_failures() {
        let mut h = health();
        h.record_probe_fail("err");
        h.record_probe_fail("err");
        h.record_probe_ok(5);
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.status, HealthStatus::Healthy);
    }

    #[test]
    fn probe_ok_increments_total_ok() {
        let mut h = health();
        h.record_probe_ok(1);
        h.record_probe_ok(2);
        assert_eq!(h.total_ok, 2);
    }

    #[test]
    fn probe_ok_sets_last_probe() {
        let mut h = health();
        assert!(h.last_probe.is_none());
        h.record_probe_ok(0);
        assert!(h.last_probe.is_some());
    }

    // ── record_probe_fail() ───────────────────────────────────────────────

    #[test]
    fn probe_fail_transitions_to_degraded() {
        let mut h = health();
        h.record_probe_fail("timeout");
        assert_eq!(h.status, HealthStatus::Degraded);
        assert_eq!(h.consecutive_failures, 1);
        assert_eq!(h.total_fail, 1);
    }

    #[test]
    fn probe_fail_transitions_to_unhealthy_at_threshold() {
        let mut h = health(); // threshold = 3
        h.record_probe_fail("e");
        h.record_probe_fail("e");
        assert_eq!(h.status, HealthStatus::Degraded);
        h.record_probe_fail("e");
        assert_eq!(h.status, HealthStatus::Unhealthy);
        assert!(!h.is_usable());
    }

    #[test]
    fn probe_fail_increments_total_fail() {
        let mut h = health();
        h.record_probe_fail("e");
        h.record_probe_fail("e");
        assert_eq!(h.total_fail, 2);
    }

    // ── is_fresh() ──────────────────────────────────────────────────────────

    #[test]
    fn is_fresh_false_when_no_probe() {
        let h = health();
        assert!(!h.is_fresh(Duration::from_secs(60)));
    }

    #[test]
    fn is_fresh_true_immediately_after_probe() {
        let mut h = health();
        h.record_probe_ok(0);
        assert!(h.is_fresh(Duration::from_secs(60)));
    }

    #[test]
    fn is_fresh_false_after_max_age_elapsed() {
        let mut h = health();
        h.record_probe_ok(0);
        // max_age of 0 means always stale
        assert!(!h.is_fresh(Duration::from_millis(0)));
    }

    // ── total_probes() ──────────────────────────────────────────────────────

    #[test]
    fn total_probes_sums_ok_and_fail() {
        let mut h = health();
        h.record_probe_ok(1);
        h.record_probe_ok(2);
        h.record_probe_fail("e");
        assert_eq!(h.total_probes(), 3);
    }

    // ── Display ───────────────────────────────────────────────────────────

    #[test]
    fn display_contains_backend_and_status() {
        let h = health();
        let s = h.to_string();
        assert!(s.contains("llama.cpp HTTP"));
        assert!(s.contains("Unknown"));
    }
}
