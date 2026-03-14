//! Persistent memory store — SQLite-backed key/value + conversation history.
//!
//! Provides two tables:
//! - `agent_memory`       — per-agent key/value pairs (survive across runs)
//! - `agent_conversations` — conversation history entries (ordered by timestamp)
//!
//! # Feature flag
//! This module is compiled unconditionally (no feature gate required) because
//! memory persistence is a core capability, not an optional one.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{query, Row, SqlitePool};

// ── MemoryEntry ───────────────────────────────────────────────────────────────

/// A single key/value memory entry for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEntry {
    /// Agent ID that owns this entry.
    pub agent_id: String,
    /// Key (e.g. `"user_name"`, `"last_topic"`).
    pub key: String,
    /// Stored value (plain text or JSON string).
    pub value: String,
    /// ISO-8601 creation time.
    pub created_at: String,
    /// ISO-8601 last-updated time.
    pub updated_at: String,
}

// ── ConversationEntry ─────────────────────────────────────────────────────────

/// One message in a stored conversation history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationEntry {
    pub id: i64,
    /// Agent ID.
    pub agent_id: String,
    /// Session / run ID.
    pub session_id: String,
    /// Role: `"user"`, `"assistant"`, `"system"`, `"tool"`.
    pub role: String,
    /// Message content.
    pub content: String,
    /// Optional tool call ID (for tool-result messages).
    pub tool_call_id: Option<String>,
    /// ISO-8601 timestamp.
    pub created_at: String,
}

// ── MemoryStore ───────────────────────────────────────────────────────────────

/// SQLite-backed persistent memory for agents.
pub struct MemoryStore {
    pool: SqlitePool,
}

