//! Circuit Breaker for the OpenClaw+ sandbox.
//!
//! Automatically trips (forcefully terminates the WasmEdge instance) when any
//! of the following anomalies are detected:
//!
//! - Too many denied operations within a sliding time window (possible attack loop).
//! - Memory usage exceeds the configured limit.
//! - Too many dangerous shell command attempts in a session.
//! - The user manually triggers an emergency stop from the UI.

use crate::types::{EventKind, SandboxEvent};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::error;

/// The reason the circuit breaker was tripped.
#[derive(Debug, Clone)]
pub enum TripReason {
    /// Too many denied operations within the sliding window — possible attack loop.
    TooManyDenials { count: u64, window_secs: u64 },
    /// Too many dangerous shell command attempts in this session.
    TooManyDangerousCommands { count: u64 },
    /// Sandbox memory usage exceeded the configured limit.
    MemoryExceeded { limit_mb: u32 },
    /// The user clicked the Emergency Stop button in the UI.
    ManualTrip,
    /// The sandbox became unresponsive for longer than the configured timeout.
    Timeout { secs: u64 },
}

impl std::fmt::Display for TripReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TripReason::TooManyDenials { count, window_secs } => write!(
                f,
                "{} denials in {} s — possible attack loop",
                count, window_secs
            ),
            TripReason::TooManyDangerousCommands { count } => {
                write!(f, "{} dangerous command attempts", count)
            }
            TripReason::MemoryExceeded { limit_mb } => {
                write!(f, "Memory exceeded {} MB limit", limit_mb)
            }
            TripReason::ManualTrip => write!(f, "Emergency stop triggered by user"),
            TripReason::Timeout { secs } => {
                write!(f, "Sandbox unresponsive for {} s", secs)
            }
        }
    }
}

/// Current state of the circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakerState {
    /// Normal operation — the sandbox is running.
    Closed,
    /// Tripped — the sandbox should be terminated. Inner string is the reason.
    Open(String),
}

/// Configuration for the [`CircuitBreaker`] thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerConfig {
    /// Length of the sliding denial-count window, in seconds.
    pub denial_window_secs: u64,
    /// Maximum number of denied operations allowed within the window before tripping.
    pub max_denials_per_window: u64,
    /// Maximum number of dangerous shell command denials before tripping.
    pub max_dangerous_commands: u64,
    /// Memory limit in megabytes (mirrors [`SecurityConfig::memory_limit_mb`]).
    pub memory_limit_mb: u32,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            denial_window_secs: 10,
            max_denials_per_window: 20,
            max_dangerous_commands: 3,
            memory_limit_mb: 512,
        }
    }
}

/// Automatic circuit breaker that monitors sandbox events and trips when
/// anomaly thresholds are exceeded.
///
/// When tripped, a [`TripReason`] is sent over the `trip_rx` channel so the
/// sandbox runner and UI can react (terminate the WasmEdge instance, show a
/// banner, etc.).
///
/// All threshold checks are lock-free where possible (atomic counters) and
/// use a `Mutex<Vec<Instant>>` only for the sliding-window timestamp list.
pub struct CircuitBreaker {
    config: BreakerConfig,
    /// `true` once the breaker has been tripped; never resets automatically.
    tripped: AtomicBool,
    /// The reason the breaker was tripped (set once, never cleared).
    trip_reason: Mutex<Option<TripReason>>,
    /// Total number of denied operations since the session started.
    denial_count: AtomicU64,
    /// Total number of dangerous shell command denials since the session started.
    dangerous_count: AtomicU64,
    /// Timestamps of denied operations within the current sliding window.
    denial_timestamps: Mutex<Vec<Instant>>,
    /// Channel used to notify the sandbox runner and UI when the breaker trips.
    trip_tx: flume::Sender<TripReason>,
}

impl CircuitBreaker {
    /// Creates a new `CircuitBreaker` and returns it together with the receiver
    /// end of the trip notification channel.
    ///
    /// The caller should poll `trip_rx` in a background task to react when the
    /// breaker trips.
    pub fn new(config: BreakerConfig) -> (Arc<Self>, flume::Receiver<TripReason>) {
        let (trip_tx, trip_rx) = flume::unbounded();
        let breaker = Arc::new(Self {
            config,
            tripped: AtomicBool::new(false),
            trip_reason: Mutex::new(None),
            denial_count: AtomicU64::new(0),
            dangerous_count: AtomicU64::new(0),
            denial_timestamps: Mutex::new(Vec::new()),
            trip_tx,
        });
        (breaker, trip_rx)
    }

