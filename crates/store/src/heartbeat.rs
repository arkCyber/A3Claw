//! Heartbeat event storage and management.
//!
//! Provides persistent storage for heartbeat events using SQLite.
//! Supports storing, querying, and cleanup of heartbeat data
//! for monitoring and diagnostic purposes.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// Heartbeat event structure matching the macOS implementation.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HeartbeatEvent {
    /// Timestamp in milliseconds since Unix epoch.
    pub ts: f64,
    /// Status string (for example, "ok", "error", "warning").
    pub status: String,
    /// Target agent or session identifier (optional).
    pub to: Option<String>,
    /// Preview message or summary (optional).
    pub preview: Option<String>,
    /// Duration in milliseconds (optional).
    pub duration_ms: Option<f64>,
    /// Whether the event has media attachments (optional).
    pub has_media: Option<bool>,
    /// Reason or error message (optional).
    pub reason: Option<String>,
}

impl HeartbeatEvent {
    /// Create a new heartbeat event.
    pub fn new(status: &str) -> Self {
        Self {
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

    /// Create a heartbeat with duration.
    pub fn with_duration(status: &str, duration_ms: f64) -> Self {
        let mut hb = Self::new(status);
        hb.duration_ms = Some(duration_ms);
        hb
    }
}

/// Persistent heartbeat storage.
#[derive(Clone)]
pub struct HeartbeatStore {
    pool: SqlitePool,
}

impl HeartbeatStore {
    /// Create a new heartbeat store.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let store = Self { pool };
        store.init_table().await?;
        Ok(store)
    }

    /// Initialize the heartbeat table.
    async fn init_table(&self) -> Result<()> {
        query(
            r#"
            CREATE TABLE IF NOT EXISTS heartbeat_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts REAL NOT NULL,
                status TEXT NOT NULL,
                "to" TEXT,
                preview TEXT,
                duration_ms REAL,
                has_media BOOLEAN,
                reason TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS idx_heartbeat_ts ON heartbeat_events(ts);
            CREATE INDEX IF NOT EXISTS idx_heartbeat_status ON heartbeat_events(status);
            "#,
        )
        .execute(&self.pool)
        .await?;

        info!("Heartbeat table initialized");
        Ok(())
    }

    /// Store a heartbeat event.
    pub async fn store(&self, event: &HeartbeatEvent) -> Result<()> {
        debug!("Storing heartbeat event: status={}", event.status);

        query(
            r#"
            INSERT INTO heartbeat_events (ts, status, "to", preview, duration_ms, has_media, reason)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.ts)
        .bind(&event.status)
        .bind(&event.to)
        .bind(&event.preview)
        .bind(event.duration_ms)
        .bind(event.has_media)
        .bind(&event.reason)
        .execute(&self.pool)
        .await?;

        debug!("Heartbeat event stored successfully");
        Ok(())
    }

    /// Get the most recent heartbeat event.
    pub async fn get_last(&self) -> Result<Option<HeartbeatEvent>> {
        let row = query_as(
            r#"
            SELECT ts, status, "to", preview, duration_ms, has_media, reason
            FROM heartbeat_events
            ORDER BY ts DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Get heartbeat events within a time range.
    pub async fn get_range(&self, start_ts: f64, end_ts: f64) -> Result<Vec<HeartbeatEvent>> {
        let events = query_as(
            r#"
            SELECT ts, status, "to", preview, duration_ms, has_media, reason
            FROM heartbeat_events
            WHERE ts BETWEEN ? AND ?
            ORDER BY ts DESC
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get recent heartbeat events (last N).
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<HeartbeatEvent>> {
        let events = query_as(
            r#"
            SELECT ts, status, "to", preview, duration_ms, has_media, reason
            FROM heartbeat_events
            ORDER BY ts DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Clean up old heartbeat events (older than specified timestamp).
    pub async fn cleanup_old(&self, before_ts: f64) -> Result<u64> {
        let result = query("DELETE FROM heartbeat_events WHERE ts < ?")
            .bind(before_ts)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            info!("Cleaned up {} old heartbeat events", deleted);
        }

        Ok(deleted)
    }

    /// Get heartbeat statistics.
    pub async fn get_stats(&self) -> Result<HeartbeatStats> {
        let row = query(
            r#"
            SELECT 
                COUNT(*) as total_count,
                COUNT(CASE WHEN status = 'ok' THEN 1 END) as ok_count,
                COUNT(CASE WHEN status = 'error' THEN 1 END) as error_count,
                MAX(ts) as last_ts,
                MIN(ts) as first_ts
            FROM heartbeat_events
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(HeartbeatStats {
            total_count: row.get("total_count"),
            ok_count: row.get("ok_count"),
            error_count: row.get("error_count"),
            last_ts: row.get("last_ts"),
            first_ts: row.get("first_ts"),
        })
    }
}

/// Heartbeat statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatStats {
    pub total_count: i64,
    pub ok_count: i64,
    pub error_count: i64,
    pub last_ts: Option<f64>,
    pub first_ts: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn create_test_store() -> HeartbeatStore {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        HeartbeatStore::new(pool).await.unwrap()
    }

    #[tokio::test]
    async fn test_store_and_retrieve_heartbeat() {
        let store = create_test_store().await;
        
        let event = HeartbeatEvent::with_preview("ok", "Test heartbeat");
        store.store(&event).await.unwrap();

        let retrieved = store.get_last().await.unwrap();
        assert!(retrieved.is_some());
        
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.status, "ok");
        assert_eq!(retrieved.preview, Some("Test heartbeat".to_string()));
    }

    #[tokio::test]
    async fn test_get_recent_heartbeats() {
        let store = create_test_store().await;
        
        // Store many events
        for i in 0..5 {
            let event = HeartbeatEvent::with_preview(&format!("status_{}", i), &format!("Event {}", i));
            store.store(&event).await.unwrap();
        }

        let recent = store.get_recent(3).await.unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].preview, Some("Event 4".to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_old_events() {
        let store = create_test_store().await;
        
        let old_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64 - 86400_000.0; // 24 hours ago
        
        let old_event = HeartbeatEvent::new("old");
        let mut old_event_with_ts = old_event;
        old_event_with_ts.ts = old_ts;
        
        let new_event = HeartbeatEvent::new("new");
        
        store.store(&old_event_with_ts).await.unwrap();
        store.store(&new_event).await.unwrap();

        let deleted = store.cleanup_old(old_ts + 1000.0).await.unwrap();
        assert_eq!(deleted, 1);

        let remaining = store.get_recent(10).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].status, "new");
    }
}
