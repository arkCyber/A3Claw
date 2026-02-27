//! Axum HTTP router for the OpenClaw plugin gateway.
//!
//! Endpoints:
//! - `GET   /health`               — liveness probe
//! - `GET   /ready`                — readiness probe
//! - `POST  /hooks/before-skill`   — intercept a Skill before execution
//! - `POST  /hooks/after-skill`    — record a Skill after execution
//! - `POST  /hooks/agent-start`    — agent session started
//! - `POST  /hooks/agent-stop`     — agent session stopped
//! - `POST  /hooks/confirm`        — UI resolves a pending confirmation (allow/deny)
//! - `GET   /skills/status`        — security status (exposed as a Skill)
//! - `GET   /skills/events`        — recent audit events
//! - `PATCH /skills/policy`        — update security policy at runtime
//! - `POST  /skills/allow/:id`     — allow a pending confirmation by event ID
//! - `POST  /skills/deny/:id`      — deny a pending confirmation by event ID

use crate::skill_registry::RiskLevel;
use crate::state::{GatewayState, SessionProfile};
use crate::types::{
    AckResponse, AfterSkillPayload, AgentStartPayload, AgentStopPayload, BeforeSkillPayload,
    BeforeSkillResponse, PolicyUpdateRequest, PolicyUpdateResponse, StatusResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use openclaw_security::{EventKind, ResourceKind, SandboxEvent};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Builds the complete Axum router, injecting shared [`GatewayState`].
pub fn build_router(state: Arc<GatewayState>) -> Router {
    Router::new()
        // Health / readiness
        .route("/health", get(health))
        .route("/ready",  get(ready))
        // OpenClaw plugin hooks
        .route("/hooks/before-skill",       post(before_skill))
        .route("/hooks/after-skill",        post(after_skill))
        .route("/hooks/agent-start",        post(agent_start))
        .route("/hooks/agent-stop",         post(agent_stop))
        .route("/hooks/confirm",            post(hook_confirm))
        .route("/hooks/session-register",   post(session_register))
        .route("/hooks/session-deregister", post(session_deregister))
        // Skills exposed by this plugin
        .route("/skills/status",      get(skill_status))
        .route("/skills/events",      get(skill_events))
        .route("/skills/policy",      patch(skill_update_policy))
        .route("/skills/allow/:id",   post(skill_allow))
        .route("/skills/deny/:id",    post(skill_deny))
        .route("/admin/emergency-stop", post(admin_emergency_stop))
        .with_state(state)
}

// ── Health / readiness ────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn ready(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let ready = state.is_ready();
    let code = if ready { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ready })))
}

// ── Hook: before-skill ────────────────────────────────────────────────────────

async fn before_skill(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<BeforeSkillPayload>,
) -> impl IntoResponse {
    info!(
        skill      = %payload.skill_name,
        session    = %payload.session_id,
        invocation = %payload.invocation_id,
        "before-skill hook"
    );

    // ── Circuit-breaker short-circuit ─────────────────────────────────────────
    // When the breaker has tripped, block ALL further Skill calls immediately.
    if state.is_breaker_tripped() {
        warn!(skill = %payload.skill_name, "Circuit breaker is tripped — Skill blocked");
        state.increment_denied();
        return (
            StatusCode::OK,
            Json(BeforeSkillResponse::deny(
                "[OpenClaw+] Circuit breaker has tripped. All Skill calls are blocked. \
                 Restart the agent session to reset.",
            )),
        );
    }

    let risk = state.registry.risk_level(&payload.skill_name);

    // Build a SandboxEvent for the audit ring buffer.
    // We borrow `payload` here before the match so the event is recorded
    // regardless of the verdict.
    let event = skill_to_sandbox_event(&payload);
    state.record_event(event);

    // ── Per-session capability filter ─────────────────────────────────────────
    // Deny the skill if the session's AgentProfile does not include this capability.
    if !state.is_skill_allowed_for_session(&payload.session_id, &payload.skill_name) {
        warn!(
            skill   = %payload.skill_name,
            session = %payload.session_id,
            "Skill blocked: not in session capability list"
        );
        state.increment_denied();
        return (
            StatusCode::OK,
            Json(BeforeSkillResponse::deny(format!(
                "[OpenClaw+] Skill '{}' is not in this agent's allowed capabilities.",
                payload.skill_name
            ))),
        );
    }

    let response = match risk {
        RiskLevel::Deny => {
            warn!(skill = %payload.skill_name, "Skill blocked by policy (Deny)");
            state.increment_denied();
            BeforeSkillResponse::deny(format!(
                "[OpenClaw+] Skill '{}' is blocked by the security policy.",
                payload.skill_name
            ))
        }

        RiskLevel::Confirm => {
            if state.is_permanently_allowed(&payload.skill_name) {
                debug!(skill = %payload.skill_name, "Skill in permanent-allow set");
                state.increment_allowed();
                BeforeSkillResponse::allow()
            } else {
                info!(skill = %payload.skill_name, "Skill requires confirmation");
                let event_id = state.last_event_id();
                state.increment_pending();
                BeforeSkillResponse::confirm(format!(
                    "OpenClaw wants to run the '{}' skill.\n\nArguments:\n{}\n\nAllow? (event_id={})",
                    payload.skill_name,
                    serde_json::to_string_pretty(&payload.args)
                        .unwrap_or_else(|_| "{}".to_string()),
                    event_id,
                ))
            }
        }

        RiskLevel::Safe => {
            debug!(skill = %payload.skill_name, "Skill allowed (Safe)");
            state.increment_allowed();
            BeforeSkillResponse::allow()
        }
    };

    (StatusCode::OK, Json(response))
}

