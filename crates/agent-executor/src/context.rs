//! Per-agent task context: goal, working memory, step log, conversation history.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Task goal ─────────────────────────────────────────────────────────────────

/// The high-level goal driving a task run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGoal {
    /// Natural-language description of what the agent should accomplish.
    pub objective: String,
    /// Optional structured constraints (e.g. output format, deadline).
    pub constraints: Vec<String>,
    /// Maximum number of ReAct steps before the executor gives up.
    pub max_steps: u32,
    /// Timeout for the entire task in seconds.
    pub timeout_secs: u64,
}

impl TaskGoal {
    pub fn new(objective: impl Into<String>) -> Self {
        Self {
            objective: objective.into(),
            constraints: Vec::new(),
            max_steps: 20,
            timeout_secs: 300,
        }
    }

    pub fn with_constraint(mut self, c: impl Into<String>) -> Self {
        self.constraints.push(c.into());
        self
    }

    pub fn with_max_steps(mut self, n: u32) -> Self {
        self.max_steps = n;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

// ── Step record ───────────────────────────────────────────────────────────────

/// One completed step in the ReAct loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    pub step_index: u32,
    /// The agent's reasoning text ("Thought: …")
    pub thought: String,
    /// The skill the agent chose to call.
    pub skill_name: String,
    /// Arguments passed to the skill.
    pub skill_args: serde_json::Value,
    /// Raw result returned by the skill.
    pub observation: String,
    /// Whether the skill call was allowed or denied by the Gateway.
    pub allowed: bool,
    /// Time taken for this step (ms).
    pub elapsed_ms: u64,
    /// Wall-clock timestamp (Unix seconds).
    pub timestamp: u64,
}

impl StepRecord {
    pub fn new(
        step_index: u32,
        thought: impl Into<String>,
        skill_name: impl Into<String>,
        skill_args: serde_json::Value,
    ) -> Self {
        Self {
            step_index,
            thought: thought.into(),
            skill_name: skill_name.into(),
            skill_args,
            observation: String::new(),
            allowed: false,
            elapsed_ms: 0,
            timestamp: now_unix_secs(),
        }
    }
}

// ── Memory store ──────────────────────────────────────────────────────────────

/// Key-value persistent memory for the agent (survives across task runs).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStore {
    entries: HashMap<String, String>,
}

impl MemoryStore {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.entries.remove(key)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn all(&self) -> &HashMap<String, String> {
        &self.entries
    }

    /// Serialize to compact JSON for injection into LLM context.
    pub fn to_context_string(&self) -> String {
        if self.entries.is_empty() {
            return "(memory empty)".to_string();
        }
        self.entries
            .iter()
            .map(|(k, v)| format!("  {}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ── Conversation message ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: String,
    /// Present when role == Tool; contains the tool/skill call result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ConversationMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: MessageRole::System, content: content.into(), tool_call_id: None }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: MessageRole::User, content: content.into(), tool_call_id: None }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: MessageRole::Assistant, content: content.into(), tool_call_id: None }
    }
    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_call_id: Some(call_id.into()),
        }
    }

    /// Convert to OpenAI-format JSON message.
    pub fn to_openai_json(&self) -> serde_json::Value {
        let role = match &self.role {
            MessageRole::System    => "system",
            MessageRole::User      => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool      => "tool",
        };
        if let Some(ref id) = self.tool_call_id {
            serde_json::json!({ "role": role, "content": self.content, "tool_call_id": id })
        } else {
            serde_json::json!({ "role": role, "content": self.content })
        }
    }
}

// ── TaskContext ───────────────────────────────────────────────────────────────

/// Complete execution context for one task run.
///
/// This is the single piece of state that flows through the ReAct loop.
/// It is serialisable so it can be checkpointed to disk for crash recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// Unique run ID.
    pub run_id: String,
    /// Agent ID this context belongs to.
    pub agent_id: String,
    /// High-level goal.
    pub goal: TaskGoal,
    /// Conversation history (system prompt + user messages + assistant turns).
    pub messages: Vec<ConversationMessage>,
    /// Completed step records.
    pub steps: Vec<StepRecord>,
    /// Working memory (short-term, cleared each run).
    pub working_memory: HashMap<String, String>,
    /// Persistent memory (survives across runs).
    pub memory: MemoryStore,
    /// Current step index.
    pub step_index: u32,
    /// Whether the task has reached a final answer.
    pub finished: bool,
    /// Final answer / output produced by the agent.
    pub final_answer: Option<String>,
    /// Task start time (Unix seconds).
    pub started_at: u64,
}

