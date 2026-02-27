//! # AgentProfile — 数字员工身份模型
//!
//! 每一个"数字员工"对应一个 `AgentProfile`，它是整个平台的基石：
//! - 身份标识（UUID + 显示名 + 描述 + 所有者）
//! - 安全边界（per-Agent 文件系统挂载、网络白名单、内存限制）
//! - 能力集合（允许的插件/技能列表）
//! - 通信渠道绑定（Telegram/Discord/Slack Token）
//! - 生命周期状态（Active / Suspended / Archived）
//! - 审计元数据（创建时间、最后活跃时间、运行统计）

use crate::config::{
    AgentConfig, ChannelConfig, FolderAccess, FsMount,
    OpenClawAiConfig, RagFolder,
};
use crate::circuit_breaker::BreakerConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── AgentId newtype ───────────────────────────────────────────────────────────

/// 数字员工的唯一标识符（UUID v4 字符串）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self { Self(uuid_v4()) }
    pub fn from_str(s: impl Into<String>) -> Self { Self(s.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl Default for AgentId {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn uuid_v4() -> String {
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

// ── AgentStatus ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Suspended,
    Archived,
    Running,
    Faulted { reason: String },
}

impl Default for AgentStatus {
    fn default() -> Self { AgentStatus::Active }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Active             => write!(f, "Active"),
            AgentStatus::Suspended          => write!(f, "Suspended"),
            AgentStatus::Archived           => write!(f, "Archived"),
            AgentStatus::Running            => write!(f, "Running"),
            AgentStatus::Faulted { reason } => write!(f, "Faulted: {}", reason),
        }
    }
}

impl AgentStatus {
    pub fn can_accept_task(&self) -> bool { matches!(self, AgentStatus::Active) }
    pub fn can_start(&self) -> bool {
        matches!(self, AgentStatus::Active | AgentStatus::Faulted { .. })
    }
    pub fn color_rgb(&self) -> (f32, f32, f32) {
        match self {
            AgentStatus::Active           => (0.2, 0.85, 0.4),
            AgentStatus::Running          => (0.3, 0.6,  1.0),
            AgentStatus::Suspended        => (0.9, 0.75, 0.2),
            AgentStatus::Archived         => (0.5, 0.5,  0.5),
            AgentStatus::Faulted { .. }   => (0.9, 0.2,  0.2),
        }
    }
}

// ── AgentRole ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    TicketAssistant,
    CodeReviewer,
    ReportGenerator,
    SecurityAuditor,
    DataAnalyst,
    CustomerSupport,
    /// 知识库与文档首席官 — RAG + 本地文档索引
    KnowledgeOfficer,
    /// 社媒运营经理 — 多平台内容分发
    SocialMediaManager,
    /// 邮件分拣员 — 邮件分类与草拟回复
    InboxTriageAgent,
    /// 财务采购员 — 付款审批与采购流程
    FinanceProcurement,
    /// 新闻信息秘书 — 定时推送热点与重要提醒
    NewsSecretary,
    /// 安全代码审计员 — SAST + Git 提交监控
    SecurityCodeAuditor,
    /// 全网情报员 — 竞品监控 + 行业新闻抓取 + AI 摘要
    IntelOfficer,
    Custom { label: String },
}

impl Default for AgentRole {
    fn default() -> Self { AgentRole::Custom { label: "General Assistant".to_string() } }
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::TicketAssistant      => write!(f, "Ticket Assistant"),
            AgentRole::CodeReviewer         => write!(f, "Code Reviewer"),
            AgentRole::ReportGenerator      => write!(f, "Report Generator"),
            AgentRole::SecurityAuditor      => write!(f, "Security Auditor"),
            AgentRole::DataAnalyst          => write!(f, "Data Analyst"),
            AgentRole::CustomerSupport      => write!(f, "Customer Support"),
            AgentRole::KnowledgeOfficer     => write!(f, "Knowledge Officer"),
            AgentRole::SocialMediaManager   => write!(f, "Social Media Manager"),
            AgentRole::InboxTriageAgent     => write!(f, "Inbox Triage Agent"),
            AgentRole::FinanceProcurement   => write!(f, "Finance & Procurement"),
            AgentRole::NewsSecretary        => write!(f, "News Secretary"),
            AgentRole::SecurityCodeAuditor  => write!(f, "Security Code Auditor"),
            AgentRole::IntelOfficer         => write!(f, "Intel Officer"),
            AgentRole::Custom { label }     => write!(f, "{}", label),
        }
    }
}