impl MemoryStore {
    /// Create a new store and initialise the schema.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let store = Self { pool };
        store.init().await?;
        Ok(store)
    }

    /// Create schema tables if they don't exist.
    async fn init(&self) -> Result<()> {
        query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_memory (
                agent_id   TEXT NOT NULL,
                key        TEXT NOT NULL,
                value      TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (agent_id, key)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_conversations (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id     TEXT NOT NULL,
                session_id   TEXT NOT NULL,
                role         TEXT NOT NULL,
                content      TEXT NOT NULL,
                tool_call_id TEXT,
                created_at   TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_conv_agent_session
                ON agent_conversations (agent_id, session_id);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ── Key/value memory ──────────────────────────────────────────────────

    /// Get a memory value by key.  Returns `None` if not set.
    pub async fn get(&self, agent_id: &str, key: &str) -> Result<Option<String>> {
        let row = query(
            "SELECT value FROM agent_memory WHERE agent_id = ? AND key = ?",
        )
        .bind(agent_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    /// Set (upsert) a memory value.
    pub async fn set(&self, agent_id: &str, key: &str, value: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        query(
            r#"
            INSERT INTO agent_memory (agent_id, key, value, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (agent_id, key)
            DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(agent_id)
        .bind(key)
        .bind(value)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a memory key.  Returns `true` if a row was removed.
    pub async fn delete(&self, agent_id: &str, key: &str) -> Result<bool> {
        let r = query("DELETE FROM agent_memory WHERE agent_id = ? AND key = ?")
            .bind(agent_id)
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /// List all memory entries for an agent.
    pub async fn list(&self, agent_id: &str) -> Result<Vec<MemoryEntry>> {
        let rows = query(
            "SELECT agent_id, key, value, created_at, updated_at \
             FROM agent_memory WHERE agent_id = ? ORDER BY key",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MemoryEntry {
                agent_id: r.get("agent_id"),
                key: r.get("key"),
                value: r.get("value"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Clear all memory for an agent.
    pub async fn clear(&self, agent_id: &str) -> Result<u64> {
        let r = query("DELETE FROM agent_memory WHERE agent_id = ?")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected())
    }

    /// Load all memory entries for an agent as a `HashMap<key, value>`.
    pub async fn load_all(&self, agent_id: &str) -> Result<std::collections::HashMap<String, String>> {
        let entries = self.list(agent_id).await?;
        Ok(entries.into_iter().map(|e| (e.key, e.value)).collect())
    }

    /// Bulk-save a `HashMap<key, value>` for an agent (upsert each entry).
    pub async fn save_all(
        &self,
        agent_id: &str,
        entries: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        for (key, value) in entries {
            self.set(agent_id, key, value).await?;
        }
        Ok(())
    }

    // ── Conversation history ──────────────────────────────────────────────

    /// Append a message to the conversation history.
    pub async fn append_message(
        &self,
        agent_id: &str,
        session_id: &str,
        role: &str,
        content: &str,
        tool_call_id: Option<&str>,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let r = query(
            r#"
            INSERT INTO agent_conversations (agent_id, session_id, role, content, tool_call_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(agent_id)
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(tool_call_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(r.last_insert_rowid())
    }

    /// Load conversation history for a session (ordered oldest-first).
    pub async fn load_conversation(
        &self,
        agent_id: &str,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<ConversationEntry>> {
        let rows = query(
            r#"
            SELECT id, agent_id, session_id, role, content, tool_call_id, created_at
            FROM agent_conversations
            WHERE agent_id = ? AND session_id = ?
            ORDER BY id ASC
            LIMIT ?
            "#,
        )
        .bind(agent_id)
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ConversationEntry {
                id: r.get("id"),
                agent_id: r.get("agent_id"),
                session_id: r.get("session_id"),
                role: r.get("role"),
                content: r.get("content"),
                tool_call_id: r.get("tool_call_id"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    /// Count messages in a session.
    pub async fn conversation_count(&self, agent_id: &str, session_id: &str) -> Result<i64> {
        let row = query(
            "SELECT COUNT(*) as cnt FROM agent_conversations WHERE agent_id = ? AND session_id = ?",
        )
        .bind(agent_id)
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<i64, _>("cnt"))
    }

    /// Delete all messages for a session.
    pub async fn clear_conversation(&self, agent_id: &str, session_id: &str) -> Result<u64> {
        let r = query(
            "DELETE FROM agent_conversations WHERE agent_id = ? AND session_id = ?",
        )
        .bind(agent_id)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(r.rows_affected())
    }

    /// List all session IDs that have conversation history for an agent.
    pub async fn list_sessions(&self, agent_id: &str) -> Result<Vec<String>> {
        let rows = query(
            "SELECT DISTINCT session_id FROM agent_conversations WHERE agent_id = ? ORDER BY session_id",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.get::<String, _>("session_id")).collect())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn make_store() -> MemoryStore {
        let pool = SqlitePool::connect(":memory:").await.expect("in-memory db");
        MemoryStore::new(pool).await.expect("store init")
    }

    // ── Key/value memory ──────────────────────────────────────────────────

    #[tokio::test]
    async fn set_and_get_roundtrip() {
        let store = make_store().await;
        store.set("agent-1", "name", "Alice").await.unwrap();
        let val = store.get("agent-1", "name").await.unwrap();
        assert_eq!(val, Some("Alice".to_string()));
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let store = make_store().await;
        let val = store.get("agent-1", "nonexistent").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn set_updates_existing_key() {
        let store = make_store().await;
        store.set("agent-1", "mood", "happy").await.unwrap();
        store.set("agent-1", "mood", "excited").await.unwrap();
        let val = store.get("agent-1", "mood").await.unwrap();
        assert_eq!(val, Some("excited".to_string()));
    }

    #[tokio::test]
    async fn delete_removes_key() {
        let store = make_store().await;
        store.set("agent-1", "tmp", "data").await.unwrap();
        let removed = store.delete("agent-1", "tmp").await.unwrap();
        assert!(removed);
        assert!(store.get("agent-1", "tmp").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_returns_false() {
        let store = make_store().await;
        let removed = store.delete("agent-1", "no-such-key").await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn list_returns_all_entries() {
        let store = make_store().await;
        store.set("agent-2", "k1", "v1").await.unwrap();
        store.set("agent-2", "k2", "v2").await.unwrap();
        let entries = store.list("agent-2").await.unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.key == "k1" && e.value == "v1"));
        assert!(entries.iter().any(|e| e.key == "k2" && e.value == "v2"));
    }

    #[tokio::test]
    async fn list_is_scoped_to_agent() {
        let store = make_store().await;
        store.set("agent-A", "shared_key", "A").await.unwrap();
        store.set("agent-B", "shared_key", "B").await.unwrap();
        let a = store.list("agent-A").await.unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].value, "A");
    }

    #[tokio::test]
    async fn clear_removes_all_for_agent() {
        let store = make_store().await;
        store.set("agent-3", "a", "1").await.unwrap();
        store.set("agent-3", "b", "2").await.unwrap();
        let count = store.clear("agent-3").await.unwrap();
        assert_eq!(count, 2);
        assert!(store.list("agent-3").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn load_all_and_save_all_roundtrip() {
        let store = make_store().await;
        let mut map = std::collections::HashMap::new();
        map.insert("lang".to_string(), "Rust".to_string());
        map.insert("version".to_string(), "2024".to_string());
        store.save_all("agent-4", &map).await.unwrap();

        let loaded = store.load_all("agent-4").await.unwrap();
        assert_eq!(loaded.get("lang").map(|s| s.as_str()), Some("Rust"));
        assert_eq!(loaded.get("version").map(|s| s.as_str()), Some("2024"));
    }

    // ── Conversation history ──────────────────────────────────────────────

    #[tokio::test]
    async fn append_and_load_conversation() {
        let store = make_store().await;
        store.append_message("agent-1", "sess-1", "user", "Hello!", None).await.unwrap();
        store.append_message("agent-1", "sess-1", "assistant", "Hi there!", None).await.unwrap();

        let msgs = store.load_conversation("agent-1", "sess-1", 10).await.unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[0].content, "Hello!");
    }

    #[tokio::test]
    async fn conversation_respects_limit() {
        let store = make_store().await;
        for i in 0..5 {
            store.append_message("agent-1", "sess-2", "user", &format!("msg {}", i), None).await.unwrap();
        }
        let msgs = store.load_conversation("agent-1", "sess-2", 3).await.unwrap();
        assert_eq!(msgs.len(), 3);
    }

    #[tokio::test]
    async fn conversation_count() {
        let store = make_store().await;
        store.append_message("agent-1", "sess-3", "user", "a", None).await.unwrap();
        store.append_message("agent-1", "sess-3", "assistant", "b", None).await.unwrap();
        assert_eq!(store.conversation_count("agent-1", "sess-3").await.unwrap(), 2);
    }

    #[tokio::test]
    async fn clear_conversation_removes_messages() {
        let store = make_store().await;
        store.append_message("agent-1", "sess-4", "user", "hi", None).await.unwrap();
        let removed = store.clear_conversation("agent-1", "sess-4").await.unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.conversation_count("agent-1", "sess-4").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn list_sessions() {
        let store = make_store().await;
        store.append_message("agent-1", "sess-A", "user", "m1", None).await.unwrap();
        store.append_message("agent-1", "sess-B", "user", "m2", None).await.unwrap();
        store.append_message("agent-1", "sess-A", "assistant", "m3", None).await.unwrap();

        let sessions = store.list_sessions("agent-1").await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"sess-A".to_string()));
        assert!(sessions.contains(&"sess-B".to_string()));
    }

    #[tokio::test]
    async fn tool_call_id_stored_and_retrieved() {
        let store = make_store().await;
        store.append_message("agent-1", "sess-5", "tool", "file contents", Some("call-42")).await.unwrap();
        let msgs = store.load_conversation("agent-1", "sess-5", 10).await.unwrap();
        assert_eq!(msgs[0].tool_call_id, Some("call-42".to_string()));
    }

    #[tokio::test]
    async fn empty_conversation_returns_empty_vec() {
        let store = make_store().await;
        let msgs = store.load_conversation("agent-x", "sess-x", 50).await.unwrap();
        assert!(msgs.is_empty());
    }
}