impl TaskContext {
    pub fn new(
        run_id: impl Into<String>,
        agent_id: impl Into<String>,
        goal: TaskGoal,
        system_prompt: impl Into<String>,
    ) -> Self {
        let mut ctx = Self {
            run_id: run_id.into(),
            agent_id: agent_id.into(),
            goal,
            messages: Vec::new(),
            steps: Vec::new(),
            working_memory: HashMap::new(),
            memory: MemoryStore::default(),
            step_index: 0,
            finished: false,
            final_answer: None,
            started_at: now_unix_secs(),
        };
        ctx.messages.push(ConversationMessage::system(system_prompt));
        ctx
    }

    /// Add user turn (the task objective).
    pub fn push_user(&mut self, content: impl Into<String>) {
        self.messages.push(ConversationMessage::user(content));
    }

    /// Add assistant response.
    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.messages.push(ConversationMessage::assistant(content));
    }

    /// Add tool result.
    pub fn push_tool_result(&mut self, call_id: impl Into<String>, result: impl Into<String>) {
        self.messages.push(ConversationMessage::tool_result(call_id, result));
    }

    /// Record a completed step.
    pub fn push_step(&mut self, step: StepRecord) {
        self.step_index = step.step_index + 1;
        self.steps.push(step);
    }

    /// Mark the task as finished with a final answer.
    pub fn finish(&mut self, answer: impl Into<String>) {
        self.final_answer = Some(answer.into());
        self.finished = true;
    }

    /// Elapsed seconds since task start.
    pub fn elapsed_secs(&self) -> u64 {
        now_unix_secs().saturating_sub(self.started_at)
    }

    /// Render messages as OpenAI-format JSON array.
    pub fn messages_json(&self) -> Vec<serde_json::Value> {
        self.messages.iter().map(|m| m.to_openai_json()).collect()
    }

    /// Build a concise context summary for injection into the system prompt.
    pub fn context_summary(&self) -> String {
        let step_summary = if self.steps.is_empty() {
            "No steps taken yet.".to_string()
        } else {
            self.steps
                .iter()
                .map(|s| {
                    format!(
                        "Step {}: {} → {} ({})",
                        s.step_index + 1,
                        s.skill_name,
                        if s.allowed { "allowed" } else { "denied" },
                        s.observation.chars().take(80).collect::<String>()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "Run ID: {}\nAgent: {}\nGoal: {}\nStep {}/{}\n\nSteps so far:\n{}\n\nMemory:\n{}",
            self.run_id,
            self.agent_id,
            self.goal.objective,
            self.step_index,
            self.goal.max_steps,
            step_summary,
            self.memory.to_context_string(),
        )
    }

    /// Checkpoint to disk (agent workspace directory).
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".openclaw-plus")
            .join("agents")
            .join(&self.agent_id)
            .join("runs");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", self.run_id));
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Load a checkpointed context from disk.
    pub fn load(agent_id: &str, run_id: &str) -> anyhow::Result<Self> {
        let path = dirs::home_dir()
            .unwrap_or_default()
            .join(".openclaw-plus")
            .join("agents")
            .join(agent_id)
            .join("runs")
            .join(format!("{}.json", run_id));
        let json = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&json)?)
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

    fn make_ctx() -> TaskContext {
        TaskContext::new(
            "run-001",
            "agent-001",
            TaskGoal::new("Test goal").with_max_steps(5),
            "You are a test agent.",
        )
    }

    #[test]
    fn context_creation() {
        let ctx = make_ctx();
        assert_eq!(ctx.run_id, "run-001");
        assert_eq!(ctx.messages.len(), 1); // system message
        assert_eq!(ctx.step_index, 0);
        assert!(!ctx.finished);
    }

    #[test]
    fn push_step_increments_index() {
        let mut ctx = make_ctx();
        let step = StepRecord::new(0, "Thinking…", "web.fetch", serde_json::json!({"url": "https://example.com"}));
        ctx.push_step(step);
        assert_eq!(ctx.step_index, 1);
        assert_eq!(ctx.steps.len(), 1);
    }

    #[test]
    fn finish_sets_answer() {
        let mut ctx = make_ctx();
        ctx.finish("Done!");
        assert!(ctx.finished);
        assert_eq!(ctx.final_answer.as_deref(), Some("Done!"));
    }

    #[test]
    fn memory_store_roundtrip() {
        let mut m = MemoryStore::default();
        m.set("key1", "value1");
        assert_eq!(m.get("key1"), Some("value1"));
        m.remove("key1");
        assert_eq!(m.get("key1"), None);
    }

    #[test]
    fn messages_json_format() {
        let mut ctx = make_ctx();
        ctx.push_user("Hello");
        ctx.push_assistant("Hi there");
        let json = ctx.messages_json();
        assert_eq!(json.len(), 3); // system + user + assistant
        assert_eq!(json[0]["role"], "system");
        assert_eq!(json[1]["role"], "user");
        assert_eq!(json[2]["role"], "assistant");
    }
}