impl AgentRole {
    pub fn all_presets() -> &'static [&'static str] {
        &[
            "Ticket Assistant", "Code Reviewer", "Report Generator",
            "Security Auditor", "Data Analyst", "Customer Support",
            "Knowledge Officer", "Social Media Manager", "Inbox Triage Agent",
            "Finance & Procurement", "News Secretary", "Security Code Auditor",
            "Intel Officer",
        ]
    }
    pub fn default_network_allowlist(&self) -> Vec<String> {
        match self {
            AgentRole::TicketAssistant      => vec!["jira.example.com".into(), "api.feishu.cn".into()],
            AgentRole::CodeReviewer         => vec!["api.github.com".into(), "github.com".into()],
            AgentRole::ReportGenerator      => vec!["api.openai.com".into()],
            AgentRole::SecurityAuditor      => vec![],
            AgentRole::DataAnalyst          => vec!["api.openai.com".into()],
            AgentRole::CustomerSupport      => vec!["api.telegram.org".into(), "discord.com".into()],
            AgentRole::KnowledgeOfficer     => vec![],  // 纯本地 RAG，无需外网
            AgentRole::SocialMediaManager   => vec![
                "weibo.com".into(), "api.weibo.com".into(),
                "api.twitter.com".into(), "api.xiaohongshu.com".into(),
            ],
            AgentRole::InboxTriageAgent     => vec![
                "imap.gmail.com".into(), "smtp.gmail.com".into(),
                "outlook.office365.com".into(),
            ],
            AgentRole::FinanceProcurement   => vec![
                "api.alipay.com".into(), "api.wechatpay.com".into(),
            ],
            AgentRole::NewsSecretary        => vec![
                "newsapi.org".into(), "feeds.feedburner.com".into(),
                "api.telegram.org".into(),
            ],
            AgentRole::SecurityCodeAuditor  => vec![
                "api.github.com".into(), "github.com".into(),
            ],
            AgentRole::IntelOfficer         => vec![
                "news.ycombinator.com".into(),
                "techcrunch.com".into(),
                "36kr.com".into(),
                "sspai.com".into(),
                "v2ex.com".into(),
                "feeds.feedburner.com".into(),
                "rss.36kr.com".into(),
                "hnrss.org".into(),
                "www.producthunt.com".into(),
            ],
            AgentRole::Custom { .. }        => vec![],
        }
    }

    /// 角色对应的中文显示名称
    pub fn display_zh(&self) -> &'static str {
        match self {
            AgentRole::TicketAssistant      => "工单助手",
            AgentRole::CodeReviewer         => "代码审查员",
            AgentRole::ReportGenerator      => "报告生成员",
            AgentRole::SecurityAuditor      => "安全审计员",
            AgentRole::DataAnalyst          => "数据分析师",
            AgentRole::CustomerSupport      => "客服助手",
            AgentRole::KnowledgeOfficer     => "知识库首席官",
            AgentRole::SocialMediaManager   => "社媒运营经理",
            AgentRole::InboxTriageAgent     => "邮件分拣员",
            AgentRole::FinanceProcurement   => "财务采购员",
            AgentRole::NewsSecretary        => "新闻信息秘书",
            AgentRole::SecurityCodeAuditor  => "安全代码审计员",
            AgentRole::IntelOfficer         => "全网情报员",
            AgentRole::Custom { .. }        => "自定义员工",
        }
    }

    /// 角色默认头像 URL（DiceBear bottts 风格，每个角色固定种子）
    pub fn default_avatar_url(&self) -> &'static str {
        match self {
            AgentRole::TicketAssistant      => "https://api.dicebear.com/7.x/bottts/svg?seed=ticket-assistant&backgroundColor=b6e3f4",
            AgentRole::CodeReviewer         => "https://api.dicebear.com/7.x/bottts/svg?seed=code-reviewer&backgroundColor=c0aede",
            AgentRole::ReportGenerator      => "https://api.dicebear.com/7.x/bottts/svg?seed=report-gen&backgroundColor=d1f4d1",
            AgentRole::SecurityAuditor      => "https://api.dicebear.com/7.x/bottts/svg?seed=security-auditor&backgroundColor=ffd5dc",
            AgentRole::DataAnalyst          => "https://api.dicebear.com/7.x/bottts/svg?seed=data-analyst&backgroundColor=ffdfbf",
            AgentRole::CustomerSupport      => "https://api.dicebear.com/7.x/bottts/svg?seed=customer-support&backgroundColor=c0e8f4",
            AgentRole::KnowledgeOfficer     => "https://api.dicebear.com/7.x/bottts/svg?seed=knowledge-officer&backgroundColor=e8d5f4",
            AgentRole::SocialMediaManager   => "https://api.dicebear.com/7.x/bottts/svg?seed=social-media&backgroundColor=fce4ec",
            AgentRole::InboxTriageAgent     => "https://api.dicebear.com/7.x/bottts/svg?seed=inbox-triage&backgroundColor=e3f2fd",
            AgentRole::FinanceProcurement   => "https://api.dicebear.com/7.x/bottts/svg?seed=finance-proc&backgroundColor=fff9c4",
            AgentRole::NewsSecretary        => "https://api.dicebear.com/7.x/bottts/svg?seed=news-secretary&backgroundColor=f3e5f5",
            AgentRole::SecurityCodeAuditor  => "https://api.dicebear.com/7.x/bottts/svg?seed=sec-code-audit&backgroundColor=fbe9e7",
            AgentRole::IntelOfficer         => "https://api.dicebear.com/7.x/bottts/svg?seed=intel-officer&backgroundColor=cfe8ff",
            AgentRole::Custom { label: _ }  => "https://api.dicebear.com/7.x/bottts/svg?seed=custom-agent&backgroundColor=eeeeee",
        }
    }

    /// 角色对应的 emoji（用于 UI 徽章）
    pub fn role_emoji(&self) -> &'static str {
        match self {
            AgentRole::TicketAssistant      => "🎫",
            AgentRole::CodeReviewer         => "🔍",
            AgentRole::ReportGenerator      => "📊",
            AgentRole::SecurityAuditor      => "🛡",
            AgentRole::DataAnalyst          => "📈",
            AgentRole::CustomerSupport      => "💬",
            AgentRole::KnowledgeOfficer     => "📚",
            AgentRole::SocialMediaManager   => "📣",
            AgentRole::InboxTriageAgent     => "📥",
            AgentRole::FinanceProcurement   => "💰",
            AgentRole::NewsSecretary        => "📰",
            AgentRole::SecurityCodeAuditor  => "🔐",
            AgentRole::IntelOfficer         => "🕵️",
            AgentRole::Custom { .. }        => "⚙",
        }
    }

    /// 角色描述（用于 UI 提示）
    pub fn description(&self) -> &'static str {
        match self {
            AgentRole::TicketAssistant      => "工单处理与飞书/Jira 集成",
            AgentRole::CodeReviewer         => "GitHub PR 自动审查与建议",
            AgentRole::ReportGenerator      => "自动生成分析报告",
            AgentRole::SecurityAuditor      => "沙盒事件审计与合规检查",
            AgentRole::DataAnalyst          => "数据处理与可视化分析",
            AgentRole::CustomerSupport      => "Telegram/Discord 多渠道客服",
            AgentRole::KnowledgeOfficer     => "本地 PDF/Markdown RAG 索引，政策与技术文档快速检索",
            AgentRole::SocialMediaManager   => "热点文案生成，自动发布至微博/小红书/X，评论情绪监测",
            AgentRole::InboxTriageAgent     => "邮件自动分类（紧急/稍后/垃圾），草拟回复，Human-in-the-loop 确认",
            AgentRole::FinanceProcurement   => "付款审批流程、采购单生成、费用报销自动化",
            AgentRole::NewsSecretary        => "每日定时推送热点新闻，重要事件实时提醒，多渠道分发",
            AgentRole::SecurityCodeAuditor  => "SAST 静态分析，Git 提交监控，单元测试用例生成",
            AgentRole::IntelOfficer         => "利用浏览器抓取能力监控竞争对手动态及行业新闻，AI 自动摘要并推送报告",
            AgentRole::Custom { .. }        => "自定义角色",
        }
    }
}

