//! Multi-channel messaging adapter framework.
//!
//! Provides the [`MessageSkillHandler`] trait and concrete stub adapters for
//! Telegram, Discord, Slack, WhatsApp, and generic webhook channels.
//!
//! # Architecture
//!
//! ```text
//! dispatch.rs  message.send / message.read / message.reply / ...
//!      │
//!      ▼
//! MessageRouter   ─── finds the right adapter by channel name
//!      │
//!      ├── TelegramAdapter   (HTTP Bot API)
//!      ├── DiscordAdapter    (Discord REST + Webhook)
//!      ├── SlackAdapter      (Slack Web API)
//!      ├── WebhookAdapter    (generic outbound webhook)
//!      └── (register custom adapters at runtime)
//! ```
//!
//! Each adapter implements [`MessageAdapter`].  When real credentials are
//! absent the adapters fall back to a descriptive stub response so the agent
//! always gets a usable observation.
//!
//! # Usage
//!
//! ```rust,ignore
//! use openclaw_agent_executor::builtin_tools::messaging::{MessageRouter, TelegramAdapter};
//! use std::sync::Arc;
//!
//! let mut router = MessageRouter::new();
//! router.register(Arc::new(TelegramAdapter::new("BOT_TOKEN")));
//! dispatcher.register_handler(Arc::new(router));
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::dispatch::SkillHandler;

// ── Message types ─────────────────────────────────────────────────────────────

/// A single message in a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Platform-specific message ID.
    pub id: String,
    /// Channel / chat identifier.
    pub channel: String,
    /// Platform name (telegram, discord, slack, …).
    pub platform: String,
    /// Sender identifier.
    pub from: Option<String>,
    /// Message text content.
    pub text: String,
    /// ISO-8601 timestamp.
    pub ts: String,
    /// Optional reply-to message ID.
    pub reply_to: Option<String>,
}

/// Result of a send / reply operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    /// Whether the message was delivered successfully.
    pub ok: bool,
    /// Platform-assigned message ID (if available).
    pub message_id: Option<String>,
    /// Human-readable status message.
    pub message: String,
}

// ── MessageAdapter trait ──────────────────────────────────────────────────────

/// Trait for per-platform messaging adapters.
///
/// Implement this to connect OpenClaw to a new messaging platform.  All
/// methods have a default implementation that returns a descriptive stub so
/// partial implementations compile and run immediately.
#[async_trait]
pub trait MessageAdapter: Send + Sync {
    /// Platform identifier (e.g. `"telegram"`, `"discord"`, `"slack"`).
    fn platform(&self) -> &str;

    /// Channel names / IDs served by this adapter (empty = serve all for platform).
    fn channels(&self) -> Vec<String> { vec![] }

    /// Send a message to a channel.
    async fn send(&self, channel: &str, _text: &str) -> Result<SendResult, String> {
        Ok(SendResult {
            ok: false,
            message_id: None,
            message: format!(
                "({}.send: adapter registered but `send` not implemented for channel '{}')",
                self.platform(), channel
            ),
        })
    }

    /// Reply to a specific message.
    async fn reply(&self, channel: &str, reply_to_id: &str, _text: &str) -> Result<SendResult, String> {
        Ok(SendResult {
            ok: false,
            message_id: None,
            message: format!(
                "({}.reply: not implemented — channel='{}', reply_to='{}')",
                self.platform(), channel, reply_to_id
            ),
        })
    }

    /// Read recent messages from a channel.
    async fn read(&self, channel: &str, _limit: usize) -> Result<Vec<ChannelMessage>, String> {
        warn!("{} read not implemented for channel '{}'", self.platform(), channel);
        Ok(vec![])
    }

    /// Search message history.
    async fn search(&self, channel: &str, query: &str) -> Result<Vec<ChannelMessage>, String> {
        warn!("{} search not implemented, channel='{}', query='{}'", self.platform(), channel, query);
        Ok(vec![])
    }

    /// Add an emoji reaction to a message.
    async fn react(&self, channel: &str, message_id: &str, emoji: &str) -> Result<bool, String> {
        warn!("{} react not implemented (channel='{}', id='{}', emoji='{}')",
            self.platform(), channel, message_id, emoji);
        Ok(false)
    }

    /// Delete a message.
    async fn delete(&self, channel: &str, message_id: &str) -> Result<bool, String> {
        warn!("{} delete not implemented (channel='{}', id='{}')",
            self.platform(), channel, message_id);
        Ok(false)
    }
}

// ── MessageRouter ─────────────────────────────────────────────────────────────

