//! Skill dispatch: sends skill calls to the OpenClaw+ Gateway via HTTP,
//! interprets verdicts, and returns structured observations.
//!
//! ## Skill execution layers
//!
//! ```text
//! run_react_loop
//!   └─ SkillDispatcher::dispatch(session_id, skill_name, args, ctx)
//!        ├─ 1. Gateway before-skill hook  (security check)
//!        ├─ 2. execute_skill(name, args, ctx)  ← built-in impls HERE
//!        │       ├─ fs.*        — std::fs
//!        │       ├─ web.*       — reqwest
//!        │       ├─ search.*    — DuckDuckGo HTML scrape
//!        │       ├─ agent.*     — TaskContext memory / introspection
//!        │       ├─ knowledge.* — RAG stub (requires inference crate)
//!        │       ├─ email.*     — stub (requires IMAP/SMTP config)
//!        │       ├─ calendar.*  — stub (requires CalDAV config)
//!        │       └─ custom/*    — SkillHandler plugin registry
//!        └─ 3. Gateway after-skill hook   (audit log)
//! ```

use crate::context::TaskContext;
use crate::error::ExecutorError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

// ── Gateway request / response types ─────────────────────────────────────────

/// Outbound call from executor to Gateway `/hooks/before-skill`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BeforeSkillRequest {
    invocation_id: String,
    skill_name: String,
    session_id: String,
    args: HashMap<String, serde_json::Value>,
    timestamp: String,
}

/// Gateway verdict.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum GatewayVerdict {
    Allow,
    Deny,
    Confirm,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BeforeSkillResponse {
    verdict: GatewayVerdict,
    #[serde(default)]
    reason: Option<String>,
}

/// After-skill notification to gateway.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AfterSkillRequest {
    invocation_id: String,
    skill_name: String,
    session_id: String,
    success: bool,
    error: Option<String>,
    duration_ms: u64,
    timestamp: String,
}

// ── Skill execution result ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SkillResult {
    pub skill_name: String,
    pub allowed: bool,
    /// Raw output / observation (string for LLM consumption).
    pub output: String,
    /// True when gateway denied the call.
    pub denied: bool,
    /// True when gateway returned confirm (pending human approval).
    pub pending_confirm: bool,
    pub elapsed_ms: u64,
}

impl SkillResult {
    pub fn denied_result(skill: &str, reason: &str) -> Self {
        Self {
            skill_name: skill.to_string(),
            allowed: false,
            output: format!("[DENIED] Gateway blocked skill '{}': {}", skill, reason),
            denied: true,
            pending_confirm: false,
            elapsed_ms: 0,
        }
    }

    pub fn error_result(skill: &str, err: &str) -> Self {
        Self {
            skill_name: skill.to_string(),
            allowed: false,
            output: format!("[ERROR] Skill '{}' failed: {}", skill, err),
            denied: false,
            pending_confirm: false,
            elapsed_ms: 0,
        }
    }
}

// ── SkillHandler plugin trait ────────────────────────────────────────────────

/// A pluggable skill executor.  Register custom implementations via
/// [`SkillDispatcher::register_handler`] to extend the built-in skill set
/// at runtime (e.g. IMAP email, CalDAV, company-internal APIs).
///
/// # Example
/// ```ignore
/// struct MyEmailHandler;
/// #[async_trait::async_trait]
/// impl SkillHandler for MyEmailHandler {
///     fn skill_names(&self) -> &[&str] { &["email.list", "email.read", "email.send"] }
///     async fn execute(&self, skill: &str, args: &serde_json::Value) -> Result<String, String> {
///         // real IMAP/SMTP logic here
///         Ok(format!("handled: {}", skill))
///     }
/// }
/// dispatcher.register_handler(Arc::new(MyEmailHandler));
/// ```
#[async_trait::async_trait]
pub trait SkillHandler: Send + Sync {
    /// Skill names this handler claims.
    fn skill_names(&self) -> &[&'static str];
    /// Execute the skill and return a text observation.
    async fn execute(
        &self,
        skill_name: &str,
        args: &serde_json::Value,
    ) -> Result<String, String>;
}

// ── Skill dispatcher ─────────────────────────────────────────────────────────

/// Dispatches skill calls through the Gateway and executes them.
///
/// Flow:
/// 1. POST `/hooks/before-skill` → get verdict (allow/deny/confirm)
/// 2. If allowed: execute the skill locally or via plugin handler
/// 3. POST `/hooks/after-skill` → audit log
#[derive(Clone)]
pub struct SkillDispatcher {
    gateway_url: String,
    client: reqwest::Client,
    /// Runtime-registered plugin handlers (email, calendar, custom APIs…).
    handlers: Vec<Arc<dyn SkillHandler>>,
}

impl SkillDispatcher {
    pub fn new(gateway_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("HTTP client build failed");
        Self { gateway_url: gateway_url.into(), client, handlers: Vec::new() }
    }

