//! # `types.rs` — Shared Data Structures
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Defines every data type shared across the OpenClaw+ Store crate:
//!
//! | Group | Types |
//! |---|---|
//! | Chat | [`ChatRole`], [`ChatMessage`], [`ChatState`] |
//! | AI Models | [`AiBackend`], [`AiModelConfig`], [`AiModelStatus`] |
//! | Bots | [`BotPlatform`], [`BotStatus`], [`BotConfig`] |
//! | Local API | [`ApiTransport`], [`ApiEndpointStatus`], [`LocalApiConfig`] |
//! | Plugins | [`LibrarySource`], [`InstallState`], [`PluginEntry`], [`RegistryIndex`] |
//! | Prefs | [`StorePrefs`] |
//!
//! All types that cross the serialisation boundary (registry JSON, config
//! files) derive `serde::Serialize` / `serde::Deserialize`.  Pure UI-state
//! types (e.g. [`ChatState`], [`AiModelStatus`]) are not serialised.

use serde::{Deserialize, Serialize};

// ── Dashboard types ───────────────────────────────────────────────────────────

/// Whether the OpenClaw agent process is currently running on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProcessStatus {
    /// Not yet checked.
    #[default]
    Unknown,
    /// Process found; contains the PID.
    Running(u32),
    /// No matching process found.
    Stopped,
    /// An error occurred while checking (e.g. permission denied).
    CheckError(String),
}

/// Severity level of a security audit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditSeverity {
    /// Informational — no action taken.
    Info,
    /// Suspicious activity — logged and flagged.
    Warning,
    /// Policy violation — action was blocked.
    Blocked,
}

/// A single entry in the security audit log shown on the Dashboard.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Monotonically increasing id for stable list rendering.
    pub id: usize,
    /// Human-readable timestamp (local time, formatted as HH:MM:SS).
    pub timestamp: String,
    /// Severity of the event.
    pub severity: AuditSeverity,
    /// Short description of what was intercepted or observed.
    pub description: String,
    /// Optional detail: the raw syscall / path / action that triggered this.
    pub detail: Option<String>,
}

/// Snapshot of WasmEdge sandbox resource usage shown on the Dashboard.
#[derive(Debug, Clone, Default)]
pub struct SandboxMetrics {
    /// Number of syscalls intercepted since the sandbox started.
    pub syscalls_intercepted: u64,
    /// Number of policy violations (blocked actions) since start.
    pub violations_blocked: u64,
    /// Number of plugins currently loaded in the sandbox.
    pub plugins_loaded: usize,
    /// Approximate memory used by the sandbox process in MiB.
    pub memory_mib: f32,
}

/// Full state for the Dashboard page.
#[derive(Debug, Clone, Default)]
pub struct DashboardState {
    /// Current status of the OpenClaw agent process.
    pub process_status: ProcessStatus,
    /// Recent security audit events (newest last, capped at 200).
    pub audit_log: Vec<AuditEvent>,
    /// Live sandbox resource metrics.
    pub metrics: SandboxMetrics,
    /// Counter used to assign stable IDs to audit events.
    pub event_counter: usize,
}

// ── Chat / conversation types ─────────────────────────────────────────────────

/// Who sent a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

/// A single turn in the conversation history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Monotonically increasing id for stable list keys.
    pub id: usize,
}

/// Runtime state of the chat page.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ChatState {
    #[default]
    Idle,
    /// Waiting for the model to respond.
    Thinking,
    /// Streaming a partial response (content so far).
    Streaming(String),
    /// Last request failed.
    Error(String),
}

// ── AI Model configuration ────────────────────────────────────────────────────

/// Which local AI backend to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiBackend {
    /// Ollama local inference server (http://localhost:11434).
    #[default]
    Ollama,
    /// LM Studio local server (http://localhost:1234).
    LmStudio,
    /// llama.cpp server (http://localhost:8080).
    LlamaCpp,
    /// Custom OpenAI-compatible endpoint.
    Custom,
}

impl std::fmt::Display for AiBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiBackend::Ollama   => write!(f, "Ollama"),
            AiBackend::LmStudio => write!(f, "LM Studio"),
            AiBackend::LlamaCpp => write!(f, "llama.cpp"),
            AiBackend::Custom   => write!(f, "Custom Endpoint"),
        }
    }
}