    /// Processes a sandbox event and trips the breaker if any threshold is exceeded.
    ///
    /// This method is a no-op if the breaker has already been tripped.
    pub async fn process_event(&self, event: &SandboxEvent) {
        if self.tripped.load(Ordering::SeqCst) {
            return;
        }

        if event.allowed == Some(false) {
            self.denial_count.fetch_add(1, Ordering::SeqCst);
            self.record_denial().await;

            // Check the sliding-window denial count.
            let window_count = self.count_recent_denials().await;
            if window_count >= self.config.max_denials_per_window {
                self.trip(TripReason::TooManyDenials {
                    count: window_count,
                    window_secs: self.config.denial_window_secs,
                })
                .await;
                return;
            }
        }

        // Check dangerous shell command threshold.
        if (event.kind == EventKind::ShellExec || event.kind == EventKind::ProcessSpawn)
            && event.allowed == Some(false)
        {
            let count = self.dangerous_count.fetch_add(1, Ordering::SeqCst) + 1;
            if count >= self.config.max_dangerous_commands {
                self.trip(TripReason::TooManyDangerousCommands { count }).await;
            }
        }

        // Memory limit exceeded.
        if event.kind == EventKind::MemoryLimit {
            self.trip(TripReason::MemoryExceeded {
                limit_mb: self.config.memory_limit_mb,
            })
            .await;
        }
    }

    /// Trips the breaker immediately with [`TripReason::ManualTrip`].
    ///
    /// Called when the user clicks the **Emergency Stop** button in the UI.
    pub async fn manual_trip(&self) {
        self.trip(TripReason::ManualTrip).await;
    }

    /// Returns `true` if the breaker has been tripped.
    pub fn is_tripped(&self) -> bool {
        self.tripped.load(Ordering::SeqCst)
    }

    /// Returns the reason the breaker was tripped, or `None` if it has not tripped yet.
    pub async fn trip_reason(&self) -> Option<TripReason> {
        self.trip_reason.lock().await.clone()
    }

    /// Returns a snapshot of the current breaker statistics for UI display.
    pub fn stats(&self) -> BreakerStats {
        BreakerStats {
            total_denials: self.denial_count.load(Ordering::SeqCst),
            dangerous_commands: self.dangerous_count.load(Ordering::SeqCst),
            is_tripped: self.is_tripped(),
        }
    }

    /// Resets the breaker to its initial state.
    ///
    /// Intended for use in tests or manual recovery flows only.
    /// Does **not** restart the sandbox.
    pub async fn reset(&self) {
        self.tripped.store(false, Ordering::SeqCst);
        *self.trip_reason.lock().await = None;
        self.denial_count.store(0, Ordering::SeqCst);
        self.dangerous_count.store(0, Ordering::SeqCst);
        self.denial_timestamps.lock().await.clear();
    }

    /// Internal: atomically sets the tripped flag and broadcasts the reason.
    ///
    /// Idempotent — subsequent calls after the first trip are silently ignored.
    async fn trip(&self, reason: TripReason) {
        if self.tripped.swap(true, Ordering::SeqCst) {
            return; // Already tripped — ignore duplicate signals.
        }
        error!("Circuit breaker tripped: {}", reason);
        *self.trip_reason.lock().await = Some(reason.clone());
        let _ = self.trip_tx.send_async(reason).await;
    }

    /// Records the current timestamp in the sliding-window list and evicts
    /// entries that have fallen outside the window.
    async fn record_denial(&self) {
        let mut timestamps = self.denial_timestamps.lock().await;
        timestamps.push(Instant::now());
        let window = Duration::from_secs(self.config.denial_window_secs);
        timestamps.retain(|t| t.elapsed() < window);
    }

    /// Counts the number of denial timestamps still within the sliding window.
    async fn count_recent_denials(&self) -> u64 {
        let timestamps = self.denial_timestamps.lock().await;
        let window = Duration::from_secs(self.config.denial_window_secs);
        timestamps.iter().filter(|t| t.elapsed() < window).count() as u64
    }
}

/// Snapshot of circuit breaker counters, used by the UI dashboard.
#[derive(Debug, Clone, Default)]
pub struct BreakerStats {
    /// Total denied operations since the sandbox session started.
    pub total_denials: u64,
    /// Total dangerous shell command denials since the sandbox session started.
    pub dangerous_commands: u64,
    /// Whether the circuit breaker has been tripped.
    pub is_tripped: bool,
}