/// Routes `message.*` skill calls to the appropriate platform adapter.
///
/// Registered with [`SkillDispatcher::register_handler`] to intercept all
/// `message.*` skills.
pub struct MessageRouter {
    /// Registered adapters, keyed by platform name.
    adapters: RwLock<HashMap<String, Arc<dyn MessageAdapter>>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self { adapters: RwLock::new(HashMap::new()) }
    }

    /// Register an adapter.  If an adapter for the same platform is already
    /// registered it will be replaced.
    pub async fn register(&self, adapter: Arc<dyn MessageAdapter>) {
        let platform = adapter.platform().to_string();
        debug!("MessageRouter: registered adapter for platform '{}'", platform);
        self.adapters.write().await.insert(platform, adapter);
    }

    /// Synchronous registration (for use during setup before the async runtime
    /// is fully active).
    pub fn register_sync(&mut self, adapter: Arc<dyn MessageAdapter>) {
        let platform = adapter.platform().to_string();
        self.adapters.get_mut().insert(platform, adapter);
    }

    /// Resolve the adapter for a given channel string.
    ///
    /// Channel string format: `"platform:channel_id"` or just `"channel_id"`.
    /// If a platform prefix is present the matching adapter is used.  Otherwise
    /// all adapters are searched for one that owns the channel.
    async fn resolve(&self, channel: &str) -> Option<(Arc<dyn MessageAdapter>, String)> {
        let adapters = self.adapters.read().await;

        // Try "platform:channel_id" format first
        if let Some((prefix, rest)) = channel.split_once(':') {
            if let Some(adapter) = adapters.get(prefix) {
                return Some((adapter.clone(), rest.to_string()));
            }
        }

        // Fall back to first adapter that claims this channel
        for adapter in adapters.values() {
            let channels = adapter.channels();
            if channels.is_empty() || channels.iter().any(|c| c == channel) {
                return Some((adapter.clone(), channel.to_string()));
            }
        }

        None
    }

    fn no_adapter_msg(skill: &str, channel: &str) -> String {
        format!(
            "({skill}: no messaging adapter configured for channel '{channel}'. \
             Register a MessageAdapter (Telegram, Discord, Slack, …) via \
             MessageRouter::register(), or prefix the channel with the platform \
             name, e.g. 'telegram:chat_id'.)"
        )
    }
}

impl Default for MessageRouter {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl SkillHandler for MessageRouter {
    fn skill_names(&self) -> &[&'static str] {
        &[
            "message.send",
            "message.reply",
            "message.react",
            "message.delete",
            "message.read",
            "message.list",
            "message.search",
        ]
    }

    async fn execute(&self, skill_name: &str, args: &serde_json::Value) -> Result<String, String> {
        let channel = args["channel"].as_str().unwrap_or("default");
        let op = skill_name.strip_prefix("message.").unwrap_or(skill_name);

        match self.resolve(channel).await {
            None => Ok(Self::no_adapter_msg(skill_name, channel)),
            Some((adapter, resolved_channel)) => {
                match op {
                    "send" => {
                        let text = args["text"].as_str()
                            .ok_or("message.send: missing 'text' argument")?;
                        let result = adapter.send(&resolved_channel, text).await?;
                        Ok(serde_json::to_string(&result).unwrap_or_else(|_| result.message))
                    }
                    "reply" => {
                        let text = args["text"].as_str()
                            .ok_or("message.reply: missing 'text' argument")?;
                        let reply_to = args["messageId"].as_str()
                            .or_else(|| args["replyTo"].as_str())
                            .ok_or("message.reply: missing 'messageId' argument")?;
                        let result = adapter.reply(&resolved_channel, reply_to, text).await?;
                        Ok(serde_json::to_string(&result).unwrap_or_else(|_| result.message))
                    }
                    "react" => {
                        let message_id = args["messageId"].as_str()
                            .ok_or("message.react: missing 'messageId' argument")?;
                        let emoji = args["emoji"].as_str().unwrap_or("👍");
                        let ok = adapter.react(&resolved_channel, message_id, emoji).await?;
                        Ok(format!("{{\"ok\":{ok},\"emoji\":\"{emoji}\"}}"))
                    }
                    "delete" => {
                        let message_id = args["messageId"].as_str()
                            .ok_or("message.delete: missing 'messageId' argument")?;
                        let ok = adapter.delete(&resolved_channel, message_id).await?;
                        Ok(format!("{{\"ok\":{ok},\"messageId\":\"{message_id}\"}}"))
                    }
                    "read" | "list" => {
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        let messages = adapter.read(&resolved_channel, limit).await?;
                        serde_json::to_string(&messages).map_err(|e| e.to_string())
                    }
                    "search" => {
                        let query = args["query"].as_str()
                            .ok_or("message.search: missing 'query' argument")?;
                        let messages = adapter.search(&resolved_channel, query).await?;
                        serde_json::to_string(&messages).map_err(|e| e.to_string())
                    }
                    other => Err(format!("message.{other}: unknown message operation")),
                }
            }
        }
    }
}

// ── TelegramAdapter ───────────────────────────────────────────────────────────

/// Telegram Bot API adapter.
///
/// Set `bot_token` to a real Telegram Bot API token for live messaging.
/// Without a token all calls return descriptive stubs.
pub struct TelegramAdapter {
    bot_token: Option<String>,
    client: reqwest::Client,
}

impl TelegramAdapter {
    /// Create with a bot token.  Pass `None` or an empty string for stub mode.
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            bot_token: if token.is_empty() { None } else { Some(token) },
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
        }
    }

    fn api_url(&self, method: &str) -> Option<String> {
        self.bot_token.as_ref().map(|t| {
            format!("https://api.telegram.org/bot{}/{}", t, method)
        })
    }
}

