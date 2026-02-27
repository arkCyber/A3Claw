//! Shared mutable state for the plugin gateway.
//!
//! [`GatewayState`] is wrapped in `Arc` and injected into every Axum handler.
//! All mutation goes through `parking_lot::Mutex` / `RwLock` so the state is
//! safe to share across the async Tokio thread pool.

use crate::skill_registry::SkillRegistry;
use openclaw_security::{BreakerStats, SandboxEvent, SecurityConfig};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::warn;

/// Maximum number of events kept in the in-memory ring buffer.
const MAX_EVENTS: usize = 500;

/// Per-session agent profile registered by the executor at task start.
/// Carries the capability whitelist so `before_skill` can enforce per-agent policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProfile {
    /// Agent UUID (matches AgentProfile::id).
    pub agent_id: String,
    /// Human-readable display name.
    pub agent_name: String,
    /// Role string (e.g. "IntelOfficer").
    pub agent_role: String,
    /// Capability IDs the agent is allowed to use (e.g. ["web.fetch", "shell.read"]).
    pub allowed_capabilities: Vec<String>,
    /// Unix timestamp when the session was registered.
    pub registered_at: u64,
}

/// Live statistics snapshot returned by `/skills/status`.
#[derive(Debug, Clone, Default)]
pub struct GatewayStats {
    pub sandbox_running: bool,
    pub total_events:    u64,
    pub allowed_count:   u64,
    pub denied_count:    u64,
    pub pending_count:   u64,
}

/// All shared state for the gateway process.
pub struct GatewayState {
    // ── Skill registry ────────────────────────────────────────────────────────
    pub registry: SkillRegistry,

    // ── Security config (hot-reloadable) ─────────────────────────────────────
    config: RwLock<SecurityConfig>,

    // ── Event ring buffer ─────────────────────────────────────────────────────
    events: RwLock<VecDeque<SandboxEvent>>,

    // ── Per-session permanent-allow set ──────────────────────────────────────
    permanent_allow: RwLock<HashSet<String>>,

    // ── Atomic counters ───────────────────────────────────────────────────────
    total_events:  AtomicU64,
    allowed_count: AtomicU64,
    denied_count:  AtomicU64,
    pending_count: AtomicU64,

    // ── Circuit-breaker shadow state ──────────────────────────────────────────
    breaker_tripped:      AtomicBool,
    dangerous_commands:   AtomicU64,
    total_denials:        AtomicU64,

    // ── Readiness flag ────────────────────────────────────────────────────────
    ready: AtomicBool,

    // ── Active session IDs ────────────────────────────────────────────────────
    active_sessions: RwLock<HashSet<String>>,

    // ── Per-session AgentProfile metadata (session_id → SessionProfile) ──────
    session_profiles: RwLock<HashMap<String, SessionProfile>>,

    // ── Event ID counter ─────────────────────────────────────────────────────
    next_event_id: AtomicU64,

    // ── Pending confirmations (event_id -> oneshot responder) ─────────────────
    pending_confirmations: RwLock<HashMap<u64, oneshot::Sender<bool>>>,
}

impl GatewayState {
    /// Creates a new `GatewayState` from the given config.
    pub fn new(config: SecurityConfig) -> Arc<Self> {
        Arc::new(Self {
            registry:              SkillRegistry::with_defaults(),
            config:                RwLock::new(config),
            events:                RwLock::new(VecDeque::with_capacity(MAX_EVENTS)),
            permanent_allow:       RwLock::new(HashSet::new()),
            total_events:          AtomicU64::new(0),
            allowed_count:         AtomicU64::new(0),
            denied_count:          AtomicU64::new(0),
            pending_count:         AtomicU64::new(0),
            breaker_tripped:       AtomicBool::new(false),
            dangerous_commands:    AtomicU64::new(0),
            total_denials:         AtomicU64::new(0),
            ready:                 AtomicBool::new(false),
            active_sessions:       RwLock::new(HashSet::new()),
            session_profiles:      RwLock::new(HashMap::new()),
            next_event_id:         AtomicU64::new(1),
            pending_confirmations: RwLock::new(HashMap::new()),
        })
    }

