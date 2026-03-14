//! Heartbeat scheduler service.
//!
//! Provides periodic heartbeat functionality that allows AI agents to
//! do self-checking and proactive monitoring tasks.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

#[cfg(feature = "heartbeat")]
use openclaw_store::heartbeat as store_heartbeat;

/// Heartbeat configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Enable/disable heartbeat scheduler.
    pub enabled: bool,
    /// Heartbeat interval in seconds.
    pub interval_secs: u64,
    /// Largest number of heartbeat events to keep in memory.
    pub max_events: usize,
    /// Whether to persist heartbeats to database.
    pub persist: bool,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 60, // 1 minute default
            max_events: 1000,
            persist: true,
        }
    }
}

/// Heartbeat event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    /// Unique heartbeat ID.
    pub id: String,
    /// Timestamp in milliseconds.
    pub ts: f64,
    /// Status string.
    pub status: String,
    /// Target agent/session (optional).
    pub to: Option<String>,
    /// Preview message (optional).
    pub preview: Option<String>,
    /// Duration in milliseconds (optional).
    pub duration_ms: Option<f64>,
    /// Has media attachments (optional).
    pub has_media: Option<bool>,
    /// Reason/error message (optional).
    pub reason: Option<String>,
}

impl HeartbeatEvent {
    /// Create a new heartbeat event.
    pub fn new(status: &str) -> Self {
        Self {
            id: format!("hb-{}", generate_short_id()),
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as f64,
            status: status.to_string(),
            to: None,
            preview: None,
            duration_ms: None,
            has_media: None,
            reason: None,
        }
    }

    /// Create a heartbeat with preview message.
    pub fn with_preview(status: &str, preview: &str) -> Self {
        let mut hb = Self::new(status);
        hb.preview = Some(preview.to_string());
        hb
    }

    /// Convert to store heartbeat format.
    #[cfg(feature = "heartbeat")]
    pub fn to_store_event(&self) -> store_heartbeat::HeartbeatEvent {
        store_heartbeat::HeartbeatEvent {
            ts: self.ts,
            status: self.status.clone(),
            to: self.to.clone(),
            preview: self.preview.clone(),
            duration_ms: self.duration_ms,
            has_media: self.has_media,
            reason: self.reason.clone(),
        }
    }
}

/// Heartbeat scheduler service.
#[derive(Clone)]
pub struct HeartbeatScheduler {
    /// Configuration.
    config: Arc<RwLock<HeartbeatConfig>>,
    /// In-memory event buffer.
    events: Arc<RwLock<Vec<HeartbeatEvent>>>,
    /// Running state.
    running: Arc<AtomicBool>,
    /// Event counter.
    event_counter: Arc<AtomicU64>,
    /// Database connection pool (optional).
    #[cfg(feature = "heartbeat")]
    db_pool: Option<sqlx::SqlitePool>,
}