// ── Hook: after-skill ─────────────────────────────────────────────────────────

async fn after_skill(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<AfterSkillPayload>,
) -> impl IntoResponse {
    debug!(
        skill    = %payload.skill_name,
        success  = payload.success,
        duration = payload.duration_ms,
        "after-skill hook"
    );

    // Feed outcome back into the circuit breaker.
    if !payload.success {
        if let Some(err) = &payload.error {
            state.record_skill_error(&payload.skill_name, err);
        }
    }

    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Hook: agent-start ─────────────────────────────────────────────────────────

async fn agent_start(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<AgentStartPayload>,
) -> impl IntoResponse {
    info!(session = %payload.session_id, "Agent session started");
    state.on_agent_start(&payload.session_id);
    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Hook: session-register ────────────────────────────────────────────────────

/// Payload for `POST /hooks/session-register`.
/// Sent by the executor right after acquiring a session ID so the gateway
/// can enforce per-agent capability policy on subsequent skill calls.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionRegisterPayload {
    session_id:           String,
    agent_id:             String,
    agent_name:           String,
    agent_role:           String,
    allowed_capabilities: Vec<String>,
}

async fn session_register(
    State(state): State<Arc<GatewayState>>,
    Json(p): Json<SessionRegisterPayload>,
) -> impl IntoResponse {
    info!(
        session = %p.session_id,
        agent   = %p.agent_id,
        caps    = p.allowed_capabilities.len(),
        "Session profile registered"
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.register_session_profile(
        p.session_id,
        SessionProfile {
            agent_id:             p.agent_id,
            agent_name:           p.agent_name,
            agent_role:           p.agent_role,
            allowed_capabilities: p.allowed_capabilities,
            registered_at:        now,
        },
    );
    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Hook: session-deregister ──────────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionDeregisterPayload {
    session_id: String,
}

async fn session_deregister(
    State(state): State<Arc<GatewayState>>,
    Json(p): Json<SessionDeregisterPayload>,
) -> impl IntoResponse {
    info!(session = %p.session_id, "Session profile deregistered");
    state.on_agent_stop(&p.session_id);
    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Hook: confirm (UI resolves a pending confirmation) ───────────────────────

/// Request body for `POST /hooks/confirm`.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmPayload {
    event_id: u64,
    allowed: bool,
}

async fn hook_confirm(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<ConfirmPayload>,
) -> impl IntoResponse {
    info!(
        event_id = payload.event_id,
        allowed  = payload.allowed,
        "Confirmation resolved via hook"
    );
    state.resolve_confirmation(payload.event_id, payload.allowed);
    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Hook: agent-stop ──────────────────────────────────────────────────────────

async fn agent_stop(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<AgentStopPayload>,
) -> impl IntoResponse {
    info!(
        session = %payload.session_id,
        reason  = payload.reason.as_deref().unwrap_or("none"),
        "Agent session stopped"
    );
    state.on_agent_stop(&payload.session_id);
    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Skill: status ─────────────────────────────────────────────────────────────

async fn skill_status(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let stats = state.stats();
    let breaker = state.breaker_stats();
    (
        StatusCode::OK,
        Json(StatusResponse {
            sandbox_running:          stats.sandbox_running,
            breaker_tripped:          breaker.is_tripped,
            total_events:             stats.total_events,
            allowed_events:           stats.allowed_count,
            denied_events:            stats.denied_count,
            pending_events:           stats.pending_count,
            dangerous_commands_blocked: breaker.dangerous_commands,
        }),
    )
}

// ── Skill: events ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct EventsQuery {
    limit: Option<usize>,
}

async fn skill_events(
    State(state): State<Arc<GatewayState>>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(50).min(500);
    let events = state.recent_events(limit);
    (StatusCode::OK, Json(events))
}

// ── Skill: allow / deny a pending confirmation ───────────────────────────────

async fn skill_allow(
    State(state): State<Arc<GatewayState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    info!(event_id = id, "Operator allowed pending confirmation");
    state.resolve_confirmation(id, true);
    (StatusCode::OK, Json(AckResponse::ok()))
}

async fn skill_deny(
    State(state): State<Arc<GatewayState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    info!(event_id = id, "Operator denied pending confirmation");
    state.resolve_confirmation(id, false);
    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Skill: update policy ──────────────────────────────────────────────────────

async fn skill_update_policy(
    State(state): State<Arc<GatewayState>>,
    Json(req): Json<PolicyUpdateRequest>,
) -> impl IntoResponse {
    let mut applied = Vec::new();

    if let Some(allowlist) = req.network_allowlist {
        state.update_network_allowlist(allowlist);
        applied.push("networkAllowlist".to_string());
    }
    if let Some(v) = req.confirm_shell_exec {
        state.set_confirm_shell_exec(v);
        applied.push("confirmShellExec".to_string());
    }
    if let Some(v) = req.confirm_file_delete {
        state.set_confirm_file_delete(v);
        applied.push("confirmFileDelete".to_string());
    }
    if let Some(v) = req.confirm_network {
        state.set_confirm_network(v);
        applied.push("confirmNetwork".to_string());
    }
    if let Some(v) = req.intercept_shell {
        state.set_intercept_shell(v);
        applied.push("interceptShell".to_string());
    }

    info!(?applied, "Security policy updated via plugin Skill");
    (StatusCode::OK, Json(PolicyUpdateResponse { ok: true, applied }))
}

// ── Admin: emergency stop ─────────────────────────────────────────────────────────

/// Trips the circuit breaker and marks all future Skill calls as denied.
/// Called by the UI when the operator clicks the Emergency Stop button.
async fn admin_emergency_stop(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    warn!("Emergency stop triggered by operator");
    state.trip_breaker_manual();
    (StatusCode::OK, Json(AckResponse::ok()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────────────────────

/// Converts a [`BeforeSkillPayload`] into a [`SandboxEvent`] for audit logging.
fn skill_to_sandbox_event(payload: &BeforeSkillPayload) -> SandboxEvent {
    // Map well-known skill categories to EventKind.
    let kind = if payload.skill_name.starts_with("shell.") || payload.skill_name.starts_with("process.") {
        EventKind::ShellExec
    } else if payload.skill_name.starts_with("fs.") {
        // Distinguish write/delete/read by action suffix.
        let action = payload.skill_name.split('.').nth(1).unwrap_or("");
        if action.contains("delete") || action.contains("unlink") || action.contains("rm") {
            EventKind::FileDelete
        } else if action.contains("write") || action.contains("append") || action.contains("mkdir") {
            EventKind::FileWrite
        } else {
            EventKind::FileAccess
        }
    } else if payload.skill_name.starts_with("web.")
        || payload.skill_name.starts_with("network.")
        || payload.skill_name.starts_with("search.")
    {
        EventKind::NetworkRequest
    } else {
        EventKind::FileAccess // generic fallback
    };

    // Extract a path/target from args if present.
    let path = payload.args.get("path")
        .or_else(|| payload.args.get("url"))
        .or_else(|| payload.args.get("host"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    SandboxEvent::new(
        0, // id assigned by interceptor
        kind,
        ResourceKind::File,
        path,
        &format!("skill:{}", payload.skill_name),
    )
}
