use crate::circuit_breaker::BreakerConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// GitHub / Git operation security policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPolicy {
    /// Intercept and evaluate all `git push` commands.
    pub intercept_push: bool,
    /// Always deny `git push --force` / `git push --force-with-lease`.
    pub deny_force_push: bool,
    /// Require confirmation before pushing to any remote.
    pub confirm_push: bool,
    /// Require confirmation before deleting a remote branch.
    pub confirm_branch_delete: bool,
    /// Require confirmation before any `git reset --hard` or `git rebase`.
    pub confirm_history_rewrite: bool,
    /// Intercept GitHub API calls (REST/GraphQL via api.github.com).
    pub intercept_github_api: bool,
    /// Allowed GitHub organisations/users (e.g. ["my-org", "my-user"]).
    /// Empty = allow all repos.
    pub allowed_orgs: Vec<String>,
    /// Allowed repository names (e.g. ["my-org/my-repo"]).
    /// Empty = allow all repos within allowed_orgs.
    pub allowed_repos: Vec<String>,
    /// Block pushes to the default branch (main/master) without confirmation.
    pub protect_default_branch: bool,
    /// Maximum number of files changed in a single commit before requiring confirmation.
    pub max_files_per_commit: Option<u32>,
}

impl Default for GitHubPolicy {
    fn default() -> Self {
        Self {
            intercept_push: true,
            deny_force_push: true,
            confirm_push: true,
            confirm_branch_delete: true,
            confirm_history_rewrite: true,
            intercept_github_api: true,
            allowed_orgs: Vec::new(),
            allowed_repos: Vec::new(),
            protect_default_branch: true,
            max_files_per_commit: Some(50),
        }
    }
}

/// Supported AI Agent frameworks that can be sandboxed by OpenClaw+.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    /// OpenClaw (Node.js) — default.
    OpenClaw,
    /// AutoGPT (Python).
    AutoGpt,
    /// Dify Agent (Node.js/Python).
    Dify,
    /// Custom JS bundle run via WasmEdge-QuickJS.
    CustomJs,
    /// Custom Python script run via WasmEdge-Python.
    CustomPython,
    /// Pre-compiled WASM module.
    CustomWasm,
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentKind::OpenClaw      => write!(f, "OpenClaw"),
            AgentKind::AutoGpt       => write!(f, "AutoGPT"),
            AgentKind::Dify          => write!(f, "Dify Agent"),
            AgentKind::CustomJs      => write!(f, "Custom JS"),
            AgentKind::CustomPython  => write!(f, "Custom Python"),
            AgentKind::CustomWasm    => write!(f, "Custom WASM"),
        }
    }
}

impl AgentKind {
    /// Returns all supported agent kinds.
    pub fn all() -> &'static [AgentKind] {
        &[
            AgentKind::OpenClaw,
            AgentKind::AutoGpt,
            AgentKind::Dify,
            AgentKind::CustomJs,
            AgentKind::CustomPython,
            AgentKind::CustomWasm,
        ]
    }

    /// Returns the typical entry-point filename for this agent kind.
    pub fn default_entry(&self) -> &'static str {
        match self {
            AgentKind::OpenClaw     => "openclaw/dist/index.js",
            AgentKind::AutoGpt      => "autogpt/main.py",
            AgentKind::Dify         => "dify/dist/index.js",
            AgentKind::CustomJs     => "agent/index.js",
            AgentKind::CustomPython => "agent/main.py",
            AgentKind::CustomWasm   => "agent/agent.wasm",
        }
    }

    /// Returns a short description shown in the Settings UI.
    pub fn description(&self) -> &'static str {
        match self {
            AgentKind::OpenClaw     => "OpenClaw AI coding agent (Node.js via WasmEdge-QuickJS)",
            AgentKind::AutoGpt      => "AutoGPT autonomous agent (Python via WasmEdge-Python)",
            AgentKind::Dify         => "Dify workflow agent (Node.js via WasmEdge-QuickJS)",
            AgentKind::CustomJs     => "Custom JavaScript agent bundle",
            AgentKind::CustomPython => "Custom Python agent script",
            AgentKind::CustomWasm   => "Pre-compiled WASM module (direct execution)",
        }
    }
}