    // ── Readiness ─────────────────────────────────────────────────────────────

    pub fn set_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    // ── Event recording ───────────────────────────────────────────────────────

    /// Appends a [`SandboxEvent`] to the ring buffer (evicts oldest if full).
    pub fn record_event(&self, event: SandboxEvent) {
        let id = self.next_event_id.fetch_add(1, Ordering::Relaxed);
        self.total_events.fetch_add(1, Ordering::Relaxed);

        // Stamp the event with the assigned ID.
        let mut stamped = event;
        stamped.id = id;

        let mut buf = self.events.write();
        if buf.len() >= MAX_EVENTS {
            buf.pop_front();
        }
        buf.push_back(stamped);
    }

    /// Returns the `n` most recent events (newest last).
    pub fn recent_events(&self, n: usize) -> Vec<SandboxEvent> {
        let buf = self.events.read();
        buf.iter().rev().take(n).cloned().collect::<Vec<_>>()
            .into_iter().rev().collect()
    }

    // ── Counters ──────────────────────────────────────────────────────────────

    pub fn increment_allowed(&self) {
        self.allowed_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_denied(&self) {
        self.denied_count.fetch_add(1, Ordering::Relaxed);
        self.total_denials.fetch_add(1, Ordering::Relaxed);
        self.check_circuit_breaker();
    }

    pub fn increment_pending(&self) {
        self.pending_count.fetch_add(1, Ordering::Relaxed);
    }

    // ── Stats snapshot ────────────────────────────────────────────────────────

    pub fn stats(&self) -> GatewayStats {
        GatewayStats {
            sandbox_running: !self.active_sessions.read().is_empty(),
            total_events:    self.total_events.load(Ordering::Relaxed),
            allowed_count:   self.allowed_count.load(Ordering::Relaxed),
            denied_count:    self.denied_count.load(Ordering::Relaxed),
            pending_count:   self.pending_count.load(Ordering::Relaxed),
        }
    }

    pub fn breaker_stats(&self) -> BreakerStats {
        BreakerStats {
            is_tripped:        self.breaker_tripped.load(Ordering::Relaxed),
            total_denials:     self.total_denials.load(Ordering::Relaxed),
            dangerous_commands: self.dangerous_commands.load(Ordering::Relaxed),
        }
    }

    // ── Circuit breaker ───────────────────────────────────────────────────────

    /// Records a Skill error; increments dangerous-command counter if the
    /// skill name matches a high-risk pattern.
    pub fn record_skill_error(&self, skill_name: &str, _error: &str) {
        if skill_name.starts_with("shell.")
            || skill_name.starts_with("process.")
            || skill_name.starts_with("system.")
        {
            self.dangerous_commands.fetch_add(1, Ordering::Relaxed);
            self.check_circuit_breaker();
        }
    }

    /// Trips the circuit breaker if thresholds are exceeded.
    fn check_circuit_breaker(&self) {
        let config = self.config.read();
        let denials   = self.total_denials.load(Ordering::Relaxed);
        let dangerous = self.dangerous_commands.load(Ordering::Relaxed);

        let trip = denials   >= config.circuit_breaker.max_denials_per_window
                || dangerous >= config.circuit_breaker.max_dangerous_commands;

        if trip && !self.breaker_tripped.load(Ordering::Relaxed) {
            self.breaker_tripped.store(true, Ordering::Release);
            warn!(
                denials, dangerous,
                "Circuit breaker tripped — all further Skill calls will be denied"
            );
        }
    }

    /// Returns `true` when the circuit breaker has tripped.
    pub fn is_breaker_tripped(&self) -> bool {
        self.breaker_tripped.load(Ordering::Acquire)
    }

    /// Trips the circuit breaker immediately (manual emergency stop).
    pub fn trip_breaker_manual(&self) {
        if !self.breaker_tripped.swap(true, Ordering::AcqRel) {
            warn!("Circuit breaker tripped manually via emergency stop");
        }
    }

    // ── Permanent-allow set ───────────────────────────────────────────────────

    pub fn is_permanently_allowed(&self, skill_name: &str) -> bool {
        self.permanent_allow.read().contains(skill_name)
    }

    #[allow(dead_code)]
    pub fn add_permanent_allow(&self, skill_name: impl Into<String>) {
        self.permanent_allow.write().insert(skill_name.into());
    }

    // ── Pending confirmation management ──────────────────────────────────────

    /// Returns the ID that was assigned to the most recently recorded event.
    pub fn last_event_id(&self) -> u64 {
        self.next_event_id.load(Ordering::Relaxed).saturating_sub(1)
    }

    /// Registers a one-shot channel for an event awaiting user confirmation.
    ///
    /// Returns the receiver end so the caller can `await` the user's decision.
    #[allow(dead_code)]
    pub fn register_confirmation(&self, event_id: u64) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        self.pending_confirmations.write().insert(event_id, tx);
        rx
    }

