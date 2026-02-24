//! Per-backend circuit breaker — aerospace fault containment pattern (DO-178C inspired).
//!
//! # State machine
//! ```text
//!  ┌────────┐  threshold failures  ┌──────┐
//!  │ Closed │ ──────────────────▶  │ Open │
//!  └────────┘                      └──────┘
//!      ▲                              │ reset_duration elapsed
//!      │  probe success               ▼
//!      └──────────────────── ┌──────────┐
//!         probe failure ───▶ │ HalfOpen │
//!                            └──────────┘
//! ```
//!
//! # Guarantees
//! - All state transitions are logged at appropriate severity levels.
//! - `allow()` is the single gate; callers MUST call `record_success` or
//!   `record_failure` after every attempt that was allowed through.
//! - Counters never overflow: `failures` saturates at `u32::MAX`.

use std::fmt;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

// ── State ─────────────────────────────────────────────────────────────────────

/// The three states of the circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakerState {
    /// Normal operation — all calls pass through.
    Closed,
    /// Backend declared unavailable — calls are rejected immediately.
    Open,
    /// One probe call allowed to test recovery.
    HalfOpen,
}

impl fmt::Display for BreakerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BreakerState::Closed   => write!(f, "Closed"),
            BreakerState::Open     => write!(f, "Open"),
            BreakerState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

// ── CircuitBreaker ────────────────────────────────────────────────────────────

/// Per-backend circuit breaker with full audit trail.
pub struct CircuitBreaker {
    /// Human-readable backend name used in log fields.
    name: String,
    /// Consecutive failure count that triggers the Open state.
    threshold: u32,
    /// How long to stay Open before transitioning to HalfOpen.
    reset_duration: Duration,
    /// Current consecutive failure count.
    failures: u32,
    /// Total lifetime success count.
    total_successes: u64,
    /// Total lifetime failure count.
    total_failures: u64,
    /// Current state.
    state: BreakerState,
    /// Timestamp when the breaker was last opened.
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker in the `Closed` state.
    ///
    /// # Arguments
    /// * `name`           — backend identifier used in log output.
    /// * `threshold`      — consecutive failures before opening (must be ≥ 1).
    /// * `reset_duration` — how long to stay open before probing.
    ///
    /// # Panics
    /// Panics in debug builds if `threshold == 0`.
    pub fn new(name: impl Into<String>, threshold: u32, reset_duration: Duration) -> Self {
        debug_assert!(threshold >= 1, "circuit breaker threshold must be >= 1");
        let name = name.into();
        info!(
            backend = %name,
            threshold,
            reset_secs = reset_duration.as_secs(),
            "circuit breaker initialised (Closed)"
        );
        Self {
            name,
            threshold,
            reset_duration,
            failures: 0,
            total_successes: 0,
            total_failures: 0,
            state: BreakerState::Closed,
            opened_at: None,
        }
    }

    // ── Gate ──────────────────────────────────────────────────────────────────

    /// Returns `true` if a call should be allowed through.
    ///
    /// Side-effect: transitions `Open → HalfOpen` when `reset_duration` has
    /// elapsed, logging the transition.
    pub fn allow(&mut self) -> bool {
        match self.state {
            BreakerState::Closed => {
                debug!(backend = %self.name, "circuit breaker: allow (Closed)");
                true
            }
            BreakerState::Open => {
                match self.opened_at {
                    Some(opened) if opened.elapsed() >= self.reset_duration => {
                        self.state = BreakerState::HalfOpen;
                        info!(
                            backend = %self.name,
                            elapsed_ms = opened.elapsed().as_millis(),
                            "circuit breaker: Open → HalfOpen (probe allowed)"
                        );
                        true
                    }
                    Some(opened) => {
                        let remaining_ms = self.reset_duration
                            .saturating_sub(opened.elapsed())
                            .as_millis();
                        debug!(
                            backend = %self.name,
                            remaining_ms,
                            "circuit breaker: call rejected (Open)"
                        );
                        false
                    }
                    None => {
                        warn!(backend = %self.name, "circuit breaker: Open but opened_at is None — treating as Open");
                        false
                    }
                }
            }
            BreakerState::HalfOpen => {
                debug!(backend = %self.name, "circuit breaker: allow (HalfOpen probe)");
                true
            }
        }
    }