    /// Register a custom [`SkillHandler`] for skills not covered by built-ins.
    pub fn register_handler(&mut self, handler: Arc<dyn SkillHandler>) {
        self.handlers.push(handler);
    }

    /// Execute a skill call:
    /// 1. Check with gateway
    /// 2. If allowed, run the skill (built-in or plugin)
    /// 3. Notify gateway of outcome
    pub async fn dispatch(
        &self,
        session_id: &str,
        skill_name: &str,
        args: serde_json::Value,
        ctx: &mut TaskContext,
    ) -> Result<SkillResult, ExecutorError> {
        let invocation_id = uuid::Uuid::new_v4().to_string();
        let t0 = std::time::Instant::now();

        // Step 1: before-skill hook
        let verdict = self
            .before_skill_hook(&invocation_id, session_id, skill_name, &args)
            .await?;

        match verdict.verdict {
            GatewayVerdict::Deny => {
                warn!(skill = skill_name, "Gateway denied skill");
                let reason = verdict.reason.unwrap_or_else(|| "policy violation".into());
                self.after_skill_hook(&invocation_id, session_id, skill_name, false, Some(&reason), t0.elapsed().as_millis() as u64).await.ok();
                return Ok(SkillResult::denied_result(skill_name, &reason));
            }
            GatewayVerdict::Confirm => {
                info!(skill = skill_name, "Gateway requires confirmation — treating as pending");
                return Ok(SkillResult {
                    skill_name: skill_name.to_string(),
                    allowed: false,
                    output: format!("[PENDING CONFIRM] Skill '{}' requires human approval.", skill_name),
                    denied: false,
                    pending_confirm: true,
                    elapsed_ms: t0.elapsed().as_millis() as u64,
                });
            }
            GatewayVerdict::Allow => {
                debug!(skill = skill_name, "Gateway allowed skill");
            }
        }

        // Step 2: execute skill (built-in first, then plugin handlers)
        let exec_result = self.execute_skill(skill_name, &args, ctx).await;

        let elapsed = t0.elapsed().as_millis() as u64;
        let (output, success, err_msg) = match exec_result {
            Ok(out) => (out, true, None),
            Err(ref e) => (
                format!("[SKILL ERROR] {}: {}", skill_name, e),
                false,
                Some(e.to_string()),
            ),
        };

        // Step 3: after-skill hook (fire-and-forget)
        self.after_skill_hook(
            &invocation_id,
            session_id,
            skill_name,
            success,
            err_msg.as_deref(),
            elapsed,
        )
        .await
        .ok();

        Ok(SkillResult {
            skill_name: skill_name.to_string(),
            allowed: true,
            output,
            denied: false,
            pending_confirm: false,
            elapsed_ms: elapsed,
        })
    }

    // ── Gateway HTTP calls ────────────────────────────────────────────────

    async fn before_skill_hook(
        &self,
        invocation_id: &str,
        session_id: &str,
        skill_name: &str,
        args: &serde_json::Value,
    ) -> Result<BeforeSkillResponse, ExecutorError> {
        let url = format!("{}/hooks/before-skill", self.gateway_url);
        let args_map: HashMap<String, serde_json::Value> = if let Some(obj) = args.as_object() {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            HashMap::new()
        };

        let payload = BeforeSkillRequest {
            invocation_id: invocation_id.to_string(),
            skill_name: skill_name.to_string(),
            session_id: session_id.to_string(),
            args: args_map,
            timestamp: iso_now(),
        };

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ExecutorError::GatewayUnreachable {
                url: url.clone(),
                source: e.to_string(),
            })?;

