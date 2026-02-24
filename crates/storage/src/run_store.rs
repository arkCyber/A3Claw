//! 任务执行记录（Run/Step）的存储与查询。

use anyhow::{Context, Result};
use rusqlite::params;
use tracing::{debug, info};

use crate::db::Database;
use crate::types::{RunRecord, RunStatus, StepRecord, StepKind, now_unix_secs};

/// 任务执行记录存储操作。
pub struct RunStore<'db> {
    db: &'db Database,
}

impl<'db> RunStore<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    // ── Run CRUD ──────────────────────────────────────────────────────────

    /// 创建新的 Run 记录。
    pub fn create_run(&self, run: &RunRecord) -> Result<()> {
        self.db.conn.execute(
            "INSERT INTO runs (id, agent_id, task_description, started_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                run.id,
                run.agent_id,
                run.task_description,
                run.started_at as i64,
                run.status.as_str(),
            ],
        ).context("create run")?;
        info!(run_id = %run.id, agent_id = %run.agent_id, "Run created");
        Ok(())
    }

    /// 更新 Run 状态（完成/失败/取消）。
    pub fn finish_run(
        &self,
        run_id: &str,
        status: RunStatus,
        summary: Option<&str>,
    ) -> Result<()> {
        let now = now_unix_secs() as i64;
        let rows = self.db.conn.execute(
            "UPDATE runs SET status=?2, finished_at=?3, summary=?4 WHERE id=?1",
            params![run_id, status.as_str(), now, summary],
        ).context("finish run")?;
        if rows == 0 {
            anyhow::bail!("Run not found: {}", run_id);
        }
        debug!(run_id, status = %status, "Run finished");
        Ok(())
    }

    /// 增加 Run 的步骤计数。
    pub fn increment_step_count(&self, run_id: &str) -> Result<()> {
        self.db.conn.execute(
            "UPDATE runs SET step_count = step_count + 1 WHERE id=?1",
            params![run_id],
        ).context("increment step_count")?;
        Ok(())
    }

    /// 增加 Run 的拒绝操作计数。
    pub fn increment_denied(&self, run_id: &str) -> Result<()> {
        self.db.conn.execute(
            "UPDATE runs SET denied_count = denied_count + 1 WHERE id=?1",
            params![run_id],
        ).context("increment denied_count")?;
        Ok(())
    }

    /// 增加 Run 的批准操作计数。
    pub fn increment_approved(&self, run_id: &str) -> Result<()> {
        self.db.conn.execute(
            "UPDATE runs SET approved_count = approved_count + 1 WHERE id=?1",
            params![run_id],
        ).context("increment approved_count")?;
        Ok(())
    }

    /// 按 ID 查询 Run 记录。
    pub fn get_run(&self, run_id: &str) -> Result<Option<RunRecord>> {
        let result = self.db.conn.query_row(
            "SELECT id, agent_id, task_description, started_at, finished_at,
                    status, summary, step_count, denied_count, approved_count
             FROM runs WHERE id=?1",
            params![run_id],
            |r| Ok(RunRecord {
                id:               r.get(0)?,
                agent_id:         r.get(1)?,
                task_description: r.get(2)?,
                started_at:       r.get::<_, i64>(3)? as u64,
                finished_at:      r.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                status:           RunStatus::from_str(&r.get::<_, String>(5)?),
                summary:          r.get(6)?,
                step_count:       r.get::<_, i64>(7)? as u32,
                denied_count:     r.get::<_, i64>(8)? as u32,
                approved_count:   r.get::<_, i64>(9)? as u32,
            }),
        );
        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("get run"),
        }
    }

    /// 列举某个 Agent 的所有 Run（按开始时间降序）。
    pub fn list_runs_for_agent(&self, agent_id: &str, limit: u32) -> Result<Vec<RunRecord>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, agent_id, task_description, started_at, finished_at,
                    status, summary, step_count, denied_count, approved_count
             FROM runs WHERE agent_id=?1 ORDER BY started_at DESC LIMIT ?2"
        ).context("prepare list_runs_for_agent")?;
        let runs = stmt.query_map(params![agent_id, limit as i64], |r| {
            Ok(RunRecord {
                id:               r.get(0)?,
                agent_id:         r.get(1)?,
                task_description: r.get(2)?,
                started_at:       r.get::<_, i64>(3)? as u64,
                finished_at:      r.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                status:           RunStatus::from_str(&r.get::<_, String>(5)?),
                summary:          r.get(6)?,
                step_count:       r.get::<_, i64>(7)? as u32,
                denied_count:     r.get::<_, i64>(8)? as u32,
                approved_count:   r.get::<_, i64>(9)? as u32,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(runs)
    }

    /// 列举所有正在运行的 Run。
    pub fn list_running(&self) -> Result<Vec<RunRecord>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, agent_id, task_description, started_at, finished_at,
                    status, summary, step_count, denied_count, approved_count
             FROM runs WHERE status='running' ORDER BY started_at ASC"
        ).context("prepare list_running")?;
        let runs = stmt.query_map([], |r| {
            Ok(RunRecord {
                id:               r.get(0)?,
                agent_id:         r.get(1)?,
                task_description: r.get(2)?,
                started_at:       r.get::<_, i64>(3)? as u64,
                finished_at:      r.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                status:           RunStatus::from_str(&r.get::<_, String>(5)?),
                summary:          r.get(6)?,
                step_count:       r.get::<_, i64>(7)? as u32,
                denied_count:     r.get::<_, i64>(8)? as u32,
                approved_count:   r.get::<_, i64>(9)? as u32,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(runs)
    }

    /// 返回 Run 总数。
    pub fn count_runs(&self) -> Result<u64> {
        let n: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM runs", [], |r| r.get(0)
        ).context("count runs")?;
        Ok(n as u64)
    }

    // ── Step CRUD ─────────────────────────────────────────────────────────

    /// 插入步骤记录，返回自增 ID。
    pub fn insert_step(&self, step: &StepRecord) -> Result<i64> {
        self.db.conn.execute(
            "INSERT INTO steps (run_id, step_index, kind, description,
                                input_summary, output_summary, started_at,
                                finished_at, success, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                step.run_id,
                step.step_index as i64,
                step.kind.as_str(),
                step.description,
                step.input_summary,
                step.output_summary,
                step.started_at as i64,
                step.finished_at.map(|v| v as i64),
                step.success as i64,
                step.error,
            ],
        ).context("insert step")?;
        let id = self.db.conn.last_insert_rowid();
        // 同步更新 Run 的步骤计数
        self.increment_step_count(&step.run_id)?;
        Ok(id)
    }

    /// 完成步骤（更新结束时间、成功标志、输出摘要）。
    pub fn finish_step(
        &self,
        step_id: i64,
        success: bool,
        output_summary: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let now = now_unix_secs() as i64;
        self.db.conn.execute(
            "UPDATE steps SET finished_at=?2, success=?3, output_summary=?4, error=?5
             WHERE id=?1",
            params![step_id, now, success as i64, output_summary, error],
        ).context("finish step")?;
        Ok(())
    }

    /// 列举某个 Run 的所有步骤（按步骤序号升序）。
    pub fn list_steps_for_run(&self, run_id: &str) -> Result<Vec<StepRecord>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, run_id, step_index, kind, description,
                    input_summary, output_summary, started_at, finished_at, success, error
             FROM steps WHERE run_id=?1 ORDER BY step_index ASC"
        ).context("prepare list_steps_for_run")?;
        let steps = stmt.query_map(params![run_id], |r| {
            Ok(StepRecord {
                id:             r.get(0)?,
                run_id:         r.get(1)?,
                step_index:     r.get::<_, i64>(2)? as u32,
                kind:           StepKind::from_str(&r.get::<_, String>(3)?),
                description:    r.get(4)?,
                input_summary:  r.get(5)?,
                output_summary: r.get(6)?,
                started_at:     r.get::<_, i64>(7)? as u64,
                finished_at:    r.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                success:        r.get::<_, i64>(9)? != 0,
                error:          r.get(10)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
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

    #[test]
    fn test_create_and_get_run() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = RunStore::new(&db);
        let run = RunRecord::new("agent-1", "Process tickets");
        store.create_run(&run).unwrap();
        let loaded = store.get_run(&run.id).unwrap().unwrap();
        assert_eq!(loaded.id, run.id);
        assert_eq!(loaded.task_description, "Process tickets");
        assert_eq!(loaded.status, RunStatus::Running);
    }

    #[test]
    fn test_finish_run() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = RunStore::new(&db);
        let run = RunRecord::new("agent-1", "Test task");
        store.create_run(&run).unwrap();
        store.finish_run(&run.id, RunStatus::Success, Some("Done")).unwrap();
        let loaded = store.get_run(&run.id).unwrap().unwrap();
        assert_eq!(loaded.status, RunStatus::Success);
        assert_eq!(loaded.summary.as_deref(), Some("Done"));
        assert!(loaded.finished_at.is_some());
    }

    #[test]
    fn test_insert_and_list_steps() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = RunStore::new(&db);
        let run = RunRecord::new("agent-1", "Multi-step task");
        store.create_run(&run).unwrap();

        for i in 0..3u32 {
            let step = StepRecord::new(&run.id, i, StepKind::ToolCall, format!("Step {}", i));
            let step_id = store.insert_step(&step).unwrap();
            store.finish_step(step_id, true, Some("ok"), None).unwrap();
        }

        let steps = store.list_steps_for_run(&run.id).unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].step_index, 0);
        assert_eq!(steps[2].step_index, 2);
        assert!(steps[0].success);

        // step_count 应该自动更新
        let loaded_run = store.get_run(&run.id).unwrap().unwrap();
        assert_eq!(loaded_run.step_count, 3);
    }

    #[test]
    fn test_list_runs_for_agent() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        seed_agent(&db, "agent-2");
        let store = RunStore::new(&db);

        for _ in 0..3 {
            store.create_run(&RunRecord::new("agent-1", "task")).unwrap();
        }
        store.create_run(&RunRecord::new("agent-2", "other task")).unwrap();

        let runs = store.list_runs_for_agent("agent-1", 100).unwrap();
        assert_eq!(runs.len(), 3);
        let runs2 = store.list_runs_for_agent("agent-2", 100).unwrap();
        assert_eq!(runs2.len(), 1);
    }

    #[test]
    fn test_list_running() {
        let db = open_db();
        seed_agent(&db, "agent-1");
        let store = RunStore::new(&db);

        let r1 = RunRecord::new("agent-1", "running task");
        let r2 = RunRecord::new("agent-1", "finished task");
        store.create_run(&r1).unwrap();
        store.create_run(&r2).unwrap();
        store.finish_run(&r2.id, RunStatus::Success, None).unwrap();

        let running = store.list_running().unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, r1.id);
    }

    #[test]
    fn test_run_status_terminal() {
        assert!(RunStatus::Success.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(RunStatus::Cancelled.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
        assert!(!RunStatus::PendingApproval.is_terminal());
    }
}