    /// Resolves a pending confirmation — called by the UI via `/hooks/confirm`
    /// or `/skills/allow/:id` / `/skills/deny/:id`.
    pub fn resolve_confirmation(&self, event_id: u64, allowed: bool) {
        if let Some(tx) = self.pending_confirmations.write().remove(&event_id) {
            let _ = tx.send(allowed);
        }
    }

    /// Returns the IDs of all events currently awaiting confirmation.
    #[allow(dead_code)]
    pub fn pending_confirmation_ids(&self) -> Vec<u64> {
        self.pending_confirmations.read().keys().copied().collect()
    }

    // ── Session lifecycle ─────────────────────────────────────────────────────

    pub fn on_agent_start(&self, session_id: &str) {
        self.active_sessions.write().insert(session_id.to_string());
    }

    pub fn on_agent_stop(&self, session_id: &str) {
        self.active_sessions.write().remove(session_id);
        self.session_profiles.write().remove(session_id);
    }

    // ── Per-session AgentProfile routing ──────────────────────────────────────

    /// Register or update the AgentProfile for an active session.
    pub fn register_session_profile(&self, session_id: impl Into<String>, profile: SessionProfile) {
        self.session_profiles.write().insert(session_id.into(), profile);
    }

    /// Look up the AgentProfile for a session.
    pub fn session_profile(&self, session_id: &str) -> Option<SessionProfile> {
        self.session_profiles.read().get(session_id).cloned()
    }

    /// Returns `true` if `skill_name` is in the session's capability whitelist.
    /// If no profile is registered for the session, the check passes (default-allow).
    pub fn is_skill_allowed_for_session(&self, session_id: &str, skill_name: &str) -> bool {
        match self.session_profiles.read().get(session_id) {
            None => true, // no profile → default-allow (backward-compat)
            Some(profile) => {
                if profile.allowed_capabilities.is_empty() {
                    return true; // empty list → allow all
                }
                profile.allowed_capabilities.iter().any(|cap| {
                    cap == skill_name
                        || skill_name.starts_with(&format!("{cap}."))
                        || cap.ends_with('*') && skill_name.starts_with(&cap[..cap.len()-1])
                })
            }
        }
    }

    // ── Policy hot-reload ─────────────────────────────────────────────────────

    pub fn update_network_allowlist(&self, list: Vec<String>) {
        self.config.write().network_allowlist = list;
    }

    pub fn set_confirm_shell_exec(&self, v: bool) {
        self.config.write().confirm_shell_exec = v;
    }

    pub fn set_confirm_file_delete(&self, v: bool) {
        self.config.write().confirm_file_delete = v;
    }

    pub fn set_confirm_network(&self, v: bool) {
        self.config.write().confirm_network = v;
    }

    pub fn set_intercept_shell(&self, v: bool) {
        self.config.write().intercept_shell = v;
    }

    /// Returns a snapshot of the current config.
    #[allow(dead_code)]
    pub fn config_snapshot(&self) -> SecurityConfig {
        self.config.read().clone()
    }
}