// ── AgentCapability ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapability {
    pub id: String,
    pub name: String,
    pub requires_approval: bool,
    pub risk_level: u8,
}

impl AgentCapability {
    pub fn new(id: impl Into<String>, name: impl Into<String>, risk_level: u8) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            requires_approval: risk_level >= 3,
            risk_level,
        }
    }
    pub fn risk_label(&self) -> &'static str {
        match self.risk_level { 0 => "None", 1 => "Low", 2 => "Medium", 3 => "High", _ => "Critical" }
    }
}

// ── AgentStats ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStats {
    pub total_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
    pub denied_operations: u64,
    pub approved_operations: u64,
    pub last_run_at: Option<u64>,
    pub total_runtime_secs: u64,
}

impl AgentStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_runs == 0 { return 0.0; }
        self.successful_runs as f64 / self.total_runs as f64
    }
}

// ── AgentProfile ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    // 身份标识
    pub id: AgentId,
    pub display_name: String,
    pub description: String,
    pub role: AgentRole,
    pub owner: String,
    // 生命周期
    pub status: AgentStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub created_by: String,
    // 安全边界
    pub memory_limit_mb: u32,
    pub fs_mounts: Vec<FsMount>,
    pub network_allowlist: Vec<String>,
    pub intercept_shell: bool,
    pub confirm_file_delete: bool,
    pub confirm_network: bool,
    pub confirm_shell_exec: bool,
    pub folder_access: Vec<FolderAccess>,
    pub rag_folders: Vec<RagFolder>,
    pub circuit_breaker: BreakerConfig,
    // 运行时配置
    pub agent_config: AgentConfig,
    pub ai_config: OpenClawAiConfig,
    // 能力集合（插件白名单）
    pub capabilities: Vec<AgentCapability>,
    // 通信渠道绑定
    pub channels: Vec<ChannelConfig>,
    // 运行统计
    pub stats: AgentStats,
    // 标签与元数据
    pub tags: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
    /// 头像 URL（空字符串 = 使用角色默认头像）
    #[serde(default)]
    pub avatar_url: String,
}

