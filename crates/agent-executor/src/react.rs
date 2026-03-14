//! ReAct reasoning loop: Thought → Action (Skill call) → Observation → repeat.
//!
//! Implements the ReAct pattern (Yao et al., 2022) adapted for OpenClaw+:
//! - The LLM produces a JSON response with either a `tool_call` or `final_answer`.
//! - Each tool call maps to a SkillRegistry skill name.
//! - The skill result is fed back as a `tool` message and the loop continues.

use crate::context::{StepRecord, TaskContext};
use crate::dispatch::{SkillDispatcher, SkillResult};
use crate::error::ExecutorError;
use crate::skill::SkillSet;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ── LLM response shapes ───────────────────────────────────────────────────────

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCall {
    pub id: String,
    pub function: LlmFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFunctionCall {
    pub name: String,
    #[serde(default)]
    pub arguments: String,
}

impl LlmFunctionCall {
    pub fn parsed_args(&self) -> serde_json::Value {
        serde_json::from_str(&self.arguments).unwrap_or(serde_json::json!({}))
    }
}

/// The LLM's choice in a chat completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmChoice {
    pub message: LlmMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmMessage {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<LlmToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmChatResponse {
    pub choices: Vec<LlmChoice>,
}

// ── ReAct loop config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReactConfig {
    /// LLM endpoint (OpenAI-compatible, e.g. Ollama /api/chat or OpenAI /v1/chat/completions).
    pub llm_endpoint: String,
    /// Model name.
    pub model: String,
    /// API key (empty for local Ollama).
    pub api_key: String,
    /// Temperature.
    pub temperature: f32,
    /// Max tokens per LLM call.
    pub max_tokens: u32,
    /// Whether the endpoint is Ollama-style (uses /api/chat with stream:false).
    pub is_ollama: bool,
    /// Gateway URL for skill dispatch.
    pub gateway_url: String,
}

impl Default for ReactConfig {
    fn default() -> Self {
        Self {
            llm_endpoint: "http://localhost:11434".to_string(),
            model: "qwen2.5:0.5b".to_string(),
            api_key: String::new(),
            temperature: 0.2,
            max_tokens: 2048,
            is_ollama: true,
            gateway_url: "http://localhost:7878".to_string(),
        }
    }
}

// ── System prompt builder ─────────────────────────────────────────────────────

pub fn build_system_prompt(skill_set: &SkillSet, agent_name: &str, agent_role: &str) -> String {
    let skill_list = skill_set
        .all_granted()
        .iter()
        .map(|s| format!("  - {}: {}", s.name, s.description))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are {name}, a Digital Worker with role: {role}.

You have access to the following skills (tools):
{skills}

Rules:
1. Always think step by step before calling a skill.
2. Call skills one at a time.
3. When you have enough information, respond with a final answer using `finish` tool.
4. Keep responses concise and focused.
5. If a skill is denied, adapt your plan and try an alternative approach.
6. Never make up information — use skills to get real data.

Use the provided tools to accomplish the user's goal."#,
        name = agent_name,
        role = agent_role,
        skills = if skill_list.is_empty() { "  (no skills granted)".to_string() } else { skill_list },
    )
}

// ── LLM client ───────────────────────────────────────────────────────────────

pub struct LlmClient {
    client: reqwest::Client,
    cfg: ReactConfig,
}

impl LlmClient {
    pub fn new(cfg: ReactConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("HTTP client");
        Self { client, cfg }
    }