// ── OpenClaw AI Model Configuration ──────────────────────────────────────────

/// Which AI provider / backend OpenClaw should use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    /// Ollama local server (default, privacy-first).
    #[default]
    Ollama,
    /// llama.cpp HTTP server (OpenAI-compatible, such as llama-server).
    LlamaCppHttp,
    /// OpenAI API (GPT-4o, GPT-4-turbo, etc.).
    OpenAi,
    /// Anthropic Claude API.
    Anthropic,
    /// DeepSeek API.
    DeepSeek,
    /// Google Gemini API.
    Gemini,
    /// Any OpenAI-compatible endpoint (e.g. LM Studio, vLLM, Groq).
    OpenAiCompat,
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiProvider::Ollama        => write!(f, "Ollama (Local)"),
            AiProvider::LlamaCppHttp  => write!(f, "llama.cpp HTTP"),
            AiProvider::OpenAi        => write!(f, "OpenAI"),
            AiProvider::Anthropic     => write!(f, "Anthropic Claude"),
            AiProvider::DeepSeek      => write!(f, "DeepSeek"),
            AiProvider::Gemini        => write!(f, "Google Gemini"),
            AiProvider::OpenAiCompat  => write!(f, "OpenAI-Compatible"),
        }
    }
}

impl AiProvider {
    pub fn all() -> &'static [AiProvider] {
        &[
            AiProvider::Ollama,
            AiProvider::LlamaCppHttp,
            AiProvider::OpenAi,
            AiProvider::Anthropic,
            AiProvider::DeepSeek,
            AiProvider::Gemini,
            AiProvider::OpenAiCompat,
        ]
    }

    pub fn default_endpoint(&self) -> &'static str {
        match self {
            AiProvider::Ollama        => "http://localhost:11434",
            AiProvider::LlamaCppHttp  => "http://localhost:8080",
            AiProvider::OpenAi        => "https://api.openai.com/v1",
            AiProvider::Anthropic     => "https://api.anthropic.com",
            AiProvider::DeepSeek      => "https://api.deepseek.com/v1",
            AiProvider::Gemini        => "https://generativelanguage.googleapis.com/v1beta",
            AiProvider::OpenAiCompat  => "http://localhost:1234/v1",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            AiProvider::Ollama        => "qwen3.5:9b",
            AiProvider::LlamaCppHttp  => "local-model",
            AiProvider::OpenAi        => "gpt-4o-mini",
            AiProvider::Anthropic     => "claude-3-haiku-20240307",
            AiProvider::DeepSeek      => "deepseek-chat",
            AiProvider::Gemini        => "gemini-1.5-flash",
            AiProvider::OpenAiCompat  => "local-model",
        }
    }

    /// Whether this provider requires an API key.
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, AiProvider::Ollama | AiProvider::LlamaCppHttp | AiProvider::OpenAiCompat)
    }
}

/// AI model configuration passed to OpenClaw.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawAiConfig {
    /// Which AI provider to use.
    pub provider: AiProvider,
    /// API endpoint URL.
    pub endpoint: String,
    /// Model name / identifier.
    pub model: String,
    /// API key (stored in config; users should use env vars for production).
    #[serde(default)]
    pub api_key: String,
    /// Max tokens per response.
    pub max_tokens: u32,
    /// Sampling temperature (0.0–1.0).
    pub temperature: f32,
    /// Whether to stream responses.
    pub stream: bool,
}

impl Default for OpenClawAiConfig {
    fn default() -> Self {
        Self {
            provider:   AiProvider::Ollama,
            endpoint:   AiProvider::Ollama.default_endpoint().to_string(),
            model:      AiProvider::Ollama.default_model().to_string(),
            api_key:    String::new(),
            max_tokens: 4096,
            temperature: 0.7,
            stream:     true,
        }
    }
}