    // ── Outcome recording ─────────────────────────────────────────────────────

    /// Record a successful call.
    ///
    /// Resets the failure counter and transitions to `Closed` from any state.
    pub fn record_success(&mut self) {
        self.total_successes = self.total_successes.saturating_add(1);
        let prev_state = self.state.clone();
        self.failures = 0;
        self.state = BreakerState::Closed;
        self.opened_at = None;

        if prev_state != BreakerState::Closed {
            info!(
                backend = %self.name,
                prev_state = %prev_state,
                total_successes = self.total_successes,
                "circuit breaker: → Closed (recovered after success)"
            );
        } else {
            debug!(
                backend = %self.name,
                total_successes = self.total_successes,
                "circuit breaker: success recorded (Closed)"
            );
        }
    }

    /// Record a failed call.
    ///
    /// Increments the failure counter and opens the breaker if the threshold
    /// is reached. Saturating arithmetic prevents counter overflow.
    pub fn record_failure(&mut self) {
        self.failures = self.failures.saturating_add(1);
        self.total_failures = self.total_failures.saturating_add(1);

        if self.failures >= self.threshold && self.state != BreakerState::Open {
            self.state = BreakerState::Open;
            self.opened_at = Some(Instant::now());
            warn!(
                backend = %self.name,
                failures = self.failures,
                threshold = self.threshold,
                total_failures = self.total_failures,
                reset_secs = self.reset_duration.as_secs(),
                "circuit breaker: → Open (threshold reached)"
            );
        } else {
            debug!(
                backend = %self.name,
                failures = self.failures,
                threshold = self.threshold,
                state = %self.state,
                "circuit breaker: failure recorded"
            );
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// Current state of the breaker.
    pub fn state(&self) -> &BreakerState { &self.state }

    /// Current consecutive failure count.
    pub fn failures(&self) -> u32 { self.failures }

    /// Backend name this breaker guards.
    pub fn name(&self) -> &str { &self.name }

    /// Configured failure threshold.
    pub fn threshold(&self) -> u32 { self.threshold }

    /// Timestamp when the breaker was last opened, if any.
    pub fn opened_at(&self) -> Option<Instant> { self.opened_at }

    /// Remaining time in the Open state before a probe is allowed.
    /// Returns `None` if not Open or if `opened_at` is unset.
    pub fn remaining_reset(&self) -> Option<Duration> {
        if self.state != BreakerState::Open {
            return None;
        }
        self.opened_at.map(|t| {
            self.reset_duration.saturating_sub(t.elapsed())
        })
    }

    /// Total lifetime successes recorded.
    pub fn total_successes(&self) -> u64 { self.total_successes }

    /// Total lifetime failures recorded.
    pub fn total_failures(&self) -> u64 { self.total_failures }

    /// Returns `true` when the breaker is in the `Open` state.
    pub fn is_open(&self) -> bool { self.state == BreakerState::Open }

    /// Returns `true` when the breaker is in the `Closed` state.
    pub fn is_closed(&self) -> bool { self.state == BreakerState::Closed }
}

impl fmt::Display for CircuitBreaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CircuitBreaker[{}] state={} failures={}/{} total=({}/{})",
            self.name, self.state, self.failures, self.threshold,
            self.total_successes, self.total_failures
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn breaker(threshold: u32) -> CircuitBreaker {
        CircuitBreaker::new("test-backend", threshold, Duration::from_secs(60))
    }

    // ── new() ─────────────────────────────────────────────────────────────

    #[test]
    fn new_starts_closed() {
        let cb = breaker(3);
        assert_eq!(cb.state(), &BreakerState::Closed);
        assert_eq!(cb.failures(), 0);
        assert!(cb.is_closed());
        assert!(!cb.is_open());
    }

    #[test]
    fn new_accessors_correct() {
        let cb = CircuitBreaker::new("my-backend", 5, Duration::from_secs(10));
        assert_eq!(cb.name(), "my-backend");
        assert_eq!(cb.threshold(), 5);
        assert_eq!(cb.total_successes(), 0);
        assert_eq!(cb.total_failures(), 0);
        assert!(cb.opened_at().is_none());
        assert!(cb.remaining_reset().is_none());
    }

    // ── allow() ───────────────────────────────────────────────────────────

    #[test]
    fn allow_closed_returns_true() {
        let mut cb = breaker(3);
        assert!(cb.allow());
    }

    #[test]
    fn allow_open_returns_false_before_reset() {
        let mut cb = breaker(1);
        cb.record_failure();
        assert!(cb.is_open());
        assert!(!cb.allow(), "should reject while Open and reset not elapsed");
    }

    #[test]
    fn allow_open_transitions_to_halfopen_after_reset() {
        let mut cb = CircuitBreaker::new("b", 1, Duration::from_millis(1));
        cb.record_failure();
        assert!(cb.is_open());
        std::thread::sleep(Duration::from_millis(5));
        assert!(cb.allow(), "should allow probe after reset elapsed");
        assert_eq!(cb.state(), &BreakerState::HalfOpen);
    }

    #[test]
    fn allow_halfopen_returns_true() {
        let mut cb = CircuitBreaker::new("b", 1, Duration::from_millis(1));
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(5));
        cb.allow(); // transitions to HalfOpen
        assert!(cb.allow());
    }

