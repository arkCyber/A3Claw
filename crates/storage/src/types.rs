//! 存储层专用类型定义。
//!
//! 这些类型对应 SQLite 数据库中的行记录，
//! 与 `openclaw-security` 中的领域模型分离，
//! 避免循环依赖并保持存储层的独立性。

use serde::{Deserialize, Serialize};

// ── RunStatus ─────────────────────────────────────────────────────────────────

/// 任务执行（Run）的状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// 正在执行中。
    Running,
    /// 执行成功完成。
    Success,
    /// 执行失败。
    Failed,
    /// 被用户或系统取消。
    Cancelled,
    /// 等待人工审批。
    PendingApproval,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Running         => write!(f, "Running"),
            RunStatus::Success         => write!(f, "Success"),
            RunStatus::Failed          => write!(f, "Failed"),
            RunStatus::Cancelled       => write!(f, "Cancelled"),
            RunStatus::PendingApproval => write!(f, "Pending Approval"),
        }
    }
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running         => "running",
            RunStatus::Success         => "success",
            RunStatus::Failed          => "failed",
            RunStatus::Cancelled       => "cancelled",
            RunStatus::PendingApproval => "pending_approval",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "running"          => RunStatus::Running,
            "success"          => RunStatus::Success,
            "failed"           => RunStatus::Failed,
            "cancelled"        => RunStatus::Cancelled,
            "pending_approval" => RunStatus::PendingApproval,
            _                  => RunStatus::Failed,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, RunStatus::Success | RunStatus::Failed | RunStatus::Cancelled)
    }
}

// ── RunRecord ─────────────────────────────────────────────────────────────────

/// 一次任务执行记录（对应 `runs` 表的一行）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    /// 执行 ID（UUID v4）。
    pub id: String,
    /// 所属数字员工 ID。
    pub agent_id: String,
    /// 任务描述（用户输入的自然语言指令）。
    pub task_description: String,
    /// 开始时间（Unix 时间戳，秒）。
    pub started_at: u64,
    /// 结束时间（Unix 时间戳，秒）。`None` 表示仍在运行。
    pub finished_at: Option<u64>,
    /// 执行状态。
    pub status: RunStatus,
    /// 执行结果摘要（成功时的输出摘要，或失败原因）。
    pub summary: Option<String>,
    /// 执行步骤数。
    pub step_count: u32,
    /// 被拒绝的操作数。
    pub denied_count: u32,
    /// 被批准的操作数。
    pub approved_count: u32,
}

impl RunRecord {
    pub fn new(agent_id: impl Into<String>, task_description: impl Into<String>) -> Self {
        Self {
            id: new_uuid(),
            agent_id: agent_id.into(),
            task_description: task_description.into(),
            started_at: now_unix_secs(),
            finished_at: None,
            status: RunStatus::Running,
            summary: None,
            step_count: 0,
            denied_count: 0,
            approved_count: 0,
        }
    }

    /// 运行时长（秒）。
    pub fn duration_secs(&self) -> Option<u64> {
        self.finished_at.map(|end| end.saturating_sub(self.started_at))
    }
}

// ── StepKind ──────────────────────────────────────────────────────────────────

/// 执行步骤的类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// LLM 推理调用。
    Inference,
    /// 工具/插件调用。
    ToolCall,
    /// 文件读取。
    FileRead,
    /// 文件写入。
    FileWrite,
    /// 文件删除。
    FileDelete,
    /// 网络请求。
    NetworkRequest,
    /// Shell 命令执行。
    ShellExec,
    /// 人工审批（等待/批准/拒绝）。
    HumanApproval,
    /// 系统事件（沙盒启动/停止等）。
    SystemEvent,
}

impl StepKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            StepKind::Inference      => "inference",
            StepKind::ToolCall       => "tool_call",
            StepKind::FileRead       => "file_read",
            StepKind::FileWrite      => "file_write",
            StepKind::FileDelete     => "file_delete",
            StepKind::NetworkRequest => "network_request",
            StepKind::ShellExec      => "shell_exec",
            StepKind::HumanApproval  => "human_approval",
            StepKind::SystemEvent    => "system_event",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "inference"       => StepKind::Inference,
            "tool_call"       => StepKind::ToolCall,
            "file_read"       => StepKind::FileRead,
            "file_write"      => StepKind::FileWrite,
            "file_delete"     => StepKind::FileDelete,
            "network_request" => StepKind::NetworkRequest,
            "shell_exec"      => StepKind::ShellExec,
            "human_approval"  => StepKind::HumanApproval,
            _                 => StepKind::SystemEvent,
        }
    }

    /// 该步骤类型的风险等级（0=无，1=低，2=中，3=高，4=关键）。
    pub fn risk_level(&self) -> u8 {
        match self {
            StepKind::Inference      => 0,
            StepKind::ToolCall       => 1,
            StepKind::FileRead       => 1,
            StepKind::NetworkRequest => 2,
            StepKind::FileWrite      => 2,
            StepKind::ShellExec      => 3,
            StepKind::FileDelete     => 3,
            StepKind::HumanApproval  => 0,
            StepKind::SystemEvent    => 0,
        }
    }
}