// ── Communication Channel Configuration ──────────────────────────────────────

/// A configured communication channel for OpenClaw.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Telegram,
    Discord,
    Slack,
    Matrix,
    WhatsApp,
    Signal,
    IMessage,
    MicrosoftTeams,
    WebChat,
}

impl std::fmt::Display for ChannelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelKind::Telegram       => write!(f, "Telegram"),
            ChannelKind::Discord        => write!(f, "Discord"),
            ChannelKind::Slack          => write!(f, "Slack"),
            ChannelKind::Matrix         => write!(f, "Matrix"),
            ChannelKind::WhatsApp       => write!(f, "WhatsApp"),
            ChannelKind::Signal         => write!(f, "Signal"),
            ChannelKind::IMessage       => write!(f, "iMessage"),
            ChannelKind::MicrosoftTeams => write!(f, "Microsoft Teams"),
            ChannelKind::WebChat        => write!(f, "Web Chat"),
        }
    }
}

impl ChannelKind {
    pub fn all() -> &'static [ChannelKind] {
        &[
            ChannelKind::Telegram,
            ChannelKind::Discord,
            ChannelKind::Slack,
            ChannelKind::Matrix,
            ChannelKind::WhatsApp,
            ChannelKind::Signal,
            ChannelKind::IMessage,
            ChannelKind::MicrosoftTeams,
            ChannelKind::WebChat,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ChannelKind::Telegram       => "✈",
            ChannelKind::Discord        => "🎮",
            ChannelKind::Slack          => "💬",
            ChannelKind::Matrix         => "🔷",
            ChannelKind::WhatsApp       => "📱",
            ChannelKind::Signal         => "🔒",
            ChannelKind::IMessage       => "💬",
            ChannelKind::MicrosoftTeams => "🏢",
            ChannelKind::WebChat        => "🌐",
        }
    }

    pub fn setup_hint(&self) -> &'static str {
        match self {
            ChannelKind::Telegram       => "Create a bot via @BotFather, get the token",
            ChannelKind::Discord        => "Create a bot at discord.com/developers, get token + channel ID",
            ChannelKind::Slack          => "Create a Slack App, get Bot OAuth Token + Channel ID",
            ChannelKind::Matrix         => "Register a bot account on your homeserver, get access_token + room_id",
            ChannelKind::WhatsApp       => "Requires WhatsApp Business API or Twilio",
            ChannelKind::Signal         => "Requires signal-cli installed locally",
            ChannelKind::IMessage       => "macOS only — requires BlueBubbles or Beeper",
            ChannelKind::MicrosoftTeams => "Create a Teams Bot via Azure Bot Service",
            ChannelKind::WebChat        => "Embedded web widget — no external credentials needed",
        }
    }
}

/// Configuration for a single communication channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel type.
    pub kind: ChannelKind,
    /// Whether this channel is active.
    pub enabled: bool,
    /// Primary credential (Bot Token, API Key, access_token, etc.).
    #[serde(default)]
    pub token: String,
    /// Secondary ID (Chat ID, Channel ID, room_id, etc.).
    #[serde(default)]
    pub channel_id: String,
    /// Optional webhook URL (Discord, Slack, Teams).
    #[serde(default)]
    pub webhook_url: String,
    /// Optional phone number (WhatsApp, Signal).
    #[serde(default)]
    pub phone_number: String,
    /// Human-readable label for this channel instance.
    #[serde(default)]
    pub label: String,
    /// Matrix homeserver URL (e.g. https://matrix.org).
    #[serde(default)]
    pub homeserver_url: String,
    /// Matrix bot user ID (e.g. @openclaw-bot:matrix.org).
    #[serde(default)]
    pub matrix_user_id: String,
    /// Discord guild ID (for slash-command registration).
    #[serde(default)]
    pub guild_id: String,
}