    // ── record_success() ──────────────────────────────────────────────────

    #[test]
    fn record_success_resets_failures() {
        let mut cb = breaker(3);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.failures(), 0);
        assert!(cb.is_closed());
    }

    #[test]
    fn record_success_from_halfopen_closes() {
        let mut cb = CircuitBreaker::new("b", 1, Duration::from_millis(1));
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(5));
        cb.allow(); // → HalfOpen
        cb.record_success();
        assert!(cb.is_closed());
        assert_eq!(cb.total_successes(), 1);
    }

    #[test]
    fn record_success_increments_total() {
        let mut cb = breaker(3);
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.total_successes(), 2);
    }

    // ── record_failure() ──────────────────────────────────────────────────

    #[test]
    fn record_failure_increments_counter() {
        let mut cb = breaker(3);
        cb.record_failure();
        assert_eq!(cb.failures(), 1);
        assert_eq!(cb.total_failures(), 1);
    }

    #[test]
    fn record_failure_opens_at_threshold() {
        let mut cb = breaker(3);
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_open());
        cb.record_failure();
        assert!(cb.is_open());
        assert!(cb.opened_at().is_some());
    }

    #[test]
    fn record_failure_does_not_reopen_already_open() {
        let mut cb = breaker(1);
        cb.record_failure(); // opens
        let opened = cb.opened_at().unwrap();
        std::thread::sleep(Duration::from_millis(2));
        cb.record_failure(); // should not reset opened_at
        // opened_at should be the original timestamp (within 1ms tolerance)
        let diff = cb.opened_at().unwrap().duration_since(opened);
        assert!(diff < Duration::from_millis(2), "opened_at must not be reset on repeated failures");
    }

    // ── remaining_reset() ─────────────────────────────────────────────────

    #[test]
    fn remaining_reset_none_when_closed() {
        let cb = breaker(3);
        assert!(cb.remaining_reset().is_none());
    }

    #[test]
    fn remaining_reset_some_when_open() {
        let mut cb = breaker(1);
        cb.record_failure();
        assert!(cb.remaining_reset().is_some());
    }

    // ── Display ───────────────────────────────────────────────────────────

    #[test]
    fn display_contains_name_and_state() {
        let cb = breaker(3);
        let s = cb.to_string();
        assert!(s.contains("test-backend"));
        assert!(s.contains("Closed"));
    }

    // ── BreakerState Display ──────────────────────────────────────────────

    #[test]
    fn breaker_state_display() {
        assert_eq!(BreakerState::Closed.to_string(),   "Closed");
        assert_eq!(BreakerState::Open.to_string(),     "Open");
        assert_eq!(BreakerState::HalfOpen.to_string(), "HalfOpen");
    }
}