// ── StepRecord ────────────────────────────────────────────────────────────────

/// 执行步骤记录（对应 `steps` 表的一行）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    /// 步骤 ID（自增）。
    pub id: i64,
    /// 所属 Run ID。
    pub run_id: String,
    /// 步骤序号（从 0 开始）。
    pub step_index: u32,
    /// 步骤类型。
    pub kind: StepKind,
    /// 步骤描述（如命令内容、URL、文件路径等）。
    pub description: String,
    /// 输入摘要（脱敏后的 prompt 摘要或参数 hash）。
    pub input_summary: Option<String>,
    /// 输出摘要（模型输出摘要或工具返回摘要）。
    pub output_summary: Option<String>,
    /// 步骤开始时间（Unix 时间戳，秒）。
    pub started_at: u64,
    /// 步骤结束时间（Unix 时间戳，秒）。
    pub finished_at: Option<u64>,
    /// 是否成功。
    pub success: bool,
    /// 错误信息（失败时）。
    pub error: Option<String>,
}

impl StepRecord {
    pub fn new(
        run_id: impl Into<String>,
        step_index: u32,
        kind: StepKind,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: 0,
            run_id: run_id.into(),
            step_index,
            kind,
            description: description.into(),
            input_summary: None,
            output_summary: None,
            started_at: now_unix_secs(),
            finished_at: None,
            success: false,
            error: None,
        }
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.finished_at.map(|end| (end.saturating_sub(self.started_at)) * 1000)
    }
}

// ── AuditDecision ─────────────────────────────────────────────────────────────

/// 审计事件的决策结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecision {
    /// 自动放行（在策略白名单内）。
    AutoAllowed,
    /// 人工批准。
    HumanApproved,
    /// 自动拒绝（策略引擎拦截）。
    AutoDenied,
    /// 人工拒绝。
    HumanDenied,
    /// 等待人工决策。
    Pending,
}

impl AuditDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditDecision::AutoAllowed    => "auto_allowed",
            AuditDecision::HumanApproved  => "human_approved",
            AuditDecision::AutoDenied     => "auto_denied",
            AuditDecision::HumanDenied    => "human_denied",
            AuditDecision::Pending        => "pending",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "auto_allowed"   => AuditDecision::AutoAllowed,
            "human_approved" => AuditDecision::HumanApproved,
            "auto_denied"    => AuditDecision::AutoDenied,
            "human_denied"   => AuditDecision::HumanDenied,
            _                => AuditDecision::Pending,
        }
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, AuditDecision::AutoAllowed | AuditDecision::HumanApproved)
    }
}

// ── AuditEventRecord ──────────────────────────────────────────────────────────

/// 不可变审计事件记录（对应 `audit_events` 表的一行）。
///
/// 审计事件一旦写入，**不可修改、不可删除**（仅追加）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventRecord {
    /// 自增 ID。
    pub id: i64,
    /// 所属 Run ID（可为空，表示非任务执行期间的事件）。
    pub run_id: Option<String>,
    /// 所属数字员工 ID。
    pub agent_id: String,
    /// 步骤序号（对应 `steps.step_index`）。
    pub step_index: Option<u32>,
    /// 事件类型（对应 `StepKind::as_str()`）。
    pub event_kind: String,
    /// 操作目标（文件路径、URL、命令等，已脱敏）。
    pub target: Option<String>,
    /// 输入内容的 SHA-256 hash（用于完整性验证，不存储原文）。
    pub input_hash: Option<String>,
    /// 决策结果。
    pub decision: AuditDecision,
    /// 决策者（"policy_engine" / 用户名）。
    pub actor: String,
    /// 决策原因（可选）。
    pub reason: Option<String>,
    /// 事件时间戳（Unix 时间戳，秒）。
    pub ts: u64,
}

impl AuditEventRecord {
    pub fn new(
        agent_id: impl Into<String>,
        event_kind: impl Into<String>,
        decision: AuditDecision,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            id: 0,
            run_id: None,
            agent_id: agent_id.into(),
            step_index: None,
            event_kind: event_kind.into(),
            target: None,
            input_hash: None,
            decision,
            actor: actor.into(),
            reason: None,
            ts: now_unix_secs(),
        }
    }
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

pub(crate) fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn new_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id() as u128;
    let a = ts ^ (pid << 32);
    let b = ts.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (a >> 96) as u32,
        (a >> 80) as u16,
        (a >> 68) as u16 & 0x0fff,
        ((b >> 48) as u16 & 0x3fff) | 0x8000,
        b as u64 & 0x0000_ffff_ffff_ffff,
    )
}