impl ChannelConfig {
    pub fn new(kind: ChannelKind) -> Self {
        let label = kind.to_string();
        Self {
            kind,
            enabled: false,
            token: String::new(),
            channel_id: String::new(),
            webhook_url: String::new(),
            phone_number: String::new(),
            label,
            homeserver_url: String::new(),
            matrix_user_id: String::new(),
            guild_id: String::new(),
        }
    }
}

/// Configuration for the active Agent runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Which agent framework to run inside the sandbox.
    pub kind: AgentKind,
    /// Path to the agent entry point (JS bundle, Python script, or WASM file).
    pub entry_path: PathBuf,
    /// Extra environment variables to inject into the agent runtime.
    pub env_vars: Vec<(String, String)>,
    /// Arguments passed to the agent on startup.
    pub args: Vec<String>,
    /// Optional working directory override (defaults to workspace_dir).
    pub working_dir: Option<PathBuf>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            kind: AgentKind::OpenClaw,
            entry_path: PathBuf::from(AgentKind::OpenClaw.default_entry()),
            env_vars: Vec::new(),
            args: Vec::new(),
            working_dir: None,
        }
    }
}

/// Runtime security configuration for the OpenClaw+ sandbox.
///
///
/// Loaded from `~/.config/openclaw-plus/config.toml` on startup.
/// If no file exists, [`SecurityConfig::default`] is written to disk and used.
///
/// All fields are serialisable so the UI can display and (in future) edit them live.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Maximum memory the sandbox may allocate, in megabytes.
    pub memory_limit_mb: u32,

    /// List of host-to-guest filesystem mount mappings.
    /// Only directories listed here are visible inside the sandbox.
    pub fs_mounts: Vec<FsMount>,

    /// Hostnames the sandbox is permitted to reach over the network.
    /// Subdomains of listed entries are also allowed.
    pub network_allowlist: Vec<String>,

    /// When `true`, all shell command execution attempts are intercepted.
    pub intercept_shell: bool,

    /// When `true`, file deletion requires explicit user confirmation.
    pub confirm_file_delete: bool,

    /// When `true`, outbound network requests to unknown hosts require confirmation.
    /// Requests to hosts in `network_allowlist` are always allowed without prompting.
    pub confirm_network: bool,

    /// When `true`, shell command execution requires explicit user confirmation.
    /// High-risk commands (e.g. `rm -rf /`) are always denied regardless of this flag.
    pub confirm_shell_exec: bool,

    /// Absolute path on the host filesystem to the OpenClaw entry script
    /// (typically the esbuild-bundled `dist/index.js`).
    pub openclaw_entry: PathBuf,

    /// Host directory mapped to `/workspace` inside the sandbox.
    /// OpenClaw reads and writes project files here.
    pub workspace_dir: PathBuf,

    /// Path where the NDJSON audit log is written.
    pub audit_log_path: PathBuf,

    /// Circuit-breaker thresholds.
    ///
    /// Controls when the circuit breaker automatically trips and terminates
    /// the sandbox (or blocks all further Skill calls in plugin mode).
    #[serde(default)]
    pub circuit_breaker: BreakerConfig,

    /// GitHub / Git operation security policy.
    #[serde(default)]
    pub github: GitHubPolicy,

    /// Active agent runtime configuration.
    #[serde(default)]
    pub agent: AgentConfig,

    /// Path to the hot-reloadable WASM policy plugin (optional).
    /// When set, the policy engine loads rules from this WASM module
    /// and reloads it automatically when the file changes.
    #[serde(default)]
    pub wasm_policy_plugin: Option<PathBuf>,

    /// Fine-grained folder access control list.
    ///
    /// Each entry specifies a host folder, an optional per-session file-count
    /// limit, write/delete permissions, and an extension filter.
    /// Folders listed here are checked *in addition to* `fs_mounts`.
    #[serde(default)]
    pub folder_access: Vec<FolderAccess>,

    /// RAG knowledge-base folders.
    ///
    /// Files in these folders are intended to be vectorized by an external
    /// tool and made searchable by the AI agent.  The agent is granted
    /// read-only access by default; `allow_agent_write` enables writing.
    #[serde(default)]
    pub rag_folders: Vec<RagFolder>,

    /// OpenClaw AI model configuration (which provider / model OpenClaw uses).
    #[serde(default)]
    pub openclaw_ai: OpenClawAiConfig,

    /// Configured communication channels (Telegram, Discord, Slack, etc.).
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
}

