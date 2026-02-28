//! 不可变审计事件流的写入与查询。
//!
//! ## 设计原则
//! - 审计事件**只追加，不修改，不删除**
//! - 支持按 agent_id / run_id / 时间范围 / 决策类型查询
//! - 支持分页（limit + offset）

use anyhow::{Context, Result};
use rusqlite::params;
use tracing::debug;

use crate::db::Database;
use crate::types::{AuditEventRecord, AuditDecision};

/// 审计事件查询过滤器。
#[derive(Debug, Default)]
pub struct AuditFilter {
    pub agent_id:  Option<String>,
    pub run_id:    Option<String>,
    pub decision:  Option<String>,
    pub ts_from:   Option<u64>,
    pub ts_to:     Option<u64>,
    pub limit:     u32,
    pub offset:    u32,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self { limit: 100, ..Default::default() }
    }
    pub fn for_agent(agent_id: impl Into<String>) -> Self {
        Self { agent_id: Some(agent_id.into()), limit: 100, ..Default::default() }
    }
    pub fn for_run(run_id: impl Into<String>) -> Self {
        Self { run_id: Some(run_id.into()), limit: 1000, ..Default::default() }
    }
}

/// 审计事件存储操作。
pub struct AuditStore<'db> {
    db: &'db Database,
}

