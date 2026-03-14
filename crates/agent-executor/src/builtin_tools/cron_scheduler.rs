//! Cron scheduler service.
//!
//! Provides background scheduling and execution of cron jobs.
//! Integrates with the persistent cron store and handles job lifecycle.

use anyhow::Result;
#[cfg(feature = "cron")]
use chrono::DateTime;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

#[cfg(feature = "cron")]
use openclaw_store::cron as store_cron;

/// Callback type for executing a cron job goal.
/// Arguments: (job_id, goal) -> Result<String>
pub type GoalExecutor = Arc<dyn Fn(String, String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> + Send + Sync>;

/// Cron scheduler configuration.
#[derive(Debug, Clone)]
pub struct CronSchedulerConfig {
    /// Enable/disable the scheduler.
    pub enabled: bool,
    /// Check interval in seconds.
    pub check_interval_secs: u64,
    /// Largest number of concurrent job executions.
    pub max_concurrent_jobs: usize,
    /// Job timeout in seconds.
    pub job_timeout_secs: u64,
    /// Whether to clean up old execution records.
    pub cleanup_old_records: bool,
    /// Days to keep execution records.
    pub retention_days: i64,
}

impl Default for CronSchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 60, // Check every minute
            max_concurrent_jobs: 5,
            job_timeout_secs: 300, // 5 minutes
            cleanup_old_records: true,
            retention_days: 30,
        }
    }
}

/// Cron scheduler service.
pub struct CronScheduler {
    /// Configuration.
    config: Arc<RwLock<CronSchedulerConfig>>,
    /// Running state.
    running: Arc<AtomicBool>,
    /// Database connection pool (optional).
    #[cfg(feature = "cron")]
    db_pool: Option<sqlx::SqlitePool>,
    /// Running jobs.
    running_jobs: Arc<RwLock<std::collections::HashSet<String>>>,
    /// Optional callback for executing job goals.
    goal_executor: Option<GoalExecutor>,
}

