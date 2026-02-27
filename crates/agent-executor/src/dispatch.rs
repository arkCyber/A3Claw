//! Skill dispatch: sends skill calls to the OpenClaw+ Gateway via HTTP,
//! interprets verdicts, and returns structured observations.

use crate::error::ExecutorError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

// ── Skill dispatcher ─────────────────────────────────────────────────────────

/// Dispatches skill calls through the Gateway and executes them.
///
/// Flow:
/// 1. POST `/hooks/before-skill` → get verdict (allow/deny/confirm)
/// 2. If allowed: execute the skill locally or via internal impl
/// 3. POST `/hooks/after-skill` → audit log
#[derive(Clone)]
pub struct SkillDispatcher {
    gateway_url: String,
    client: reqwest::Client,
}

impl SkillDispatcher {
    pub fn new(gateway_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("HTTP client build failed");
        Self { gateway_url: gateway_url.into(), client }
    }

    /// Execute a skill call:
    /// 1. Check with gateway
    /// 2. If allowed, run the skill
    /// 3. Notify gateway of outcome
    pub async fn dispatch(
        &self,
        session_id: &str,
        skill_name: &str,
        args: serde_json::Value,
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

        // Step 2: execute skill
        let exec_result = self.execute_skill(skill_name, &args).await;

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

    /// Execute a skill locally.  For skills that have built-in implementations
    /// (web.fetch, fs.readFile, etc.) we run them here.  For skills that require
    /// the full OpenClaw JS runtime, this falls through to a stub until the
    /// WasmEdge integration is complete.
    async fn execute_skill(
        &self,
        skill_name: &str,
        args: &serde_json::Value,
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
                Ok("(context available in system prompt)".to_string())
            }

            // ── agent.getMemory ──────────────────────────────────────────
            "agent.getMemory" => {
                let key = args["key"].as_str().unwrap_or("(no key)");
                Ok(format!("(memory key '{}' — use TaskContext.memory.get())", key))
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
                    .map_err(|e| ExecutorError::IoError(e))?;
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
                    .map_err(|e| ExecutorError::IoError(e))?
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
                    .map_err(|e| ExecutorError::IoError(e))?;
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
                std::fs::write(path, content).map_err(|e| ExecutorError::IoError(e))?;
                Ok(format!("Written {} bytes to {}", content.len(), path))
            }

            "fs.mkdir" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| ExecutorError::DispatchFailed {
                        skill: skill_name.into(),
                        reason: "missing 'path'".into(),
                    })?;
                std::fs::create_dir_all(path).map_err(|e| ExecutorError::IoError(e))?;
                Ok(format!("Directory created: {}", path))
            }

            // ── Default: stub for skills not yet fully implemented ────────
            other => {
                Ok(format!(
                    "(skill '{}' accepted by gateway — execution stub; args: {})",
                    other,
                    serde_json::to_string(args).unwrap_or_default()
                ))
            }
        }
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
}