/// Configuration for a local AI model endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelConfig {
    /// Display name for this configuration profile.
    pub name: String,
    /// Which backend type.
    pub backend: AiBackend,
    /// Base URL of the inference server.
    pub endpoint: String,
    /// Model name / tag to use (e.g. `llama3`, `mistral`).
    pub model: String,
    /// Optional API key (for custom endpoints that require auth).
    pub api_key: String,
    /// Max tokens to generate per request.
    pub max_tokens: u32,
    /// Temperature (0.0–2.0).
    pub temperature: f32,
    /// Whether this profile is the active default.
    pub is_active: bool,
    /// Connection status (populated at runtime, not persisted).
    #[serde(skip)]
    pub status: AiModelStatus,
}

impl Default for AiModelConfig {
    fn default() -> Self {
        Self {
            name: "Local Ollama".into(),
            backend: AiBackend::Ollama,
            endpoint: "http://localhost:11434".into(),
            model: "llama3".into(),
            api_key: String::new(),
            max_tokens: 2048,
            temperature: 0.7,
            is_active: true,
            status: AiModelStatus::Unknown,
        }
    }
}

/// Runtime connection status of an AI model endpoint.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AiModelStatus {
    #[default]
    Unknown,
    Checking,
    Online { model_count: usize },
    Offline(String),
}

// ── Bot / phone local API configuration ──────────────────────────────────────

/// Transport protocol for the local bot / phone interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiTransport {
    /// Plain HTTP REST.
    #[default]
    Http,
    /// WebSocket for real-time bidirectional messaging.
    WebSocket,
    /// gRPC (requires protobuf schema).
    Grpc,
    /// MQTT for IoT / phone push.
    Mqtt,
}

impl std::fmt::Display for ApiTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiTransport::Http      => write!(f, "HTTP REST"),
            ApiTransport::WebSocket => write!(f, "WebSocket"),
            ApiTransport::Grpc      => write!(f, "gRPC"),
            ApiTransport::Mqtt      => write!(f, "MQTT"),
        }
    }
}

/// A local API endpoint exposed for bots or mobile devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalApiConfig {
    /// Human-readable name for this endpoint.
    pub name: String,
    /// Transport protocol.
    pub transport: ApiTransport,
    /// Bind address (e.g. `0.0.0.0` to accept LAN connections).
    pub bind_host: String,
    /// Port to listen on.
    pub port: u16,
    /// Optional bearer token required by clients.
    pub auth_token: String,
    /// Enable TLS (requires cert/key paths).
    pub tls_enabled: bool,
    /// Path to TLS certificate file.
    pub tls_cert_path: String,
    /// Path to TLS private key file.
    pub tls_key_path: String,
    /// Whether this endpoint is currently active/running.
    pub enabled: bool,
    /// Runtime status (not persisted).
    #[serde(skip)]
    pub status: ApiEndpointStatus,
}

impl Default for LocalApiConfig {
    fn default() -> Self {
        Self {
            name: "Local Bot API".into(),
            transport: ApiTransport::Http,
            bind_host: "127.0.0.1".into(),
            port: 8765,
            auth_token: String::new(),
            tls_enabled: false,
            tls_cert_path: String::new(),
            tls_key_path: String::new(),
            enabled: false,
            status: ApiEndpointStatus::Stopped,
        }
    }
}

/// Runtime status of a local API endpoint.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ApiEndpointStatus {
    #[default]
    Stopped,
    Starting,
    Running { connections: usize },
    Error(String),
}

/// Configuration for a bot integration (Telegram, Discord, WeChat, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Display name.
    pub name: String,
    /// Bot platform identifier.
    pub platform: BotPlatform,
    /// Bot token / API key.
    pub token: String,
    /// Webhook URL (if platform uses webhooks).
    pub webhook_url: String,
    /// Whether this bot integration is enabled.
    pub enabled: bool,
    /// Runtime connection status.
    #[serde(skip)]
    pub status: BotStatus,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            name: "My Bot".into(),
            platform: BotPlatform::Telegram,
            token: String::new(),
            webhook_url: String::new(),
            enabled: false,
            status: BotStatus::Disconnected,
        }
    }
}

/// Supported bot platforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BotPlatform {
    #[default]
    Telegram,
    Discord,
    WeChat,
    Slack,
    Custom,
}

impl std::fmt::Display for BotPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotPlatform::Telegram => write!(f, "Telegram"),
            BotPlatform::Discord  => write!(f, "Discord"),
            BotPlatform::WeChat   => write!(f, "WeChat"),
            BotPlatform::Slack    => write!(f, "Slack"),
            BotPlatform::Custom   => write!(f, "Custom"),
        }
    }
}

/// Runtime connection status of a bot.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum BotStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected { username: String },
    Error(String),
}

// ── Library source selector ───────────────────────────────────────────────────

/// Which plugin library the user has chosen as their active source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySource {
    /// Official OpenClaw plugin registry (JS-based plugins).
    OpenClaw,
    /// ClawPlus registry: Rust/WASM plugins with sandboxed execution.
    #[default]
    ClawPlus,
}

