//! Top-level AgentExecutor: spawns ReAct loops, manages lifecycle, reports progress.

use crate::bootstrap::{bootstrap, BootstrapConfig, BootstrapResult};
use crate::context::{TaskContext, TaskGoal};
use crate::dispatch::SkillDispatcher;
use crate::error::ExecutorError;
use crate::react::{build_system_prompt, run_react_loop, LlmClient, ReactConfig};
use crate::session::{AgentSession, SessionRegistry};
use crate::skill::SkillSet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

// ── Executor configuration ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// LLM endpoint (OpenAI-compat or Ollama).
    pub llm_endpoint: String,
    pub model: String,
    pub api_key: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub is_ollama: bool,
    /// Gateway HTTP port.
    pub gateway_port: u16,
    /// Max ReAct steps per run.
    pub max_steps: u32,
    /// Task timeout (seconds).
    pub timeout_secs: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            llm_endpoint: "http://localhost:11434".into(),
            model: "qwen2.5:0.5b".into(),
            api_key: String::new(),
            temperature: 0.2,
            max_tokens: 2048,
            is_ollama: true,
            gateway_port: 7878,
            max_steps: 20,
            timeout_secs: 300,
        }
    }
}

// ── Executor events (streamed to UI) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ExecutorEvent {
    /// Bootstrap started for agent.
    Bootstrapping { agent_id: String },
    /// Bootstrap complete.
    BootstrapDone { agent_id: String, openclaw_available: bool, quickjs_available: bool },
    /// Task run started.
    RunStarted { agent_id: String, run_id: String, goal: String },
    /// A ReAct step completed.
    StepCompleted {
        run_id: String,
        step: u32,
        skill_name: String,
        allowed: bool,
        observation_snippet: String,
        elapsed_ms: u64,
    },
    /// Task finished with a final answer.
    RunFinished { run_id: String, answer: String, elapsed_secs: u64 },
    /// Task failed.
    RunFailed { run_id: String, error_code: String, message: String },
    /// Skill was denied by gateway.
    SkillDenied { run_id: String, skill_name: String, reason: String },
    /// Agent session registered.
    SessionRegistered { agent_id: String, session_id: String },
    /// Progress message (free text for UI log).
    Log { run_id: String, message: String },
}

// ── Executor handle (returned to caller) ─────────────────────────────────────

/// Handle to a running or completed executor task.
pub struct ExecutorHandle {
    /// Receive events from the executor.
    pub events: mpsc::Receiver<ExecutorEvent>,
    /// Run ID assigned to this task.
    pub run_id: String,
}

// ── AgentExecutor ─────────────────────────────────────────────────────────────

pub struct AgentExecutor {
    config: ExecutorConfig,
    sessions: Arc<SessionRegistry>,
}