/// A single host-to-guest filesystem mount mapping.
///
/// Only directories explicitly listed in [`SecurityConfig::fs_mounts`] are
/// pre-opened and visible to the WASI sandbox; everything else is inaccessible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsMount {
    /// Absolute path on the host machine to expose inside the sandbox.
    pub host_path: PathBuf,

    /// Path at which `host_path` appears inside the WASI sandbox.
    pub guest_path: String,

    /// If `true`, the sandbox can read but not write to this mount.
    pub readonly: bool,
}

/// An entry in the folder access whitelist.
///
/// The AI agent is **only** permitted to access folders that appear in
/// [`SecurityConfig::folder_access`].  Any path outside these entries is
/// denied by the policy engine, regardless of `fs_mounts`.
///
/// Read access is always granted for whitelisted folders.
/// Write and delete access must be explicitly enabled per entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FolderAccess {
    /// Absolute path on the host machine that the agent is allowed to access.
    pub host_path: PathBuf,

    /// Human-readable label shown in the Settings UI (e.g. "Project Source").
    pub label: String,

    /// Allow the agent to create or modify files inside this folder.
    /// Default: `false` (read-only).
    pub allow_write: bool,

    /// Allow the agent to delete files inside this folder.
    /// Default: `false`.
    pub allow_delete: bool,

    /// Restrict access to specific file extensions (e.g. `["rs", "toml"]`).
    /// Empty list = all extensions are accessible.
    pub allowed_extensions: Vec<String>,
}

impl FolderAccess {
    /// Creates a read-only whitelist entry (no write, no delete).
    pub fn readonly(host_path: PathBuf, label: impl Into<String>) -> Self {
        Self {
            host_path,
            label: label.into(),
            allow_write: false,
            allow_delete: false,
            allowed_extensions: Vec::new(),
        }
    }

    /// Creates a read-write whitelist entry (write allowed, delete not).
    pub fn readwrite(host_path: PathBuf, label: impl Into<String>) -> Self {
        Self {
            host_path,
            label: label.into(),
            allow_write: true,
            allow_delete: false,
            allowed_extensions: Vec::new(),
        }
    }

    /// Returns `true` if `file_path` is inside this whitelisted folder.
    pub fn contains(&self, file_path: &std::path::Path) -> bool {
        file_path.starts_with(&self.host_path)
    }

    /// Returns `true` if the file's extension passes the filter (or no filter set).
    pub fn extension_allowed(&self, file_path: &std::path::Path) -> bool {
        if self.allowed_extensions.is_empty() {
            return true;
        }
        file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.allowed_extensions.iter().any(|a| a == ext))
            .unwrap_or(false)
    }
}

/// A RAG (Retrieval-Augmented Generation) knowledge-base folder.
///
/// Files added here are intended to be vectorized by an external tool
/// (e.g. LlamaIndex, Chroma, Qdrant) and made available to the AI agent
/// as a searchable knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagFolder {
    /// Absolute path on the host machine.
    pub host_path: PathBuf,

    /// Human-readable name shown in the UI.
    pub name: String,

    /// Optional description of what this knowledge base contains.
    pub description: String,

    /// File extensions to include in RAG indexing (e.g. ["md", "txt", "pdf"]).
    /// Empty = include all files.
    pub include_extensions: Vec<String>,

    /// Whether the agent may write new files into this folder
    /// (e.g. to save generated summaries).
    pub allow_agent_write: bool,

    /// Maximum total size of files in this folder (MB). `None` = unlimited.
    pub max_size_mb: Option<u32>,

    /// Whether this folder is actively being watched for changes by the vectorizer.
    pub watch_enabled: bool,
}