impl CronScheduler {
    /// Create a new cron scheduler.
    pub fn new(config: CronSchedulerConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            running: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "cron")]
            db_pool: None,
            running_jobs: Arc::new(RwLock::new(std::collections::HashSet::new())),
            goal_executor: None,
        }
    }

    /// Create a new scheduler with database pool.
    #[cfg(feature = "cron")]
    pub fn with_db(config: CronSchedulerConfig, pool: sqlx::SqlitePool) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            running: Arc::new(AtomicBool::new(false)),
            db_pool: Some(pool),
            running_jobs: Arc::new(RwLock::new(std::collections::HashSet::new())),
            goal_executor: None,
        }
    }

    /// Attach a goal executor callback for running job goals.
    pub fn with_goal_executor(mut self, executor: GoalExecutor) -> Self {
        self.goal_executor = Some(executor);
        self
    }

    /// Start the cron scheduler.
    pub async fn start(&self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            warn!("Cron scheduler is already running");
            return Ok(());
        }

        let config = self.config.read().await;
        if !config.enabled {
            info!("Cron scheduler is turned off");
            return Ok(());
        }

        self.running.store(true, Ordering::Relaxed);
        info!("Starting cron scheduler with check interval {}s", config.check_interval_secs);

        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run().await;
        });

        Ok(())
    }

    /// Stop the cron scheduler.
    pub async fn stop(&self) {
        if !self.running.load(Ordering::Relaxed) {
            return;
        }

        info!("Stopping cron scheduler");
        self.running.store(false, Ordering::Relaxed);
    }

    /// Main scheduler loop.
    async fn run(&self) {
        let mut interval = interval(Duration::from_secs(
            self.config.read().await.check_interval_secs,
        ));

        while self.running.load(Ordering::Relaxed) {
            interval.tick().await;

            if let Err(e) = self.check_and_run_jobs().await {
                error!("Error in cron scheduler: {}", e);
            }

            // Periodic cleanup
            if let Err(e) = self.cleanup_if_needed().await {
                warn!("Cleanup error: {}", e);
            }
        }

        info!("Cron scheduler stopped");
    }

    /// Check for due jobs and execute them.
    async fn check_and_run_jobs(&self) -> Result<()> {
        #[cfg(feature = "cron")]
        {
            if let Some(pool) = &self.db_pool {
                let store = store_cron::CronStore::new(pool.clone()).await?;
                
                // Get due jobs
                let due_jobs = store.get_due_jobs().await?;
                
                for job in due_jobs {
                    // Check concurrency limit
                    {
                        let running = self.running_jobs.read().await;
                        if running.len() >= self.config.read().await.max_concurrent_jobs {
                            debug!("Max concurrent jobs reached, skipping job: {}", job.id);
                            continue;
                        }
                    }

                    // Mark job as running
                    {
                        let mut running = self.running_jobs.write().await;
                        running.insert(job.id.clone());
                    }

                    // Execute job asynchronously
                    let scheduler = self.clone();
                    let job_id = job.id.clone();
                    tokio::spawn(async move {
                        if let Err(e) = scheduler.execute_job(&job).await {
                            error!("Failed to execute job {}: {}", job_id, e);
                        }
                        
                        // Mark job as no longer running
                        let mut running = scheduler.running_jobs.write().await;
                        running.remove(&job_id);
                    });
                }
            }
        }

        Ok(())
    }

    /// Execute a single cron job.
    #[cfg(feature = "cron")]
    async fn execute_job(&self, job: &store_cron::CronJob) -> Result<()> {
        let start_time = Utc::now();
        let execution_id = format!("exec-{}-{}", job.id, start_time.timestamp_millis());
        
        info!("Executing cron job: {} ({})", job.name, job.id);

        // Create execution record
        let mut execution = store_cron::CronExecution::new(job.id.clone(), "running".to_string());
        execution.id = execution_id.clone();

        let result = match self.run_job_goal(&job.id, &job.goal).await {
            Ok(output) => {
                execution.status = "success".to_string();
                execution.result = Some(output);
                Ok(())
            }
            Err(e) => {
                execution.status = "error".to_string();
                execution.result = Some(e.to_string());
                Err(e)
            }
        };

        // Calculate duration
        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;
        execution.duration_ms = Some(duration_ms as i64);

        // Record execution
        if let Some(pool) = &self.db_pool {
            let store = store_cron::CronStore::new(pool.clone()).await?;
            
            if let Err(e) = store.record_execution(&execution).await {
                error!("Failed to record execution for job {}: {}", job.id, e);
            }

            // Update next run time
            if let Ok(next_run) = self.calculate_next_run(&job.schedule, &end_time) {
                let updates = store_cron::CronJobUpdate {
                    next_run: Some(next_run),
                    ..Default::default()
                };
                
                if let Err(e) = store.update(&job.id, &updates).await {
                    warn!("Failed to update next run time for job {}: {}", job.id, e);
                }
            }
        }

        match &result {
            Ok(_) => info!("Cron job {} completed successfully in {}ms", job.id, duration_ms),
            Err(e) => error!("Cron job {} failed: {}", job.id, e),
        }

        result
    }

    /// Run the job goal via registered executor or log-only fallback.
    #[allow(dead_code)]
    async fn run_job_goal(&self, job_id: &str, goal: &str) -> Result<String> {
        debug!("Executing goal for job {}: {}", job_id, goal);

        if let Some(executor) = &self.goal_executor {
            let fut = executor(job_id.to_string(), goal.to_string());
            return fut.await;
        }

        // No executor registered — record intent only
        info!("[cron] job {} goal scheduled (no executor attached): {}", job_id, goal);
        Ok(format!("scheduled: {}", goal))
    }

    /// Calculate next run time based on cron expression.
    #[cfg(feature = "cron")]
    fn calculate_next_run(&self, cron_expr: &str, after: &DateTime<Utc>) -> Result<String> {
        // Handle common shorthand aliases before passing to the cron parser
        let normalized = match cron_expr.trim() {
            "@yearly" | "@annually" => "0 0 1 1 *",
            "@monthly"              => "0 0 1 * *",
            "@weekly"               => "0 0 * * 0",
            "@daily" | "@midnight"  => "0 0 * * *",
            "@hourly"               => "0 * * * *",
            other                   => other,
        };

        #[cfg(feature = "cron")]
        {
            use cron::Schedule;
            use std::str::FromStr;

            match Schedule::from_str(&format!("0 {} *", normalized)) {
                Ok(schedule) => {
                    if let Some(next) = schedule.after(after).next() {
                        return Ok(next.to_rfc3339());
                    }
                }
                Err(_) => {
                    // Try parsing as-is (some expressions already include seconds field)
                    if let Ok(schedule) = Schedule::from_str(normalized) {
                        if let Some(next) = schedule.after(after).next() {
                            return Ok(next.to_rfc3339());
                        }
                    }
                }
            }
        }

        // Fallback: +1 hour if expression cannot be parsed
        warn!("Could not parse cron expression '{}', using +1h fallback", cron_expr);
        Ok((*after + chrono::Duration::hours(1)).to_rfc3339())
    }

    /// Clean up old execution records if needed.
    async fn cleanup_if_needed(&self) -> Result<()> {
        let config = self.config.read().await;
        
        if !config.cleanup_old_records {
            return Ok(());
        }

        // Run cleanup once per day
        static LAST_CLEANUP: std::sync::LazyLock<std::sync::Mutex<Option<chrono::DateTime<Utc>>>> = 
            std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

        let now = Utc::now();
        let should_cleanup = {
            let mut last_cleanup = LAST_CLEANUP.lock().unwrap();
            match *last_cleanup {
                None => {
                    *last_cleanup = Some(now);
                    true
                }
                Some(last) => {
                    if (now - last).num_hours() >= 24 {
                        *last_cleanup = Some(now);
                        true
                    } else {
                        false
                    }
                }
            }
        };

        if should_cleanup {
            #[cfg(feature = "cron")]
            if let Some(pool) = &self.db_pool {
                let store = store_cron::CronStore::new(pool.clone()).await?;
                let deleted = store.cleanup_executions(config.retention_days).await?;
                if deleted > 0 {
                    info!("Cleaned up {} old cron execution records", deleted);
                }
            }
        }

        Ok(())
    }

    /// Update scheduler configuration.
    pub async fn update_config(&self, new_config: CronSchedulerConfig) {
        let (was_enabled, interval_changed, now_enabled) = {
            let mut config = self.config.write().await;
            let was_enabled = config.enabled;
            let interval_changed = config.check_interval_secs != new_config.check_interval_secs;
            let now_enabled = new_config.enabled;
            *config = new_config;
            (was_enabled, interval_changed, now_enabled)
            // write lock drops here
        };

        let is_running = self.running.load(Ordering::Relaxed);

        // Restart scheduler if interval changed and it's running
        if interval_changed && is_running {
            info!("Cron scheduler interval changed, restarting scheduler");
            self.stop().await;
            if now_enabled {
                if let Err(e) = self.start().await {
                    error!("Failed to restart cron scheduler: {}", e);
                }
            }
        } else if !was_enabled && now_enabled && !is_running {
            // Start if newly enabled
            if let Err(e) = self.start().await {
                error!("Failed to start cron scheduler: {}", e);
            }
        } else if was_enabled && !now_enabled {
            // Stop if newly turned off
            self.stop().await;
        }
    }

    /// Get current configuration.
    pub async fn get_config(&self) -> CronSchedulerConfig {
        self.config.read().await.clone()
    }

    /// Check if scheduler is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get running job count.
    pub async fn get_running_job_count(&self) -> usize {
        self.running_jobs.read().await.len()
    }

    /// Trigger a specific job to run immediately.
    #[allow(unused_variables)]
    pub async fn trigger_job(&self, job_id: &str) -> Result<()> {
        #[cfg(feature = "cron")]
        {
            if let Some(pool) = &self.db_pool {
                let store = store_cron::CronStore::new(pool.clone()).await?;
                
                if let Some(job) = store.get(job_id).await? {
                    // Check if already running
                    {
                        let running = self.running_jobs.read().await;
                        if running.contains(job_id) {
                            return Err(anyhow::anyhow!("Job {} is already running", job_id));
                        }
                    }

                    // Mark as running
                    self.running_jobs.write().await.insert(job_id.to_string());
                    
                    // Execute job
                    let result = self.execute_job(&job).await;
                    
                    // Mark as not running
                    self.running_jobs.write().await.remove(job_id);
                    
                    result
                } else {
                    Err(anyhow::anyhow!("Job {} not found", job_id))
                }
            } else {
                Err(anyhow::anyhow!("Database not available"))
            }
        }
        #[cfg(not(feature = "cron"))]
        {
            Err(anyhow::anyhow!("Cron feature not enabled"))
        }
    }
}