#[async_trait]
impl MessageAdapter for TelegramAdapter {
    fn platform(&self) -> &str { "telegram" }

    async fn send(&self, channel: &str, text: &str) -> Result<SendResult, String> {
        let Some(url) = self.api_url("sendMessage") else {
            return Ok(SendResult {
                ok: false,
                message_id: None,
                message: format!(
                    "(telegram.send: no bot token configured — \
                     set TELEGRAM_BOT_TOKEN or pass token to TelegramAdapter::new(). \
                     Would send to chat_id='{}': \"{}\")",
                    channel, text
                ),
            });
        };

        let body = serde_json::json!({
            "chat_id": channel,
            "text": text,
            "parse_mode": "HTML"
        });

        match self.client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let msg_id = json["result"]["message_id"].as_i64()
                    .map(|id| id.to_string());
                Ok(SendResult {
                    ok: true,
                    message_id: msg_id,
                    message: "Message sent via Telegram.".to_string(),
                })
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!("Telegram API error {}: {}", status, body))
            }
            Err(e) => Err(format!("Telegram request failed: {}", e)),
        }
    }

    async fn reply(&self, channel: &str, reply_to_id: &str, text: &str) -> Result<SendResult, String> {
        let Some(url) = self.api_url("sendMessage") else {
            return Ok(SendResult {
                ok: false,
                message_id: None,
                message: format!(
                    "(telegram.reply: no bot token — would reply to msg '{}' in chat '{}')",
                    reply_to_id, channel
                ),
            });
        };

        let body = serde_json::json!({
            "chat_id": channel,
            "text": text,
            "reply_to_message_id": reply_to_id.parse::<i64>().unwrap_or(0),
            "parse_mode": "HTML"
        });

        match self.client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let msg_id = json["result"]["message_id"].as_i64().map(|id| id.to_string());
                Ok(SendResult { ok: true, message_id: msg_id, message: "Reply sent.".to_string() })
            }
            Ok(resp) => Err(format!("Telegram API error {}", resp.status())),
            Err(e) => Err(format!("Telegram request failed: {}", e)),
        }
    }
}

// ── DiscordAdapter ────────────────────────────────────────────────────────────

/// Discord REST API adapter (Bot token).
pub struct DiscordAdapter {
    bot_token: Option<String>,
    client: reqwest::Client,
}

impl DiscordAdapter {
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            bot_token: if token.is_empty() { None } else { Some(token) },
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
        }
    }
}

#[async_trait]
impl MessageAdapter for DiscordAdapter {
    fn platform(&self) -> &str { "discord" }

    async fn send(&self, channel: &str, text: &str) -> Result<SendResult, String> {
        let Some(token) = &self.bot_token else {
            return Ok(SendResult {
                ok: false,
                message_id: None,
                message: format!(
                    "(discord.send: no bot token configured — \
                     set DISCORD_BOT_TOKEN. Would send to channel '{}': \"{}\")",
                    channel, text
                ),
            });
        };

        let url = format!("https://discord.com/api/v10/channels/{}/messages", channel);
        let body = serde_json::json!({ "content": text });

        match self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", token))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let msg_id = json["id"].as_str().map(|s| s.to_string());
                Ok(SendResult {
                    ok: true,
                    message_id: msg_id,
                    message: "Message sent via Discord.".to_string(),
                })
            }
            Ok(resp) => Err(format!("Discord API error {}", resp.status())),
            Err(e) => Err(format!("Discord request failed: {}", e)),
        }
    }
}

