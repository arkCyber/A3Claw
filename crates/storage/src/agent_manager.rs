//! # AgentManager — 数字员工生命周期管理器
//!
//! 提供创建、启动、暂停、归档、列举数字员工的高级接口。
//! 协调 `AgentStore`（SQLite）与 `AgentProfile`（文件系统）两层存储。

use anyhow::{Context, Result};
use openclaw_security::{AgentProfile, AgentRole, AgentCapability};
use tracing::info;

use crate::db::Database;
use crate::agent_store::AgentStore;
use crate::run_store::RunStore;
use crate::audit_store::AuditStore;
use crate::types::{RunRecord, RunStatus, AuditEventRecord, AuditDecision};

/// 数字员工生命周期管理器。
pub struct AgentManager<'db> {
    db: &'db Database,
}

impl<'db> AgentManager<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    // ── 创建 ──────────────────────────────────────────────────────────────

    /// 创建新的数字员工，持久化到 SQLite 和文件系统。
    pub fn create(
        &self,
        display_name: impl Into<String>,
        role: AgentRole,
        owner: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Result<AgentProfile> {
        let profile = AgentProfile::new(display_name, role, owner, created_by);
        let store = AgentStore::new(self.db);
        store.insert(&profile).context("persist new agent to SQLite")?;
        profile.save().context("persist new agent profile to filesystem")?;

        // 写入审计事件
        let mut event = AuditEventRecord::new(
            profile.id.as_str(),
            "agent_created",
            AuditDecision::AutoAllowed,
            &profile.created_by,
        );
        event.target = Some(profile.display_name.clone());
        AuditStore::new(self.db).append(&event).context("audit agent_created")?;

        info!(agent_id = %profile.id, name = %profile.display_name, "Agent created");
        Ok(profile)
    }

    // ── 查询 ──────────────────────────────────────────────────────────────

    /// 按 ID 加载数字员工档案。
    pub fn get(&self, agent_id: &str) -> Result<Option<AgentProfile>> {
        AgentStore::new(self.db).get(agent_id)
    }

    /// 列举所有数字员工（按创建时间降序）。
    pub fn list_all(&self) -> Result<Vec<AgentProfile>> {
        AgentStore::new(self.db).list_all()
    }

    /// 列举活跃的数字员工。
    pub fn list_active(&self) -> Result<Vec<AgentProfile>> {
        AgentStore::new(self.db).list_by_status("active")
    }

    // ── 更新 ──────────────────────────────────────────────────────────────

    /// 保存对数字员工档案的修改（同步到 SQLite 和文件系统）。
    pub fn save(&self, profile: &AgentProfile) -> Result<()> {
        AgentStore::new(self.db).upsert(profile).context("upsert agent to SQLite")?;
        profile.save().context("save agent profile to filesystem")?;
        Ok(())
    }

    // ── 生命周期操作 ──────────────────────────────────────────────────────

    /// 暂停数字员工。
    pub fn suspend(&self, agent_id: &str, by: &str) -> Result<()> {
        let mut profile = self.get(agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
        profile.suspend();
        self.save(&profile)?;
        self.audit(agent_id, None, "agent_suspended", AuditDecision::AutoAllowed, by, None)?;
        info!(agent_id, "Agent suspended");
        Ok(())
    }

    /// 恢复数字员工。
    pub fn resume(&self, agent_id: &str, by: &str) -> Result<()> {
        let mut profile = self.get(agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
        profile.resume();
        self.save(&profile)?;
        self.audit(agent_id, None, "agent_resumed", AuditDecision::AutoAllowed, by, None)?;
        info!(agent_id, "Agent resumed");
        Ok(())
    }

    /// 归档数字员工（软删除）。
    pub fn archive(&self, agent_id: &str, by: &str) -> Result<()> {
        let mut profile = self.get(agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
        profile.archive();
        self.save(&profile)?;
        self.audit(agent_id, None, "agent_archived", AuditDecision::AutoAllowed, by, None)?;
        info!(agent_id, "Agent archived");
        Ok(())
    }

    // ── 任务执行 ──────────────────────────────────────────────────────────

    /// 为数字员工创建一次任务执行记录（Run）。
    pub fn start_run(&self, agent_id: &str, task: impl Into<String>) -> Result<RunRecord> {
        let profile = self.get(agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
        if !profile.status.can_accept_task() {
            anyhow::bail!("Agent {} cannot accept tasks (status: {})", agent_id, profile.status);
        }
        let run = RunRecord::new(agent_id, task);
        RunStore::new(self.db).create_run(&run)?;
        self.audit(agent_id, Some(&run.id), "run_started", AuditDecision::AutoAllowed, "system", None)?;
        info!(agent_id, run_id = %run.id, "Run started");
        Ok(run)
    }

    /// 完成一次任务执行记录。
    pub fn finish_run(
        &self,
        run_id: &str,
        agent_id: &str,
        success: bool,
        summary: Option<&str>,
    ) -> Result<()> {
        let status = if success { RunStatus::Success } else { RunStatus::Failed };
        RunStore::new(self.db).finish_run(run_id, status.clone(), summary)?;

        // 更新 Agent 统计
        if let Ok(Some(mut profile)) = self.get(agent_id) {
            if success {
                profile.record_run_success(0);
            } else {
                profile.record_run_failure(0);
            }
            let _ = self.save(&profile);
        }

        let event_kind = if success { "run_success" } else { "run_failed" };
        self.audit(agent_id, Some(run_id), event_kind, AuditDecision::AutoAllowed, "system", summary)?;
        info!(run_id, success, "Run finished");
        Ok(())
    }

    // ── 能力管理 ──────────────────────────────────────────────────────────

    /// 为数字员工添加能力（插件）。
    pub fn add_capability(&self, agent_id: &str, cap: AgentCapability, by: &str) -> Result<()> {
        let mut profile = self.get(agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
        let cap_id = cap.id.clone();
        profile.add_capability(cap);
        self.save(&profile)?;
        self.audit(
            agent_id, None, "capability_added",
            AuditDecision::AutoAllowed, by,
            Some(&cap_id),
        )?;
        Ok(())
    }

    /// 移除数字员工的能力（插件）。
    pub fn remove_capability(&self, agent_id: &str, cap_id: &str, by: &str) -> Result<()> {
        let mut profile = self.get(agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
        profile.remove_capability(cap_id);
        self.save(&profile)?;
        self.audit(
            agent_id, None, "capability_removed",
            AuditDecision::AutoAllowed, by,
            Some(cap_id),
        )?;
        Ok(())
    }

    // ── 统计 ──────────────────────────────────────────────────────────────

    /// 返回平台统计摘要。
    pub fn platform_summary(&self) -> Result<PlatformSummary> {
        let store = AgentStore::new(self.db);
        let run_store = RunStore::new(self.db);
        let audit_store = AuditStore::new(self.db);

        let total_agents = store.count()?;
        let status_summary = store.status_summary()?;
        let active_agents = status_summary.iter()
            .find(|(s, _)| s.eq_ignore_ascii_case("active"))
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let total_runs = run_store.count_runs()?;
        let running_now = run_store.list_running()?.len() as u64;
        let total_audit_events = audit_store.total_count()?;

        Ok(PlatformSummary {
            total_agents,
            active_agents,
            total_runs,
            running_now,
            total_audit_events,
            status_breakdown: status_summary,
        })
    }

    // ── 内部辅助 ──────────────────────────────────────────────────────────

    fn audit(
        &self,
        agent_id: &str,
        run_id: Option<&str>,
        event_kind: &str,
        decision: AuditDecision,
        actor: &str,
        target: Option<&str>,
    ) -> Result<()> {
        let mut event = AuditEventRecord::new(agent_id, event_kind, decision, actor);
        event.run_id = run_id.map(|s| s.to_string());
        event.target = target.map(|s| s.to_string());
        AuditStore::new(self.db).append(&event)?;
        Ok(())
    }
}

/// 平台统计摘要。
#[derive(Debug, Clone)]
pub struct PlatformSummary {
    pub total_agents: u64,
    pub active_agents: u64,
    pub total_runs: u64,
    pub running_now: u64,
    pub total_audit_events: u64,
    pub status_breakdown: Vec<(String, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use openclaw_security::AgentStatus;
    use tempfile::tempdir;

    fn open_db() -> Database {
        let dir = tempdir().unwrap();
        Database::open(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn test_create_agent() {
        let db = open_db();
        let mgr = AgentManager::new(&db);
        let p = mgr.create("工单助手", AgentRole::TicketAssistant, "acme", "admin").unwrap();
        assert!(!p.id.as_str().is_empty());
        assert_eq!(p.display_name, "工单助手");

        // 验证已持久化到 SQLite
        let loaded = mgr.get(p.id.as_str()).unwrap().unwrap();
        assert_eq!(loaded.id, p.id);
    }

    #[test]
    fn test_lifecycle_suspend_resume() {
        let db = open_db();
        let mgr = AgentManager::new(&db);
        let p = mgr.create("助手-A", AgentRole::CodeReviewer, "org", "admin").unwrap();

        mgr.suspend(p.id.as_str(), "admin").unwrap();
        let suspended = mgr.get(p.id.as_str()).unwrap().unwrap();
        assert!(matches!(suspended.status, AgentStatus::Suspended));

        mgr.resume(p.id.as_str(), "admin").unwrap();
        let resumed = mgr.get(p.id.as_str()).unwrap().unwrap();
        assert!(matches!(resumed.status, AgentStatus::Active));
    }

    #[test]
    fn test_archive() {
        let db = open_db();
        let mgr = AgentManager::new(&db);
        let p = mgr.create("助手-归档", AgentRole::DataAnalyst, "org", "admin").unwrap();
        mgr.archive(p.id.as_str(), "admin").unwrap();
        let archived = mgr.get(p.id.as_str()).unwrap().unwrap();
        assert!(matches!(archived.status, AgentStatus::Archived));
    }

    #[test]
    fn test_start_run_blocked_when_suspended() {
        let db = open_db();
        let mgr = AgentManager::new(&db);
        let p = mgr.create("助手-暂停", AgentRole::TicketAssistant, "org", "admin").unwrap();
        mgr.suspend(p.id.as_str(), "admin").unwrap();
        let result = mgr.start_run(p.id.as_str(), "Process tickets");
        assert!(result.is_err(), "Suspended agent should not accept tasks");
    }

    #[test]
    fn test_run_lifecycle() {
        let db = open_db();
        let mgr = AgentManager::new(&db);
        let p = mgr.create("助手-运行", AgentRole::ReportGenerator, "org", "admin").unwrap();
        let run = mgr.start_run(p.id.as_str(), "Generate daily report").unwrap();
        assert_eq!(run.status, RunStatus::Running);

        mgr.finish_run(&run.id, p.id.as_str(), true, Some("Report generated")).unwrap();

        // 验证 Agent 统计已更新
        let updated = mgr.get(p.id.as_str()).unwrap().unwrap();
        assert_eq!(updated.stats.total_runs, 1);
        assert_eq!(updated.stats.successful_runs, 1);
    }

    #[test]
    fn test_capability_management() {
        let db = open_db();
        let mgr = AgentManager::new(&db);
        let p = mgr.create("助手-能力", AgentRole::TicketAssistant, "org", "admin").unwrap();

        let cap = AgentCapability::new("jira.create", "Create Jira Ticket", 2);
        mgr.add_capability(p.id.as_str(), cap, "admin").unwrap();

        let loaded = mgr.get(p.id.as_str()).unwrap().unwrap();
        assert_eq!(loaded.capabilities.len(), 1);

        mgr.remove_capability(p.id.as_str(), "jira.create", "admin").unwrap();
        let loaded2 = mgr.get(p.id.as_str()).unwrap().unwrap();
        assert!(loaded2.capabilities.is_empty());
    }

    #[test]
    fn test_platform_summary() {
        let db = open_db();
        let mgr = AgentManager::new(&db);

        for i in 0..3 {
            mgr.create(format!("助手-{}", i), AgentRole::TicketAssistant, "org", "admin").unwrap();
        }
        let p = mgr.create("助手-归档", AgentRole::DataAnalyst, "org", "admin").unwrap();
        mgr.archive(p.id.as_str(), "admin").unwrap();

        let summary = mgr.platform_summary().unwrap();
        assert_eq!(summary.total_agents, 4);
        assert_eq!(summary.active_agents, 3);
    }

    #[test]
    fn test_audit_trail_on_create() {
        let db = open_db();
        let mgr = AgentManager::new(&db);
        let p = mgr.create("审计测试", AgentRole::SecurityAuditor, "org", "admin").unwrap();

        let audit = AuditStore::new(&db);
        let count = audit.count_for_agent(p.id.as_str()).unwrap();
        assert!(count >= 1, "At least one audit event should be created on agent creation");
    }
}