        if !resp.status().is_success() {
            return Err(ExecutorError::GatewayUnreachable {
                url,
                source: format!("HTTP {}", resp.status()),
            });
        }

        resp.json::<BeforeSkillResponse>()
            .await
            .map_err(|e| ExecutorError::HttpError(e.to_string()))
    }

    async fn after_skill_hook(
        &self,
        invocation_id: &str,
        session_id: &str,
        skill_name: &str,
        success: bool,
        error: Option<&str>,
        duration_ms: u64,
    ) -> Result<(), ExecutorError> {
        let url = format!("{}/hooks/after-skill", self.gateway_url);
        let payload = AfterSkillRequest {
            invocation_id: invocation_id.to_string(),
            skill_name: skill_name.to_string(),
            session_id: session_id.to_string(),
            success,
            error: error.map(|s| s.to_string()),
            duration_ms,
            timestamp: iso_now(),
        };
        self.client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ExecutorError::HttpError(e.to_string()))?;
        Ok(())
    }

    // ── Skill execution (built-in implementations) ────────────────────────

    /// Execute a skill.  Order of resolution:
    /// 1. Built-in skills (`fs.*`, `web.*`, `search.*`, `agent.*`, `security.*`)
    /// 2. Registered [`SkillHandler`] plugins (email, calendar, custom APIs)
    /// 3. Default stub — returns descriptive message for unimplemented skills
    async fn execute_skill(
        &self,
        skill_name: &str,
        args: &serde_json::Value,
        ctx: &mut TaskContext,
    ) -> Result<String, ExecutorError> {
        match skill_name {
            // ── web.fetch ────────────────────────────────────────────────
            "web.fetch" => {
                let url = args["url"].as_str()
                    .ok_or_else(|| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'url' argument".into(),
                    })?;
                let method = args["method"].as_str().unwrap_or("GET").to_uppercase();
                let resp = self.client.request(
                    reqwest::Method::from_bytes(method.as_bytes())
                        .unwrap_or(reqwest::Method::GET),
                    url,
                ).send().await.map_err(|e| ExecutorError::HttpError(e.to_string()))?;
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                let truncated: String = body.chars().take(2000).collect();
                Ok(format!("HTTP {}\n{}", status, truncated))
            }

            // ── agent.listSkills ─────────────────────────────────────────
            "agent.listSkills" => {
                let names: Vec<&str> = crate::skill::BUILTIN_SKILLS
                    .iter()
                    .filter(|s| s.is_grantable())
                    .map(|s| s.name)
                    .collect();
                Ok(serde_json::to_string(&names).unwrap_or_default())
            }

            // ── agent.getContext ─────────────────────────────────────────
            "agent.getContext" => {
                Ok(ctx.context_summary())
            }

            // ── agent.getMemory ──────────────────────────────────────────
            "agent.getMemory" => {
                let key = args["key"].as_str().unwrap_or("");
                if key.is_empty() {
                    return Ok(format!(
                        "All memory keys: {}",
                        ctx.memory
                            .all()
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                match ctx.memory.get(key) {
                    Some(val) => Ok(val.to_string()),
                    None => Ok(format!("(memory key '{}' not found)", key)),
                }
            }

            // ── agent.setMemory ──────────────────────────────────────────
            "agent.setMemory" => {
                let key = args["key"].as_str().unwrap_or("");
                let value = args["value"].as_str().unwrap_or("");
                if key.is_empty() {
                    return Err(ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'key' argument".into(),
                    });
                }
                ctx.memory.set(key, value);
                let _ = ctx.save();
                Ok(format!("Memory '{}' set to: {}", key, value))
            }

            // ── agent.clearMemory ────────────────────────────────────────
            "agent.clearMemory" => {
                ctx.memory.clear();
                let _ = ctx.save();
                Ok("Agent memory cleared.".to_string())
            }

            // ── agent.delegate ───────────────────────────────────────────
            // Delegates a sub-task to another agent via the Gateway.
            // Requires the target agent to be running and registered.
            "agent.delegate" => {
                let target_agent_id = args["agent_id"].as_str().unwrap_or("");
                let goal = args["goal"].as_str().unwrap_or("");
                if target_agent_id.is_empty() || goal.is_empty() {
                    return Err(ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'agent_id' or 'goal' argument".into(),
                    });
                }
                let url = format!("{}/agent/delegate", self.gateway_url);
                let payload = serde_json::json!({
                    "fromSessionId": ctx.run_id,
                    "targetAgentId": target_agent_id,
                    "goal": goal,
                    "timeoutSecs": args["timeout_secs"].as_u64().unwrap_or(60),
                });
                match self.client.post(&url).json(&payload).send().await {
                    Ok(r) if r.status().is_success() => {
                        let body = r.text().await.unwrap_or_default();
                        Ok(format!("Delegated to agent '{}': {}", target_agent_id, body))
                    }
                    Ok(r) => Ok(format!(
                        "(agent.delegate: gateway returned HTTP {} — \
                         ensure target agent '{}' is running)",
                        r.status(), target_agent_id
                    )),
                    Err(e) => Ok(format!(
                        "(agent.delegate: gateway unreachable — {})", e
                    )),
                }
            }

            // ── security.getStatus ───────────────────────────────────────
            "security.getStatus" => {
                let url = format!("{}/skills/status", self.gateway_url);
                match self.client.get(&url).send().await {
                    Ok(r) => {
                        let body = r.text().await.unwrap_or_default();
                        Ok(body)
                    }
                    Err(e) => Ok(format!("(gateway unreachable: {})", e)),
                }
            }

            // ── security.listEvents ──────────────────────────────────────
            "security.listEvents" => {
                let limit = args["limit"].as_u64().unwrap_or(50);
                let url = format!("{}/events?limit={}", self.gateway_url, limit);
                match self.client.get(&url).send().await {
                    Ok(r) => {
                        let body = r.text().await.unwrap_or_default();
                        Ok(body)
                    }
                    Err(e) => Ok(format!("(gateway unreachable: {})", e)),
                }
            }

            // ── knowledge.query / knowledge.retrieve ─────────────────────
            // These require the inference/RAG backend.
            // Register a SkillHandler plugin (via a vector DB client)
            // to replace this stub with real semantic search.
            "knowledge.query" | "knowledge.retrieve" => {
                // First, try registered handlers
                if let Some(out) = self.try_handlers(skill_name, args).await {
                    return out.map_err(|e| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: e,
                    });
                }
                let query = args["question"].as_str()
                    .or_else(|| args["query"].as_str())
                    .unwrap_or("");
                // Fallback: search in agent working memory for matching context
                let memory_hit: Vec<String> = ctx.memory
                    .all()
                    .iter()
                    .filter(|(k, v)| {
                        let q = query.to_lowercase();
                        k.to_lowercase().contains(&q) || v.to_lowercase().contains(&q)
                    })
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                if !memory_hit.is_empty() {
                    Ok(format!("Found in agent memory:\n{}", memory_hit.join("\n")))
                } else {
                    Ok(format!(
                        "(knowledge.{} not configured — register a SkillHandler with RAG backend. \
                         Query was: '{}'. \
                         Tip: use web.fetch or search.web to retrieve information.)",
                        if skill_name.ends_with("query") { "query" } else { "retrieve" },
                        query
                    ))
                }
            }

            // ── email.* ───────────────────────────────────────────────────
            // Requires IMAP/SMTP configuration. Register an EmailSkillHandler.
            "email.list" | "email.read" | "email.send" | "email.reply" | "email.delete" => {
                if let Some(out) = self.try_handlers(skill_name, args).await {
                    return out.map_err(|e| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: e,
                    });
                }
                Ok(format!(
                    "(email.{} requires an EmailSkillHandler — \
                     configure IMAP/SMTP credentials and register the handler \
                     via SkillDispatcher::register_handler())",
                    skill_name.trim_start_matches("email.")
                ))
            }

            // ── calendar.* ───────────────────────────────────────────────
            // Requires CalDAV/Google Calendar config. Register a CalendarSkillHandler.
            "calendar.list" | "calendar.get" | "calendar.create"
            | "calendar.update" | "calendar.delete" => {
                if let Some(out) = self.try_handlers(skill_name, args).await {
                    return out.map_err(|e| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: e,
                    });
                }
                Ok(format!(
                    "(calendar.{} requires a CalendarSkillHandler — \
                     configure CalDAV/Google Calendar and register the handler \
                     via SkillDispatcher::register_handler())",
                    skill_name.trim_start_matches("calendar.")
                ))
            }

            // ── search.web (stub → web.fetch on DuckDuckGo) ──────────────
            "search.web" | "search.query" => {
                let query = args["query"].as_str().unwrap_or("");
                let encoded = urlencoding_simple(query);
                let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);
                let resp = self.client.get(&url)
                    .header("User-Agent", "Mozilla/5.0")
                    .send().await
                    .map_err(|e| ExecutorError::HttpError(e.to_string()))?;
                let body = resp.text().await.unwrap_or_default();
                // Extract visible text (very rough)
                let text: String = body
                    .lines()
                    .filter(|l| !l.trim().starts_with('<') && l.trim().len() > 20)
                    .take(20)
                    .map(|l| l.trim().to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(if text.is_empty() { "(no results)".into() } else { text })
            }

            // ── fs.readFile ───────────────────────────────────────────────
            "fs.readFile" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'path' argument".into(),
                    })?;
                let content = std::fs::read_to_string(path)
                    .map_err(ExecutorError::IoError)?;
                let truncated: String = content.chars().take(3000).collect();
                Ok(truncated)
            }

            "fs.readDir" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'path' argument".into(),
                    })?;
                let entries: Vec<String> = std::fs::read_dir(path)
                    .map_err(ExecutorError::IoError)?
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                Ok(entries.join("\n"))
            }

            "fs.exists" => {
                let path = args["path"].as_str().unwrap_or("");
                Ok(std::path::Path::new(path).exists().to_string())
            }

            "fs.stat" => {
                let path = args["path"].as_str().unwrap_or("");
                let meta = std::fs::metadata(path)
                    .map_err(ExecutorError::IoError)?;
                Ok(format!(
                    "size: {} bytes, is_dir: {}, is_file: {}",
                    meta.len(), meta.is_dir(), meta.is_file()
                ))
            }

            "fs.writeFile" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'path'".into(),
                    })?;
                let content = args["content"].as_str().unwrap_or("");
                std::fs::write(path, content).map_err(ExecutorError::IoError)?;
                Ok(format!("Written {} bytes to {}", content.len(), path))
            }

            "fs.mkdir" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'path'".into(),
                    })?;
                std::fs::create_dir_all(path).map_err(ExecutorError::IoError)?;
                Ok(format!("Directory created: {}", path))
            }

            // ── Default: try registered plugin handlers, then stub ───────
            other => {
                if let Some(out) = self.try_handlers(other, args).await {
                    return out.map_err(|e| ExecutorError::DispatchFailed {
                        skill: other.into(),
                        reason: e,
                    });
                }
                Ok(format!(
                    "(skill '{}' has no built-in implementation — \
                     register a SkillHandler to provide it; args: {})",
                    other,
                    serde_json::to_string(args).unwrap_or_default()
                ))
            }
        }
    }

    /// Try each registered handler in order, returning the first that claims the skill.
    async fn try_handlers(
        &self,
        skill_name: &str,
        args: &serde_json::Value,
    ) -> Option<Result<String, String>> {
        for handler in &self.handlers {
            if handler.skill_names().contains(&skill_name) {
                return Some(handler.execute(skill_name, args).await);
            }
        }
        None
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn iso_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Minimal ISO 8601 without chrono dependency
    format!("{}Z", secs)
}