    /// Call the LLM with the current message history and tool definitions.
    pub async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        tools: Vec<serde_json::Value>,
    ) -> Result<LlmChatResponse, ExecutorError> {
        if self.cfg.is_ollama {
            self.call_ollama(messages, tools).await
        } else {
            self.call_openai_compat(messages, tools).await
        }
    }

    async fn call_ollama(
        &self,
        messages: Vec<serde_json::Value>,
        tools: Vec<serde_json::Value>,
    ) -> Result<LlmChatResponse, ExecutorError> {
        let url = format!("{}/api/chat", self.cfg.llm_endpoint);
        let mut payload = serde_json::json!({
            "model": self.cfg.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": self.cfg.temperature,
                "num_predict": self.cfg.max_tokens,
            }
        });

        // Ollama supports tools in recent versions
        if !tools.is_empty() {
            payload["tools"] = serde_json::Value::Array(tools);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ExecutorError::LlmError(format!("Ollama unreachable: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutorError::LlmError(format!("Ollama HTTP {}: {}", status, body)));
        }

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutorError::LlmError(e.to_string()))?;

        // Ollama response: { "message": { "role": "assistant", "content": "...", "tool_calls": [...] } }
        let message = &raw["message"];
        let content = message["content"].as_str().map(|s| s.to_string());
        let tool_calls: Option<Vec<LlmToolCall>> = message["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        let name = tc["function"]["name"].as_str()?.to_string();
                        let arguments = tc["function"]["arguments"]
                            .as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| {
                                serde_json::to_string(&tc["function"]["arguments"])
                                    .unwrap_or_default()
                            });
                        Some(LlmToolCall {
                            id: uuid::Uuid::new_v4().to_string(),
                            function: LlmFunctionCall { name, arguments },
                        })
                    })
                    .collect()
            });

        Ok(LlmChatResponse {
            choices: vec![LlmChoice {
                message: LlmMessage { content, tool_calls },
                finish_reason: raw["done_reason"].as_str().map(|s| s.to_string()),
            }],
        })
    }

    async fn call_openai_compat(
        &self,
        messages: Vec<serde_json::Value>,
        tools: Vec<serde_json::Value>,
    ) -> Result<LlmChatResponse, ExecutorError> {
        let url = format!("{}/chat/completions", self.cfg.llm_endpoint);
        let mut payload = serde_json::json!({
            "model": self.cfg.model,
            "messages": messages,
            "temperature": self.cfg.temperature,
            "max_tokens": self.cfg.max_tokens,
        });

        if !tools.is_empty() {
            payload["tools"] = serde_json::Value::Array(tools);
            payload["tool_choice"] = serde_json::json!("auto");
        }

        let mut req = self.client.post(&url).json(&payload);
        if !self.cfg.api_key.is_empty() {
            req = req.bearer_auth(&self.cfg.api_key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ExecutorError::LlmError(format!("LLM endpoint unreachable: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutorError::LlmError(format!("LLM HTTP {}: {}", status, body)));
        }

        resp.json::<LlmChatResponse>()
            .await
            .map_err(|e| ExecutorError::LlmError(e.to_string()))
    }
}

// ── ReAct loop ────────────────────────────────────────────────────────────────

/// One iteration result.
pub enum LoopStep {
    /// Skill was called; observation recorded; continue loop.
    Continue,
    /// Agent produced a final answer; loop should stop.
    Done(String),
    /// Max steps reached.
    MaxSteps,
    /// LLM produced no tool call and no content (stall).
    Stalled,
}

/// Run the ReAct loop to completion.
///
/// Returns the final answer string, or an error.
pub async fn run_react_loop(
    ctx: &mut TaskContext,
    skill_set: &SkillSet,
    llm: &LlmClient,
    dispatcher: &SkillDispatcher,
    session_id: &str,
) -> Result<String, ExecutorError> {
    // Add `finish` pseudo-tool so the LLM can signal task completion.
    let finish_tool = serde_json::json!({
        "type": "function",
        "function": {
            "name": "finish",
            "description": "Call this when you have a complete final answer for the user.",
            "parameters": {
                "type": "object",
                "properties": {
                    "answer": {
                        "type": "string",
                        "description": "The final answer or result to return to the user."
                    }
                },
                "required": ["answer"]
            }
        }
    });

    let mut tools = skill_set.to_tool_schemas();
    tools.push(finish_tool);

    loop {
        if ctx.step_index >= ctx.goal.max_steps {
            warn!(run_id = %ctx.run_id, "ReAct loop hit max steps");
            return Err(ExecutorError::MaxStepsExceeded(ctx.goal.max_steps));
        }

        if ctx.elapsed_secs() > ctx.goal.timeout_secs {
            return Err(ExecutorError::Timeout(ctx.goal.timeout_secs));
        }

        debug!(run_id = %ctx.run_id, step = ctx.step_index, "ReAct step");

        // Call LLM
        let messages_json = ctx.messages_json();
        let llm_resp = llm.chat(messages_json, tools.clone()).await?;

        let choice = llm_resp.choices.into_iter().next()
            .ok_or_else(|| ExecutorError::LlmError("Empty choices in LLM response".into()))?;

        // Check for finish_reason == "stop" with no tool calls → treat as final answer
        let is_stop = choice.finish_reason.as_deref() == Some("stop");
        let has_tool_calls = choice.message.tool_calls.as_ref()
            .map(|tc| !tc.is_empty())
            .unwrap_or(false);
        let content = choice.message.content.clone().unwrap_or_default();

        if !has_tool_calls {
            if is_stop || !content.is_empty() {
                let answer = if content.is_empty() {
                    "(task completed with no output)".to_string()
                } else {
                    content.clone()
                };
                ctx.push_assistant(&answer);
                ctx.finish(&answer);
                info!(run_id = %ctx.run_id, "ReAct loop finished with direct answer");
                return Ok(answer);
            }
            // True stall: no content, no tool calls
            warn!(run_id = %ctx.run_id, "LLM stalled — no tool call and no content");
            let stall_msg = "(no action taken — agent stalled)".to_string();
            ctx.finish(&stall_msg);
            return Ok(stall_msg);
        }

        // Record assistant message with tool calls
        if !content.is_empty() {
            ctx.push_assistant(&content);
        }

        // Process each tool call
        let tool_calls = choice.message.tool_calls.unwrap_or_default();
        for tc in tool_calls {
            let skill_name = &tc.function.name;
            let args = tc.function.parsed_args();
            let call_id = tc.id.clone();

            info!(
                run_id = %ctx.run_id,
                step = ctx.step_index,
                skill = skill_name,
                "Dispatching skill"
            );

            // `finish` is handled locally, not via gateway
            if skill_name == "finish" {
                let answer = args["answer"].as_str()
                    .unwrap_or("(done)")
                    .to_string();
                ctx.push_tool_result(&call_id, &answer);
                ctx.finish(&answer);
                info!(run_id = %ctx.run_id, "ReAct loop: finish tool called");
                return Ok(answer);
            }

            let t0 = std::time::Instant::now();
            let result = dispatcher
                .dispatch(session_id, skill_name, args.clone(), ctx)
                .await
                .unwrap_or_else(|e| SkillResult::error_result(skill_name, &e.to_string()));

            let mut step = StepRecord::new(
                ctx.step_index,
                format!("Calling skill: {}", skill_name),
                skill_name.clone(),
                args,
            );
            step.observation = result.output.clone();
            step.allowed = result.allowed;
            step.elapsed_ms = t0.elapsed().as_millis() as u64;

            ctx.push_step(step);
            ctx.push_tool_result(&call_id, &result.output);

            // Checkpoint after each step
            let _ = ctx.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::SkillSet;

    #[test]
    fn system_prompt_contains_skills() {
        let set = SkillSet::default_safe();
        let prompt = build_system_prompt(&set, "TestAgent", "Data Analyst");
        assert!(prompt.contains("TestAgent"));
        assert!(prompt.contains("Data Analyst"));
        assert!(prompt.contains("web.fetch"));
    }

    #[test]
    fn finish_tool_schema() {
        let tool = serde_json::json!({
            "type": "function",
            "function": {
                "name": "finish",
                "description": "Call this when you have a complete final answer for the user.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "answer": { "type": "string", "description": "The final answer." }
                    },
                    "required": ["answer"]
                }
            }
        });
        assert_eq!(tool["function"]["name"], "finish");
    }

    #[test]
    fn llm_function_call_parsed_args() {
        let fc = LlmFunctionCall {
            name: "web.fetch".into(),
            arguments: r#"{"url": "https://example.com"}"#.into(),
        };
        let args = fc.parsed_args();
        assert_eq!(args["url"], "https://example.com");
    }

    #[test]
    fn llm_function_call_bad_json_returns_empty_object() {
        let fc = LlmFunctionCall {
            name: "web.fetch".into(),
            arguments: "not json".into(),
        };
        let args = fc.parsed_args();
        assert!(args.is_object());
        assert!(args.as_object().unwrap().is_empty());
    }
}