impl AgentProfile {
    /// 创建新的数字员工档案（最小权限默认）。
    pub fn new(
        display_name: impl Into<String>,
        role: AgentRole,
        owner: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        let now = now_unix_secs();
        let network_allowlist = role.default_network_allowlist();
        Self {
            id: AgentId::new(),
            display_name: display_name.into(),
            description: String::new(),
            role,
            owner: owner.into(),
            status: AgentStatus::Active,
            created_at: now,
            updated_at: now,
            created_by: created_by.into(),
            memory_limit_mb: 256,
            fs_mounts: Vec::new(),
            network_allowlist,
            intercept_shell: true,
            confirm_file_delete: true,
            confirm_network: true,
            confirm_shell_exec: true,
            folder_access: Vec::new(),
            rag_folders: Vec::new(),
            circuit_breaker: BreakerConfig::default(),
            agent_config: AgentConfig::default(),
            ai_config: OpenClawAiConfig::default(),
            capabilities: Vec::new(),
            channels: Vec::new(),
            stats: AgentStats::default(),
            tags: Vec::new(),
            metadata: std::collections::HashMap::new(),
            avatar_url: String::new(),
        }
    }

    /// 工作空间目录路径
    pub fn workspace_dir(&self) -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".openclaw-plus").join("agents").join(self.id.as_str()).join("workspace")
    }

    /// 有效头像 URL：优先使用用户自定义，否则返回角色默认
    pub fn effective_avatar_url(&self) -> &str {
        if self.avatar_url.is_empty() {
            self.role.default_avatar_url()
        } else {
            &self.avatar_url
        }
    }

    /// 将该 AgentProfile 的安全边界转换为 `SecurityConfig`，
    /// 用于启动对应的 WasmEdge 沙盒实例。
    pub fn to_security_config(&self) -> crate::config::SecurityConfig {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let base = home.join(".openclaw-plus").join("agents").join(self.id.as_str());
        crate::config::SecurityConfig {
            memory_limit_mb: self.memory_limit_mb,
            fs_mounts: self.fs_mounts.clone(),
            network_allowlist: self.network_allowlist.clone(),
            intercept_shell: self.intercept_shell,
            confirm_file_delete: self.confirm_file_delete,
            confirm_network: self.confirm_network,
            confirm_shell_exec: self.confirm_shell_exec,
            openclaw_entry: self.agent_config.entry_path.clone(),
            workspace_dir: base.join("workspace"),
            audit_log_path: base.join("audit.log"),
            circuit_breaker: self.circuit_breaker.clone(),
            github: crate::config::GitHubPolicy::default(),
            agent: self.agent_config.clone(),
            wasm_policy_plugin: None,
            folder_access: self.folder_access.clone(),
            rag_folders: self.rag_folders.clone(),
            openclaw_ai: self.ai_config.clone(),
            channels: self.channels.clone(),
        }
    }

    pub fn to_toml(&self) -> anyhow::Result<String> { Ok(toml::to_string_pretty(self)?) }
    pub fn from_toml(s: &str) -> anyhow::Result<Self> { Ok(toml::from_str(s)?) }
    pub fn to_json(&self) -> anyhow::Result<String> { Ok(serde_json::to_string_pretty(self)?) }

    pub fn profile_path(agent_id: &str) -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
            .join(".openclaw-plus").join("agents").join(agent_id).join("profile.toml")
    }

    pub fn data_dir(agent_id: &str) -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
            .join(".openclaw-plus").join("agents").join(agent_id)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::profile_path(self.id.as_str());
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(&path, self.to_toml()?)?;
        tracing::info!(agent_id = %self.id, name = %self.display_name, "AgentProfile saved");
        Ok(())
    }

    pub fn load(agent_id: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(Self::profile_path(agent_id))?;
        Self::from_toml(&content)
    }

    pub fn list_all() -> Vec<Self> {
        let agents_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".openclaw-plus").join("agents");
        let Ok(entries) = std::fs::read_dir(&agents_dir) else { return Vec::new(); };
        let mut profiles: Vec<Self> = entries.flatten()
            .filter_map(|e| {
                let p = e.path().join("profile.toml");
                std::fs::read_to_string(&p).ok()
                    .and_then(|s| Self::from_toml(&s).ok())
            })
            .collect();
        profiles.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        profiles
    }

    pub fn touch(&mut self) { self.updated_at = now_unix_secs(); }
    pub fn suspend(&mut self) { self.status = AgentStatus::Suspended; self.touch(); }
    pub fn resume(&mut self) { self.status = AgentStatus::Active; self.touch(); }
    pub fn archive(&mut self) { self.status = AgentStatus::Archived; self.touch(); }
    pub fn mark_faulted(&mut self, reason: impl Into<String>) {
        self.status = AgentStatus::Faulted { reason: reason.into() };
        self.touch();
    }

    pub fn add_capability(&mut self, cap: AgentCapability) {
        if !self.capabilities.iter().any(|c| c.id == cap.id) {
            self.capabilities.push(cap);
            self.touch();
        }
    }

    pub fn remove_capability(&mut self, cap_id: &str) {
        self.capabilities.retain(|c| c.id != cap_id);
        self.touch();
    }

    pub fn allow_network(&mut self, host: impl Into<String>) {
        let host = host.into();
        if !self.network_allowlist.contains(&host) {
            self.network_allowlist.push(host);
            self.touch();
        }
    }

    pub fn is_network_allowed(&self, host: &str) -> bool {
        self.network_allowlist.iter().any(|a| host == a || host.ends_with(&format!(".{}", a)))
    }

    pub fn record_run_success(&mut self, runtime_secs: u64) {
        self.stats.total_runs += 1;
        self.stats.successful_runs += 1;
        self.stats.total_runtime_secs += runtime_secs;
        self.stats.last_run_at = Some(now_unix_secs());
        self.touch();
    }

    pub fn record_run_failure(&mut self, runtime_secs: u64) {
        self.stats.total_runs += 1;
        self.stats.failed_runs += 1;
        self.stats.total_runtime_secs += runtime_secs;
        self.stats.last_run_at = Some(now_unix_secs());
        self.touch();
    }
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── 单元测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile() -> AgentProfile {
        AgentProfile::new("工单助手-01", AgentRole::TicketAssistant, "acme-corp", "admin")
    }

    #[test]
    fn test_agent_id_unique() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1.as_str(), id2.as_str());
        let parts: Vec<&str> = id1.as_str().split('-').collect();
        assert_eq!(parts.len(), 5, "UUID should have 5 dash-separated parts");
    }

    #[test]
    fn test_profile_defaults_least_privilege() {
        let p = make_profile();
        assert!(p.capabilities.is_empty(), "Default capabilities must be empty (least privilege)");
        assert!(p.fs_mounts.is_empty(), "Default fs_mounts must be empty");
        assert_eq!(p.memory_limit_mb, 256);
        assert!(p.intercept_shell);
        assert!(p.confirm_shell_exec);
        assert!(p.confirm_file_delete);
        assert!(p.confirm_network);
        assert!(matches!(p.status, AgentStatus::Active));
    }

    #[test]
    fn test_ticket_assistant_default_network() {
        let p = make_profile();
        assert!(p.network_allowlist.contains(&"jira.example.com".to_string()));
        assert!(p.network_allowlist.contains(&"api.feishu.cn".to_string()));
        assert!(!p.is_network_allowed("evil.example.com"));
        assert!(p.is_network_allowed("jira.example.com"));
    }

    #[test]
    fn test_capability_management() {
        let mut p = make_profile();
        let cap = AgentCapability::new("jira.create_ticket", "Create Jira Ticket", 2);
        p.add_capability(cap.clone());
        assert_eq!(p.capabilities.len(), 1);
        // 重复添加不应增加
        p.add_capability(cap);
        assert_eq!(p.capabilities.len(), 1);
        p.remove_capability("jira.create_ticket");
        assert!(p.capabilities.is_empty());
    }

    #[test]
    fn test_lifecycle_transitions() {
        let mut p = make_profile();
        assert!(p.status.can_accept_task());
        p.suspend();
        assert!(!p.status.can_accept_task());
        assert!(matches!(p.status, AgentStatus::Suspended));
        p.resume();
        assert!(matches!(p.status, AgentStatus::Active));
        p.mark_faulted("sandbox crash");
        assert!(matches!(p.status, AgentStatus::Faulted { .. }));
        assert!(p.status.can_start());
        p.archive();
        assert!(matches!(p.status, AgentStatus::Archived));
        assert!(!p.status.can_start());
    }

    #[test]
    fn test_stats_tracking() {
        let mut p = make_profile();
        p.record_run_success(120);
        p.record_run_success(60);
        p.record_run_failure(30);
        assert_eq!(p.stats.total_runs, 3);
        assert_eq!(p.stats.successful_runs, 2);
        assert_eq!(p.stats.failed_runs, 1);
        assert_eq!(p.stats.total_runtime_secs, 210);
        let rate = p.stats.success_rate();
        assert!((rate - 2.0/3.0).abs() < 1e-9);
    }

    #[test]
    fn test_toml_roundtrip() {
        let mut p = make_profile();
        p.add_capability(AgentCapability::new("test.cap", "Test Cap", 1));
        p.allow_network("api.example.com");
        let toml_str = p.to_toml().expect("serialize to TOML");
        let loaded = AgentProfile::from_toml(&toml_str).expect("deserialize from TOML");
        assert_eq!(loaded.id, p.id);
        assert_eq!(loaded.display_name, p.display_name);
        assert_eq!(loaded.capabilities.len(), 1);
        assert_eq!(loaded.network_allowlist.len(), p.network_allowlist.len());
    }

    #[test]
    fn test_json_roundtrip() {
        let p = make_profile();
        let json_str = p.to_json().expect("serialize to JSON");
        let loaded: AgentProfile = serde_json::from_str(&json_str).expect("deserialize from JSON");
        assert_eq!(loaded.id, p.id);
        assert_eq!(loaded.owner, "acme-corp");
    }

    #[test]
    fn test_to_security_config() {
        let p = make_profile();
        let sc = p.to_security_config();
        assert_eq!(sc.memory_limit_mb, p.memory_limit_mb);
        assert_eq!(sc.network_allowlist, p.network_allowlist);
        assert_eq!(sc.intercept_shell, p.intercept_shell);
        // workspace_dir 应包含 agent_id
        assert!(sc.workspace_dir.to_str().unwrap().contains(p.id.as_str()));
    }

    #[test]
    fn test_capability_risk_levels() {
        let low  = AgentCapability::new("a", "A", 1);
        let high = AgentCapability::new("b", "B", 3);
        let crit = AgentCapability::new("c", "C", 4);
        assert!(!low.requires_approval);
        assert!(high.requires_approval);
        assert!(crit.requires_approval);
        assert_eq!(low.risk_label(), "Low");
        assert_eq!(high.risk_label(), "High");
        assert_eq!(crit.risk_label(), "Critical");
    }

    #[test]
    fn test_network_subdomain_matching() {
        let mut p = make_profile();
        p.network_allowlist = vec!["example.com".to_string()];
        assert!(p.is_network_allowed("example.com"));
        assert!(p.is_network_allowed("api.example.com"));
        assert!(p.is_network_allowed("deep.api.example.com"));
        assert!(!p.is_network_allowed("evil.com"));
        assert!(!p.is_network_allowed("notexample.com"));
    }

    #[test]
    fn test_role_emoji_coverage() {
        // Every preset role must return a non-empty emoji string
        let roles = [
            AgentRole::TicketAssistant,
            AgentRole::CodeReviewer,
            AgentRole::ReportGenerator,
            AgentRole::SecurityAuditor,
            AgentRole::DataAnalyst,
            AgentRole::CustomerSupport,
            AgentRole::KnowledgeOfficer,
            AgentRole::SocialMediaManager,
            AgentRole::InboxTriageAgent,
            AgentRole::FinanceProcurement,
            AgentRole::NewsSecretary,
            AgentRole::SecurityCodeAuditor,
            AgentRole::Custom { label: "Test".to_string() },
        ];
        for role in &roles {
            let emoji = role.role_emoji();
            assert!(!emoji.is_empty(), "role_emoji must not be empty for {:?}", role);
        }
        // Spot-check specific mappings
        assert_eq!(AgentRole::TicketAssistant.role_emoji(), "🎫");
        assert_eq!(AgentRole::FinanceProcurement.role_emoji(), "💰");
        assert_eq!(AgentRole::SecurityCodeAuditor.role_emoji(), "🔐");
        assert_eq!(AgentRole::Custom { label: "x".into() }.role_emoji(), "⚙");
    }

    #[test]
    fn test_role_emoji_distinct_from_display_zh() {
        // Emoji and zh name must be different strings
        let role = AgentRole::CodeReviewer;
        assert_ne!(role.role_emoji(), role.display_zh());
    }
}
