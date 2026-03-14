//! Per-agent session registry.
//!
//! Maps `session_id` (UUID string) → `AgentSession` so the Gateway can look
//! up which `AgentProfile` owns an incoming hook call.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Opaque session identifier (UUID v4 string).
pub type SessionId = String;

/// Runtime state for one active agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: SessionId,
    pub agent_id: String,
    pub agent_name: String,
    pub agent_role: String,
    /// Active task run ID (set when a run starts, cleared when it finishes).
    pub run_id: Option<String>,
    /// Gateway port this session is connected to.
    pub gateway_port: u16,
    /// Session start time (Unix seconds).
    pub started_at: u64,
    /// Last activity time (Unix seconds).
    pub last_active_at: u64,
    /// Whether the session is healthy (not circuit-broken).
    pub healthy: bool,
}

impl AgentSession {
    pub fn new(
        agent_id: impl Into<String>,
        agent_name: impl Into<String>,
        agent_role: impl Into<String>,
        gateway_port: u16,
    ) -> Self {
        let now = now_unix_secs();
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            agent_name: agent_name.into(),
            agent_role: agent_role.into(),
            run_id: None,
            gateway_port,
            started_at: now,
            last_active_at: now,
            healthy: true,
        }
    }

    pub fn touch(&mut self) {
        self.last_active_at = now_unix_secs();
    }

    pub fn age_secs(&self) -> u64 {
        now_unix_secs().saturating_sub(self.started_at)
    }
}

// ── Session registry ──────────────────────────────────────────────────────────

/// Thread-safe registry of all active agent sessions.
#[derive(Clone)]
pub struct SessionRegistry {
    inner: Arc<RwLock<HashMap<SessionId, AgentSession>>>,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self { inner: Arc::new(RwLock::new(HashMap::new())) }
    }
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new session and return its ID.
    pub fn register(&self, session: AgentSession) -> SessionId {
        let id = session.session_id.clone();
        self.inner.write().insert(id.clone(), session);
        id
    }

    /// Look up a session by ID.
    pub fn get(&self, session_id: &str) -> Option<AgentSession> {
        self.inner.read().get(session_id).cloned()
    }

    /// Look up the session for a given agent (returns the most recent one).
    pub fn get_by_agent(&self, agent_id: &str) -> Option<AgentSession> {
        self.inner
            .read()
            .values()
            .filter(|s| s.agent_id == agent_id)
            .max_by_key(|s| s.started_at)
            .cloned()
    }

    /// Update a session in-place.
    pub fn update<F>(&self, session_id: &str, f: F)
    where
        F: FnOnce(&mut AgentSession),
    {
        if let Some(session) = self.inner.write().get_mut(session_id) {
            f(session);
        }
    }

    /// Remove a session (called when agent stops).
    pub fn remove(&self, session_id: &str) -> Option<AgentSession> {
        self.inner.write().remove(session_id)
    }

    /// All active sessions.
    pub fn all(&self) -> Vec<AgentSession> {
        self.inner.read().values().cloned().collect()
    }

    /// Sessions for a specific agent.
    pub fn for_agent(&self, agent_id: &str) -> Vec<AgentSession> {
        self.inner
            .read()
            .values()
            .filter(|s| s.agent_id == agent_id)
            .cloned()
            .collect()
    }

    /// Number of active sessions.
    pub fn count(&self) -> usize {
        self.inner.read().len()
    }

    /// Evict sessions older than `max_age_secs` with no recent activity.
    pub fn evict_stale(&self, max_age_secs: u64) {
        let now = now_unix_secs();
        let mut map = self.inner.write();
        map.retain(|_, s| now.saturating_sub(s.last_active_at) < max_age_secs);
    }
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(agent_id: &str) -> AgentSession {
        AgentSession::new(agent_id, "Test Agent", "DataAnalyst", 7878)
    }

    #[test]
    fn register_and_get() {
        let reg = SessionRegistry::new();
        let s = make_session("agent-001");
        let id = reg.register(s);
        let found = reg.get(&id).unwrap();
        assert_eq!(found.agent_id, "agent-001");
    }

    #[test]
    fn get_by_agent() {
        let reg = SessionRegistry::new();
        reg.register(make_session("agent-A"));
        reg.register(make_session("agent-B"));
        assert!(reg.get_by_agent("agent-A").is_some());
        assert!(reg.get_by_agent("agent-C").is_none());
    }

    #[test]
    fn remove_session() {
        let reg = SessionRegistry::new();
        let s = make_session("agent-X");
        let id = reg.register(s);
        assert_eq!(reg.count(), 1);
        reg.remove(&id);
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn update_session() {
        let reg = SessionRegistry::new();
        let s = make_session("agent-Y");
        let id = reg.register(s);
        reg.update(&id, |s| s.run_id = Some("run-123".into()));
        let found = reg.get(&id).unwrap();
        assert_eq!(found.run_id.as_deref(), Some("run-123"));
    }

    #[test]
    fn evict_stale_removes_old() {
        let reg = SessionRegistry::new();
        let mut s = make_session("agent-old");
        // Backdate last_active_at by 1000 seconds
        s.last_active_at = s.last_active_at.saturating_sub(1000);
        reg.register(s);
        reg.evict_stale(500);
        assert_eq!(reg.count(), 0);
    }
}