impl RagFolder {
    /// Creates a new RAG folder with sensible defaults.
    pub fn new(host_path: PathBuf, name: impl Into<String>) -> Self {
        Self {
            host_path,
            name: name.into(),
            description: String::new(),
            include_extensions: vec![
                "md".into(), "txt".into(), "pdf".into(),
                "rst".into(), "html".into(), "json".into(),
            ],
            allow_agent_write: false,
            max_size_mb: Some(500),
            watch_enabled: true,
        }
    }

    /// Returns `true` if the given file should be included in RAG indexing.
    pub fn should_index(&self, file_path: &std::path::Path) -> bool {
        if !file_path.starts_with(&self.host_path) {
            return false;
        }
        if self.include_extensions.is_empty() {
            return true;
        }
        file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.include_extensions.iter().any(|a| a == ext))
            .unwrap_or(false)
    }
}

impl Default for SecurityConfig {
    /// Returns a safe default configuration suitable for first-run use.
    ///
    /// - Workspace directory: `~/.openclaw-plus/workspace`
    /// - Memory limit: 512 MB
    /// - Shell interception and deletion confirmation: enabled
    /// - Network allowlist: OpenAI and Anthropic API endpoints
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let workspace = home.join(".openclaw-plus").join("workspace");
        let audit = home.join(".openclaw-plus").join("audit.log");

        Self {
            memory_limit_mb: 512,
            fs_mounts: vec![FsMount {
                host_path: workspace.clone(),
                guest_path: "/workspace".to_string(),
                readonly: false,
            }],
            network_allowlist: vec![
                "api.openai.com".to_string(),
                "api.anthropic.com".to_string(),
            ],
            intercept_shell: true,
            confirm_file_delete: true,
            confirm_network: false,
            confirm_shell_exec: true,
            openclaw_entry: PathBuf::from("openclaw/dist/index.js"),
            workspace_dir: workspace,
            audit_log_path: audit,
            circuit_breaker: BreakerConfig::default(),
            github: GitHubPolicy::default(),
            agent: AgentConfig::default(),
            wasm_policy_plugin: None,
            folder_access: Vec::new(),
            rag_folders: Vec::new(),
            openclaw_ai: OpenClawAiConfig::default(),
            channels: Vec::new(),
        }
    }
}

impl SecurityConfig {
    /// Deserialises a `SecurityConfig` from a TOML file at `path`.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or the TOML is malformed.
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Serialises this configuration to a pretty-printed TOML file at `path`.
    ///
    /// Parent directories are created automatically if they do not exist.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Loads the configuration from the platform config directory, or writes
    /// and returns the default configuration if no file exists yet.
    ///
    /// Config path: `{config_dir}/openclaw-plus/config.toml`
    pub fn load_or_default() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openclaw-plus")
            .join("config.toml");

        if config_path.exists() {
            Self::load_from_file(&config_path).unwrap_or_default()
        } else {
            let default = Self::default();
            let _ = default.save_to_file(&config_path);
            default
        }
    }

    /// Returns `true` if `host` is present in the network allowlist or is a
    /// subdomain of an allowlisted entry.
    ///
    /// # Examples
    /// ```
    /// use openclaw_security::SecurityConfig;
    /// let cfg = SecurityConfig::default();
    /// assert!(cfg.is_network_allowed("api.openai.com"));
    /// assert!(!cfg.is_network_allowed("evil.example.com"));
    /// ```
    pub fn is_network_allowed(&self, host: &str) -> bool {
        self.network_allowlist.iter().any(|allowed| {
            host == allowed || host.ends_with(&format!(".{}", allowed))
        })
    }
}