impl std::fmt::Display for LibrarySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LibrarySource::OpenClaw => write!(f, "OpenClaw Official"),
            LibrarySource::ClawPlus => write!(f, "ClawPlus (Rust/WASM)"),
        }
    }
}

// ── Registry index ────────────────────────────────────────────────────────────

/// Top-level registry index fetched from the remote registry server.
/// Serialised as `index.json` at the registry root URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    /// Registry format version (currently `1`).
    pub version: u32,
    /// Human-readable name of the registry.
    pub name: String,
    /// Short description shown in the UI header.
    pub description: String,
    /// All plugins available in this registry.
    pub plugins: Vec<PluginEntry>,
}

/// A single plugin listed in the registry index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Unique reverse-domain identifier, e.g. `com.clawplus.downloader`.
    pub id: String,
    /// Display name shown in the store card.
    pub name: String,
    /// One-line summary.
    pub description: String,
    /// Semver version string.
    pub version: String,
    /// Plugin author or organisation.
    pub author: String,
    /// SPDX licence identifier.
    pub license: String,
    /// URL to the compiled `.wasm` artifact.
    pub wasm_url: String,
    /// Lowercase hex SHA-256 of the `.wasm` file for integrity verification.
    pub sha256: String,
    /// Minimum ClawPlus host version required.
    pub min_host_version: String,
    /// Categorisation tags shown as chips in the UI.
    pub tags: Vec<String>,
    /// Which library source this entry belongs to.
    pub source: LibrarySource,
    /// Download count (informational, supplied by the registry server).
    #[serde(default)]
    pub downloads: u64,
    /// OpenClaw Skill names exposed by this plugin (dot-notation, e.g. `"fs.readFile"`).
    /// Used to auto-register risk levels in the SkillRegistry after installation.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Whether this plugin ships pre-installed with ClawPlus (no download required).
    /// Pre-installed plugins are marked `InstallState::Installed` at startup.
    #[serde(default)]
    pub preinstalled: bool,
    /// Whether the plugin is currently installed locally.
    #[serde(skip)]
    pub installed: bool,
    /// Download progress 0.0–1.0 while a download is in flight.
    #[serde(skip)]
    pub download_progress: Option<f32>,
}

// ── Install state ─────────────────────────────────────────────────────────────

/// Lifecycle state of a plugin installation.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum InstallState {
    /// Not yet started.
    #[default]
    Idle,
    /// Fetching the `.wasm` from the registry.
    Downloading { progress: f32 },
    /// Verifying the SHA-256 digest.
    Verifying,
    /// Writing to the local plugin directory.
    Installing,
    /// Successfully installed and ready to use.
    Installed,
    /// Installation failed; inner string is the human-readable reason.
    Failed(String),
}

// ── Store user preferences ────────────────────────────────────────────────────

/// Persisted store preferences saved to `~/.config/openclaw-plus/store.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePrefs {
    /// Currently active library source.
    pub active_source: LibrarySource,
    /// URL of the ClawPlus registry index.
    pub clawplus_registry_url: String,
    /// URL of the OpenClaw official registry index.
    pub openclaw_registry_url: String,
    /// Local directory where downloaded `.wasm` files are stored.
    pub plugin_dir: String,
}

impl Default for StorePrefs {
    fn default() -> Self {
        let plugin_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("plugins")
            .to_string_lossy()
            .into_owned();

        // Priority: env var > local file:// > remote HTTPS
        let local_registry = std::env::var("CLAWPLUS_REGISTRY_URL").ok()
            .unwrap_or_else(|| {
                // Try the workspace registry/index.json relative to the binary.
                let exe_dir = std::env::current_exe().ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()));
                // Walk up from the binary to find the workspace root.
                let candidates = [
                    exe_dir.as_ref().and_then(|d| {
                        // release binary: target/release/ → ../../registry
                        d.parent().and_then(|p| p.parent())
                            .map(|root| root.join("registry").join("index.json"))
                    }),
                    std::env::current_dir().ok()
                        .map(|p| p.join("registry").join("index.json")),
                ];
                candidates.into_iter()
                    .flatten()
                    .find(|p| p.exists())
                    .map(|p| format!("file://{}", p.display()))
                    .unwrap_or_else(|| "https://registry.clawplus.dev/index.json".to_string())
            });

        Self {
            active_source: LibrarySource::ClawPlus,
            clawplus_registry_url: local_registry,
            openclaw_registry_url: "https://registry.openclaw.dev/index.json".to_string(),
            plugin_dir,
        }
    }
}