impl Clone for CronScheduler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            running: self.running.clone(),
            #[cfg(feature = "cron")]
            db_pool: self.db_pool.clone(),
            running_jobs: self.running_jobs.clone(),
            goal_executor: self.goal_executor.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_scheduler() -> CronScheduler {
        let config = CronSchedulerConfig {
            enabled: false, // Turned off for tests
            check_interval_secs: 1,
            max_concurrent_jobs: 2,
            job_timeout_secs: 10,
            cleanup_old_records: false,
            retention_days: 7,
        };
        CronScheduler::new(config)
    }

    #[tokio::test]
    async fn test_scheduler_config() {
        let scheduler = create_test_scheduler().await;
        
        let config = scheduler.get_config().await;
        assert!(!config.enabled);
        assert_eq!(config.check_interval_secs, 1);
        assert_eq!(config.max_concurrent_jobs, 2);
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let scheduler = create_test_scheduler().await;

        // Should not start when disabled
        scheduler.start().await.unwrap();
        assert!(!scheduler.is_running());

        // Manually enable via config write (bypass auto-start in update_config)
        {
            let mut cfg = scheduler.config.write().await;
            cfg.enabled = true;
        }

        // Now start manually
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());

        // Stop immediately
        scheduler.stop().await;
        // Give the spawned task a moment to observe the flag
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_running_job_count() {
        let scheduler = create_test_scheduler().await;
        
        let count = scheduler.get_running_job_count().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    #[cfg(feature = "cron")]
    async fn test_trigger_job() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let config = CronSchedulerConfig::default();
        let scheduler = CronScheduler::with_db(config, pool);
        
        // This test would need setting up a job in the database first
        // For now, test that the method exists and handles missing jobs
        let result = scheduler.trigger_job("nonexistent-job").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_update() {
        let scheduler = create_test_scheduler().await;

        // Use a config that keeps enabled=false to avoid auto-starting a background task
        let new_config = CronSchedulerConfig {
            enabled: false,
            check_interval_secs: 30,
            max_concurrent_jobs: 5,
            job_timeout_secs: 600,
            cleanup_old_records: true,
            retention_days: 90,
        };

        scheduler.update_config(new_config).await;
        let config = scheduler.get_config().await;

        assert!(!config.enabled);
        assert_eq!(config.check_interval_secs, 30);
        assert_eq!(config.max_concurrent_jobs, 5);
        assert!(config.cleanup_old_records);
        assert_eq!(config.retention_days, 90);
    }
}
