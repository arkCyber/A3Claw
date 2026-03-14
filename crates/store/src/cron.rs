//! Cron job and execution storage.
//!
//! Provides persistent storage for cron jobs and their execution history
//! using SQLite through sqlx.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{query, Row, SqlitePool};
use tracing::info;

/// Cron job structure with scheduling and execution metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Unique job identifier.
    pub id: String,
    /// Human-readable job name.
    pub name: String,
    /// Cron expression (5-field: minute hour day month weekday).
    pub schedule: String,
    /// Agent goal or task to execute.
    pub goal: String,
    /// Whether the job is enabled.
    pub enabled: bool,
    /// Last execution timestamp (ISO string or null).
    pub last_run: Option<String>,
    /// Next scheduled execution timestamp (ISO string or null).
    pub next_run: Option<String>,
    /// Total execution count.
    pub run_count: i64,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
    /// Job metadata (optional).
    pub metadata: Option<serde_json::Value>,
}

/// Cron job execution history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronExecution {
    /// Unique execution identifier.
    pub id: String,
    /// Associated job identifier.
    pub job_id: String,
    /// Execution timestamp (ISO string).
    pub executed_at: String,
    /// Execution status (running, success, error).
    pub status: String,
    /// Execution result or error message.
    pub result: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: Option<i64>,
}

/// Cron job statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronStats {
    /// Total number of jobs.
    pub total_jobs: i64,
    /// Number of enabled jobs.
    pub enabled_jobs: i64,
    /// Number of disabled jobs.
    pub disabled_jobs: i64,
    /// Total number of executions.
    pub total_executions: i64,
    /// Number of successful executions.
    pub successful_executions: i64,
    /// Number of failed executions.
    pub failed_executions: i64,
}

/// Cron job update structure for partial updates.
#[derive(Debug, Default)]
pub struct CronJobUpdate {
    pub name: Option<String>,
    pub schedule: Option<String>,
    pub goal: Option<String>,
    pub enabled: Option<bool>,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub run_count_increment: bool,
    pub metadata: Option<Option<serde_json::Value>>,
}

/// Cron store for persistent storage.
pub struct CronStore {
    pool: SqlitePool,
}