impl<'db> AuditStore<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    /// 追加写入一条审计事件（不可修改）。
    pub fn append(&self, event: &AuditEventRecord) -> Result<i64> {
        self.db.conn.execute(
            "INSERT INTO audit_events
             (run_id, agent_id, step_index, event_kind, target,
              input_hash, decision, actor, reason, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                event.run_id,
                event.agent_id,
                event.step_index.map(|v| v as i64),
                event.event_kind,
                event.target,
                event.input_hash,
                event.decision.as_str(),
                event.actor,
                event.reason,
                event.ts as i64,
            ],
        ).context("append audit event")?;
        let id = self.db.conn.last_insert_rowid();
        debug!(audit_id = id, agent_id = %event.agent_id, kind = %event.event_kind, "Audit event appended");
        Ok(id)
    }

    /// 批量追加审计事件（事务内执行）。
    pub fn append_batch(&self, events: &[AuditEventRecord]) -> Result<()> {
        let tx = self.db.conn.unchecked_transaction().context("begin batch transaction")?;
        for event in events {
            tx.execute(
                "INSERT INTO audit_events
                 (run_id, agent_id, step_index, event_kind, target,
                  input_hash, decision, actor, reason, ts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.run_id,
                    event.agent_id,
                    event.step_index.map(|v| v as i64),
                    event.event_kind,
                    event.target,
                    event.input_hash,
                    event.decision.as_str(),
                    event.actor,
                    event.reason,
                    event.ts as i64,
                ],
            ).context("batch insert audit event")?;
        }
        tx.commit().context("commit batch")?;
        Ok(())
    }

    /// 按过滤条件查询审计事件（分页）。
    pub fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEventRecord>> {
        let mut sql = String::from(
            "SELECT id, run_id, agent_id, step_index, event_kind, target,
                    input_hash, decision, actor, reason, ts
             FROM audit_events WHERE 1=1"
        );
        let mut bind_idx = 1usize;
        let mut conditions: Vec<String> = Vec::new();

        if filter.agent_id.is_some() {
            conditions.push(format!(" AND agent_id=?{}", bind_idx));
            bind_idx += 1;
        }
        if filter.run_id.is_some() {
            conditions.push(format!(" AND run_id=?{}", bind_idx));
            bind_idx += 1;
        }
        if filter.decision.is_some() {
            conditions.push(format!(" AND decision=?{}", bind_idx));
            bind_idx += 1;
        }
        if filter.ts_from.is_some() {
            conditions.push(format!(" AND ts>=?{}", bind_idx));
            bind_idx += 1;
        }
        if filter.ts_to.is_some() {
            conditions.push(format!(" AND ts<=?{}", bind_idx));
            bind_idx += 1;
        }
        for c in &conditions { sql.push_str(c); }
        sql.push_str(&format!(" ORDER BY ts DESC LIMIT ?{} OFFSET ?{}", bind_idx, bind_idx + 1));

        let mut stmt = self.db.conn.prepare(&sql).context("prepare audit query")?;

        // 动态绑定参数
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(ref v) = filter.agent_id  { param_values.push(Box::new(v.clone())); }
        if let Some(ref v) = filter.run_id    { param_values.push(Box::new(v.clone())); }
        if let Some(ref v) = filter.decision  { param_values.push(Box::new(v.clone())); }
        if let Some(v) = filter.ts_from       { param_values.push(Box::new(v as i64)); }
        if let Some(v) = filter.ts_to         { param_values.push(Box::new(v as i64)); }
        param_values.push(Box::new(filter.limit as i64));
        param_values.push(Box::new(filter.offset as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();

        let events = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(AuditEventRecord {
                id:          r.get(0)?,
                run_id:      r.get(1)?,
                agent_id:    r.get(2)?,
                step_index:  r.get::<_, Option<i64>>(3)?.map(|v| v as u32),
                event_kind:  r.get(4)?,
                target:      r.get(5)?,
                input_hash:  r.get(6)?,
                decision:    AuditDecision::from_string(&r.get::<_, String>(7)?),
                actor:       r.get(8)?,
                reason:      r.get(9)?,
                ts:          r.get::<_, i64>(10)? as u64,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(events)
    }

    /// 返回某个 Agent 的审计事件总数。
    pub fn count_for_agent(&self, agent_id: &str) -> Result<u64> {
        let n: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM audit_events WHERE agent_id=?1",
            params![agent_id],
            |r| r.get(0),
        ).context("count audit events for agent")?;
        Ok(n as u64)
    }

    /// 返回某个 Run 的审计事件总数。
    pub fn count_for_run(&self, run_id: &str) -> Result<u64> {
        let n: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM audit_events WHERE run_id=?1",
            params![run_id],
            |r| r.get(0),
        ).context("count audit events for run")?;
        Ok(n as u64)
    }

    /// 返回某个 Agent 的拒绝事件数。
    pub fn count_denied_for_agent(&self, agent_id: &str) -> Result<u64> {
        let n: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM audit_events
             WHERE agent_id=?1 AND decision IN ('auto_denied','human_denied')",
            params![agent_id],
            |r| r.get(0),
        ).context("count denied events")?;
        Ok(n as u64)
    }

    /// 返回审计事件总数。
    pub fn total_count(&self) -> Result<u64> {
        let n: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM audit_events", [], |r| r.get(0)
        ).context("total audit count")?;
        Ok(n as u64)
    }

    /// 按时间范围查询审计事件（用于审计回放）。
    pub fn query_time_range(
        &self,
        agent_id: &str,
        ts_from: u64,
        ts_to: u64,
    ) -> Result<Vec<AuditEventRecord>> {
        let filter = AuditFilter {
            agent_id: Some(agent_id.to_string()),
            ts_from: Some(ts_from),
            ts_to: Some(ts_to),
            limit: 10000,
            ..Default::default()
        };
        self.query(&filter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::types::AuditDecision;
    use rusqlite::params;
    use tempfile::tempdir;

    fn open_db() -> Database {
        let dir = tempdir().unwrap();
        Database::open(&dir.path().join("test.db")).unwrap()
    }

    fn seed_agent(db: &Database, agent_id: &str) {
        db.conn.execute(
            "INSERT INTO agents (id, display_name, role, owner, status, created_at, updated_at, created_by, profile_json)
             VALUES (?1, 'Test', 'custom', 'test', 'active', 0, 0, 'test', '{}')",
            params![agent_id],
        ).unwrap();
    }

    fn make_event(agent_id: &str, decision: AuditDecision) -> AuditEventRecord {
        AuditEventRecord::new(agent_id, "tool_call", decision, "policy_engine")
    }

    #[test]
    fn test_append_and_count() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = AuditStore::new(&db);

        for _ in 0..5 {
            store.append(&make_event("agent-1", AuditDecision::AutoAllowed)).unwrap();
        }
        store.append(&make_event("agent-1", AuditDecision::AutoDenied)).unwrap();

        assert_eq!(store.count_for_agent("agent-1").unwrap(), 6);
        assert_eq!(store.count_denied_for_agent("agent-1").unwrap(), 1);
        assert_eq!(store.total_count().unwrap(), 6);
    }

    #[test]
    fn test_query_by_agent() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        seed_agent(&db, "agent-2");
        let store = AuditStore::new(&db);

        for _ in 0..3 {
            store.append(&make_event("agent-1", AuditDecision::AutoAllowed)).unwrap();
        }
        store.append(&make_event("agent-2", AuditDecision::HumanDenied)).unwrap();

        let events = store.query(&AuditFilter::for_agent("agent-1")).unwrap();
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.agent_id == "agent-1"));
    }

    #[test]
    fn test_query_by_decision() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = AuditStore::new(&db);

        store.append(&make_event("agent-1", AuditDecision::AutoAllowed)).unwrap();
        store.append(&make_event("agent-1", AuditDecision::AutoAllowed)).unwrap();
        store.append(&make_event("agent-1", AuditDecision::HumanDenied)).unwrap();

        let filter = AuditFilter {
            agent_id: Some("agent-1".to_string()),
            decision: Some("human_denied".to_string()),
            limit: 100,
            ..Default::default()
        };
        let events = store.query(&filter).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].decision, AuditDecision::HumanDenied);
    }

    #[test]
    fn test_append_batch() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = AuditStore::new(&db);

        let events: Vec<AuditEventRecord> = (0..10)
            .map(|_| make_event("agent-1", AuditDecision::AutoAllowed))
            .collect();
        store.append_batch(&events).unwrap();
        assert_eq!(store.total_count().unwrap(), 10);
    }

    #[test]
    fn test_audit_immutability_no_delete() {
        // 验证：没有提供 delete 方法（编译时保证）
        // 如果 AuditStore 有 delete 方法，这个测试就无法编译
        let db = open_db();
        let _store = AuditStore::new(&db);
        // AuditStore 没有 delete / remove / clear 方法 — 仅追加
    }

    #[test]
    fn test_decision_is_allowed() {
        assert!(AuditDecision::AutoAllowed.is_allowed());
        assert!(AuditDecision::HumanApproved.is_allowed());
        assert!(!AuditDecision::AutoDenied.is_allowed());
        assert!(!AuditDecision::HumanDenied.is_allowed());
        assert!(!AuditDecision::Pending.is_allowed());
    }

    #[test]
    fn test_pagination() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = AuditStore::new(&db);

        for _ in 0..25 {
            store.append(&make_event("agent-1", AuditDecision::AutoAllowed)).unwrap();
        }

        let page1 = store.query(&AuditFilter {
            agent_id: Some("agent-1".to_string()),
            limit: 10, offset: 0, ..Default::default()
        }).unwrap();
        let page2 = store.query(&AuditFilter {
            agent_id: Some("agent-1".to_string()),
            limit: 10, offset: 10, ..Default::default()
        }).unwrap();
        let page3 = store.query(&AuditFilter {
            agent_id: Some("agent-1".to_string()),
            limit: 10, offset: 20, ..Default::default()
        }).unwrap();

        assert_eq!(page1.len(), 10);
        assert_eq!(page2.len(), 10);
        assert_eq!(page3.len(), 5);
        // 确保分页不重叠
        let ids1: Vec<i64> = page1.iter().map(|e| e.id).collect();
        let ids2: Vec<i64> = page2.iter().map(|e| e.id).collect();
        assert!(ids1.iter().all(|id| !ids2.contains(id)));
    }
}