impl HeartbeatScheduler {
    /// Create a new heartbeat scheduler.
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            events: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            event_counter: Arc::new(AtomicU64::new(0)),
            #[cfg(feature = "heartbeat")]
            db_pool: None,
        }
    }

    /// Create a new scheduler with database pool.
    #[cfg(feature = "heartbeat")]
    pub fn with_db(config: HeartbeatConfig, pool: sqlx::SqlitePool) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            events: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            event_counter: Arc::new(AtomicU64::new(0)),
            db_pool: Some(pool),
        }
    }

    /// Start the heartbeat scheduler.
    pub async fn start(&self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            warn!("Heartbeat scheduler is already running");
            return Ok(());
        }

        let config = self.config.read().await;
        if !config.enabled {
            info!("Heartbeat scheduler is disabled");
            return Ok(());
        }

        self.running.store(true, Ordering::Relaxed);
        info!("Starting heartbeat scheduler with interval {}s", config.interval_secs);

        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run().await;
        });

        Ok(())
    }

    /// Stop the heartbeat scheduler.
    pub async fn stop(&self) {
        if !self.running.load(Ordering::Relaxed) {
            return;
        }

        info!("Stopping heartbeat scheduler");
        self.running.store(false, Ordering::Relaxed);
    }

    /// Main scheduler loop.
    async fn run(&self) {
        let mut interval = interval(Duration::from_secs(
            self.config.read().await.interval_secs,
        ));

        while self.running.load(Ordering::Relaxed) {
            interval.tick().await;

            if let Err(e) = self.generate_heartbeat().await {
                error!("Failed to generate heartbeat: {}", e);
            }
        }

        info!("Heartbeat scheduler stopped");
    }

    /// Generate a heartbeat event.
    async fn generate_heartbeat(&self) -> Result<()> {
        let event_id = self.event_counter.fetch_add(1, Ordering::Relaxed);
        let event = HeartbeatEvent::with_preview(
            "ok",
            &format!("Heartbeat #{}", event_id),
        );

        debug!("Generated heartbeat: {}", event.id);

        // Store in memory
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
            
            // Trim to max_events
            let max_events = self.config.read().await.max_events;
            if events.len() > max_events {
                let drain_count = events.len() - max_events;
                events.drain(0..drain_count);
            }
        }

        // Persist to database if enabled
        #[cfg(feature = "heartbeat")]
        if self.config.read().await.persist {
            if let Some(pool) = &self.db_pool {
                if let Ok(store) = store_heartbeat::HeartbeatStore::new(pool.clone()).await {
                    if let Err(e) = store.store(&event.to_store_event()).await {
                        warn!("Failed to persist heartbeat to database: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get recent heartbeat events.
    pub async fn get_recent(&self, limit: usize) -> Vec<HeartbeatEvent> {
        let events = self.events.read().await;
        let start = if events.len() > limit { events.len() - limit } else { 0 };
        events[start..].to_vec()
    }

    /// Get the last heartbeat event.
    pub async fn get_last(&self) -> Option<HeartbeatEvent> {
        let events = self.events.read().await;
        events.last().cloned()
    }

    /// Update configuration.
    pub async fn update_config(&self, new_config: HeartbeatConfig) {
        let (was_enabled, interval_changed, now_enabled) = {
            let mut config = self.config.write().await;
            let was_enabled = config.enabled;
            let interval_changed = config.interval_secs != new_config.interval_secs;
            let now_enabled = new_config.enabled;
            *config = new_config;
            (was_enabled, interval_changed, now_enabled)
            // write lock drops here
        };

        let is_running = self.running.load(Ordering::Relaxed);

        // Restart scheduler if interval changed and it's running
        if interval_changed && is_running {
            info!("Heartbeat interval changed, restarting scheduler");
            self.stop().await;
            if now_enabled {
                if let Err(e) = self.start().await {
                    error!("Failed to restart heartbeat scheduler: {}", e);
                }
            }
        } else if !was_enabled && now_enabled && !is_running {
            // Start if newly enabled
            if let Err(e) = self.start().await {
                error!("Failed to start heartbeat scheduler: {}", e);
            }
        } else if was_enabled && !now_enabled {
            // Stop if newly disabled
            self.stop().await;
        }
    }

    /// Get current configuration.
    pub async fn get_config(&self) -> HeartbeatConfig {
        self.config.read().await.clone()
    }

    /// Check if scheduler is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Manually trigger a heartbeat.
    pub async fn trigger_manual(&self, status: &str, preview: Option<&str>) -> Result<()> {
        let event = match preview {
            Some(msg) => HeartbeatEvent::with_preview(status, msg),
            None => HeartbeatEvent::new(status),
        };

        debug!("Manual heartbeat triggered: {}", event.id);

        // Store in memory
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
            
            // Trim to max_events
            let max_events = self.config.read().await.max_events;
            if events.len() > max_events {
                let drain_count = events.len() - max_events;
                events.drain(0..drain_count);
            }
        }

        // Persist to database if enabled
        #[cfg(feature = "heartbeat")]
        if self.config.read().await.persist {
            if let Some(pool) = &self.db_pool {
                if let Ok(store) = store_heartbeat::HeartbeatStore::new(pool.clone()).await {
                    if let Err(e) = store.store(&event.to_store_event()).await {
                        warn!("Failed to persist manual heartbeat to database: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Generate a short unique ID.
fn generate_short_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    format!("{}-{:x}", timestamp, counter)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_scheduler() -> HeartbeatScheduler {
        let config = HeartbeatConfig {
            enabled: false, // Turned off for tests
            interval_secs: 1,
            max_events: 10,
            persist: false,
        };
        HeartbeatScheduler::new(config)
    }

    #[tokio::test]
    async fn test_heartbeat_event_creation() {
        let event = HeartbeatEvent::with_preview("test", "Test message");
        
        assert_eq!(event.status, "test");
        assert_eq!(event.preview, Some("Test message".to_string()));
        assert!(event.ts > 0.0);
        assert!(!event.id.is_empty());
    }

    #[tokio::test]
    async fn test_manual_heartbeat_trigger() {
        let scheduler = create_test_scheduler().await;
        
        scheduler.trigger_manual("manual", Some("Manual test")).await.unwrap();
        
        let events = scheduler.get_recent(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, "manual");
        assert_eq!(events[0].preview, Some("Manual test".to_string()));
    }

    #[tokio::test]
    async fn test_events_trimming() {
        let config = HeartbeatConfig {
            enabled: false,
            interval_secs: 1,
            max_events: 3,
            persist: false,
        };
        let scheduler = HeartbeatScheduler::new(config);
        
        // Add 5 events
        for i in 0..5 {
            scheduler.trigger_manual(&format!("event_{}", i), None).await.unwrap();
        }
        
        let events = scheduler.get_recent(10).await;
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].status, "event_2");
        assert_eq!(events[2].status, "event_4");
    }

    #[tokio::test]
    async fn test_config_update() {
        let scheduler = create_test_scheduler().await;

        // Keep enabled=false to avoid spawning a background task
        let new_config = HeartbeatConfig {
            enabled: false,
            interval_secs: 30,
            max_events: 100,
            persist: true,
        };

        scheduler.update_config(new_config).await;
        let config = scheduler.get_config().await;

        assert!(!config.enabled);
        assert_eq!(config.interval_secs, 30);
        assert_eq!(config.max_events, 100);
        assert!(config.persist);
    }

    #[tokio::test]
    #[cfg(feature = "heartbeat")]
    async fn test_with_database() {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        let config = HeartbeatConfig::default();
        let scheduler = HeartbeatScheduler::with_db(config, pool);
        
        scheduler.trigger_manual("db_test", Some("Database test")).await.unwrap();
        
        let events = scheduler.get_recent(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, "db_test");
    }
}