// ── SlackAdapter ──────────────────────────────────────────────────────────────

/// Slack Web API adapter (Bot OAuth token).
pub struct SlackAdapter {
    bot_token: Option<String>,
    client: reqwest::Client,
}

impl SlackAdapter {
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            bot_token: if token.is_empty() { None } else { Some(token) },
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
        }
    }
}

#[async_trait]
impl MessageAdapter for SlackAdapter {
    fn platform(&self) -> &str { "slack" }

    async fn send(&self, channel: &str, text: &str) -> Result<SendResult, String> {
        let Some(token) = &self.bot_token else {
            return Ok(SendResult {
                ok: false,
                message_id: None,
                message: format!(
                    "(slack.send: no bot token configured — \
                     set SLACK_BOT_TOKEN. Would send to '{}': \"{}\")",
                    channel, text
                ),
            });
        };

        let body = serde_json::json!({
            "channel": channel,
            "text": text
        });

        match self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                if json["ok"].as_bool().unwrap_or(false) {
                    let ts = json["ts"].as_str().map(|s| s.to_string());
                    Ok(SendResult {
                        ok: true,
                        message_id: ts,
                        message: "Message sent via Slack.".to_string(),
                    })
                } else {
                    let err = json["error"].as_str().unwrap_or("unknown_error");
                    Err(format!("Slack API error: {}", err))
                }
            }
            Ok(resp) => Err(format!("Slack HTTP error {}", resp.status())),
            Err(e) => Err(format!("Slack request failed: {}", e)),
        }
    }

    async fn reply(&self, channel: &str, reply_to_id: &str, text: &str) -> Result<SendResult, String> {
        let Some(token) = &self.bot_token else {
            return Ok(SendResult {
                ok: false,
                message_id: None,
                message: format!(
                    "(slack.reply: no bot token — would reply to ts='{}' in '{}')",
                    reply_to_id, channel
                ),
            });
        };

        let body = serde_json::json!({
            "channel": channel,
            "text": text,
            "thread_ts": reply_to_id
        });

        match self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                if json["ok"].as_bool().unwrap_or(false) {
                    let ts = json["ts"].as_str().map(|s| s.to_string());
                    Ok(SendResult { ok: true, message_id: ts, message: "Thread reply sent.".to_string() })
                } else {
                    Err(format!("Slack error: {}", json["error"].as_str().unwrap_or("unknown")))
                }
            }
            Ok(resp) => Err(format!("Slack HTTP {}", resp.status())),
            Err(e) => Err(format!("Slack request failed: {}", e)),
        }
    }
}

// ── WebhookAdapter ────────────────────────────────────────────────────────────

/// Generic outbound webhook adapter.
///
/// Posts a JSON body to a configured URL for any `message.send` call.
/// Useful for Feishu (Lark), DingTalk, custom notification systems, etc.
pub struct WebhookAdapter {
    platform_name: String,
    webhook_url: String,
    client: reqwest::Client,
}

impl WebhookAdapter {
    pub fn new(platform_name: impl Into<String>, webhook_url: impl Into<String>) -> Self {
        Self {
            platform_name: platform_name.into(),
            webhook_url: webhook_url.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
        }
    }
}

#[async_trait]
impl MessageAdapter for WebhookAdapter {
    fn platform(&self) -> &str { &self.platform_name }

