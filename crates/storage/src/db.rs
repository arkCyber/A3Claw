//! SQLite 数据库连接与 schema 初始化。
//!
//! ## Schema 设计原则（航空航天级别）
//! 1. **审计事件不可删除**：`audit_events` 表无 DELETE 权限（应用层保证）
//! 2. **外键约束**：所有关联表启用 FOREIGN KEY
//! 3. **WAL 模式**：Write-Ahead Logging 提升并发读性能
//! 4. **原子迁移**：schema 版本通过 `user_version` pragma 管理

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tracing::info;

/// 当前 schema 版本（每次 schema 变更递增）。
const SCHEMA_VERSION: u32 = 1;

/// 数据库连接句柄（单进程单连接，线程安全由调用方保证）。
pub struct Database {
    pub(crate) conn: Connection,
    pub path: PathBuf,
}

impl Database {
    /// 打开（或创建）数据库文件，并执行 schema 迁移。
    ///
    /// 路径：`~/.openclaw-plus/platform.db`
    pub fn open_default() -> Result<Self> {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".openclaw-plus")
            .join("platform.db");
        Self::open(&path)
    }

    /// 打开指定路径的数据库文件。
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db dir: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("open SQLite: {}", path.display()))?;

        let mut db = Self { conn, path: path.to_path_buf() };
        db.configure()?;
        db.migrate()?;
        info!(path = %db.path.display(), "Database opened");
        Ok(db)
    }

    /// 配置 SQLite pragma（WAL、外键、busy timeout）。
    fn configure(&self) -> Result<()> {
        self.conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -8000;
        ").context("configure SQLite pragmas")?;
        Ok(())
    }

    /// 执行 schema 迁移（幂等）。
    fn migrate(&mut self) -> Result<()> {
        let current_version: u32 = self.conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);

        if current_version >= SCHEMA_VERSION {
            return Ok(());
        }

        info!(from = current_version, to = SCHEMA_VERSION, "Running schema migration");

        // 在事务内执行所有 DDL，保证原子性
        self.conn.execute_batch(Self::schema_v1()).context("apply schema v1")?;
        self.conn.execute_batch(
            &format!("PRAGMA user_version = {}", SCHEMA_VERSION)
        ).context("set user_version")?;

        info!(version = SCHEMA_VERSION, "Schema migration complete");
        Ok(())
    }

    /// Schema v1 DDL。
    fn schema_v1() -> &'static str {
        "
        BEGIN;

        -- ── 数字员工档案表 ────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS agents (
            id           TEXT    PRIMARY KEY NOT NULL,
            display_name TEXT    NOT NULL,
            role         TEXT    NOT NULL DEFAULT 'custom',
            owner        TEXT    NOT NULL DEFAULT '',
            status       TEXT    NOT NULL DEFAULT 'active',
            created_at   INTEGER NOT NULL,
            updated_at   INTEGER NOT NULL,
            created_by   TEXT    NOT NULL DEFAULT '',
            profile_json TEXT    NOT NULL  -- 完整 AgentProfile 的 JSON 序列化
        );

        CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
        CREATE INDEX IF NOT EXISTS idx_agents_owner  ON agents(owner);

        -- ── 任务执行记录表 ────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS runs (
            id               TEXT    PRIMARY KEY NOT NULL,
            agent_id         TEXT    NOT NULL REFERENCES agents(id),
            task_description TEXT    NOT NULL DEFAULT '',
            started_at       INTEGER NOT NULL,
            finished_at      INTEGER,
            status           TEXT    NOT NULL DEFAULT 'running',
            summary          TEXT,
            step_count       INTEGER NOT NULL DEFAULT 0,
            denied_count     INTEGER NOT NULL DEFAULT 0,
            approved_count   INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_runs_agent_id   ON runs(agent_id);
        CREATE INDEX IF NOT EXISTS idx_runs_status     ON runs(status);
        CREATE INDEX IF NOT EXISTS idx_runs_started_at ON runs(started_at DESC);

        -- ── 执行步骤表 ────────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS steps (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id         TEXT    NOT NULL REFERENCES runs(id),
            step_index     INTEGER NOT NULL,
            kind           TEXT    NOT NULL,
            description    TEXT    NOT NULL DEFAULT '',
            input_summary  TEXT,
            output_summary TEXT,
            started_at     INTEGER NOT NULL,
            finished_at    INTEGER,
            success        INTEGER NOT NULL DEFAULT 0,
            error          TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_steps_run_id ON steps(run_id);

        -- ── 不可变审计事件表 ──────────────────────────────────────────────
        -- 注意：应用层不应执行 DELETE 操作，仅追加写入
        CREATE TABLE IF NOT EXISTS audit_events (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id      TEXT    REFERENCES runs(id),
            agent_id    TEXT    NOT NULL,
            step_index  INTEGER,
            event_kind  TEXT    NOT NULL,
            target      TEXT,
            input_hash  TEXT,
            decision    TEXT    NOT NULL DEFAULT 'pending',
            actor       TEXT    NOT NULL DEFAULT 'system',
            reason      TEXT,
            ts          INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_audit_agent_id ON audit_events(agent_id);
        CREATE INDEX IF NOT EXISTS idx_audit_run_id   ON audit_events(run_id);
        CREATE INDEX IF NOT EXISTS idx_audit_ts       ON audit_events(ts DESC);
        CREATE INDEX IF NOT EXISTS idx_audit_decision ON audit_events(decision);

        COMMIT;
        "
    }

    /// 检查数据库健康状态（完整性检查）。
    pub fn health_check(&self) -> Result<()> {
        let result: String = self.conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .context("integrity_check")?;
        if result != "ok" {
            anyhow::bail!("SQLite integrity check failed: {}", result);
        }
        Ok(())
    }

    /// 返回数据库文件大小（字节）。
    pub fn file_size_bytes(&self) -> u64 {
        std::fs::metadata(&self.path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// 执行 WAL checkpoint，将 WAL 文件合并回主数据库文件。
    pub fn checkpoint(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .context("wal_checkpoint")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_test_db() -> Database {
        let dir = tempdir().unwrap();
        Database::open(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn test_open_creates_schema() {
        let db = open_test_db();
        // 验证所有表存在
        for table in &["agents", "runs", "steps", "audit_events"] {
            let count: i64 = db.conn.query_row(
                &format!("SELECT COUNT(*) FROM {}", table),
                [],
                |r| r.get(0),
            ).unwrap();
            assert_eq!(count, 0, "Table {} should exist and be empty", table);
        }
    }

    #[test]
    fn test_schema_version() {
        let db = open_test_db();
        let version: u32 = db.conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_health_check() {
        let db = open_test_db();
        assert!(db.health_check().is_ok());
    }

    #[test]
    fn test_idempotent_open() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        // 第一次打开
        let _db1 = Database::open(&path).unwrap();
        // 第二次打开同一文件（schema 已存在，应幂等）
        let db2 = Database::open(&path).unwrap();
        assert!(db2.health_check().is_ok());
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let db = open_test_db();
        let fk: i64 = db.conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1, "Foreign keys must be enabled");
    }
}