impl CronStore {
    /// Create a new cron store with database connection pool.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let store = Self { pool };
        store.init().await?;
        Ok(store)
    }

    /// Initialize database schema.
    async fn init(&self) -> Result<()> {
        // Create cron_jobs table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS cron_jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                schedule TEXT NOT NULL,
                goal TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                last_run TEXT,
                next_run TEXT,
                run_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                metadata TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create cron_executions table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS cron_executions (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                executed_at TEXT NOT NULL,
                status TEXT NOT NULL,
                result TEXT,
                duration_ms INTEGER,
                FOREIGN KEY (job_id) REFERENCES cron_jobs (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        query("CREATE INDEX IF NOT EXISTS idx_cron_jobs_enabled ON cron_jobs (enabled)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_cron_jobs_next_run ON cron_jobs (next_run)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_cron_executions_job_id ON cron_executions (job_id)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_cron_executions_executed_at ON cron_executions (executed_at)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store a new cron job.
    pub async fn store(&self, job: &CronJob) -> Result<()> {
        query(
            r#"
            INSERT INTO cron_jobs 
            (id, name, schedule, goal, enabled, last_run, next_run, run_count, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&job.id)
        .bind(&job.name)
        .bind(&job.schedule)
        .bind(&job.goal)
        .bind(job.enabled)
        .bind(&job.last_run)
        .bind(&job.next_run)
        .bind(job.run_count)
        .bind(&job.created_at)
        .bind(&job.updated_at)
        .bind(&job.metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a cron job by ID.
    pub async fn get(&self, job_id: &str) -> Result<Option<CronJob>> {
        let rows = query("SELECT id, name, schedule, goal, enabled, last_run, next_run, run_count, created_at, updated_at, metadata FROM cron_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_all(&self.pool)
            .await?;
        
        if rows.is_empty() {
            return Ok(None);
        }
        
        let row = &rows[0];
        Ok(Some(CronJob {
            id: row.get("id"),
            name: row.get("name"),
            schedule: row.get("schedule"),
            goal: row.get("goal"),
            enabled: row.get("enabled"),
            last_run: row.get("last_run"),
            next_run: row.get("next_run"),
            run_count: row.get("run_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            metadata: row.get("metadata"),
        }))
    }

    /// List all cron jobs.
    pub async fn list(&self) -> Result<Vec<CronJob>> {
        let rows = query("SELECT id, name, schedule, goal, enabled, last_run, next_run, run_count, created_at, updated_at, metadata FROM cron_jobs ORDER BY created_at")
            .fetch_all(&self.pool)
            .await?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(CronJob {
                id: row.get("id"),
                name: row.get("name"),
                schedule: row.get("schedule"),
                goal: row.get("goal"),
                enabled: row.get("enabled"),
                last_run: row.get("last_run"),
                next_run: row.get("next_run"),
                run_count: row.get("run_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                metadata: row.get("metadata"),
            });
        }
        Ok(jobs)
    }

    /// List enabled cron jobs.
    pub async fn list_enabled(&self) -> Result<Vec<CronJob>> {
        let rows = query("SELECT id, name, schedule, goal, enabled, last_run, next_run, run_count, created_at, updated_at, metadata FROM cron_jobs WHERE enabled = 1 ORDER BY created_at")
            .fetch_all(&self.pool)
            .await?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(CronJob {
                id: row.get("id"),
                name: row.get("name"),
                schedule: row.get("schedule"),
                goal: row.get("goal"),
                enabled: row.get("enabled"),
                last_run: row.get("last_run"),
                next_run: row.get("next_run"),
                run_count: row.get("run_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                metadata: row.get("metadata"),
            });
        }
        Ok(jobs)
    }

    /// Get jobs that are due for execution.
    pub async fn get_due_jobs(&self) -> Result<Vec<CronJob>> {
        let now = Utc::now().to_rfc3339();
        let rows = query("SELECT id, name, schedule, goal, enabled, last_run, next_run, run_count, created_at, updated_at, metadata FROM cron_jobs WHERE enabled = 1 AND (next_run IS NULL OR next_run <= ?) ORDER BY next_run")
            .bind(&now)
            .fetch_all(&self.pool)
            .await?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(CronJob {
                id: row.get("id"),
                name: row.get("name"),
                schedule: row.get("schedule"),
                goal: row.get("goal"),
                enabled: row.get("enabled"),
                last_run: row.get("last_run"),
                next_run: row.get("next_run"),
                run_count: row.get("run_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                metadata: row.get("metadata"),
            });
        }
        Ok(jobs)
    }

    /// Update a cron job.
    pub async fn update(&self, job_id: &str, updates: &CronJobUpdate) -> Result<bool> {
        let mut updated = false;
        
        if let Some(name) = &updates.name {
            query("UPDATE cron_jobs SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }
        
        if let Some(schedule) = &updates.schedule {
            query("UPDATE cron_jobs SET schedule = ?, updated_at = ? WHERE id = ?")
                .bind(schedule)
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }
        
        if let Some(goal) = &updates.goal {
            query("UPDATE cron_jobs SET goal = ?, updated_at = ? WHERE id = ?")
                .bind(goal)
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }
        
        if let Some(enabled) = updates.enabled {
            query("UPDATE cron_jobs SET enabled = ?, updated_at = ? WHERE id = ?")
                .bind(enabled)
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }
        
        if let Some(last_run) = &updates.last_run {
            query("UPDATE cron_jobs SET last_run = ?, updated_at = ? WHERE id = ?")
                .bind(last_run)
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        if updates.run_count_increment {
            query("UPDATE cron_jobs SET run_count = run_count + 1, updated_at = ? WHERE id = ?")
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        if let Some(next_run) = &updates.next_run {
            query("UPDATE cron_jobs SET next_run = ?, updated_at = ? WHERE id = ?")
                .bind(next_run)
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }
        
        if let Some(metadata) = &updates.metadata {
            query("UPDATE cron_jobs SET metadata = ?, updated_at = ? WHERE id = ?")
                .bind(metadata)
                .bind(Utc::now().to_rfc3339())
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        Ok(updated)
    }

    /// Drop a cron job.
    pub async fn delete(&self, job_id: &str) -> Result<bool> {
        let result = query("DELETE FROM cron_jobs WHERE id = ?")
            .bind(job_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record job execution.
    pub async fn record_execution(&self, execution: &CronExecution) -> Result<()> {
        query(
            r#"
            INSERT INTO cron_executions 
            (id, job_id, executed_at, status, result, duration_ms)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&execution.id)
        .bind(&execution.job_id)
        .bind(&execution.executed_at)
        .bind(&execution.status)
        .bind(&execution.result)
        .bind(execution.duration_ms)
        .execute(&self.pool)
        .await?;

        // Update job run count and last run
        if execution.status == "success" {
            query("UPDATE cron_jobs SET run_count = run_count + 1, last_run = ?, updated_at = ? WHERE id = ?")
                .bind(&execution.executed_at)
                .bind(Utc::now().to_rfc3339())
                .bind(&execution.job_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Get execution history for a job.
    pub async fn get_executions(&self, job_id: &str, limit: Option<i64>) -> Result<Vec<CronExecution>> {
        let rows = if let Some(limit) = limit {
            query("SELECT id, job_id, executed_at, status, result, duration_ms FROM cron_executions WHERE job_id = ? ORDER BY executed_at DESC LIMIT ?")
                .bind(job_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        } else {
            query("SELECT id, job_id, executed_at, status, result, duration_ms FROM cron_executions WHERE job_id = ? ORDER BY executed_at DESC")
                .bind(job_id)
                .fetch_all(&self.pool)
                .await?
        };

        let mut executions = Vec::new();
        for row in rows {
            executions.push(CronExecution {
                id: row.get("id"),
                job_id: row.get("job_id"),
                executed_at: row.get("executed_at"),
                status: row.get("status"),
                result: row.get("result"),
                duration_ms: row.get("duration_ms"),
            });
        }
        Ok(executions)
    }

    /// Get cron statistics.
    pub async fn get_stats(&self) -> Result<CronStats> {
        let job_rows = query(
            r#"
            SELECT 
                COUNT(*) as total_jobs,
                COUNT(CASE WHEN enabled = 1 THEN 1 END) as enabled_jobs,
                COUNT(CASE WHEN enabled = 0 THEN 1 END) as disabled_jobs
            FROM cron_jobs
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let exec_rows = query(
            r#"
            SELECT 
                COUNT(*) as total_executions,
                COUNT(CASE WHEN status = 'success' THEN 1 END) as successful_executions,
                COUNT(CASE WHEN status = 'error' THEN 1 END) as failed_executions
            FROM cron_executions
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let job_row = job_rows.get(0).unwrap();
        let exec_row = exec_rows.get(0).unwrap();

        Ok(CronStats {
            total_jobs: job_row.get("total_jobs"),
            enabled_jobs: job_row.get("enabled_jobs"),
            disabled_jobs: job_row.get("disabled_jobs"),
            total_executions: exec_row.get("total_executions"),
            successful_executions: exec_row.get("successful_executions"),
            failed_executions: exec_row.get("failed_executions"),
        })
    }

    /// Clean up old execution records.
    pub async fn cleanup_executions(&self, days_to_keep: i64) -> Result<i64> {
        let cutoff = Utc::now()
            .checked_sub_signed(chrono::Duration::days(days_to_keep))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

        let result = query("DELETE FROM cron_executions WHERE executed_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            info!("Cleaned up {} old execution records", deleted);
        }
        
        Ok(deleted as i64)
    }
}

impl CronJob {
    /// Create a new cron job.
    pub fn new(id: String, name: String, schedule: String, goal: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id,
            name,
            schedule,
            goal,
            enabled: true,
            last_run: None,
            next_run: None,
            run_count: 0,
            created_at: now.clone(),
            updated_at: now,
            metadata: None,
        }
    }

    /// Create a new job with metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Increment run count and update last run time.
    pub fn mark_executed(&mut self) {
        self.last_run = Some(Utc::now().to_rfc3339());
        self.run_count += 1;
        self.updated_at = Utc::now().to_rfc3339();
    }
}

impl CronExecution {
    /// Create a new execution record.
    pub fn new(job_id: String, status: String) -> Self {
        Self {
            id: format!("exec-{}-{}", job_id, Utc::now().timestamp_millis()),
            job_id,
            executed_at: Utc::now().to_rfc3339(),
            status,
            result: None,
            duration_ms: None,
        }
    }

    /// Create a new execution with result.
    pub fn with_result(mut self, result: String) -> Self {
        self.result = Some(result);
        self
    }

    /// Create a new execution with duration.
    pub fn with_duration(mut self, duration_ms: i64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn create_test_store() -> CronStore {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        CronStore::new(pool).await.unwrap()
    }

    #[tokio::test]
    async fn test_store_and_retrieve_job() {
        let store = create_test_store().await;
        
        let job = CronJob::new(
            "test-job".to_string(),
            "Test Job".to_string(),
            "0 9 * * *".to_string(),
            "Generate daily report".to_string(),
        );

        store.store(&job).await.unwrap();
        
        let retrieved = store.get("test-job").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "Test Job");
        assert_eq!(retrieved.schedule, "0 9 * * *");
        assert_eq!(retrieved.goal, "Generate daily report");
        assert!(retrieved.enabled);
    }

    #[tokio::test]
    async fn test_list_jobs() {
        let store = create_test_store().await;
        
        // Add many jobs
        for i in 0..3 {
            let job = CronJob::new(
                format!("job-{}", i),
                format!("Job {}", i),
                format!("{} 9 * * *", i),
                format!("Goal {}", i),
            );
            store.store(&job).await.unwrap();
        }
        
        let jobs = store.list().await.unwrap();
        assert_eq!(jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_update_job() {
        let store = create_test_store().await;
        
        let job = CronJob::new(
            "test-job".to_string(),
            "Test Job".to_string(),
            "0 9 * * *".to_string(),
            "Generate daily report".to_string(),
        );

        store.store(&job).await.unwrap();
        
        let updates = CronJobUpdate {
            name: Some("Updated Job".to_string()),
            enabled: Some(false),
            ..Default::default()
        };
        
        let updated = store.update("test-job", &updates).await.unwrap();
        assert!(updated);
        
        let retrieved = store.get("test-job").await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Updated Job");
        assert!(!retrieved.enabled);
    }

    #[tokio::test]
    async fn test_delete_job() {
        let store = create_test_store().await;
        
        let job = CronJob::new(
            "test-job".to_string(),
            "Test Job".to_string(),
            "0 9 * * *".to_string(),
            "Generate daily report".to_string(),
        );

        store.store(&job).await.unwrap();
        
        let deleted = store.delete("test-job").await.unwrap();
        assert!(deleted);
        
        let retrieved = store.get("test-job").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_record_execution() {
        let store = create_test_store().await;
        
        let job = CronJob::new(
            "test-job".to_string(),
            "Test Job".to_string(),
            "0 9 * * *".to_string(),
            "Generate daily report".to_string(),
        );

        store.store(&job).await.unwrap();
        
        let execution = CronExecution::new(
            "test-job".to_string(),
            "success".to_string(),
        ).with_result("Report generated successfully".to_string());
        
        store.record_execution(&execution).await.unwrap();
        
        let executions = store.get_executions("test-job", Some(10)).await.unwrap();
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].status, "success");
        assert_eq!(executions[0].result, Some("Report generated successfully".to_string()));
    }

    #[tokio::test]
    async fn test_get_stats() {
        let store = create_test_store().await;
        
        // Add jobs
        let job1 = CronJob::new(
            "job-1".to_string(),
            "Job 1".to_string(),
            "0 9 * * *".to_string(),
            "Goal 1".to_string(),
        );
        let job2 = CronJob::new(
            "job-2".to_string(),
            "Job 2".to_string(),
            "0 10 * * *".to_string(),
            "Goal 2".to_string(),
        );
        
        store.store(&job1).await.unwrap();
        store.store(&job2).await.unwrap();
        
        // Disable one job
        let updates = CronJobUpdate {
            enabled: Some(false),
            ..Default::default()
        };
        store.update("job-2", &updates).await.unwrap();
        
        // Add executions
        let mut exec1 = CronExecution::new("job-1".to_string(), "success".to_string());
        let mut exec2 = CronExecution::new("job-1".to_string(), "error".to_string());
        
        // Ensure different IDs by modifying them
        exec2.id = "exec-job-1-different".to_string();
        
        store.record_execution(&exec1).await.unwrap();
        store.record_execution(&exec2).await.unwrap();
        
        let stats = store.get_stats().await.unwrap();
        assert_eq!(stats.total_jobs, 2);
        assert_eq!(stats.enabled_jobs, 1);
        assert_eq!(stats.disabled_jobs, 1);
        assert_eq!(stats.total_executions, 2);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.failed_executions, 1);
    }
}