impl AgentExecutor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(SessionRegistry::new()),
        }
    }

    pub fn with_shared_sessions(config: ExecutorConfig, sessions: Arc<SessionRegistry>) -> Self {
        Self { config, sessions }
    }

    pub fn sessions(&self) -> Arc<SessionRegistry> {
        self.sessions.clone()
    }

    /// Launch a task for an agent.  Returns an [`ExecutorHandle`] immediately;
    /// events stream in via the contained `mpsc::Receiver`.
    pub fn run(
        &self,
        agent_id: impl Into<String>,
        agent_name: impl Into<String>,
        agent_role: impl Into<String>,
        capability_ids: Vec<String>,
        goal: impl Into<String>,
    ) -> ExecutorHandle {
        let agent_id = agent_id.into();
        let agent_name = agent_name.into();
        let agent_role = agent_role.into();
        let goal_str = goal.into();
        let run_id = uuid::Uuid::new_v4().to_string();
        let config = self.config.clone();
        let sessions = self.sessions.clone();

        let (tx, rx) = mpsc::channel::<ExecutorEvent>(128);

        let run_id_clone = run_id.clone();
        tokio::spawn(async move {
            let result = run_task(
                agent_id,
                agent_name,
                agent_role,
                capability_ids,
                goal_str,
                run_id_clone.clone(),
                config,
                sessions,
                tx.clone(),
            )
            .await;

            if let Err(e) = result {
                let _ = tx.send(ExecutorEvent::RunFailed {
                    run_id: run_id_clone,
                    error_code: e.code().to_string(),
                    message: e.to_string(),
                }).await;
            }
        });

        ExecutorHandle { events: rx, run_id }
    }

    /// Synchronous blocking version — runs the task to completion and returns
    /// the final answer.  Intended for tests and CLI use.
    pub async fn run_sync(
        &self,
        agent_id: impl Into<String>,
        agent_name: impl Into<String>,
        agent_role: impl Into<String>,
        capability_ids: Vec<String>,
        goal: impl Into<String>,
    ) -> Result<String, ExecutorError> {
        let mut handle = self.run(agent_id, agent_name, agent_role, capability_ids, goal);
        let mut answer = String::new();

        while let Some(event) = handle.events.recv().await {
            match event {
                ExecutorEvent::RunFinished { answer: a, .. } => {
                    answer = a;
                    break;
                }
                ExecutorEvent::RunFailed { error_code, message, .. } => {
                    return Err(ExecutorError::Other(anyhow::anyhow!(
                        "[{}] {}",
                        error_code,
                        message
                    )));
                }
                ExecutorEvent::Log { message, .. } => {
                    info!("[executor] {}", message);
                }
                _ => {}
            }
        }

        if answer.is_empty() {
            Err(ExecutorError::Other(anyhow::anyhow!("Executor closed without answer")))
        } else {
            Ok(answer)
        }
    }
}

// ── Inner task runner ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn run_task(
    agent_id: String,
    agent_name: String,
    agent_role: String,
    capability_ids: Vec<String>,
    goal_str: String,
    run_id: String,
    config: ExecutorConfig,
    sessions: Arc<SessionRegistry>,
    tx: mpsc::Sender<ExecutorEvent>,
) -> Result<(), ExecutorError> {
    let send = |ev: ExecutorEvent| {
        let tx2 = tx.clone();
        async move { let _ = tx2.send(ev).await; }
    };

    // ── 1. Bootstrap workspace ────────────────────────────────────────────────
    send(ExecutorEvent::Bootstrapping { agent_id: agent_id.clone() }).await;

    let boot_cfg = BootstrapConfig::new(agent_id.clone(), config.gateway_port);
    let boot: BootstrapResult = bootstrap(&boot_cfg).await.map_err(|e| {
        ExecutorError::BootstrapError(e.to_string())
    })?;

    send(ExecutorEvent::BootstrapDone {
        agent_id: agent_id.clone(),
        openclaw_available: boot.openclaw_available,
        quickjs_available: boot.quickjs_available,
    }).await;

    // ── 2. Register session ───────────────────────────────────────────────────
    let session = AgentSession::new(
        agent_id.clone(),
        agent_name.clone(),
        agent_role.clone(),
        config.gateway_port,
    );
    let session_id = sessions.register(session);
    sessions.update(&session_id, |s| s.run_id = Some(run_id.clone()));

    send(ExecutorEvent::SessionRegistered {
        agent_id: agent_id.clone(),
        session_id: session_id.clone(),
    }).await;

    // Notify gateway: session started + register capability profile
    let gw_url = format!("http://localhost:{}", config.gateway_port);
    notify_gateway_start(&gw_url, &session_id, &agent_name).await;
    notify_gateway_session_register(
        &gw_url,
        &session_id,
        &agent_id,
        &agent_name,
        &agent_role,
        &capability_ids,
    ).await;

    // ── 3. Build skill set ────────────────────────────────────────────────────
    let skill_set = if capability_ids.is_empty() {
        SkillSet::default_safe()
    } else {
        SkillSet::from_capability_ids(&capability_ids)
    };

    // ── 4. Build task context ─────────────────────────────────────────────────
    let system_prompt = build_system_prompt(&skill_set, &agent_name, &agent_role);
    let goal = TaskGoal::new(goal_str.clone())
        .with_max_steps(config.max_steps)
        .with_timeout(config.timeout_secs);

    let mut ctx = TaskContext::new(run_id.clone(), agent_id.clone(), goal, system_prompt);
    ctx.push_user(&goal_str);

    send(ExecutorEvent::RunStarted {
        agent_id: agent_id.clone(),
        run_id: run_id.clone(),
        goal: goal_str.clone(),
    }).await;

    // ── 5. Build ReAct components ─────────────────────────────────────────────
    let react_cfg = ReactConfig {
        llm_endpoint: config.llm_endpoint.clone(),
        model: config.model.clone(),
        api_key: config.api_key.clone(),
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        is_ollama: config.is_ollama,
        gateway_url: gw_url.clone(),
    };
    let llm = LlmClient::new(react_cfg);
    let dispatcher = SkillDispatcher::new(&gw_url);

    // ── 6. Run ReAct loop with step event forwarding ──────────────────────────
    let loop_result = run_react_loop(&mut ctx, &skill_set, &llm, &dispatcher, &session_id).await;

    // Forward all completed steps as events
    for step in &ctx.steps {
        let _ = tx.send(ExecutorEvent::StepCompleted {
            run_id: run_id.clone(),
            step: step.step_index,
            skill_name: step.skill_name.clone(),
            allowed: step.allowed,
            observation_snippet: step.observation.chars().take(120).collect(),
            elapsed_ms: step.elapsed_ms,
        }).await;
    }

    // ── 7. Finalise ───────────────────────────────────────────────────────────
    notify_gateway_stop(&gw_url, &session_id).await;
    sessions.update(&session_id, |s| s.run_id = None);
    sessions.remove(&session_id);

    match loop_result {
        Ok(answer) => {
            let elapsed = ctx.elapsed_secs();
            let _ = tx.send(ExecutorEvent::RunFinished {
                run_id: run_id.clone(),
                answer,
                elapsed_secs: elapsed,
            }).await;

            // Persist final context
            let _ = ctx.save();
            info!(agent_id, run_id = %run_id, elapsed, "Task completed");
            Ok(())
        }
        Err(e) => {
            warn!(agent_id, run_id = %run_id, error = %e, "Task failed");
            Err(e)
        }
    }
}

