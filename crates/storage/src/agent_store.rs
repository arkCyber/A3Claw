//! 数字员工档案的 CRUD 操作。

use anyhow::{Context, Result};
use openclaw_security::AgentProfile;
use rusqlite::params;
use tracing::{debug, info};

use crate::db::Database;

/// 数字员工档案存储操作。
pub struct AgentStore<'db> {
    db: &'db Database,
}

impl<'db> AgentStore<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    /// 插入新的数字员工档案。
    pub fn insert(&self, profile: &AgentProfile) -> Result<()> {
        let json = profile.to_json().context("serialize AgentProfile to JSON")?;
        self.db.conn.execute(
            "INSERT INTO agents (id, display_name, role, owner, status, created_at, updated_at, created_by, profile_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                profile.id.as_str(),
                profile.display_name,
                profile.role.to_string(),
                profile.owner,
                profile.status.to_string(),
                profile.created_at as i64,
                profile.updated_at as i64,
                profile.created_by,
                json,
            ],
        ).context("insert agent")?;
        info!(agent_id = %profile.id, name = %profile.display_name, "Agent inserted");
        Ok(())
    }

    /// 更新已有的数字员工档案（全量替换 profile_json）。
    pub fn update(&self, profile: &AgentProfile) -> Result<()> {
        let json = profile.to_json().context("serialize AgentProfile to JSON")?;
        let rows = self.db.conn.execute(
            "UPDATE agents SET display_name=?2, role=?3, status=?4, updated_at=?5, profile_json=?6
             WHERE id=?1",
            params![
                profile.id.as_str(),
                profile.display_name,
                profile.role.to_string(),
                profile.status.to_string(),
                profile.updated_at as i64,
                json,
            ],
        ).context("update agent")?;
        if rows == 0 {
            anyhow::bail!("Agent not found: {}", profile.id);
        }
        debug!(agent_id = %profile.id, "Agent updated");
        Ok(())
    }

    /// 插入或更新（upsert）。
    pub fn upsert(&self, profile: &AgentProfile) -> Result<()> {
        if self.exists(profile.id.as_str())? {
            self.update(profile)
        } else {
            self.insert(profile)
        }
    }

    /// 检查 Agent 是否存在。
    pub fn exists(&self, agent_id: &str) -> Result<bool> {
        let count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM agents WHERE id=?1",
            params![agent_id],
            |r| r.get(0),
        ).context("check agent exists")?;
        Ok(count > 0)
    }

    /// 按 ID 加载数字员工档案。
    pub fn get(&self, agent_id: &str) -> Result<Option<AgentProfile>> {
        let result = self.db.conn.query_row(
            "SELECT profile_json FROM agents WHERE id=?1",
            params![agent_id],
            |r| r.get::<_, String>(0),
        );
        match result {
            Ok(json) => {
                let profile: AgentProfile = serde_json::from_str(&json)
                    .context("deserialize AgentProfile from JSON")?;
                Ok(Some(profile))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("get agent"),
        }
    }

    /// 列举所有数字员工档案（按创建时间降序）。
    pub fn list_all(&self) -> Result<Vec<AgentProfile>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT profile_json FROM agents ORDER BY created_at DESC"
        ).context("prepare list_all")?;
        let profiles: Vec<AgentProfile> = stmt.query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str::<AgentProfile>(&json).ok())
            .collect();
        Ok(profiles)
    }

    /// 按状态筛选数字员工档案。
    pub fn list_by_status(&self, status: &str) -> Result<Vec<AgentProfile>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT profile_json FROM agents WHERE status=?1 ORDER BY created_at DESC"
        ).context("prepare list_by_status")?;
        let profiles: Vec<AgentProfile> = stmt.query_map(params![status], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str::<AgentProfile>(&json).ok())
            .collect();
        Ok(profiles)
    }

    /// 按所有者筛选数字员工档案。
    pub fn list_by_owner(&self, owner: &str) -> Result<Vec<AgentProfile>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT profile_json FROM agents WHERE owner=?1 ORDER BY created_at DESC"
        ).context("prepare list_by_owner")?;
        let profiles: Vec<AgentProfile> = stmt.query_map(params![owner], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str::<AgentProfile>(&json).ok())
            .collect();
        Ok(profiles)
    }

    /// 软删除（归档）数字员工。
    pub fn archive(&self, agent_id: &str) -> Result<()> {
        let now = crate::types::now_unix_secs() as i64;
        let rows = self.db.conn.execute(
            "UPDATE agents SET status='archived', updated_at=?2 WHERE id=?1",
            params![agent_id, now],
        ).context("archive agent")?;
        if rows == 0 {
            anyhow::bail!("Agent not found: {}", agent_id);
        }
        info!(agent_id, "Agent archived");
        Ok(())
    }

    /// 返回数字员工总数。
    pub fn count(&self) -> Result<u64> {
        let n: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM agents", [], |r| r.get(0)
        ).context("count agents")?;
        Ok(n as u64)
    }

    /// 返回各状态的数字员工数量统计。
    pub fn status_summary(&self) -> Result<Vec<(String, u64)>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT status, COUNT(*) FROM agents GROUP BY status"
        ).context("prepare status_summary")?;
        let rows: Vec<(String, u64)> = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use openclaw_security::{AgentProfile, AgentRole, AgentCapability};
    use tempfile::tempdir;

    fn open_db() -> Database {
        let dir = tempdir().unwrap();
        Database::open(&dir.path().join("test.db")).unwrap()
    }

    fn make_profile(name: &str) -> AgentProfile {
        AgentProfile::new(name, AgentRole::TicketAssistant, "acme", "admin")
    }

    #[test]
    fn test_insert_and_get() {
        let db = open_db();
        let store = AgentStore::new(&db);
        let p = make_profile("工单助手-01");
        store.insert(&p).unwrap();
        let loaded = store.get(p.id.as_str()).unwrap().unwrap();
        assert_eq!(loaded.id, p.id);
        assert_eq!(loaded.display_name, "工单助手-01");
    }

    #[test]
    fn test_get_nonexistent() {
        let db = open_db();
        let store = AgentStore::new(&db);
        let result = store.get("nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update() {
        let db = open_db();
        let store = AgentStore::new(&db);
        let mut p = make_profile("助手-A");
        store.insert(&p).unwrap();
        p.display_name = "助手-A-更新".to_string();
        p.touch();
        store.update(&p).unwrap();
        let loaded = store.get(p.id.as_str()).unwrap().unwrap();
        assert_eq!(loaded.display_name, "助手-A-更新");
    }

    #[test]
    fn test_upsert() {
        let db = open_db();
        let store = AgentStore::new(&db);
        let mut p = make_profile("助手-B");
        // 第一次 upsert = insert
        store.upsert(&p).unwrap();
        assert_eq!(store.count().unwrap(), 1);
        // 第二次 upsert = update
        p.display_name = "助手-B-v2".to_string();
        p.touch();
        store.upsert(&p).unwrap();
        assert_eq!(store.count().unwrap(), 1);
        let loaded = store.get(p.id.as_str()).unwrap().unwrap();
        assert_eq!(loaded.display_name, "助手-B-v2");
    }

    #[test]
    fn test_list_all() {
        let db = open_db();
        let store = AgentStore::new(&db);
        for i in 0..5 {
            store.insert(&make_profile(&format!("助手-{}", i))).unwrap();
        }
        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_archive() {
        let db = open_db();
        let store = AgentStore::new(&db);
        let p = make_profile("助手-归档");
        store.insert(&p).unwrap();
        store.archive(p.id.as_str()).unwrap();
        let archived = store.list_by_status("archived").unwrap();
        assert_eq!(archived.len(), 1);
    }

    #[test]
    fn test_capabilities_preserved_in_json() {
        let db = open_db();
        let store = AgentStore::new(&db);
        let mut p = make_profile("助手-能力测试");
        p.add_capability(AgentCapability::new("jira.create", "Create Jira", 2));
        p.add_capability(AgentCapability::new("slack.send", "Send Slack", 1));
        store.insert(&p).unwrap();
        let loaded = store.get(p.id.as_str()).unwrap().unwrap();
        assert_eq!(loaded.capabilities.len(), 2);
        assert_eq!(loaded.capabilities[0].id, "jira.create");
    }

    #[test]
    fn test_status_summary() {
        let db = open_db();
        let store = AgentStore::new(&db);
        store.insert(&make_profile("A")).unwrap();
        store.insert(&make_profile("B")).unwrap();
        let p = make_profile("C");
        store.insert(&p).unwrap();
        store.archive(p.id.as_str()).unwrap();
        let summary = store.status_summary().unwrap();
        let total: u64 = summary.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 3);
    }
}