fn urlencoding_simple(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() || "-_.~".contains(c) {
                vec![c]
            } else if c == ' ' {
                vec!['+']
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{TaskContext, TaskGoal};

    fn make_ctx() -> TaskContext {
        TaskContext::new("run-1", "agent-1", TaskGoal::new("test"), "system")
    }

    #[test]
    fn urlencoding_simple_spaces() {
        assert_eq!(urlencoding_simple("hello world"), "hello+world");
    }

    #[test]
    fn urlencoding_preserves_alphanumeric() {
        assert_eq!(urlencoding_simple("test123"), "test123");
    }

    #[test]
    fn skill_result_denied_contains_reason() {
        let r = SkillResult::denied_result("shell.exec", "always blocked");
        assert!(!r.allowed);
        assert!(r.denied);
        assert!(r.output.contains("always blocked"));
    }

    // ── Memory skill tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn agent_set_and_get_memory_roundtrip() {
        let d = SkillDispatcher::new("http://127.0.0.1:9"); // unreachable port, skill is local
        let mut ctx = make_ctx();

        let result = d
            .execute_skill("agent.setMemory", &serde_json::json!({"key": "foo", "value": "bar"}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("foo"));
        assert_eq!(ctx.memory.get("foo"), Some("bar"));

        let result2 = d
            .execute_skill("agent.getMemory", &serde_json::json!({"key": "foo"}), &mut ctx)
            .await
            .unwrap();
        assert_eq!(result2, "bar");
    }

    #[tokio::test]
    async fn agent_get_memory_missing_key_returns_not_found() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        let result = d
            .execute_skill("agent.getMemory", &serde_json::json!({"key": "nonexistent"}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("not found"));
    }

    #[tokio::test]
    async fn agent_clear_memory_removes_all_keys() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        ctx.memory.set("k1", "v1");
        ctx.memory.set("k2", "v2");
        d.execute_skill("agent.clearMemory", &serde_json::json!({}), &mut ctx)
            .await
            .unwrap();
        assert!(ctx.memory.all().is_empty());
    }

    #[tokio::test]
    async fn agent_get_context_returns_summary() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        let result = d
            .execute_skill("agent.getContext", &serde_json::json!({}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("run-1"));
        assert!(result.contains("test")); // goal
    }

    #[tokio::test]
    async fn knowledge_query_hits_memory_fallback() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        ctx.memory.set("rust_tips", "Use cargo clippy regularly");
        let result = d
            .execute_skill("knowledge.query", &serde_json::json!({"question": "rust"}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("Found in agent memory"));
        assert!(result.contains("cargo clippy"));
    }

    #[tokio::test]
    async fn knowledge_query_no_match_returns_stub() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        let result = d
            .execute_skill("knowledge.query", &serde_json::json!({"question": "xyzzy"}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("not configured"));
    }

    // ── SkillHandler plugin tests ──────────────────────────────────────

    struct TestEmailHandler;

    #[async_trait::async_trait]
    impl SkillHandler for TestEmailHandler {
        fn skill_names(&self) -> &[&'static str] {
            &["email.list", "email.send"]
        }
        async fn execute(&self, skill_name: &str, _args: &serde_json::Value) -> Result<String, String> {
            Ok(format!("[TestEmail] handled: {}", skill_name))
        }
    }

    #[tokio::test]
    async fn registered_handler_intercepts_email_skill() {
        let mut d = SkillDispatcher::new("http://127.0.0.1:9");
        d.register_handler(Arc::new(TestEmailHandler));
        let mut ctx = make_ctx();
        let result = d
            .execute_skill("email.list", &serde_json::json!({}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("[TestEmail] handled: email.list"));
    }

    #[tokio::test]
    async fn unregistered_email_skill_returns_config_hint() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        let result = d
            .execute_skill("email.send", &serde_json::json!({}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("EmailSkillHandler"));
    }

    #[tokio::test]
    async fn unregistered_calendar_skill_returns_config_hint() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        let result = d
            .execute_skill("calendar.create", &serde_json::json!({}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("CalendarSkillHandler"));
    }

    #[tokio::test]
    async fn unknown_skill_returns_handler_hint() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        let result = d
            .execute_skill("custom.mySkill", &serde_json::json!({"x": 1}), &mut ctx)
            .await
            .unwrap();
        assert!(result.contains("no built-in implementation"));
    }

    #[tokio::test]
    async fn agent_set_memory_missing_key_returns_error() {
        let d = SkillDispatcher::new("http://127.0.0.1:9");
        let mut ctx = make_ctx();
        let err = d
            .execute_skill("agent.setMemory", &serde_json::json!({"value": "v"}), &mut ctx)
            .await;
        assert!(err.is_err());
    }
}