// ── Gateway lifecycle notifications ──────────────────────────────────────────

async fn notify_gateway_start(gw_url: &str, session_id: &str, agent_name: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/hooks/agent-start", gw_url))
        .json(&serde_json::json!({
            "sessionId": session_id,
            "agentName": agent_name,
            "timestamp": iso_now()
        }))
        .send()
        .await;
}

async fn notify_gateway_session_register(
    gw_url: &str,
    session_id: &str,
    agent_id: &str,
    agent_name: &str,
    agent_role: &str,
    capability_ids: &[String],
) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/hooks/session-register", gw_url))
        .json(&serde_json::json!({
            "sessionId":           session_id,
            "agentId":             agent_id,
            "agentName":           agent_name,
            "agentRole":           agent_role,
            "allowedCapabilities": capability_ids,
        }))
        .send()
        .await;
}

async fn notify_gateway_stop(gw_url: &str, session_id: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/hooks/agent-stop", gw_url))
        .json(&serde_json::json!({
            "sessionId": session_id,
            "reason": "task_completed",
            "timestamp": iso_now()
        }))
        .send()
        .await;
}

fn iso_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}Z", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executor_config_default_values() {
        let cfg = ExecutorConfig::default();
        assert_eq!(cfg.gateway_port, 7878);
        assert_eq!(cfg.max_steps, 20);
        assert!(cfg.is_ollama);
        assert_eq!(cfg.model, "qwen2.5:0.5b");
    }

    #[test]
    fn executor_event_serialization() {
        let ev = ExecutorEvent::RunStarted {
            agent_id: "agent-001".into(),
            run_id: "run-001".into(),
            goal: "Test".into(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("RunStarted"));
        assert!(json.contains("agent-001"));
    }

    #[test]
    fn run_failed_event_has_error_code() {
        let ev = ExecutorEvent::RunFailed {
            run_id: "run-001".into(),
            error_code: "TIMEOUT".into(),
            message: "Task timed out".into(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("TIMEOUT"));
    }

    #[tokio::test]
    async fn executor_new_has_empty_sessions() {
        let ex = AgentExecutor::new(ExecutorConfig::default());
        assert_eq!(ex.sessions().count(), 0);
    }
}
