use crate::types::SandboxEvent;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

/// A single entry in the security audit log.
///
/// Each entry captures the original [`SandboxEvent`], the policy decision
/// that was made (`"Allow"` or `"Deny: <reason>"`), and an optional
/// description of any manual user action that influenced the outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// The sandbox event that was evaluated.
    pub event: SandboxEvent,
    /// The final policy decision string (e.g. `"Allow"`, `"Deny: outside workspace"`).
    pub decision: String,
    /// Optional human-readable note describing a manual user action
    /// (e.g. `"Approved by user"`, `"Denied by user"`, `"Confirmation timed out (30 s)"`).
    pub user_action: Option<String>,
}

/// Append-only audit log that persists every [`AuditEvent`] to disk as NDJSON.
///
/// Events are written asynchronously to avoid blocking the interception hot-path.
/// An in-memory buffer is maintained so the UI can display recent events without
/// reading from disk.
///
/// Log format: one JSON object per line (NDJSON), appended to `log_path`.
pub struct AuditLog {
    /// Absolute path to the NDJSON log file on the host filesystem.
    log_path: PathBuf,
    /// In-memory ring buffer of recent audit events for UI display.
    buffer: Arc<Mutex<Vec<AuditEvent>>>,
}

impl AuditLog {
    /// Creates a new `AuditLog` targeting `log_path`.
    ///
    /// Parent directories are created automatically if they do not exist.
    /// The log file itself is created (or appended to) on the first write.
    pub fn new(log_path: PathBuf) -> Self {
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self {
            log_path,
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Appends an audit entry to the in-memory buffer and asynchronously
    /// writes it to the NDJSON log file.
    ///
    /// # Parameters
    /// - `event`       — The sandbox event being recorded.
    /// - `decision`    — The policy decision string.
    /// - `user_action` — Optional description of a manual user action.
    pub async fn record(&self, event: SandboxEvent, decision: &str, user_action: Option<&str>) {
        let audit_event = AuditEvent {
            event,
            decision: decision.to_string(),
            user_action: user_action.map(|s| s.to_string()),
        };

        let mut buffer = self.buffer.lock().await;
        buffer.push(audit_event.clone());

        // Serialise and write to disk asynchronously.
        let log_path = self.log_path.clone();
        let line = match serde_json::to_string(&audit_event) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to serialise audit event: {}", e);
                return;
            }
        };

        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .await
            {
                Ok(mut file) => {
                    let _ = file.write_all(format!("{}\n", line).as_bytes()).await;
                }
                Err(e) => {
                    error!("Failed to write audit log: {}", e);
                }
            }
        });
    }

    /// Returns the most recent `n` audit entries from the in-memory buffer.
    ///
    /// Used by the UI to populate the Events page without disk I/O.
    pub async fn recent(&self, n: usize) -> Vec<AuditEvent> {
        let buffer = self.buffer.lock().await;
        let len = buffer.len();
        if len <= n {
            buffer.clone()
        } else {
            buffer[len - n..].to_vec()
        }
    }

    /// Clears the in-memory buffer without touching the on-disk log file.
    ///
    /// Useful when the UI operator clicks "Clear Events".
    pub async fn clear_buffer(&self) {
        let mut buffer = self.buffer.lock().await;
        buffer.clear();
    }
}