    async fn send(&self, channel: &str, text: &str) -> Result<SendResult, String> {
        if self.webhook_url.is_empty() {
            return Ok(SendResult {
                ok: false,
                message_id: None,
                message: format!(
                    "({}.send: no webhook URL configured — would post to channel '{}')",
                    self.platform_name, channel
                ),
            });
        }

        let body = serde_json::json!({
            "platform": self.platform_name,
            "channel": channel,
            "text": text,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        match self.client.post(&self.webhook_url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                Ok(SendResult {
                    ok: true,
                    message_id: None,
                    message: format!("Webhook delivered to '{}'.", self.webhook_url),
                })
            }
            Ok(resp) => Err(format!("Webhook error {}", resp.status())),
            Err(e) => Err(format!("Webhook request failed: {}", e)),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_router() -> MessageRouter {
        MessageRouter::new()
    }

    // ── MessageRouter stub fallback ───────────────────────────────────────

    #[tokio::test]
    async fn router_no_adapter_returns_stub() {
        let router = make_router();
        let result = router.execute(
            "message.send",
            &serde_json::json!({"channel": "discord", "text": "hello"}),
        ).await.unwrap();
        assert!(result.contains("message.send") || result.contains("adapter"),
            "result: {result}");
    }

    #[tokio::test]
    async fn router_missing_text_errors() {
        let router = make_router();
        let router_with_adapter = {
            let mut r = MessageRouter::new();
            r.register_sync(Arc::new(TelegramAdapter::new("")));
            r
        };
        let err = router_with_adapter.execute(
            "message.send",
            &serde_json::json!({"channel": "telegram:12345"}),
        ).await;
        assert!(err.is_err(), "missing text should error");
    }

    #[tokio::test]
    async fn router_unknown_op_errors() {
        let mut router = make_router();
        router.register_sync(Arc::new(TelegramAdapter::new("")));
        let err = router.execute(
            "message.foobar",
            &serde_json::json!({"channel": "telegram:123", "text": "x"}),
        ).await;
        assert!(err.is_err(), "unknown op should error");
    }

    #[tokio::test]
    async fn router_skill_names_correct() {
        let router = make_router();
        let names = router.skill_names();
        assert!(names.contains(&"message.send"));
        assert!(names.contains(&"message.reply"));
        assert!(names.contains(&"message.read"));
        assert!(names.contains(&"message.search"));
        assert!(names.contains(&"message.react"));
        assert!(names.contains(&"message.delete"));
    }

    // ── TelegramAdapter stub (no token) ───────────────────────────────────

    #[tokio::test]
    async fn telegram_send_stub_no_token() {
        let adapter = TelegramAdapter::new("");
        let result = adapter.send("12345", "Hello!").await.unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("no bot token") || result.message.contains("TELEGRAM_BOT_TOKEN"),
            "message: {}", result.message);
    }

    #[tokio::test]
    async fn telegram_reply_stub_no_token() {
        let adapter = TelegramAdapter::new("");
        let result = adapter.reply("123", "456", "reply text").await.unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("no bot token"), "message: {}", result.message);
    }

    // ── DiscordAdapter stub (no token) ────────────────────────────────────

    #[tokio::test]
    async fn discord_send_stub_no_token() {
        let adapter = DiscordAdapter::new("");
        let result = adapter.send("channel-id", "Hello Discord!").await.unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("DISCORD_BOT_TOKEN") || result.message.contains("no bot token"),
            "message: {}", result.message);
    }

    // ── SlackAdapter stub (no token) ──────────────────────────────────────

    #[tokio::test]
    async fn slack_send_stub_no_token() {
        let adapter = SlackAdapter::new("");
        let result = adapter.send("#general", "Hello Slack!").await.unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("SLACK_BOT_TOKEN") || result.message.contains("no bot token"),
            "message: {}", result.message);
    }

    #[tokio::test]
    async fn slack_reply_stub_no_token() {
        let adapter = SlackAdapter::new("");
        let result = adapter.reply("#general", "1234567890.123456", "thread reply").await.unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("no bot token"), "message: {}", result.message);
    }

    // ── WebhookAdapter stub (empty URL) ───────────────────────────────────

    #[tokio::test]
    async fn webhook_send_stub_no_url() {
        let adapter = WebhookAdapter::new("feishu", "");
        let result = adapter.send("hr-channel", "Hello Feishu!").await.unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("no webhook URL"), "message: {}", result.message);
    }

    // ── Platform routing ──────────────────────────────────────────────────

    #[tokio::test]
    async fn router_routes_by_platform_prefix() {
        let mut router = MessageRouter::new();
        router.register_sync(Arc::new(TelegramAdapter::new("")));
        router.register_sync(Arc::new(DiscordAdapter::new("")));

        let tg = router.execute(
            "message.send",
            &serde_json::json!({"channel": "telegram:chat123", "text": "hi"}),
        ).await.unwrap();
        assert!(tg.contains("TELEGRAM_BOT_TOKEN") || tg.contains("no bot token"),
            "should route to telegram: {tg}");

        let dc = router.execute(
            "message.send",
            &serde_json::json!({"channel": "discord:channel456", "text": "hi"}),
        ).await.unwrap();
        assert!(dc.contains("DISCORD_BOT_TOKEN") || dc.contains("no bot token"),
            "should route to discord: {dc}");
    }

    // ── Read / search fallback ────────────────────────────────────────────

    #[tokio::test]
    async fn telegram_read_returns_empty_vec() {
        let adapter = TelegramAdapter::new("");
        let msgs = adapter.read("chat123", 10).await.unwrap();
        assert!(msgs.is_empty());
    }

    #[tokio::test]
    async fn telegram_search_returns_empty_vec() {
        let adapter = TelegramAdapter::new("");
        let msgs = adapter.search("chat123", "hello").await.unwrap();
        assert!(msgs.is_empty());
    }
}
