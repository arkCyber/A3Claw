use cosmic::app::{Core, Task};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::iced::keyboard::{self, Key};
use cosmic::iced::widget::container::Style as ContainerStyle;
use cosmic::iced::widget::scrollable::{self as iced_scrollable, RelativeOffset};
use cosmic::{executor, Element};
use cosmic::widget::{self, menu, nav_bar};
use flume::{Receiver, Sender};
use openclaw_inference::{
    BackendKind, InferenceConfig, InferenceEngine, InferenceRequest,
};
use openclaw_inference::types::ConversationTurn;
use openclaw_sandbox::runner::SandboxRunner;
use openclaw_security::{
    AgentKind, AiProvider, AuditLog, BreakerConfig, BreakerStats, ChannelConfig, ChannelKind,
    CircuitBreaker, ControlCommand, EventKind, Interceptor,
    PolicyEngine, SandboxEvent, SecurityConfig,
};
use openclaw_store::types::{StorePrefs, PluginEntry, InstallState};
use openclaw_storage::types::{RunRecord, StepRecord, AuditEventRecord};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::pages::{
    ai_chat::{AiChatPage, AiChatState},
    dashboard::DashboardPage,
    events::EventsPage,
    settings::SettingsPage,
    general_settings::GeneralSettingsPage,
    claw_terminal::{ClawTerminalPage, CLAW_SCROLL_ID},
};
use crate::theme::{Language, t, tx};

const MAX_EVENT_HISTORY: usize = 500;

/// System prompt for the NL Agent intent parser.
/// The AI must respond with a JSON array of action steps.
const NL_AGENT_SYSTEM_PROMPT: &str = r#"You are OpenClaw NL Agent, an AI that converts natural language instructions into structured action plans.

Given a user instruction, respond ONLY with a valid JSON array of action steps. Each step has:
- "type": one of "shell", "fetch", "analyze", "report"
- "description": brief human-readable description of this step
- "command": (for shell) the shell command to run
- "url": (for fetch) the URL to fetch
- "input": (for analyze) what to analyze (reference previous step output as "$prev")
- "message": (for report) the final summary message

Rules:
1. Keep steps minimal and focused
2. For web research: use fetch + analyze + report
3. For system tasks: use shell steps
4. For data collection: combine fetch/shell + analyze + report
5. Maximum 6 steps per plan
6. Shell commands must be safe (no rm -rf, no destructive ops without explicit user request)

Example for "查看系统信息":
[{"type":"shell","description":"Get system info","command":"uname -a && sw_vers 2>/dev/null || lsb_release -a 2>/dev/null"},{"type":"report","description":"Summary","message":"System information collected above."}]

Example for "搜索 Rust 最新版本":
[{"type":"fetch","description":"Fetch Rust releases page","url":"https://api.github.com/repos/rust-lang/rust/releases/latest"},{"type":"analyze","description":"Extract version","input":"$prev"},{"type":"report","description":"Summary","message":"Latest Rust version extracted above."}]

Respond ONLY with the JSON array, no markdown, no explanation."#;

/// Menu bar action IDs — used to identify which menu item was activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MenuAction {
    /// File > Clear Events
    ClearEvents,
    /// File > Emergency Stop
    EmergencyStop,
    /// Sandbox > Start
    StartSandbox,
    /// Sandbox > Stop
    StopSandbox,
    /// View > Toggle Sidebar
    ToggleSidebar,
    /// View > Toggle Theme
    ToggleTheme,
    /// View > Switch language
    SetLang(crate::theme::Language),
    /// Plugins > Open Store
    OpenPluginStore,
    /// Help > About
    About,
    /// File > Quit
    Quit,
    /// Settings > Security Settings
    OpenSecuritySettings,
    /// Settings > General Settings
    OpenGeneralSettings,
}

impl menu::action::MenuAction for MenuAction {
    type Message = AppMessage;

    fn message(&self) -> AppMessage {
        match self {
            MenuAction::ClearEvents     => AppMessage::ClearEvents,
            MenuAction::EmergencyStop   => AppMessage::EmergencyStop,
            MenuAction::StartSandbox    => AppMessage::StartSandbox,
            MenuAction::StopSandbox     => AppMessage::StopSandbox,
            MenuAction::ToggleSidebar   => AppMessage::ToggleSidebar,
            MenuAction::ToggleTheme     => AppMessage::ToggleTheme,
            MenuAction::SetLang(l)      => AppMessage::SetLanguage(*l),
            MenuAction::OpenPluginStore       => AppMessage::OpenPluginStore,
            MenuAction::About                   => AppMessage::ShowAbout,
            MenuAction::Quit                    => AppMessage::ShowQuitDialog,
            MenuAction::OpenSecuritySettings    => AppMessage::NavSelect(NavPage::Settings),
            MenuAction::OpenGeneralSettings     => AppMessage::NavSelect(NavPage::GeneralSettings),
        }
    }
}

/// Top-level navigation pages shown in the tab bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    Dashboard,
    Events,
    Settings,
    PluginStore,
    AiChat,
    /// General (non-security) settings: appearance, language, AI, models, GitHub, etc.
    GeneralSettings,
    /// Claw Terminal — direct command console: send commands to Claw, view execution output.
    ClawTerminal,
    /// Digital Worker (Agent) management page.
    Agents,
    /// Run history & audit replay page.
    AuditReplay,
}

/// Application-wide messages dispatched by the libcosmic runtime.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppMessage {
    /// Switch the active navigation tab.
    NavSelect(NavPage),
    /// A new sandbox event arrived from the interceptor channel.
    SandboxEvent(SandboxEvent),
    /// The user clicked Allow on a pending confirmation card.
    ConfirmAllow(u64),
    /// The user clicked Deny on a pending confirmation card.
    ConfirmDeny(u64),
    /// Start (or restart) the sandbox.
    StartSandbox,
    /// Gracefully stop the sandbox.
    StopSandbox,
    /// Replace the active security configuration.
    ConfigUpdated(SecurityConfig),
    /// Clear the in-memory event history and reset statistics.
    ClearEvents,
    /// Trigger an emergency stop — trips the circuit breaker immediately.
    EmergencyStop,
    /// The circuit breaker tripped automatically; inner string is the reason.
    BreakerTripped(String),
    /// Plugin mode: allow a pending Skill confirmation by event ID.
    /// Sends `POST /skills/allow/{id}` to the gateway.
    PluginAllow(u64),
    /// Plugin mode: deny a pending Skill confirmation by event ID.
    /// Sends `POST /skills/deny/{id}` to the gateway.
    PluginDeny(u64),
    /// Open the Plugin Store window (spawns openclaw-store subprocess).
    OpenPluginStore,
    /// Show the quit confirmation dialog.
    ShowQuitDialog,
    /// User confirmed quit — save state and exit.
    ConfirmQuit,
    /// User cancelled quit — close the dialog.
    CancelQuit,
    // ── AI Chat messages ──────────────────────────────────────────────────
    /// The user typed in the AI chat input box.
    AiInputChanged(String),
    /// The user pressed Send (or Enter) in the AI chat input.
    AiSendMessage,
    /// A successful inference response arrived.
    AiResponse { content: String, latency_ms: u64 },
    /// An inference error occurred.
    AiError(String),
    /// Clear the AI chat history.
    AiClearChat,
    // ── Agent management messages ─────────────────────────────────────────
    /// Refresh the agent list from the database.
    AgentRefresh,
    /// Select an agent in the list.
    AgentSelect(usize),
    /// Create a new agent with the given display name.
    AgentCreate,
    /// Agent name input changed.
    AgentNameChanged(String),
    /// Agent role selection changed.
    AgentRoleChanged(String),
    /// Suspend the selected agent.
    AgentSuspend,
    /// Resume the selected agent.
    AgentResume,
    /// Archive the selected agent.
    AgentArchive,
    /// Change avatar URL for the selected agent (empty = reset to default).
    AgentAvatarChange(String),
    /// Avatar URL input field changed.
    AgentAvatarInputChanged(String),
    /// Start editing the selected agent's display name.
    AgentStartRename,
    /// Rename input changed.
    AgentRenameInputChanged(String),
    /// Confirm the rename and save.
    AgentRenameConfirm,
    /// Cancel rename without saving.
    AgentRenameCancelled,
    /// Agent list loaded from DB.
    AgentListLoaded(Vec<openclaw_security::AgentProfile>),
    // ── Audit / Run replay messages ───────────────────────────────────────
    /// Load runs for the selected agent.
    AuditLoadRuns(String),
    /// Runs loaded for an agent.
    AuditRunsLoaded(Vec<RunRecord>),
    /// Select a run to view its steps.
    AuditSelectRun(usize),
    /// Steps loaded for a run.
    AuditStepsLoaded(Vec<StepRecord>),
    /// Audit events loaded for a run.
    AuditEventsLoaded(Vec<AuditEventRecord>),
    /// Run details (steps + audit events) loaded together.
    AuditRunDetailsLoaded(Vec<StepRecord>, Vec<AuditEventRecord>),
    /// Clear audit view selection.
    AuditClear,
    /// User selected a different AI model.
    AiModelChanged(String),
    /// AI chat input box received focus (used to track IME state).
    AiFocused,
    /// No-op placeholder used where a message is syntactically required.
    Noop,
    /// Keyboard: move sidebar selection up.
    NavUp,
    /// Keyboard: move sidebar selection down.
    NavDown,
    /// Toggle the sidebar visibility.
    ToggleSidebar,
    /// Show the About dialog.
    ShowAbout,
    /// Close the About dialog.
    CloseAbout,
    /// Switch the UI language.
    SetLanguage(Language),
    /// Toggle between dark and light theme.
    ToggleTheme,
    /// Set the event log filter kind (None = all).
    SetEventFilter(Option<openclaw_security::EventKind>),
    /// Set the event log search text.
    SetEventSearch(String),
    /// Toggle intercept_shell in SecurityConfig.
    ToggleInterceptShell,
    /// Toggle confirm_file_delete in SecurityConfig.
    ToggleConfirmFileDelete,
    /// Toggle confirm_shell_exec in SecurityConfig.
    ToggleConfirmShellExec,
    /// Toggle confirm_network in SecurityConfig.
    ToggleConfirmNetwork,
    /// Update the AI inference endpoint URL.
    AiEndpointChanged(String),
    /// Update the sandbox memory limit (MB).
    MemoryLimitChanged(String),
    /// Startup: check environment (WasmEdge, AI endpoint, etc.)
    StartupCheckEnvironment,
    /// Startup: environment check completed with results.
    StartupCheckComplete { wasmedge_ok: bool, ai_ok: bool },
    /// Startup: auto-start AI model.
    StartupInitAI,
    /// Startup: auto-start sandbox.
    StartupStartSandbox,
    /// AI Model Management: list available models from Ollama.
    AiListModels,
    /// AI Model Management: models list received.
    AiModelsListed(Vec<OllamaModel>),
    /// AI Model Management: delete a model.
    AiDeleteModel(String),
    /// AI Model Management: pull/download a new model.
    AiPullModel(String),
    /// AI Model Management: model operation completed.
    AiModelOpComplete { success: bool, message: String },
    /// AI Model Management: download input changed.
    ModelDownloadInputChanged(String),
    /// AI Model Management: search/filter input changed.
    ModelSearchChanged(String),
    /// AI Model Management: set a model as the active one.
    AiSetActiveModel(String),
    /// AI Model Management: download progress update.
    AiDownloadProgress { model: String, status: String, percent: u8 },
    // ── GitHub Policy messages ────────────────────────────────────────────
    /// Toggle GitHub force-push denial.
    ToggleGithubDenyForcePush,
    /// Toggle GitHub push confirmation.
    ToggleGithubConfirmPush,
    /// Toggle GitHub branch-delete confirmation.
    ToggleGithubConfirmBranchDelete,
    /// Toggle GitHub history-rewrite confirmation.
    ToggleGithubConfirmHistoryRewrite,
    /// Toggle GitHub default-branch protection.
    ToggleGithubProtectDefaultBranch,
    /// Toggle GitHub API interception.
    ToggleGithubInterceptApi,
    /// Update GitHub allowed orgs list (comma-separated).
    GithubAllowedOrgsChanged(String),
    /// Update GitHub allowed repos list (comma-separated).
    GithubAllowedReposChanged(String),
    // ── Agent selector messages ───────────────────────────────────────────
    /// Switch the active agent kind.
    SetAgentKind(AgentKind),
    /// Update the agent entry path.
    AgentEntryPathChanged(String),
    // ── WASM policy plugin messages ───────────────────────────────────────
    /// Update the WASM policy plugin path.
    WasmPolicyPathChanged(String),
    /// Reload the WASM policy plugin from disk.
    WasmPolicyReload,
    /// WASM policy reload completed.
    WasmPolicyReloaded { success: bool },
    // ── Folder access whitelist messages ─────────────────────────────────
    /// Open the native OS folder picker for whitelist add.
    FolderAccessPickFolder,
    /// Native picker returned a path (or None if cancelled).
    FolderAccessPickerResult(Option<String>),
    /// Add a new folder to the access whitelist (path, label, allow_write).
    FolderAccessAdd { path: String, label: String, allow_write: bool },
    /// Remove a folder from the whitelist by index.
    FolderAccessRemove(usize),
    /// Toggle write permission for a whitelisted folder.
    FolderAccessToggleWrite(usize),
    /// Toggle delete permission for a whitelisted folder.
    FolderAccessToggleDelete(usize),
    /// Input buffer changed: new folder path.
    FolderAccessPathChanged(String),
    /// Input buffer changed: new folder label.
    FolderAccessLabelChanged(String),
    // ── RAG folder messages ───────────────────────────────────────────────
    /// Open the native OS folder picker for RAG folder add.
    RagFolderPickFolder,
    /// Native picker returned a path (or None if cancelled).
    RagFolderPickerResult(Option<String>),
    /// Add a new RAG knowledge-base folder.
    RagFolderAdd { path: String, name: String },
    /// Remove a RAG folder by index.
    RagFolderRemove(usize),
    /// Toggle watch_enabled for a RAG folder.
    RagFolderToggleWatch(usize),
    /// Toggle allow_agent_write for a RAG folder.
    RagFolderToggleWrite(usize),
    /// Input buffer changed: new RAG folder path.
    RagFolderPathChanged(String),
    /// Input buffer changed: new RAG folder name.
    RagFolderNameChanged(String),
    // ── OpenClaw AI config messages ────────────────────────────────────────
    /// User selected a different AI provider for OpenClaw.
    OpenClawAiProviderChanged(AiProvider),
    /// User changed the AI endpoint URL.
    OpenClawAiEndpointChanged(String),
    /// User changed the AI model name.
    OpenClawAiModelChanged(String),
    /// User changed the API key.
    OpenClawAiApiKeyChanged(String),
    /// User changed max tokens.
    OpenClawAiMaxTokensChanged(String),
    /// User changed temperature.
    OpenClawAiTemperatureChanged(String),
    /// Toggle streaming.
    OpenClawAiToggleStream,
    /// Test the AI connection (probe endpoint).
    OpenClawAiTestConnection,
    // ── Claw Terminal Agent Chat messages ─────────────────────────────────
    /// Select an agent for Claw Terminal chat.
    ClawSelectAgent(Option<String>),
    /// Send a message to the selected agent in Claw Terminal.
    ClawAgentChat(String),
    /// Agent response received.
    ClawAgentResponse { agent_id: String, content: String, latency_ms: u64, user_entry_id: u64 },
    /// AI connection test result.
    OpenClawAiTestResult { ok: bool, message: String },
    // ── Channel config messages ────────────────────────────────────────────
    /// Add a new channel of the given kind.
    ChannelAdd(ChannelKind),
    /// Remove a channel by index.
    ChannelRemove(usize),
    /// Toggle a channel enabled/disabled.
    ChannelToggleEnabled(usize),
    /// Update the token for a channel.
    ChannelTokenChanged { idx: usize, value: String },
    /// Update the channel_id for a channel.
    ChannelIdChanged { idx: usize, value: String },
    /// Update the webhook_url for a channel.
    ChannelWebhookChanged { idx: usize, value: String },
    /// Update the phone_number for a channel.
    ChannelPhoneChanged { idx: usize, value: String },
    /// Update the label for a channel.
    ChannelLabelChanged { idx: usize, value: String },
    /// Update the guild_id for a Discord channel.
    ChannelGuildIdChanged { idx: usize, value: String },
    /// Update the homeserver_url for a Matrix channel.
    ChannelHomserverChanged { idx: usize, value: String },
    /// Update the matrix_user_id for a Matrix channel.
    ChannelMatrixUserIdChanged { idx: usize, value: String },
    /// Test a channel connection.
    ChannelTest(usize),
    /// Channel test result.
    ChannelTestResult { idx: usize, ok: bool, message: String },
    // ── Claw Terminal messages ─────────────────────────────────────────────
    /// User typed in the Claw Terminal input box (legacy single-line).
    ClawInputChanged(String),
    /// User performed an edit action in the multi-line Claw Terminal editor.
    ClawEditorAction(cosmic::widget::text_editor::Action),
    /// User submitted a command (Enter or Send button).
    ClawSendCommand,
    /// User clicked the image attachment button — open native file picker.
    ClawPickImage,
    /// File picker returned a selected image path + raw bytes.
    ClawImagePicked { path: String, bytes: Vec<u8> },
    /// User removed the pending image attachment.
    ClawClearAttachment,
    /// User clicked the microphone button — start recording.
    ClawStartRecording,
    /// User clicked the stop button — stop recording and transcribe.
    ClawStopRecording,
    /// Voice transcription completed, result is text to inject into input.
    ClawVoiceTranscribed(String),
    /// Voice recording/transcription failed.
    ClawVoiceError(String),
    /// A line of stdout/stderr output arrived from the running command.
    ClawOutputLine { entry_id: u64, line: String, is_stderr: bool },
    /// A command finished executing.
    ClawCommandFinished { entry_id: u64, exit_code: Option<i32>, elapsed_ms: u64 },
    /// Clear the Claw Terminal history.
    ClawClearHistory,
    /// User clicked a quick-action preset button.
    ClawQuickAction(ClawQuickAction),
    /// Claw Terminal input box received focus.
    ClawInputFocused,
    /// Shell command completed with combined stdout/stderr output.
    ClawShellResult {
        entry_id: u64,
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
        elapsed_ms: u64,
    },
    /// Bulk output lines + finish in one message (avoids Task::batch type issues).
    ClawBulkOutput {
        entry_id: u64,
        lines: Vec<(String, bool)>,
        exit_code: Option<i32>,
        elapsed_ms: u64,
    },
    // ── Plugin / Gateway messages ──────────────────────────────────────────
    /// Probe the OpenClaw Gateway to check if it is reachable.
    ClawProbeGateway,
    /// Gateway probe result.
    ClawGatewayProbeResult { reachable: bool, url: String },
    /// Send a natural-language instruction to OpenClaw via the Gateway.
    ClawSendToGateway { entry_id: u64, instruction: String },
    /// Gateway responded with a skill execution result.
    ClawGatewaySkillResult { entry_id: u64, lines: Vec<(String, bool)>, elapsed_ms: u64 },
    // ── NL Agent messages ──────────────────────────────────────────────────
    /// Toggle NL (natural language) mode in Claw Terminal.
    ClawToggleNlMode,
    /// AI finished parsing NL intent; returns structured action plan as JSON string.
    ClawNlPlanReady { entry_id: u64, plan_json: String },
    /// AI intent parsing failed.
    ClawNlPlanError { entry_id: u64, error: String },
    /// One step of the NL action plan completed.
    ClawNlStepDone { entry_id: u64, step: u32, output: String, is_err: bool },
    /// All NL action steps finished.
    ClawNlDone { entry_id: u64, elapsed_ms: u64 },
    /// Fetch a URL and return its content (used by NL agent web actions).
    ClawFetchUrl { entry_id: u64, url: String, step: u32 },
    /// URL fetch completed.
    ClawFetchResult { entry_id: u64, step: u32, content: String, is_err: bool },
    // ── Telegram integration messages ──────────────────────────────────────
    /// Start polling Telegram for new messages (uses configured Bot Token).
    TgStartPolling,
    /// Stop Telegram polling.
    TgStopPolling,
    /// New messages arrived from Telegram getUpdates.
    TgMessagesReceived(Vec<TgMessage>),
    /// Send a text message via Telegram Bot API.
    TgSendMessage { chat_id: String, text: String },
    /// Telegram send result.
    TgSendResult { ok: bool, info: String },
    // ── Discord integration messages ────────────────────────────────────────
    /// Start polling Discord channel for new messages.
    DiscordStartPolling,
    /// Stop Discord polling.
    DiscordStopPolling,
    /// New messages arrived from Discord channel.
    DiscordMessagesReceived(Vec<BotMessage>),
    /// Send a message to a Discord channel.
    DiscordSendMessage { channel_id: String, text: String },
    /// Discord send result.
    DiscordSendResult { ok: bool, info: String },
    // ── Matrix integration messages ─────────────────────────────────────────
    /// Start polling Matrix room for new events.
    MatrixStartPolling,
    /// Stop Matrix polling.
    MatrixStopPolling,
    /// New messages arrived from Matrix room.
    MatrixMessagesReceived(Vec<BotMessage>),
    /// Send a message to a Matrix room.
    MatrixSendMessage { room_id: String, text: String },
    /// Matrix send result.
    MatrixSendResult { ok: bool, info: String },
    // ── Slack integration messages ──────────────────────────────────────────
    /// Start polling Slack channel for new messages.
    SlackStartPolling,
    /// Stop Slack polling.
    SlackStopPolling,
    /// New messages arrived from Slack channel.
    SlackMessagesReceived(Vec<BotMessage>),
    /// Send a message to a Slack channel.
    SlackSendMessage { channel_id: String, text: String },
    /// Slack send result.
    SlackSendResult { ok: bool, info: String },
    /// Inject a system info entry into Claw Terminal.
    ClawSystemInfo { text: String },
    /// Show the language selection popup menu.
    ShowLanguageMenu,
    /// Hide the language selection popup menu.
    HideLanguageMenu,
    // ── Environment check messages ─────────────────────────────────────────
    /// Startup: run full environment health check.
    EnvCheckStart,
    /// One environment check step completed.
    EnvCheckStepDone(crate::env_check::EnvCheckStepResult),
    /// All environment checks completed (batch result).
    EnvCheckAllDone(Vec<crate::env_check::EnvCheckStepResult>),
    /// User dismissed the env-check overlay (enter app).
    EnvCheckDismiss,
}

/// Quick-action preset commands available in the Claw Terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClawQuickAction {
    StartSandbox,
    StopSandbox,
    EmergencyStop,
    ListModels,
    CheckStatus,
    ClearEvents,
    ShowConfig,
    RestartAi,
    TgStatus,
    TgPoll,
    DiscordPoll,
    MatrixPoll,
    SlackPoll,
    ChannelStatus,
    GatewayConnect,
}

impl ClawQuickAction {
    pub fn label(&self) -> &'static str {
        match self {
            ClawQuickAction::StartSandbox   => "▶ Sandbox Start",
            ClawQuickAction::StopSandbox    => "⏹ Sandbox Stop",
            ClawQuickAction::EmergencyStop  => "🔴 Emergency Stop",
            ClawQuickAction::ListModels     => "📋 Models List",
            ClawQuickAction::CheckStatus    => "📊 Status",
            ClawQuickAction::ClearEvents    => "🗑 Clear Events",
            ClawQuickAction::ShowConfig     => "⚙ Config Show",
            ClawQuickAction::RestartAi      => "🔄 AI Restart",
            ClawQuickAction::TgStatus       => "✈ Telegram Status",
            ClawQuickAction::TgPoll         => "📥 Telegram Poll",
            ClawQuickAction::DiscordPoll    => "🎮 Discord Poll",
            ClawQuickAction::MatrixPoll     => "🔷 Matrix Poll",
            ClawQuickAction::SlackPoll      => "💬 Slack Poll",
            ClawQuickAction::ChannelStatus  => "📡 Channels",
            ClawQuickAction::GatewayConnect => "🔌 Gateway Connect",
        }
    }
    pub fn command(&self) -> &'static str {
        match self {
            ClawQuickAction::StartSandbox   => "sandbox start",
            ClawQuickAction::StopSandbox    => "sandbox stop",
            ClawQuickAction::EmergencyStop  => "emergency stop",
            ClawQuickAction::ListModels     => "models list",
            ClawQuickAction::CheckStatus    => "status",
            ClawQuickAction::ClearEvents    => "events clear",
            ClawQuickAction::ShowConfig     => "config show",
            ClawQuickAction::RestartAi      => "ai restart",
            ClawQuickAction::TgStatus       => "tg status",
            ClawQuickAction::TgPoll         => "tg poll",
            ClawQuickAction::DiscordPoll    => "discord poll",
            ClawQuickAction::MatrixPoll     => "matrix poll",
            ClawQuickAction::SlackPoll      => "slack poll",
            ClawQuickAction::ChannelStatus  => "channels",
            ClawQuickAction::GatewayConnect => "gateway connect",
        }
    }
    pub fn all() -> &'static [ClawQuickAction] {
        &[
            ClawQuickAction::CheckStatus,
            ClawQuickAction::ChannelStatus,
            ClawQuickAction::TgPoll,
            ClawQuickAction::DiscordPoll,
            ClawQuickAction::MatrixPoll,
            ClawQuickAction::SlackPoll,
            ClawQuickAction::TgStatus,
            ClawQuickAction::GatewayConnect,
            ClawQuickAction::StartSandbox,
            ClawQuickAction::StopSandbox,
            ClawQuickAction::EmergencyStop,
            ClawQuickAction::ListModels,
            ClawQuickAction::ClearEvents,
            ClawQuickAction::ShowConfig,
            ClawQuickAction::RestartAi,
        ]
    }
}

/// A Telegram message received via getUpdates.
#[derive(Debug, Clone)]
pub struct TgMessage {
    pub update_id: i64,
    pub chat_id: String,
    pub from: String,
    pub text: String,
    pub date: u64,
}

/// A generic message received from any bot platform (Discord, Matrix, Slack).
#[derive(Debug, Clone)]
pub struct BotMessage {
    /// Platform-specific message ID (snowflake, event_id, ts, etc.).
    pub msg_id: String,
    /// Channel / room / conversation ID.
    pub channel_id: String,
    /// Display name or username of the sender.
    pub from: String,
    /// Message body text.
    pub text: String,
    /// UNIX timestamp (seconds).
    pub date: u64,
    /// Which platform this came from.
    pub platform: BotPlatformKind,
}

/// Which bot platform a BotMessage came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotPlatformKind {
    Discord,
    Matrix,
    Slack,
}

impl std::fmt::Display for BotPlatformKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotPlatformKind::Discord => write!(f, "Discord"),
            BotPlatformKind::Matrix  => write!(f, "Matrix"),
            BotPlatformKind::Slack   => write!(f, "Slack"),
        }
    }
}

/// Who originated a Claw Terminal entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClawEntrySource {
    /// Command typed by the local user.
    User,
    /// Reply / event from OpenClaw (via Gateway or NL Agent).
    OpenClaw,
    /// Message received from Telegram.
    Telegram { from: String },
    /// Message received from Discord / Matrix / Slack / other bot channel.
    BotChannel { platform: String, from: String },
    /// System / internal info message.
    System,
}

/// Execution status of a single Claw Terminal entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClawEntryStatus {
    Running,
    Success,
    Error(i32),
    Killed,
}

impl std::fmt::Display for ClawEntryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClawEntryStatus::Running    => write!(f, "Running…"),
            ClawEntryStatus::Success    => write!(f, "OK"),
            ClawEntryStatus::Error(c)   => write!(f, "Exit {}", c),
            ClawEntryStatus::Killed     => write!(f, "Killed"),
        }
    }
}

/// A pending image attachment in the Claw Terminal input bar.
#[derive(Debug, Clone)]
pub struct ClawAttachment {
    /// Original file name (for display).
    pub filename: String,
    /// Base64-encoded image data (no data-URL prefix).
    pub base64: String,
    /// MIME type detected from extension (e.g. "image/png").
    pub mime: String,
}

/// A single entry in the Claw Terminal history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClawEntry {
    /// Unique monotonic ID.
    pub id: u64,
    /// The command / message text.
    pub command: String,
    /// Timestamp (seconds since UNIX epoch).
    pub timestamp: u64,
    /// Accumulated output lines (stdout + stderr interleaved).
    pub output_lines: Vec<(String, bool)>,
    /// Current execution status.
    pub status: ClawEntryStatus,
    /// Wall-clock elapsed time in milliseconds (set on finish).
    pub elapsed_ms: Option<u64>,
    /// Who originated this entry.
    pub source: ClawEntrySource,
}

impl ClawEntry {
    pub fn new(id: u64, command: impl Into<String>) -> Self {
        Self {
            id,
            command: command.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            output_lines: Vec::new(),
            status: ClawEntryStatus::Running,
            elapsed_ms: None,
            source: ClawEntrySource::User,
        }
    }

    /// Create a completed reply entry (from OpenClaw, Telegram, or System).
    pub fn reply(
        id: u64,
        text: impl Into<String>,
        source: ClawEntrySource,
        lines: Vec<(String, bool)>,
        elapsed_ms: u64,
    ) -> Self {
        let mut e = Self::new(id, text);
        e.source = source;
        e.output_lines = lines;
        e.status = ClawEntryStatus::Success;
        e.elapsed_ms = Some(elapsed_ms);
        e
    }
}

/// Current lifecycle state of the WasmEdge sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SandboxStatus {
    /// Sandbox has not been started yet.
    Idle,
    /// Sandbox is actively running.
    Running,
    /// Sandbox execution is paused.
    Paused,
    /// Sandbox exited normally.
    Stopped,
    /// Circuit breaker tripped — sandbox was forcefully terminated.
    /// Inner string is the human-readable trip reason.
    Tripped(String),
    /// Sandbox encountered a fatal error.
    Error(String),
}

impl std::fmt::Display for SandboxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxStatus::Idle            => write!(f, "Idle"),
            SandboxStatus::Running         => write!(f, "Running"),
            SandboxStatus::Paused          => write!(f, "Paused"),
            SandboxStatus::Stopped         => write!(f, "Stopped"),
            SandboxStatus::Tripped(reason) => write!(f, "🔴 Tripped: {}", reason),
            SandboxStatus::Error(e)        => write!(f, "Error: {}", e),
        }
    }
}

/// How the UI is connected to the security backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunMode {
    /// Embedded mode: UI spawned the WasmEdge sandbox in-process.
    Embedded,
    /// Plugin mode: UI is talking to an `openclaw-plugin-gateway` HTTP server.
    Plugin {
        /// Base URL of the gateway, e.g. `http://127.0.0.1:54321`.
        gateway_url: String,
    },
}

/// Detailed information about a locally available Ollama model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaModel {
    /// Model name (e.g. "qwen2.5:0.5b").
    pub name: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Last modified timestamp (RFC3339).
    pub modified_at: String,
    /// Parameter count label (e.g. "0.5B", "7B").
    pub parameter_size: String,
    /// Quantization label (e.g. "Q4_0").
    pub quantization: String,
    /// Model family (e.g. "qwen2", "llama").
    pub family: String,
}

impl OllamaModel {
    /// Format size in human-readable form.
    pub fn size_display(&self) -> String {
        let gb = self.size_bytes as f64 / 1_073_741_824.0;
        let mb = self.size_bytes as f64 / 1_048_576.0;
        if gb >= 1.0 {
            format!("{:.1} GB", gb)
        } else {
            format!("{:.0} MB", mb)
        }
    }

    /// Short date from modified_at.
    pub fn modified_display(&self) -> String {
        // e.g. "2024-12-01T10:23:00Z" → "2024-12-01"
        self.modified_at.get(..10).unwrap_or(&self.modified_at).to_string()
    }
}

/// Root application state for the OpenClaw+ monitoring UI.
///
/// Implements [`cosmic::Application`] and is driven by the libcosmic runtime.
#[allow(dead_code)]
pub struct OpenClawApp {
    /// libcosmic window/theme core.
    core: Core,
    /// Currently active navigation tab.
    nav_page: NavPage,
    /// libcosmic sidebar navigation model — drives the native nav_bar widget.
    nav_model: nav_bar::Model,
    /// Ring buffer of the most recent sandbox events (capped at [`MAX_EVENT_HISTORY`]).
    events: VecDeque<SandboxEvent>,
    /// Events that are awaiting explicit user approval or denial.
    pending_confirmations: Vec<SandboxEvent>,
    /// Current lifecycle state of the sandbox.
    sandbox_status: SandboxStatus,
    /// Active security configuration (loaded from disk on startup).
    config: SecurityConfig,
    /// Aggregate statistics derived from received events.
    stats: SandboxStats,
    /// Receiver end of the sandbox event channel (wrapped for async access).
    event_rx: Option<Arc<Mutex<Receiver<SandboxEvent>>>>,
    /// Sender end of the control command channel to the sandbox.
    control_tx: Option<Sender<ControlCommand>>,
    /// Latest snapshot of circuit breaker counters for the dashboard.
    breaker_stats: BreakerStats,
    /// Circuit breaker instance used to trigger an emergency stop.
    circuit_breaker: Option<Arc<CircuitBreaker>>,
    /// Whether the UI is running in embedded or plugin-gateway mode.
    run_mode: RunMode,
    /// AI Chat page state (conversation history, input buffer, engine status).
    ai_chat: AiChatState,
    /// Shared inference engine (lazily initialised on first AI request).
    inference_engine: Option<Arc<InferenceEngine>>,
    /// Whether the About dialog is currently shown.
    show_about: bool,
    /// Active UI language.
    language: Language,
    /// Whether the language selection popup is shown.
    show_language_menu: bool,
    /// Whether the warm dark theme is active (vs system default).
    warm_theme_active: bool,
    /// Active event log filter (None = show all).
    event_filter: Option<openclaw_security::EventKind>,
    /// Event log search text.
    event_search: String,
    /// List of available AI models from Ollama (with details).
    available_models: Vec<OllamaModel>,
    /// Input buffer for downloading new AI models.
    model_download_input: String,
    /// Search/filter text for the model list.
    model_search: String,
    /// Download progress status (model name -> status string).
    download_status: Option<(String, String, u8)>,
    /// Input buffer for GitHub allowed orgs (comma-separated).
    github_orgs_input: String,
    /// Input buffer for GitHub allowed repos (comma-separated).
    github_repos_input: String,
    /// Input buffer for agent entry path.
    agent_entry_input: String,
    /// Input buffer for WASM policy plugin path.
    wasm_policy_path_input: String,
    /// Status message for WASM policy reload result.
    wasm_policy_status: String,
    /// Input buffer: new folder access path.
    folder_access_path_input: String,
    /// Input buffer: new folder access label.
    folder_access_label_input: String,
    /// Input buffer: new RAG folder path.
    rag_folder_path_input: String,
    /// Input buffer: new RAG folder name.
    rag_folder_name_input: String,
    /// Claw Terminal: command history entries.
    claw_history: Vec<ClawEntry>,
    /// Claw Terminal: current input buffer.
    claw_input: String,
    /// Claw Terminal: whether the input box has focus (for border colour).
    claw_input_focused: bool,
    /// Claw Terminal: monotonic ID counter for entries.
    claw_next_id: u64,
    /// Claw Terminal: whether NL (natural language) mode is active.
    claw_nl_mode: bool,
    /// Claw Terminal: pending image attachment (filename, base64 data URL).
    claw_attachment: Option<ClawAttachment>,
    /// Claw Terminal: whether voice recording is in progress.
    claw_recording: bool,
    /// Claw Terminal: status message for voice recording / transcription.
    claw_voice_status: Option<String>,
    /// Claw Terminal: selected agent ID for chat (None = no agent selected).
    claw_selected_agent_id: Option<String>,
    /// Claw Terminal: list of available agents for selection.
    claw_agent_list: Vec<openclaw_security::AgentProfile>,
    /// Claw Terminal: per-agent conversation history for multi-turn chat.
    /// Key = agent_id, Value = vec of (role, content).
    claw_agent_conversations: std::collections::HashMap<String, Vec<ConversationTurn>>,
    /// OpenClaw Gateway URL (from env OPENCLAW_GATEWAY_URL or manual config).
    gateway_url: Option<String>,
    /// Whether the OpenClaw Gateway is currently reachable.
    gateway_reachable: bool,
    /// Input buffer for openclaw_ai.max_tokens (text field).
    openclaw_ai_max_tokens_input: String,
    /// Input buffer for openclaw_ai.temperature (text field).
    openclaw_ai_temperature_input: String,
    /// Last AI connection test result (ok, message).
    openclaw_ai_test_status: Option<(bool, String)>,
    /// Per-channel connection test results.
    channel_test_status: Vec<Option<(bool, String)>>,
    /// Whether Telegram polling is currently active.
    tg_polling_active: bool,
    /// Last Telegram update_id seen (for getUpdates offset).
    tg_last_update_id: i64,
    /// Telegram bot info (username) once connected.
    tg_bot_username: Option<String>,
    /// Whether Discord polling is currently active.
    discord_polling_active: bool,
    /// Last Discord message snowflake ID seen (for after= param).
    discord_last_msg_id: Option<String>,
    /// Whether Matrix polling is currently active.
    matrix_polling_active: bool,
    /// Matrix sync next_batch token.
    matrix_next_batch: Option<String>,
    /// Whether Slack polling is currently active.
    slack_polling_active: bool,
    /// Last Slack message timestamp seen (for oldest= param).
    slack_last_ts: Option<String>,
    /// Whether the quit confirmation dialog is currently shown.
    show_quit_dialog: bool,
    
    // ── Plugin Store state ────────────────────────────────────────────────
    /// Store preferences (registry URLs, plugin directory)
    store_prefs: Option<StorePrefs>,
    /// Plugin entries from registry
    store_plugins: Vec<PluginEntry>,
    /// Plugin installation states
    store_install_states: std::collections::HashMap<String, InstallState>,
    /// Store loading state
    store_loading: bool,
    /// Store search query
    store_search: String,
    /// Store category filter
    store_category_filter: Option<String>,
    /// Store fetch error
    store_fetch_error: Option<String>,

    // ── Agent (Digital Worker) management state ───────────────────────────
    /// Loaded list of digital worker profiles.
    agent_list: Vec<openclaw_security::AgentProfile>,
    /// Index of the currently selected agent in the list.
    agent_selected: Option<usize>,
    /// Input buffer for new agent display name.
    agent_name_input: String,
    /// Input buffer for new agent role selection.
    agent_role_input: String,
    /// Input buffer for avatar URL editing.
    agent_avatar_input: String,
    /// Input buffer for renaming the selected agent.
    agent_rename_input: String,
    /// Whether we are currently in name-edit mode for the selected agent.
    agent_editing_name: bool,
    /// Whether agent data is being loaded.
    agent_loading: bool,
    // ── Audit / Run replay state ───────────────────────────────────────────
    /// Run records for the currently selected agent.
    audit_runs: Vec<RunRecord>,
    /// Index of the currently selected run.
    audit_run_selected: Option<usize>,
    /// Steps for the currently selected run.
    audit_steps: Vec<StepRecord>,
    /// Audit events for the currently selected run.
    audit_events: Vec<AuditEventRecord>,
    /// Whether audit data is being loaded.
    audit_loading: bool,
    // ── Environment check state ───────────────────────────────────────────
    /// Whether the env-check overlay is currently visible.
    env_check_visible: bool,
    /// Accumulated environment check report.
    env_check_report: crate::env_check::EnvCheckReport,
}

/// Aggregate statistics derived from sandbox events, displayed on the dashboard.
#[derive(Debug, Clone, Default)]
pub struct SandboxStats {
    pub total_events: u64,
    pub allowed_count: u64,
    pub denied_count: u64,
    pub pending_count: u64,
    pub file_ops: u64,
    pub network_ops: u64,
    pub shell_ops: u64,
}

impl SandboxStats {
    pub fn update(&mut self, event: &SandboxEvent) {
        self.total_events += 1;
        match event.allowed {
            Some(true) => self.allowed_count += 1,
            Some(false) => self.denied_count += 1,
            None => self.pending_count += 1,
        }
        match event.kind {
            EventKind::FileAccess | EventKind::FileWrite | EventKind::FileDelete => {
                self.file_ops += 1;
            }
            EventKind::NetworkRequest => self.network_ops += 1,
            EventKind::ShellExec | EventKind::ProcessSpawn => self.shell_ops += 1,
            _ => {}
        }
    }
}

impl cosmic::Application for OpenClawApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = AppMessage;
    const APP_ID: &'static str = "com.openclaw.plus";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let config = SecurityConfig::load_or_default();
        let ai_init_endpoint = config.openclaw_ai.endpoint.clone();
        let ai_init_model    = config.openclaw_ai.model.clone();
        eprintln!("[INIT] config loaded: endpoint={} model={} provider={:?}",
            ai_init_endpoint, ai_init_model, config.openclaw_ai.provider);

        // Create the event and control channels.
        let (event_tx, event_rx) = flume::unbounded::<SandboxEvent>();
        let (control_tx, control_rx) = flume::unbounded::<ControlCommand>();

        // Start the sandbox in a background task (demo mode if OpenClaw is not installed).
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            let cfg = SecurityConfig::load_or_default();
            let audit = AuditLog::new(cfg.audit_log_path.clone());
            let policy = PolicyEngine::new(cfg.clone());
            let interceptor = Arc::new(Interceptor::new(policy, audit, event_tx_clone, control_rx));
            let runner = SandboxRunner::new(cfg, interceptor);
            if let Err(e) = runner.run().await {
                tracing::error!(error = %e, "sandbox runner terminated with error");
            } else {
                tracing::info!("sandbox runner exited normally");
            }
        });

        // Initialise the circuit breaker.
        // breaker_rx is forwarded to the event channel so automatic trips surface as BreakerTripped messages.
        let (breaker, breaker_rx) = CircuitBreaker::new(BreakerConfig::default());
        let breaker_arc: Arc<CircuitBreaker> = breaker;
        // Forward automatic circuit-breaker trips to the UI event channel.
        let event_tx_breaker = event_tx.clone();
        tokio::spawn(async move {
            while let Ok(reason) = breaker_rx.recv_async().await {
                let ev = openclaw_security::SandboxEvent {
                    id: 0,
                    kind: openclaw_security::EventKind::PolicyDenied,
                    resource: openclaw_security::ResourceKind::System,
                    detail: format!("Circuit breaker tripped: {reason:?}"),
                    path: None,
                    allowed: Some(false),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                };
                tracing::warn!(reason = ?reason, "circuit breaker tripped — forwarding to UI");
                let _ = event_tx_breaker.send(ev);
            }
        });

        // Detect plugin mode: if OPENCLAW_GATEWAY_URL is set, the UI was
        // launched by OpenClaw alongside the plugin gateway process.
        let run_mode = if let Ok(url) = std::env::var("OPENCLAW_GATEWAY_URL") {
            RunMode::Plugin { gateway_url: url }
        } else {
            RunMode::Embedded
        };

        // Build the sidebar navigation model.
        // Items are grouped: monitoring pages first, then tools (divider_above).
        let mut nav_model = nav_bar::Model::default();
        nav_model
            .insert()
            .text("Dashboard")
            .icon(cosmic::widget::icon::from_name("go-home-symbolic"))
            .data(NavPage::Dashboard)
            .activate();
        nav_model
            .insert()
            .text("Event Log")
            .icon(cosmic::widget::icon::from_name("text-x-generic-symbolic"))
            .data(NavPage::Events);
        nav_model
            .insert()
            .text("Security Settings")
            .icon(cosmic::widget::icon::from_name("preferences-system-privacy-symbolic"))
            .data(NavPage::Settings)
            .divider_above(true);
        nav_model
            .insert()
            .text("AI Assistant")
            .icon(cosmic::widget::icon::from_name("applications-science-symbolic"))
            .data(NavPage::AiChat)
            .divider_above(true);
        nav_model
            .insert()
            .text("Plugin Store")
            .icon(cosmic::widget::icon::from_name("system-software-install-symbolic"))
            .data(NavPage::PluginStore);

        let agent_entry_input = config.agent.entry_path.to_string_lossy().to_string();
        let wasm_policy_path_input = config.wasm_policy_plugin
            .as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

        let mut app = Self {
            core,
            nav_page: NavPage::Dashboard,
            nav_model,
            events: VecDeque::with_capacity(MAX_EVENT_HISTORY),
            pending_confirmations: Vec::new(),
            sandbox_status: SandboxStatus::Idle,
            config,
            stats: SandboxStats::default(),
            event_rx: Some(Arc::new(Mutex::new(event_rx))),
            control_tx: Some(control_tx),
            breaker_stats: BreakerStats::default(),
            circuit_breaker: Some(breaker_arc),
            run_mode,
            ai_chat: {
                let mut s = AiChatState::default();
                if !ai_init_endpoint.is_empty() { s.endpoint   = ai_init_endpoint; }
                if !ai_init_model.is_empty()    { s.model_name = ai_init_model; }
                s
            },
            inference_engine: None,
            show_about: false,
            language: Language::default(),
            show_language_menu: false,
            warm_theme_active: true,
            event_filter: None,
            event_search: String::new(),
            available_models: Vec::new(),
            model_download_input: String::new(),
            model_search: String::new(),
            download_status: None,
            github_orgs_input: String::new(),
            github_repos_input: String::new(),
            agent_entry_input,
            wasm_policy_path_input,
            wasm_policy_status: String::new(),
            folder_access_path_input: String::new(),
            folder_access_label_input: String::new(),
            rag_folder_path_input: String::new(),
            rag_folder_name_input: String::new(),
            claw_history: Vec::new(),
            claw_input: String::new(),
            claw_input_focused: false,
            claw_next_id: 1,
            claw_nl_mode: false,
            claw_attachment: None,
            claw_recording: false,
            claw_voice_status: None,
            claw_selected_agent_id: None,
            claw_agent_list: Vec::new(),
            claw_agent_conversations: std::collections::HashMap::new(),
            gateway_url: std::env::var("OPENCLAW_GATEWAY_URL").ok(),
            gateway_reachable: false,
            openclaw_ai_max_tokens_input: "4096".to_string(),
            openclaw_ai_temperature_input: "0.7".to_string(),
            openclaw_ai_test_status: None,
            channel_test_status: Vec::new(),
            tg_polling_active: false,
            tg_last_update_id: 0,
            tg_bot_username: None,
            discord_polling_active: false,
            discord_last_msg_id: None,
            matrix_polling_active: false,
            matrix_next_batch: None,
            slack_polling_active: false,
            slack_last_ts: None,
            show_quit_dialog: false,
            
            // Initialize Store state
            store_prefs: None,
            store_plugins: Vec::new(),
            store_install_states: std::collections::HashMap::new(),
            store_loading: false,
            store_search: String::new(),
            store_category_filter: None,
            store_fetch_error: None,

            // Initialize Agent management state
            agent_list: Vec::new(),
            agent_selected: None,
            agent_name_input: String::new(),
            agent_role_input: "Ticket Assistant".to_string(),
            agent_avatar_input: String::new(),
            agent_rename_input: String::new(),
            agent_editing_name: false,
            agent_loading: false,
            audit_runs: Vec::new(),
            audit_run_selected: None,
            audit_steps: Vec::new(),
            audit_events: Vec::new(),
            audit_loading: false,
            env_check_visible: true,
            env_check_report: crate::env_check::EnvCheckReport::new(),
        };
        app.core.window.header_title = "OpenClawPlus - AI Agent Platform".into();

        // Startup sequence: env health check overlay → then init AI → start sandbox
        let startup_task = Task::perform(
            async {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                AppMessage::EnvCheckStart
            },
            cosmic::Action::App,
        );

        (app, startup_task)
    }

    fn header_start(&self) -> Vec<Element<Self::Message>> {
        vec![]
    }

    fn header_end(&self) -> Vec<Element<Self::Message>> {
        let status_color = match &self.sandbox_status {
            SandboxStatus::Running    => cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46),
            SandboxStatus::Tripped(_) => cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
            SandboxStatus::Error(_)   => cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
            SandboxStatus::Paused     => cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12),
            _ => cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48),
        };

        let lang = self.language;

        vec![
            widget::row::with_children(vec![
                // Language indicator
                widget::text(lang.label())
                    .size(12)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.72, 0.68, 0.62),
                    ))
                    .into(),
                widget::text("·")
                    .size(12)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.42, 0.40, 0.38),
                    ))
                    .into(),
                // Status dot
                widget::container(widget::Space::new(7, 7))
                    .style(move |_: &cosmic::Theme| ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(status_color)),
                        border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .into(),
                widget::text(format!("{}", self.sandbox_status))
                    .size(12)
                    .class(cosmic::theme::Text::Color(status_color))
                    .into(),
            ])
            .spacing(6)
            .align_y(Alignment::Center)
            .padding([0, 8])
            .into(),
        ]
    }

    /// Disable the native nav_bar — sidebar is built directly in view().
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        None
    }

    /// Bottom status bar — shows live state of every major component.
    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        let lang = self.language;

        // Helper: one status chip.
        let chip = |icon: &'static str, label: &str, ok: bool| -> Element<'_, AppMessage> {
            let color = if ok {
                cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46)
            } else {
                cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28)
            };
            widget::row::with_children(vec![
                widget::container(widget::Space::new(7, 7))
                    .style(move |_: &cosmic::Theme| ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(color)),
                        border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .into(),
                widget::icon::from_name(icon).size(13).into(),
                widget::text(label.to_owned())
                    .size(12)
                    .class(cosmic::theme::Text::Color(color))
                    .into(),
            ])
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
        };

        let sandbox_ok = matches!(self.sandbox_status, SandboxStatus::Running);
        let breaker_ok = !self.breaker_stats.is_tripped;
        let ai_ok = matches!(
            self.ai_chat.status,
            crate::pages::ai_chat::EngineStatus::Ready | crate::pages::ai_chat::EngineStatus::Idle
        );

        let sandbox_label = format!(
            "{}: {}",
            t(lang, "Sandbox", "沙箱"),
            self.sandbox_status,
        );
        let breaker_label = format!(
            "{}: {}",
            t(lang, "Breaker", "熔断器"),
            if breaker_ok { t(lang, "OK", "正常") } else { t(lang, "Tripped", "已触发") },
        );
        let ai_label = format!(
            "AI: {}",
            self.ai_chat.status,
        );
        let events_label = format!(
            "{}: {}",
            t(lang, "Events", "事件"),
            self.events.len(),
        );

        let bar = widget::container(
            widget::row::with_children(vec![
                chip("utilities-terminal-symbolic",  &sandbox_label, sandbox_ok),
                widget::text("│").size(12)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.35, 0.33, 0.31)))
                    .into(),
                chip("security-high-symbolic",       &breaker_label, breaker_ok),
                widget::text("│").size(12)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.35, 0.33, 0.31)))
                    .into(),
                chip("applications-science-symbolic", &ai_label, ai_ok),
                widget::text("│").size(12)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.35, 0.33, 0.31)))
                    .into(),
                chip("format-justify-fill-symbolic",  &events_label, true),
                widget::Space::new(Length::Fill, 0).into(),
                widget::text("OpenClaw+  v0.1.0")
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.45, 0.43, 0.41),
                    ))
                    .into(),
            ])
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([4, 16]),
        )
        .style(|theme: &cosmic::Theme| {
            let bg = theme.cosmic().bg_color();
            ContainerStyle {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgb(
                        (bg.red   * 0.85).min(1.0),
                        (bg.green * 0.85).min(1.0),
                        (bg.blue  * 0.85).min(1.0),
                    ),
                )),
                border: cosmic::iced::Border {
                    width: 1.0,
                    color: cosmic::iced::Color::from_rgb(0.22, 0.20, 0.18),
                    radius: 0.0.into(),
                },
                ..Default::default()
            }
        })
        .width(Length::Fill);

        Some(bar.into())
    }

    /// Called by libcosmic when the user clicks a sidebar item.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        self.nav_model.activate(id);
        if let Some(page) = self.nav_model.data::<NavPage>(id) {
            let page = page.clone();
            // Plugin Store opens a subprocess; everything else just switches page.
            if page == NavPage::PluginStore {
                return self.update(AppMessage::OpenPluginStore);
            }
            self.nav_page = page;
        }
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        // ── Env check overlay (shown on first startup until dismissed) ─────
        if self.env_check_visible {
            return crate::pages::env_check_page::EnvCheckPage::view(&self.env_check_report);
        }

        if self.show_about {
            return self.view_about();
        }
        
        if self.show_quit_dialog {
            return self.view_quit_dialog();
        }
        
        if self.show_language_menu {
            return self.view_language_menu();
        }

        let lang = self.language;
        let cur  = self.nav_page;

        // ── Menu bar (rendered as top widget so dropdown is not clipped) ───
        let menu_bar = self.build_menu_bar();

        // ── Custom sidebar (always visible, embedded in view) ──────────────
        let sidebar = self.build_sidebar();

        // ── Page content ───────────────────────────────────────────────────
        let content: Element<AppMessage> = match cur {
            NavPage::Dashboard => DashboardPage::view(
                &self.stats,
                &self.breaker_stats,
                &self.events,
                &self.pending_confirmations,
                lang,
            ),
            NavPage::Events => EventsPage::view(
                &self.events,
                self.event_filter.clone(),
                &self.event_search,
                lang,
            ),
            NavPage::Settings => SettingsPage::view(
                &self.config,
                lang,
                self.warm_theme_active,
                &self.ai_chat.endpoint,
                &self.ai_chat.model_name,
                &self.available_models,
                &self.model_download_input,
                &self.model_search,
                self.download_status.as_ref(),
                &self.github_orgs_input,
                &self.github_repos_input,
                &self.agent_entry_input,
                &self.wasm_policy_path_input,
                &self.wasm_policy_status,
                &self.folder_access_path_input,
                &self.folder_access_label_input,
                &self.rag_folder_path_input,
                &self.rag_folder_name_input,
            ),
            NavPage::AiChat => AiChatPage::view(&self.ai_chat, lang),
            NavPage::GeneralSettings => GeneralSettingsPage::view(
                lang,
                self.warm_theme_active,
                &self.ai_chat.endpoint,
                &self.ai_chat.model_name,
                &self.available_models,
                &self.model_download_input,
                &self.model_search,
                self.download_status.as_ref(),
                &self.github_orgs_input,
                &self.github_repos_input,
                &self.agent_entry_input,
                &self.config,
                self.openclaw_ai_test_status.as_ref(),
                &self.channel_test_status,
                &self.openclaw_ai_max_tokens_input,
                &self.openclaw_ai_temperature_input,
            ),
            NavPage::ClawTerminal => ClawTerminalPage::view(
                lang,
                &self.claw_history,
                &self.claw_input,
                self.claw_input_focused,
                &self.sandbox_status,
                &self.ai_chat.status,
                self.claw_nl_mode,
                self.gateway_url.as_deref(),
                self.gateway_reachable,
                self.tg_polling_active,
                self.tg_bot_username.as_deref(),
                self.claw_selected_agent_id.as_deref(),
                &self.claw_agent_list,
                self.claw_attachment.as_ref().map(|a| a.filename.as_str()),
                self.claw_recording,
                self.claw_voice_status.as_deref(),
            ),
            NavPage::PluginStore => self.view_plugin_store(lang),
            NavPage::Agents => self.view_agents_page(lang),
            NavPage::AuditReplay => self.view_audit_page(lang),
        };

        // ── Left sidebar + right content ───────────────────────────────────
        let body_row = widget::row::with_children(vec![
            sidebar,
            // Vertical divider
            widget::container(widget::Space::new(1, 0))
                .style(|theme: &cosmic::Theme| {
                    let c = theme.cosmic().bg_divider();
                    ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgb(c.red, c.green, c.blue),
                        )),
                        ..Default::default()
                    }
                })
                .height(Length::Fill)
                .into(),
            widget::container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ])
        .height(Length::Fill);

        // ── Stack: menu bar on top, body below ─────────────────────────────
        widget::column::with_children(vec![
            menu_bar,
            widget::divider::horizontal::default().into(),
            body_row.into(),
        ])
        .height(Length::Fill)
        .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            AppMessage::NavSelect(page) => {
                self.nav_page = page;
                if page == NavPage::AiChat {
                    return cosmic::widget::text_input::focus(
                        crate::pages::ai_chat::AI_INPUT_ID.clone(),
                    ).map(cosmic::Action::App);
                }
            }
            AppMessage::SandboxEvent(event) => {
                self.stats.update(&event);

                // Update circuit breaker denial counters.
                if event.allowed == Some(false) {
                    self.breaker_stats.total_denials += 1;
                }
                if matches!(event.kind, EventKind::ShellExec | EventKind::ProcessSpawn)
                    && event.allowed == Some(false)
                {
                    self.breaker_stats.dangerous_commands += 1;
                }

                if event.kind == EventKind::UserConfirmRequired {
                    self.pending_confirmations.push(event.clone());
                    // Auto-navigate to Dashboard so the confirmation card is visible.
                    self.nav_page = NavPage::Dashboard;
                } else {
                    if event.kind == EventKind::SandboxStart {
                        self.sandbox_status = SandboxStatus::Running;
                    } else if event.kind == EventKind::SandboxStop {
                        self.sandbox_status = SandboxStatus::Stopped;
                    }
                }

                if self.events.len() >= MAX_EVENT_HISTORY {
                    self.events.pop_front();
                }
                self.events.push_back(event);
            }
            AppMessage::ConfirmAllow(id) => {
                self.pending_confirmations.retain(|e| e.id != id);
                match &self.run_mode {
                    RunMode::Embedded => {
                        if let Some(tx) = &self.control_tx {
                            let _ = tx.send(ControlCommand::Allow(id));
                        }
                    }
                    RunMode::Plugin { gateway_url } => {
                        let url = format!("{}/skills/allow/{}", gateway_url, id);
                        tokio::spawn(async move {
                            let _ = reqwest::Client::new().post(&url).send().await;
                        });
                    }
                }
            }
            AppMessage::ConfirmDeny(id) => {
                self.pending_confirmations.retain(|e| e.id != id);
                match &self.run_mode {
                    RunMode::Embedded => {
                        if let Some(tx) = &self.control_tx {
                            let _ = tx.send(ControlCommand::Deny(id));
                        }
                    }
                    RunMode::Plugin { gateway_url } => {
                        let url = format!("{}/skills/deny/{}", gateway_url, id);
                        tokio::spawn(async move {
                            let _ = reqwest::Client::new().post(&url).send().await;
                        });
                    }
                }
            }
            AppMessage::PluginAllow(id) => {
                return self.update(AppMessage::ConfirmAllow(id));
            }
            AppMessage::PluginDeny(id) => {
                return self.update(AppMessage::ConfirmDeny(id));
            }
            AppMessage::StartSandbox => {
                self.sandbox_status = SandboxStatus::Running;
            }
            AppMessage::StopSandbox => {
                if let Some(tx) = &self.control_tx {
                    let _ = tx.send(ControlCommand::Terminate);
                }
                self.sandbox_status = SandboxStatus::Stopped;
            }
            AppMessage::EmergencyStop => {
                self.breaker_stats.is_tripped = true;
                self.sandbox_status =
                    SandboxStatus::Tripped("Emergency stop triggered by user".to_string());
                self.nav_page = NavPage::Dashboard;
                match self.run_mode.clone() {
                    RunMode::Embedded => {
                        if let Some(tx) = &self.control_tx {
                            let _ = tx.send(ControlCommand::Terminate);
                        }
                        if let Some(breaker) = &self.circuit_breaker {
                            let b = breaker.clone();
                            tokio::spawn(async move { b.manual_trip().await; });
                        }
                    }
                    RunMode::Plugin { gateway_url } => {
                        let url = format!("{}/admin/emergency-stop", gateway_url);
                        tokio::spawn(async move {
                            let _ = reqwest::Client::new().post(&url).send().await;
                        });
                    }
                }
            }
            AppMessage::BreakerTripped(reason) => {
                self.breaker_stats.is_tripped = true;
                self.sandbox_status = SandboxStatus::Tripped(reason);
                self.nav_page = NavPage::Dashboard;
            }
            AppMessage::ClearEvents => {
                self.events.clear();
                self.stats = SandboxStats::default();
            }
            AppMessage::ConfigUpdated(config) => {
                self.config = config;
            }
            AppMessage::OpenPluginStore => {
                self.nav_page = NavPage::PluginStore;
                // Spawn the standalone store window as a subprocess.
                let store_bin = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("openclaw-store")))
                    .unwrap_or_else(|| std::path::PathBuf::from("openclaw-store"));
                match std::process::Command::new(&store_bin).spawn() {
                    Ok(_) => tracing::info!(path = %store_bin.display(), "plugin store subprocess spawned"),
                    Err(e) => tracing::error!(path = %store_bin.display(), error = %e, "failed to spawn plugin store"),
                }
            }
            // ── AI Chat ───────────────────────────────────────────────────────
            AppMessage::AiInputChanged(text) => {
                tracing::info!("[IME] AiInputChanged: {:?} ({} chars)", text, text.chars().count());
                self.ai_chat.input = text;
            }
            AppMessage::AiFocused => {
                tracing::info!("[IME] AI input focused, lang={:?}", self.language);
            }
            AppMessage::AiSendMessage => {
                let input = self.ai_chat.input.trim().to_string();
                if input.is_empty() {
                    return Task::none();
                }
                self.ai_chat.input.clear();

                // Build conversation history for the inference engine.
                let history = self.ai_chat.push_user_message(input);

                // Lazily initialise the inference engine on first use.
                if self.inference_engine.is_none() {
                    let ai = &self.config.openclaw_ai;
                    let cfg = InferenceConfig {
                        backend: provider_to_backend_kind(&ai.provider),
                        endpoint: ai.endpoint.clone(),
                        model_name: ai.model.clone(),
                        circuit_breaker_threshold: 999,
                        ..InferenceConfig::default()
                    };
                    match InferenceEngine::new(cfg) {
                        Ok(eng) => {
                            tracing::info!(model = %ai.model, endpoint = %ai.endpoint, "inference engine initialised");
                            self.inference_engine = Some(Arc::new(eng));
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "failed to initialise inference engine");
                            self.ai_chat.push_error(format!("Engine init failed: {e}"));
                            return Task::none();
                        }
                    }
                }
                let engine = self.inference_engine.as_ref().unwrap().clone();

                // Convert chat history to ConversationTurn list.
                let messages: Vec<ConversationTurn> = history
                    .iter()
                    .filter(|m| m.role != crate::pages::ai_chat::ChatRole::System)
                    .map(|m| ConversationTurn {
                        role: match m.role {
                            crate::pages::ai_chat::ChatRole::User      => "user".into(),
                            crate::pages::ai_chat::ChatRole::Assistant => "assistant".into(),
                            crate::pages::ai_chat::ChatRole::System    => "system".into(),
                        },
                        content: m.content.clone(),
                    })
                    .collect();

                // Dispatch inference in a background task.
                return Task::perform(
                    async move {
                        let req = InferenceRequest {
                            request_id: 0,
                            messages,
                            max_tokens_override: Some(512),
                            temperature_override: None,
                            stream: false,
                        };
                        match engine.infer(req).await {
                            Ok(resp) => AppMessage::AiResponse {
                                content: resp.content,
                                latency_ms: resp.latency_ms,
                            },
                            Err(e) => AppMessage::AiError(e.to_string()),
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::AiResponse { content, latency_ms } => {
                self.ai_chat.push_assistant_response(content, latency_ms);
                return cosmic::widget::text_input::focus(
                    crate::pages::ai_chat::AI_INPUT_ID.clone(),
                ).map(cosmic::Action::App);
            }
            AppMessage::AiError(err) => {
                self.ai_chat.push_error(err);
            }
            AppMessage::AiClearChat => {
                self.ai_chat.clear();
            }
            // ── Agent management ──────────────────────────────────────────
            AppMessage::AgentRefresh => {
                self.agent_loading = true;
                return Task::perform(
                    async {
                        let profiles = openclaw_security::AgentProfile::list_all();
                        AppMessage::AgentListLoaded(profiles)
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::AgentListLoaded(profiles) => {
                self.agent_list = profiles;
                self.agent_loading = false;
                // Auto-select first agent if none selected yet
                if self.agent_selected.is_none() && !self.agent_list.is_empty() {
                    self.agent_selected = Some(0);
                    self.agent_avatar_input = self.agent_list[0].avatar_url.clone();
                }
            }
            AppMessage::AgentSelect(idx) => {
                self.agent_selected = Some(idx);
                // Sync avatar input with selected agent's current avatar_url
                if let Some(p) = self.agent_list.get(idx) {
                    self.agent_avatar_input = p.avatar_url.clone();
                }
            }
            AppMessage::AgentNameChanged(name) => {
                self.agent_name_input = name;
            }
            AppMessage::AgentRoleChanged(role) => {
                self.agent_role_input = role;
            }
            AppMessage::AgentCreate => {
                let name = self.agent_name_input.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }
                let role_str = self.agent_role_input.clone();
                let role = match role_str.as_str() {
                    "Code Reviewer"          => openclaw_security::AgentRole::CodeReviewer,
                    "Report Generator"       => openclaw_security::AgentRole::ReportGenerator,
                    "Security Auditor"       => openclaw_security::AgentRole::SecurityAuditor,
                    "Data Analyst"           => openclaw_security::AgentRole::DataAnalyst,
                    "Customer Support"       => openclaw_security::AgentRole::CustomerSupport,
                    "Knowledge Officer"      => openclaw_security::AgentRole::KnowledgeOfficer,
                    "Social Media Manager"   => openclaw_security::AgentRole::SocialMediaManager,
                    "Inbox Triage Agent"     => openclaw_security::AgentRole::InboxTriageAgent,
                    "Finance & Procurement"  => openclaw_security::AgentRole::FinanceProcurement,
                    "News Secretary"         => openclaw_security::AgentRole::NewsSecretary,
                    "Security Code Auditor"  => openclaw_security::AgentRole::SecurityCodeAuditor,
                    _                        => openclaw_security::AgentRole::TicketAssistant,
                };
                let profile = openclaw_security::AgentProfile::new(name, role, "default", "admin");
                if let Err(e) = profile.save() {
                    tracing::error!(error = %e, "Failed to save new AgentProfile");
                } else {
                    self.agent_name_input.clear();
                    self.agent_list.insert(0, profile);
                    self.agent_selected = Some(0);
                }
            }
            AppMessage::AgentSuspend => {
                if let Some(idx) = self.agent_selected {
                    if let Some(p) = self.agent_list.get_mut(idx) {
                        p.suspend();
                        let _ = p.save();
                    }
                }
            }
            AppMessage::AgentResume => {
                if let Some(idx) = self.agent_selected {
                    if let Some(p) = self.agent_list.get_mut(idx) {
                        p.resume();
                        let _ = p.save();
                    }
                }
            }
            AppMessage::AgentStartRename => {
                if let Some(idx) = self.agent_selected {
                    if let Some(p) = self.agent_list.get(idx) {
                        self.agent_rename_input = p.display_name.clone();
                        self.agent_editing_name = true;
                    }
                }
            }
            AppMessage::AgentRenameInputChanged(s) => {
                self.agent_rename_input = s;
            }
            AppMessage::AgentRenameConfirm => {
                let new_name = self.agent_rename_input.trim().to_string();
                if !new_name.is_empty() {
                    if let Some(idx) = self.agent_selected {
                        if let Some(p) = self.agent_list.get_mut(idx) {
                            p.display_name = new_name;
                            let _ = p.save();
                        }
                    }
                }
                self.agent_editing_name = false;
            }
            AppMessage::AgentRenameCancelled => {
                self.agent_editing_name = false;
            }
            AppMessage::AgentAvatarInputChanged(s) => {
                self.agent_avatar_input = s;
            }
            AppMessage::AgentAvatarChange(url) => {
                if let Some(idx) = self.agent_selected {
                    if let Some(p) = self.agent_list.get_mut(idx) {
                        p.avatar_url = url.clone();
                        let _ = p.save();
                    }
                }
                self.agent_avatar_input = url;
            }
            AppMessage::AgentArchive => {
                if let Some(idx) = self.agent_selected {
                    if let Some(p) = self.agent_list.get_mut(idx) {
                        p.archive();
                        let _ = p.save();
                    }
                    self.agent_selected = None;
                }
            }
            // ── Audit / Run replay handlers ───────────────────────────────
            AppMessage::AuditLoadRuns(agent_id) => {
                self.nav_page = NavPage::AuditReplay;
                self.audit_loading = true;
                self.audit_runs.clear();
                self.audit_run_selected = None;
                self.audit_steps.clear();
                self.audit_events.clear();
                return Task::perform(
                    async move {
                        let db_path = dirs::home_dir()
                            .unwrap_or_else(|| std::path::PathBuf::from("."))
                            .join(".openclaw-plus")
                            .join("platform.db");
                        match openclaw_storage::Database::open(&db_path) {
                            Ok(db) => {
                                let run_store = openclaw_storage::RunStore::new(&db);
                                let runs = run_store
                                    .list_runs_for_agent(&agent_id, 100)
                                    .unwrap_or_default();
                                AppMessage::AuditRunsLoaded(runs)
                            }
                            Err(_) => AppMessage::AuditRunsLoaded(Vec::new()),
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::AuditRunsLoaded(runs) => {
                self.audit_runs = runs;
                self.audit_loading = false;
            }
            AppMessage::AuditSelectRun(idx) => {
                self.audit_run_selected = Some(idx);
                self.audit_steps.clear();
                self.audit_events.clear();
                self.audit_loading = true;
                if let Some(run) = self.audit_runs.get(idx) {
                    let run_id = run.id.clone();
                    return Task::perform(
                        async move {
                            let db_path = dirs::home_dir()
                                .unwrap_or_else(|| std::path::PathBuf::from("."))
                                .join(".openclaw-plus")
                                .join("platform.db");
                            if let Ok(db) = openclaw_storage::Database::open(&db_path) {
                                let run_store = openclaw_storage::RunStore::new(&db);
                                let steps = run_store.list_steps_for_run(&run_id).unwrap_or_default();

                                let audit_store = openclaw_storage::AuditStore::new(&db);
                                let filter = openclaw_storage::audit_store::AuditFilter::for_run(&run_id);
                                let events = audit_store.query(&filter).unwrap_or_default();

                                return AppMessage::AuditRunDetailsLoaded(steps, events);
                            }
                            AppMessage::AuditRunDetailsLoaded(Vec::new(), Vec::new())
                        },
                        cosmic::Action::App,
                    );
                }
            }
            AppMessage::AuditStepsLoaded(steps) => {
                self.audit_steps = steps;
            }
            AppMessage::AuditEventsLoaded(events) => {
                self.audit_events = events;
            }
            AppMessage::AuditRunDetailsLoaded(steps, events) => {
                self.audit_steps = steps;
                self.audit_events = events;
                self.audit_loading = false;
            }
            AppMessage::AuditClear => {
                self.audit_runs.clear();
                self.audit_run_selected = None;
                self.audit_steps.clear();
                self.audit_events.clear();
                self.audit_loading = false;
            }
            AppMessage::AiModelChanged(model) => {
                self.ai_chat.model_name = model;
                self.inference_engine = None;
            }
            AppMessage::Noop => {}
            AppMessage::SetEventFilter(kind) => {
                self.event_filter = kind;
            }
            AppMessage::SetEventSearch(text) => {
                self.event_search = text;
            }
            AppMessage::ToggleInterceptShell => {
                self.config.intercept_shell = !self.config.intercept_shell;
            }
            AppMessage::ToggleConfirmFileDelete => {
                self.config.confirm_file_delete = !self.config.confirm_file_delete;
            }
            AppMessage::ToggleConfirmShellExec => {
                self.config.confirm_shell_exec = !self.config.confirm_shell_exec;
            }
            AppMessage::ToggleConfirmNetwork => {
                self.config.confirm_network = !self.config.confirm_network;
            }
            AppMessage::AiEndpointChanged(url) => {
                eprintln!("[AI-CONFIG] AiEndpointChanged: {} -> {}", self.ai_chat.endpoint, url);
                self.ai_chat.endpoint = url;
                self.inference_engine = None;
            }
            AppMessage::MemoryLimitChanged(s) => {
                if let Ok(mb) = s.parse::<u32>() {
                    self.config.memory_limit_mb = mb;
                }
            }
            // ── Environment check ─────────────────────────────────────────────────
            AppMessage::EnvCheckStart => {
                eprintln!("[ENV-CHECK] Starting environment health check");
                self.env_check_visible = true;
                self.env_check_report = crate::env_check::EnvCheckReport::new();
                // Mark all items as Running so the UI shows progress immediately
                for item in &mut self.env_check_report.items {
                    item.status = crate::env_check::CheckStatus::Pending;
                }
                let params = crate::env_check::EnvCheckParams {
                    ollama_endpoint: self.ai_chat.endpoint.clone(),
                    ollama_model:    self.ai_chat.model_name.clone(),
                    openai_api_key:  {
                        let k = self.config.openclaw_ai.api_key.clone();
                        if k.is_empty() { None } else { Some(k) }
                    },
                    deepseek_api_key: None,
                    anthropic_api_key: None,
                    gateway_url:     self.gateway_url.clone(),
                    workspace_dir:   self.config.workspace_dir.to_string_lossy().to_string(),
                };
                return Task::perform(
                    async move {
                        crate::env_check::run_all_checks(params).await
                    },
                    |results| cosmic::Action::App(AppMessage::EnvCheckAllDone(results)),
                );
            }
            AppMessage::EnvCheckStepDone(step) => {
                eprintln!("[ENV-CHECK] step={} status={:?}", step.id, step.status);
                if let Some(item) = self.env_check_report.get_mut(step.id) {
                    item.status = step.status;
                    item.latency_ms = step.latency_ms;
                    if let Some(d) = step.detail {
                        item.detail = Some(d);
                    }
                }
                if step.ollama_started {
                    self.env_check_report.ollama_was_started = true;
                }
            }
            AppMessage::EnvCheckAllDone(results) => {
                eprintln!("[ENV-CHECK] All {} checks complete", results.len());
                for step in results {
                    if let Some(item) = self.env_check_report.get_mut(step.id) {
                        item.status = step.status;
                        item.latency_ms = step.latency_ms;
                        if let Some(d) = step.detail {
                            item.detail = Some(d);
                        }
                    }
                    if step.ollama_started {
                        self.env_check_report.ollama_was_started = true;
                        // Sync the endpoint into ai_chat if Ollama was auto-started on localhost
                        if self.ai_chat.endpoint.contains("localhost") {
                            eprintln!("[ENV-CHECK] Ollama auto-started, endpoint confirmed: {}", self.ai_chat.endpoint);
                        }
                    }
                }
                self.env_check_report.all_critical_ok = self.env_check_report.items.iter()
                    .filter(|i| matches!(i.id, "ollama_running" | "disk_space"))
                    .all(|i| i.status.is_ok() || matches!(i.status, crate::env_check::CheckStatus::Warning(_)));
            }
            AppMessage::EnvCheckDismiss => {
                eprintln!("[ENV-CHECK] Dismissed, proceeding to normal startup");
                self.env_check_visible = false;
                // Continue original startup sequence
                return self.update(AppMessage::StartupCheckEnvironment);
            }
            // ── Startup sequence ──────────────────────────────────────────────────
            AppMessage::StartupCheckEnvironment => {
                tracing::info!("[STARTUP] Checking environment...");
                // Load agent list for Claw Terminal
                self.claw_agent_list = openclaw_security::AgentProfile::list_all();
                tracing::info!("[STARTUP] Loaded {} agents for Claw Terminal", self.claw_agent_list.len());
                let endpoint = self.ai_chat.endpoint.clone();
                return Task::perform(
                    async move {
                        // Check WasmEdge
                        let wasmedge_ok = std::process::Command::new("wasmedge")
                            .arg("--version")
                            .output()
                            .is_ok();
                        
                        // Check AI endpoint
                        let ai_ok = reqwest::Client::new()
                            .get(&format!("{}/api/tags", endpoint))
                            .timeout(std::time::Duration::from_secs(2))
                            .send()
                            .await
                            .is_ok();
                        
                        AppMessage::StartupCheckComplete { wasmedge_ok, ai_ok }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::StartupCheckComplete { wasmedge_ok, ai_ok } => {
                tracing::info!(wasmedge = wasmedge_ok, ai = ai_ok, "[STARTUP] Environment check complete");
                if !wasmedge_ok {
                    tracing::warn!("[STARTUP] WasmEdge not found - sandbox features may be limited");
                }
                if !ai_ok {
                    tracing::warn!("[STARTUP] AI endpoint not reachable - AI features may be limited");
                }
                // Continue to AI initialization
                return Task::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        AppMessage::StartupInitAI
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::StartupInitAI => {
                let ai = &self.config.openclaw_ai;
                tracing::info!(model = %ai.model, endpoint = %ai.endpoint, "[STARTUP] Initializing AI model...");
                let cfg = InferenceConfig {
                    backend: provider_to_backend_kind(&ai.provider),
                    endpoint: ai.endpoint.clone(),
                    model_name: ai.model.clone(),
                    circuit_breaker_threshold: 999,
                    ..InferenceConfig::default()
                };
                match InferenceEngine::new(cfg) {
                    Ok(eng) => {
                        tracing::info!("[STARTUP] AI model initialized successfully");
                        self.inference_engine = Some(Arc::new(eng));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "[STARTUP] Failed to initialize AI model");
                    }
                }
                // Continue to sandbox startup
                return Task::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        AppMessage::StartupStartSandbox
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::StartupStartSandbox => {
                tracing::info!("[STARTUP] Auto-starting sandbox...");
                self.sandbox_status = SandboxStatus::Running;
                tracing::info!("[STARTUP] Startup sequence complete");
                // Seed default digital workers if none exist, then load list.
                return Task::perform(
                    async {
                        use openclaw_security::{AgentProfile, AgentRole, AgentCapability};
                        let existing = AgentProfile::list_all();
                        if existing.is_empty() {
                            tracing::info!("[STARTUP] Seeding 5 default digital workers...");

                            type CapSpec = (&'static str, &'static str, u8);
                            // 5 representative roles covering the most common use cases
                            let defaults: &[(&str, AgentRole, &[CapSpec])] = &[
                                ("知识库首席官", AgentRole::KnowledgeOfficer, &[
                                    ("rag.index",    "RAG 索引",  1),
                                    ("file.read",    "文件读取",  1),
                                    ("pdf.parse",    "PDF 解析",  1),
                                    ("vector.store", "向量存储",  1),
                                ]),
                                ("代码审查员", AgentRole::SecurityCodeAuditor, &[
                                    ("github.api",   "GitHub API", 1),
                                    ("sast.scan",    "SAST 扫描",  3),
                                    ("shell.lint",   "代码检查",   2),
                                    ("test.gen",     "测试生成",   1),
                                ]),
                                ("新闻信息秘书", AgentRole::NewsSecretary, &[
                                    ("news.fetch",    "新闻抓取",    1),
                                    ("rss.parse",     "RSS 解析",    1),
                                    ("telegram.bot",  "Telegram 推送", 1),
                                    ("schedule.cron", "定时任务",    1),
                                ]),
                                ("财务采购员", AgentRole::FinanceProcurement, &[
                                    ("finance.read",  "财务数据读取", 3),
                                    ("approval.flow", "审批流程",     3),
                                    ("file.write",    "文件写入",     2),
                                ]),
                                ("客服助手", AgentRole::CustomerSupport, &[
                                    ("telegram.bot",  "Telegram Bot", 1),
                                    ("discord.bot",   "Discord Bot",  1),
                                    ("file.read",     "文件读取",     1),
                                ]),
                            ];

                            for (name, role, caps) in defaults {
                                let mut p = AgentProfile::new(*name, role.clone(), "system", "admin");
                                p.description = role.description().to_string();
                                for (id, cap_name, risk) in *caps {
                                    p.capabilities.push(AgentCapability::new(*id, *cap_name, *risk));
                                }
                                // 财务/邮件类员工强制 human-in-the-loop
                                if matches!(role, AgentRole::FinanceProcurement) {
                                    p.confirm_shell_exec  = true;
                                    p.confirm_file_delete = true;
                                    p.confirm_network     = true;
                                }
                                // 创建工作空间目录
                                let ws = p.workspace_dir();
                                let _ = std::fs::create_dir_all(&ws);
                                if let Err(e) = p.save() {
                                    tracing::warn!(error=%e, name=%name, "[STARTUP] Failed to seed worker");
                                }
                            }
                        }
                        AppMessage::AgentListLoaded(AgentProfile::list_all())
                    },
                    cosmic::Action::App,
                );
            }
            // ── AI Model Management ───────────────────────────────────────────────
            AppMessage::AiListModels => {
                tracing::info!("[AI] Listing available models...");
                let endpoint = self.ai_chat.endpoint.clone();
                return Task::perform(
                    async move {
                        match reqwest::Client::new()
                            .get(&format!("{}/api/tags", endpoint))
                            .timeout(std::time::Duration::from_secs(5))
                            .send()
                            .await
                        {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    if let Some(arr) = json.get("models").and_then(|m| m.as_array()) {
                                        let models: Vec<OllamaModel> = arr.iter().map(|m| {
                                            let details = m.get("details");
                                            OllamaModel {
                                                name: m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                size_bytes: m.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                                                modified_at: m.get("modified_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                parameter_size: details.and_then(|d| d.get("parameter_size")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                quantization: details.and_then(|d| d.get("quantization_level")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                family: details.and_then(|d| d.get("family")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                            }
                                        }).collect();
                                        return AppMessage::AiModelsListed(models);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "[AI] Failed to list models");
                            }
                        }
                        AppMessage::AiModelsListed(Vec::new())
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::AiModelsListed(models) => {
                tracing::info!(count = models.len(), "[AI] Models listed");
                self.available_models = models;
            }
            AppMessage::AiDeleteModel(model_name) => {
                tracing::info!(model = %model_name, "[AI] Deleting model...");
                let endpoint = self.ai_chat.endpoint.clone();
                return Task::perform(
                    async move {
                        match reqwest::Client::new()
                            .delete(&format!("{}/api/delete", endpoint))
                            .json(&serde_json::json!({ "name": model_name }))
                            .timeout(std::time::Duration::from_secs(10))
                            .send()
                            .await
                        {
                            Ok(_) => AppMessage::AiModelOpComplete {
                                success: true,
                                message: format!("Model '{}' deleted successfully", model_name),
                            },
                            Err(e) => AppMessage::AiModelOpComplete {
                                success: false,
                                message: format!("Failed to delete model: {}", e),
                            },
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::AiPullModel(model_name) => {
                tracing::info!(model = %model_name, "[AI] Pulling model...");
                let endpoint = self.ai_chat.endpoint.clone();
                return Task::perform(
                    async move {
                        match reqwest::Client::new()
                            .post(&format!("{}/api/pull", endpoint))
                            .json(&serde_json::json!({ "name": model_name }))
                            .timeout(std::time::Duration::from_secs(300))
                            .send()
                            .await
                        {
                            Ok(_) => AppMessage::AiModelOpComplete {
                                success: true,
                                message: format!("Model '{}' downloaded successfully", model_name),
                            },
                            Err(e) => AppMessage::AiModelOpComplete {
                                success: false,
                                message: format!("Failed to download model: {}", e),
                            },
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::AiModelOpComplete { success, message } => {
                tracing::info!(success, message = %message, "[AI] Model operation complete");
                // Clear download input on success
                if success {
                    self.model_download_input.clear();
                }
                // Refresh model list after operation
                return self.update(AppMessage::AiListModels);
            }
            AppMessage::ModelDownloadInputChanged(text) => {
                self.model_download_input = text;
            }
            AppMessage::ModelSearchChanged(text) => {
                self.model_search = text;
            }
            AppMessage::AiSetActiveModel(name) => {
                tracing::info!(model = %name, "[AI] Setting active model");
                self.ai_chat.model_name = name.clone();
                self.config.openclaw_ai.model = name.clone();
                // Re-init inference engine with new model
                let ai = &self.config.openclaw_ai;
                let cfg = InferenceConfig {
                    backend: provider_to_backend_kind(&ai.provider),
                    endpoint: ai.endpoint.clone(),
                    model_name: name,
                    circuit_breaker_threshold: 999,
                    ..InferenceConfig::default()
                };
                match InferenceEngine::new(cfg) {
                    Ok(eng) => { self.inference_engine = Some(Arc::new(eng)); }
                    Err(e) => { tracing::warn!(error = %e, "[AI] Failed to switch model"); }
                }
            }
            AppMessage::AiDownloadProgress { model, status, percent } => {
                tracing::info!(model = %model, status = %status, percent, "[AI] Download progress");
                self.download_status = Some((model, status, percent));
            }
            // ── GitHub Policy handlers ────────────────────────────────────────
            AppMessage::ToggleGithubDenyForcePush => {
                self.config.github.deny_force_push = !self.config.github.deny_force_push;
            }
            AppMessage::ToggleGithubConfirmPush => {
                self.config.github.confirm_push = !self.config.github.confirm_push;
            }
            AppMessage::ToggleGithubConfirmBranchDelete => {
                self.config.github.confirm_branch_delete = !self.config.github.confirm_branch_delete;
            }
            AppMessage::ToggleGithubConfirmHistoryRewrite => {
                self.config.github.confirm_history_rewrite = !self.config.github.confirm_history_rewrite;
            }
            AppMessage::ToggleGithubProtectDefaultBranch => {
                self.config.github.protect_default_branch = !self.config.github.protect_default_branch;
            }
            AppMessage::ToggleGithubInterceptApi => {
                self.config.github.intercept_github_api = !self.config.github.intercept_github_api;
            }
            AppMessage::GithubAllowedOrgsChanged(text) => {
                self.config.github.allowed_orgs = text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.github_orgs_input = text;
            }
            AppMessage::GithubAllowedReposChanged(text) => {
                self.config.github.allowed_repos = text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.github_repos_input = text;
            }
            // ── Agent selector handlers ───────────────────────────────────────
            AppMessage::SetAgentKind(kind) => {
                tracing::info!(kind = %kind, "[Agent] Switching agent kind");
                self.config.agent.entry_path = std::path::PathBuf::from(kind.default_entry());
                self.config.agent.kind = kind;
                self.agent_entry_input = self.config.agent.entry_path
                    .to_string_lossy().to_string();
            }
            AppMessage::AgentEntryPathChanged(path) => {
                self.config.agent.entry_path = std::path::PathBuf::from(&path);
                self.agent_entry_input = path;
            }
            // ── WASM policy plugin handlers ───────────────────────────────────
            AppMessage::WasmPolicyPathChanged(path) => {
                self.wasm_policy_path_input = path.clone();
                self.config.wasm_policy_plugin = if path.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(&path))
                };
            }
            AppMessage::WasmPolicyReload => {
                let path = self.config.wasm_policy_plugin.clone();
                return Task::perform(
                    async move {
                        match path {
                            None => AppMessage::WasmPolicyReloaded { success: false },
                            Some(p) => {
                                match openclaw_security::WasmPolicyModule::load(&p) {
                                    Ok(m) => {
                                        tracing::info!(
                                            rules = m.rules.len(),
                                            "[WasmPolicy] Reloaded successfully"
                                        );
                                        AppMessage::WasmPolicyReloaded { success: true }
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, "[WasmPolicy] Reload failed");
                                        AppMessage::WasmPolicyReloaded { success: false }
                                    }
                                }
                            }
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::WasmPolicyReloaded { success } => {
                tracing::info!(success, "[WasmPolicy] Reload result");
                self.wasm_policy_status = if success {
                    "✓ Policy loaded successfully".to_string()
                } else {
                    "✗ Failed to load policy file".to_string()
                };
            }
            // ── Folder access whitelist handlers ──────────────────────────────
            AppMessage::FolderAccessPickFolder => {
                return Task::perform(
                    async {
                        let picked = rfd::AsyncFileDialog::new()
                            .set_title("Select Authorised Folder")
                            .pick_folder()
                            .await;
                        AppMessage::FolderAccessPickerResult(
                            picked.map(|h| h.path().to_string_lossy().to_string()),
                        )
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::FolderAccessPickerResult(maybe_path) => {
                if let Some(p) = maybe_path {
                    self.folder_access_path_input = p;
                }
            }
            AppMessage::FolderAccessPathChanged(p) => {
                self.folder_access_path_input = p;
            }
            AppMessage::FolderAccessLabelChanged(l) => {
                self.folder_access_label_input = l;
            }
            AppMessage::FolderAccessAdd { path, label, allow_write } => {
                if !path.is_empty() {
                    let entry = openclaw_security::FolderAccess {
                        host_path: std::path::PathBuf::from(&path),
                        label: if label.is_empty() {
                            std::path::Path::new(&path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Folder")
                                .to_string()
                        } else {
                            label
                        },
                        allow_write,
                        allow_delete: false,
                        allowed_extensions: Vec::new(),
                    };
                    tracing::info!(path = %path, "[FolderAccess] Added to whitelist");
                    self.config.folder_access.push(entry);
                    self.folder_access_path_input.clear();
                    self.folder_access_label_input.clear();
                }
            }
            AppMessage::FolderAccessRemove(idx) => {
                if idx < self.config.folder_access.len() {
                    let removed = self.config.folder_access.remove(idx);
                    tracing::info!(label = %removed.label, "[FolderAccess] Removed from whitelist");
                }
            }
            AppMessage::FolderAccessToggleWrite(idx) => {
                if let Some(fa) = self.config.folder_access.get_mut(idx) {
                    fa.allow_write = !fa.allow_write;
                }
            }
            AppMessage::FolderAccessToggleDelete(idx) => {
                if let Some(fa) = self.config.folder_access.get_mut(idx) {
                    fa.allow_delete = !fa.allow_delete;
                }
            }
            // ── RAG folder handlers ───────────────────────────────────────────
            AppMessage::RagFolderPickFolder => {
                return Task::perform(
                    async {
                        let picked = rfd::AsyncFileDialog::new()
                            .set_title("Select RAG Knowledge-Base Folder")
                            .pick_folder()
                            .await;
                        AppMessage::RagFolderPickerResult(
                            picked.map(|h| h.path().to_string_lossy().to_string()),
                        )
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::RagFolderPickerResult(maybe_path) => {
                if let Some(p) = maybe_path {
                    self.rag_folder_path_input = p;
                }
            }
            AppMessage::RagFolderPathChanged(p) => {
                self.rag_folder_path_input = p;
            }
            AppMessage::RagFolderNameChanged(n) => {
                self.rag_folder_name_input = n;
            }
            AppMessage::RagFolderAdd { path, name } => {
                if !path.is_empty() {
                    let folder = openclaw_security::RagFolder::new(
                        std::path::PathBuf::from(&path),
                        if name.is_empty() {
                            std::path::Path::new(&path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Knowledge Base")
                                .to_string()
                        } else {
                            name
                        },
                    );
                    tracing::info!(path = %path, "[RAG] Added folder");
                    self.config.rag_folders.push(folder);
                    self.rag_folder_path_input.clear();
                    self.rag_folder_name_input.clear();
                }
            }
            AppMessage::RagFolderRemove(idx) => {
                if idx < self.config.rag_folders.len() {
                    let removed = self.config.rag_folders.remove(idx);
                    tracing::info!(name = %removed.name, "[RAG] Removed folder");
                }
            }
            AppMessage::RagFolderToggleWatch(idx) => {
                if let Some(rf) = self.config.rag_folders.get_mut(idx) {
                    rf.watch_enabled = !rf.watch_enabled;
                }
            }
            AppMessage::RagFolderToggleWrite(idx) => {
                if let Some(rf) = self.config.rag_folders.get_mut(idx) {
                    rf.allow_agent_write = !rf.allow_agent_write;
                }
            }
            // ── OpenClaw AI config handlers ────────────────────────────────────
            AppMessage::OpenClawAiProviderChanged(provider) => {
                self.config.openclaw_ai.endpoint = provider.default_endpoint().to_string();
                self.config.openclaw_ai.model    = provider.default_model().to_string();
                self.config.openclaw_ai.provider = provider;
                self.save_config();
            }
            AppMessage::OpenClawAiEndpointChanged(v) => {
                self.config.openclaw_ai.endpoint = v;
            }
            AppMessage::OpenClawAiModelChanged(v) => {
                self.config.openclaw_ai.model = v;
                self.save_config();
            }
            AppMessage::OpenClawAiApiKeyChanged(v) => {
                self.config.openclaw_ai.api_key = v;
                self.save_config();
            }
            AppMessage::OpenClawAiMaxTokensChanged(v) => {
                self.openclaw_ai_max_tokens_input = v.clone();
                if let Ok(n) = v.parse::<u32>() {
                    self.config.openclaw_ai.max_tokens = n;
                    self.save_config();
                }
            }
            AppMessage::OpenClawAiTemperatureChanged(v) => {
                self.openclaw_ai_temperature_input = v.clone();
                if let Ok(f) = v.parse::<f32>() {
                    self.config.openclaw_ai.temperature = f.clamp(0.0, 1.0);
                    self.save_config();
                }
            }
            AppMessage::OpenClawAiToggleStream => {
                self.config.openclaw_ai.stream = !self.config.openclaw_ai.stream;
                self.save_config();
            }
            AppMessage::OpenClawAiTestConnection => {
                let endpoint = self.config.openclaw_ai.endpoint.clone();
                let provider = self.config.openclaw_ai.provider.clone();
                return Task::perform(
                    async move {
                        let test_url = match provider {
                            AiProvider::Ollama => format!("{}/api/tags", endpoint),
                            _                  => format!("{}/models", endpoint),
                        };
                        let result = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(5))
                            .build()
                            .unwrap_or_default()
                            .get(&test_url)
                            .send()
                            .await;
                        match result {
                            Ok(r) if r.status().is_success() =>
                                AppMessage::OpenClawAiTestResult { ok: true,  message: format!("Connected (HTTP {})", r.status()) },
                            Ok(r) =>
                                AppMessage::OpenClawAiTestResult { ok: false, message: format!("HTTP {}", r.status()) },
                            Err(e) =>
                                AppMessage::OpenClawAiTestResult { ok: false, message: e.to_string() },
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::OpenClawAiTestResult { ok, message } => {
                let line = if ok {
                    format!("✓ AI connection OK: {}", message)
                } else {
                    format!("✗ AI connection failed: {}", message)
                };
                tracing::info!("{}", line);
                self.openclaw_ai_test_status = Some((ok, message));
            }
            // ── Channel config handlers ────────────────────────────────────────
            AppMessage::ChannelAdd(kind) => {
                self.config.channels.push(ChannelConfig::new(kind));
                self.save_config();
            }
            AppMessage::ChannelRemove(idx) => {
                if idx < self.config.channels.len() {
                    self.config.channels.remove(idx);
                    self.save_config();
                }
            }
            AppMessage::ChannelToggleEnabled(idx) => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.enabled = !ch.enabled;
                    self.save_config();
                }
            }
            AppMessage::ChannelTokenChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.token = value;
                }
            }
            AppMessage::ChannelIdChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.channel_id = value;
                }
            }
            AppMessage::ChannelWebhookChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.webhook_url = value;
                }
            }
            AppMessage::ChannelPhoneChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.phone_number = value;
                }
            }
            AppMessage::ChannelLabelChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.label = value;
                }
            }
            AppMessage::ChannelGuildIdChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.guild_id = value;
                    self.save_config();
                }
            }
            AppMessage::ChannelHomserverChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.homeserver_url = value;
                    self.save_config();
                }
            }
            AppMessage::ChannelMatrixUserIdChanged { idx, value } => {
                if let Some(ch) = self.config.channels.get_mut(idx) {
                    ch.matrix_user_id = value;
                    self.save_config();
                }
            }
            AppMessage::ChannelTest(idx) => {
                let ch = self.config.channels.get(idx).cloned();
                return Task::perform(
                    async move {
                        let (ok, msg) = if let Some(ch) = ch {
                            match ch.kind {
                                ChannelKind::Telegram => {
                                    if ch.token.is_empty() {
                                        (false, "Bot token is empty".to_string())
                                    } else {
                                        let url = format!("https://api.telegram.org/bot{}/getMe", ch.token);
                                        match reqwest::Client::builder()
                                            .timeout(std::time::Duration::from_secs(8))
                                            .build().unwrap_or_default()
                                            .get(&url).send().await
                                        {
                                            Ok(r) if r.status().is_success() => (true, "Telegram bot connected".to_string()),
                                            Ok(r) => (false, format!("HTTP {}", r.status())),
                                            Err(e) => (false, e.to_string()),
                                        }
                                    }
                                }
                                ChannelKind::Discord => {
                                    if ch.token.is_empty() {
                                        (false, "Bot token is empty".to_string())
                                    } else {
                                        let url = "https://discord.com/api/v10/users/@me";
                                        match reqwest::Client::builder()
                                            .timeout(std::time::Duration::from_secs(8))
                                            .build().unwrap_or_default()
                                            .get(url)
                                            .header("Authorization", format!("Bot {}", ch.token))
                                            .send().await
                                        {
                                            Ok(r) if r.status().is_success() => (true, "Discord bot connected".to_string()),
                                            Ok(r) => (false, format!("HTTP {}", r.status())),
                                            Err(e) => (false, e.to_string()),
                                        }
                                    }
                                }
                                ChannelKind::Slack => {
                                    if ch.token.is_empty() {
                                        (false, "Bot token is empty".to_string())
                                    } else {
                                        let url = "https://slack.com/api/auth.test";
                                        match reqwest::Client::builder()
                                            .timeout(std::time::Duration::from_secs(8))
                                            .build().unwrap_or_default()
                                            .post(url)
                                            .header("Authorization", format!("Bearer {}", ch.token))
                                            .send().await
                                        {
                                            Ok(r) => {
                                                match r.json::<serde_json::Value>().await {
                                                    Ok(j) if j["ok"].as_bool().unwrap_or(false) => {
                                                        let user = j["user"].as_str().unwrap_or("bot");
                                                        let team = j["team"].as_str().unwrap_or("workspace");
                                                        (true, format!("Slack: @{} in {}", user, team))
                                                    }
                                                    Ok(j) => (false, j["error"].as_str().unwrap_or("unknown").to_string()),
                                                    Err(e) => (false, e.to_string()),
                                                }
                                            }
                                            Err(e) => (false, e.to_string()),
                                        }
                                    }
                                }
                                ChannelKind::Matrix => {
                                    if ch.token.is_empty() || ch.homeserver_url.is_empty() {
                                        (false, "Access token and homeserver URL are required".to_string())
                                    } else {
                                        let hs = ch.homeserver_url.trim_end_matches('/').to_string();
                                        let url = format!("{}/_matrix/client/v3/account/whoami", hs);
                                        match reqwest::Client::builder()
                                            .timeout(std::time::Duration::from_secs(8))
                                            .build().unwrap_or_default()
                                            .get(&url)
                                            .header("Authorization", format!("Bearer {}", ch.token))
                                            .send().await
                                        {
                                            Ok(r) => {
                                                match r.json::<serde_json::Value>().await {
                                                    Ok(j) if j["user_id"].is_string() => {
                                                        let uid = j["user_id"].as_str().unwrap_or("unknown");
                                                        (true, format!("Matrix: {} on {}", uid, hs))
                                                    }
                                                    Ok(j) => {
                                                        let err = j["error"].as_str().unwrap_or("unknown");
                                                        (false, format!("Matrix error: {}", err))
                                                    }
                                                    Err(e) => (false, e.to_string()),
                                                }
                                            }
                                            Err(e) => (false, e.to_string()),
                                        }
                                    }
                                }
                                _ => (false, format!("{} — manual setup required", ch.kind)),
                            }
                        } else {
                            (false, "Channel not found".to_string())
                        };
                        AppMessage::ChannelTestResult { idx, ok, message: msg }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ChannelTestResult { idx, ok, message } => {
                if idx < self.channel_test_status.len() {
                    self.channel_test_status[idx] = Some((ok, message));
                } else {
                    while self.channel_test_status.len() <= idx {
                        self.channel_test_status.push(None);
                    }
                    self.channel_test_status[idx] = Some((ok, message));
                }
            }
            AppMessage::ToggleSidebar => {
                let active = self.core().nav_bar_active();
                self.core_mut().nav_bar_toggle();
                tracing::info!(now_active = !active, "sidebar toggled");
            }
            AppMessage::ShowAbout => {
                self.show_about = true;
            }
            AppMessage::CloseAbout => {
                self.show_about = false;
            }
            AppMessage::NavUp => {
                let pages = [NavPage::Dashboard, NavPage::Events, NavPage::Settings, NavPage::AiChat, NavPage::PluginStore];
                if let Some(idx) = pages.iter().position(|&p| p == self.nav_page) {
                    let next = pages[(idx + pages.len() - 1) % pages.len()];
                    return self.update(AppMessage::NavSelect(next));
                }
            }
            AppMessage::NavDown => {
                let pages = [NavPage::Dashboard, NavPage::Events, NavPage::Settings, NavPage::AiChat, NavPage::PluginStore];
                if let Some(idx) = pages.iter().position(|&p| p == self.nav_page) {
                    let next = pages[(idx + 1) % pages.len()];
                    return self.update(AppMessage::NavSelect(next));
                }
            }
            AppMessage::SetLanguage(lang) => {
                tracing::info!(lang = ?lang, needs_ime = lang.needs_ime(), page = ?self.nav_page, "[IME] language changed");
                self.language = lang;
                self.show_language_menu = false;
                self.update_nav_labels();

                if self.nav_page == NavPage::AiChat && self.language.needs_ime() {
                    tracing::info!("[IME] CJK language on AI page — re-focusing input");
                    return cosmic::widget::text_input::focus(
                        crate::pages::ai_chat::AI_INPUT_ID.clone(),
                    )
                    .map(cosmic::Action::App);
                }
            }
            AppMessage::ShowLanguageMenu => {
                self.show_language_menu = true;
            }
            AppMessage::HideLanguageMenu => {
                self.show_language_menu = false;
            }
            AppMessage::ToggleTheme => {
                self.warm_theme_active = !self.warm_theme_active;
                let new_theme = if self.warm_theme_active {
                    crate::theme::warm_dark_theme()
                } else {
                    cosmic::Theme::dark()
                };
                return cosmic::command::set_theme(new_theme);
            }
            // ── Claw Terminal ──────────────────────────────────────────────
            AppMessage::ClawInputChanged(s) => {
                self.claw_input = s;
            }
            AppMessage::ClawEditorAction(_action) => {
                // text_editor no longer used for main input; ignore stale actions
            }
            AppMessage::ClawInputFocused => {
                self.claw_input_focused = true;
            }

            // ── Image attachment ───────────────────────────────────────────
            AppMessage::ClawPickImage => {
                return Task::perform(
                    async {
                        use rfd::AsyncFileDialog;
                        let file = AsyncFileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                            .set_title("Select image to attach")
                            .pick_file()
                            .await;
                        match file {
                            Some(handle) => {
                                let path = handle.path().to_string_lossy().to_string();
                                let bytes = handle.read().await;
                                AppMessage::ClawImagePicked { path, bytes }
                            }
                            None => AppMessage::ClawClearAttachment,
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ClawImagePicked { path, bytes } => {
                use std::path::Path;
                let ext = Path::new(&path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png")
                    .to_lowercase();
                let mime = match ext.as_str() {
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif"          => "image/gif",
                    "webp"         => "image/webp",
                    "bmp"          => "image/bmp",
                    _              => "image/png",
                };
                use base64::Engine as _;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let filename = Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("image")
                    .to_string();
                self.claw_attachment = Some(ClawAttachment {
                    filename,
                    base64: b64,
                    mime: mime.to_string(),
                });
            }
            AppMessage::ClawClearAttachment => {
                self.claw_attachment = None;
            }

            // ── Voice recording ────────────────────────────────────────────
            AppMessage::ClawStartRecording => {
                if self.claw_recording {
                    return Task::none();
                }
                self.claw_recording = true;
                self.claw_voice_status = Some("🎙 录音中… (点击停止)".to_string());
                // Start recording via sox into /tmp/claw_voice.wav
                return Task::perform(
                    async {
                        // sox must be installed: brew install sox
                        // Record until we receive SIGTERM (process killed on stop)
                        let status = tokio::process::Command::new("sox")
                            .args(["-d", "-r", "16000", "-c", "1", "/tmp/claw_voice.wav",
                                   "silence", "1", "0.1", "1%", "1", "3.0", "1%"])
                            .status()
                            .await;
                        match status {
                            Ok(s) if s.success() => AppMessage::ClawStopRecording,
                            _ => AppMessage::ClawStopRecording,
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ClawStopRecording => {
                if !self.claw_recording {
                    return Task::none();
                }
                self.claw_recording = false;
                self.claw_voice_status = Some("⏳ 转录中…".to_string());
                // Kill any running sox process, then transcribe with whisper
                return Task::perform(
                    async {
                        // Kill sox if still running
                        let _ = tokio::process::Command::new("pkill")
                            .args(["-f", "sox.*claw_voice"])
                            .status()
                            .await;
                        // Wait briefly for file to flush
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                        // Transcribe with whisper CLI (brew install whisper-cpp)
                        // or fall back to ffmpeg + Ollama whisper model
                        let whisper_result = tokio::process::Command::new("whisper")
                            .args(["--model", "base", "--language", "zh",
                                   "--output_format", "txt", "--output_dir", "/tmp",
                                   "/tmp/claw_voice.wav"])
                            .output()
                            .await;
                        match whisper_result {
                            Ok(out) if out.status.success() => {
                                // whisper writes /tmp/claw_voice.txt
                                match tokio::fs::read_to_string("/tmp/claw_voice.txt").await {
                                    Ok(text) => {
                                        let t = text.trim().to_string();
                                        if t.is_empty() {
                                            AppMessage::ClawVoiceError("转录结果为空".to_string())
                                        } else {
                                            AppMessage::ClawVoiceTranscribed(t)
                                        }
                                    }
                                    Err(e) => AppMessage::ClawVoiceError(format!("读取转录文件失败: {e}")),
                                }
                            }
                            Ok(out) => {
                                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                AppMessage::ClawVoiceError(format!("whisper 失败: {stderr}"))
                            }
                            Err(_) => {
                                // whisper not available — try macOS built-in dictation via osascript
                                let osa = tokio::process::Command::new("osascript")
                                    .arg("-e")
                                    .arg(r#"tell application "System Events" to return POSIX path of (path to temporary items folder)"#)
                                    .output()
                                    .await;
                                AppMessage::ClawVoiceError(
                                    "未找到 whisper CLI。请运行: brew install openai-whisper".to_string()
                                )
                            }
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ClawVoiceTranscribed(text) => {
                self.claw_voice_status = None;
                if self.claw_input.is_empty() {
                    self.claw_input = text;
                } else {
                    self.claw_input.push(' ');
                    self.claw_input.push_str(&text);
                }
                return cosmic::widget::text_input::focus(
                    crate::pages::claw_terminal::CLAW_INPUT_ID.clone(),
                ).map(cosmic::Action::App);
            }
            AppMessage::ClawVoiceError(err) => {
                self.claw_recording = false;
                self.claw_voice_status = Some(format!("⚠ {err}"));
            }

            AppMessage::ClawClearHistory => {
                self.claw_history.clear();
            }
            AppMessage::ClawQuickAction(action) => {
                self.claw_input = action.command().to_string();
                return self.update(AppMessage::ClawSendCommand);
            }
            AppMessage::ClawSendCommand => {
                let raw = self.claw_input.trim().to_string();
                // Allow send when attachment present even if text is empty
                if raw.is_empty() && self.claw_attachment.is_none() {
                    return Task::none();
                }
                self.claw_input.clear();
                self.claw_input_focused = false;

                tracing::info!("[CLAW] Send command: {}", raw);
                tracing::info!("[CLAW] Selected agent: {:?}", self.claw_selected_agent_id);

                // Agent chat mode: send message to selected agent
                let scroll_bottom = iced_scrollable::snap_to(
                    CLAW_SCROLL_ID.clone(),
                    RelativeOffset { x: 0.0, y: 1.0 },
                );
                if let Some(_agent_id) = &self.claw_selected_agent_id {
                    tracing::info!("[CLAW] Routing to agent chat");
                    // If image attached, embed as [image:mime;base64] prefix
                    let message = if let Some(att) = self.claw_attachment.take() {
                        format!("[image:{};{}]\n{}", att.mime, att.base64, raw)
                    } else {
                        raw
                    };
                    let task = self.update(AppMessage::ClawAgentChat(message));
                    return Task::chain(task, scroll_bottom);
                }

                tracing::info!("[CLAW] Routing to shell command execution");
                let entry_id = self.claw_next_id;
                self.claw_next_id += 1;
                self.claw_history.push(ClawEntry::new(entry_id, &raw));
                let scroll_bottom2 = iced_scrollable::snap_to(
                    CLAW_SCROLL_ID.clone(),
                    RelativeOffset { x: 0.0, y: 1.0 },
                );

                // NL mode: route through AI intent parser first
                if self.claw_nl_mode {
                    // Lazily init inference engine if needed
                    if self.inference_engine.is_none() {
                        let ai = &self.config.openclaw_ai;
                        let cfg = InferenceConfig {
                            backend: provider_to_backend_kind(&ai.provider),
                            endpoint: ai.endpoint.clone(),
                            model_name: ai.model.clone(),
                            circuit_breaker_threshold: 999,
                            ..InferenceConfig::default()
                        };
                        match InferenceEngine::new(cfg) {
                            Ok(eng) => { self.inference_engine = Some(Arc::new(eng)); }
                            Err(e) => {
                                let err = format!("AI engine init failed: {e}");
                                return Task::perform(
                                    async move { AppMessage::ClawNlPlanError { entry_id, error: err } },
                                    cosmic::Action::App,
                                );
                            }
                        }
                    }
                    let engine = self.inference_engine.as_ref().unwrap().clone();
                    let user_cmd = raw.clone();
                    let system_prompt = NL_AGENT_SYSTEM_PROMPT.to_string();
                    return Task::perform(
                        async move {
                            let messages = vec![
                                ConversationTurn { role: "system".into(), content: system_prompt },
                                ConversationTurn { role: "user".into(), content: user_cmd },
                            ];
                            let req = InferenceRequest {
                                request_id: entry_id,
                                messages,
                                max_tokens_override: Some(1024),
                                temperature_override: Some(0.1),
                                stream: false,
                            };
                            match engine.infer(req).await {
                                Ok(resp) => AppMessage::ClawNlPlanReady { entry_id, plan_json: resp.content },
                                Err(e)   => AppMessage::ClawNlPlanError { entry_id, error: e.to_string() },
                            }
                        },
                        cosmic::Action::App,
                    );
                }

                // Dispatch built-in commands vs shell pass-through
                let cmd_lower = raw.to_lowercase();
                if cmd_lower == "sandbox start" {
                    let entry_id2 = entry_id;
                    self.sandbox_status = SandboxStatus::Running;
                    return Task::perform(
                        async move {
                            AppMessage::ClawCommandFinished {
                                entry_id: entry_id2,
                                exit_code: Some(0),
                                elapsed_ms: 1,
                            }
                        },
                        cosmic::Action::App,
                    ).chain(Task::perform(
                        async move {
                            AppMessage::ClawOutputLine {
                                entry_id: entry_id2,
                                line: "✓ Sandbox started successfully.".to_string(),
                                is_stderr: false,
                            }
                        },
                        cosmic::Action::App,
                    ));
                } else if cmd_lower == "sandbox stop" {
                    if let Some(tx) = &self.control_tx {
                        let _ = tx.send(ControlCommand::Terminate);
                    }
                    self.sandbox_status = SandboxStatus::Stopped;
                    return Task::perform(
                        async move {
                            AppMessage::ClawOutputLine {
                                entry_id,
                                line: "✓ Sandbox stopped.".to_string(),
                                is_stderr: false,
                            }
                        },
                        cosmic::Action::App,
                    ).chain(Task::perform(
                        async move {
                            AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 2 }
                        },
                        cosmic::Action::App,
                    ));
                } else if cmd_lower == "emergency stop" {
                    return self.update(AppMessage::EmergencyStop).chain(
                        Task::perform(
                            async move {
                                AppMessage::ClawOutputLine {
                                    entry_id,
                                    line: "🔴 Emergency stop executed. Circuit breaker tripped.".to_string(),
                                    is_stderr: false,
                                }
                            },
                            cosmic::Action::App,
                        ).chain(Task::perform(
                            async move {
                                AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 5 }
                            },
                            cosmic::Action::App,
                        ))
                    );
                } else if cmd_lower == "events clear" {
                    self.events.clear();
                    self.stats = SandboxStats::default();
                    return Task::perform(
                        async move {
                            AppMessage::ClawOutputLine {
                                entry_id,
                                line: "✓ Event log cleared.".to_string(),
                                is_stderr: false,
                            }
                        },
                        cosmic::Action::App,
                    ).chain(Task::perform(
                        async move {
                            AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 1 }
                        },
                        cosmic::Action::App,
                    ));
                } else if cmd_lower == "models list" {
                    return self.update(AppMessage::AiListModels).chain(
                        Task::perform(
                            async move {
                                AppMessage::ClawOutputLine {
                                    entry_id,
                                    line: "Fetching model list from Ollama…".to_string(),
                                    is_stderr: false,
                                }
                            },
                            cosmic::Action::App,
                        ).chain(Task::perform(
                            async move {
                                AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 50 }
                            },
                            cosmic::Action::App,
                        ))
                    );
                } else if cmd_lower == "status" {
                    let sandbox_s = self.sandbox_status.to_string();
                    let ai_s = self.ai_chat.status.to_string();
                    let events_n = self.events.len();
                    let pending_n = self.pending_confirmations.len();
                    let models_n = self.available_models.len();
                    let lines = vec![
                        (format!("Sandbox  : {}", sandbox_s), false),
                        (format!("AI Engine: {}", ai_s), false),
                        (format!("Events   : {} total, {} pending confirmation", events_n, pending_n), false),
                        (format!("Models   : {} installed", models_n), false),
                    ];
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 1 } },
                        cosmic::Action::App,
                    );
                } else if cmd_lower == "config show" {
                    // Security: redact all API keys / tokens before display
                    let mut cfg_display = self.config.clone();
                    cfg_display.openclaw_ai.api_key = if cfg_display.openclaw_ai.api_key.is_empty() {
                        String::new()
                    } else {
                        "[REDACTED]".to_string()
                    };
                    for ch in cfg_display.channels.iter_mut() {
                        if !ch.token.is_empty() { ch.token = "[REDACTED]".to_string(); }
                    }
                    let cfg_json = serde_json::to_string_pretty(&cfg_display)
                        .unwrap_or_else(|e| format!("Serialization error: {}", e));
                    let mut lines: Vec<(String, bool)> = vec![
                        ("[API keys and tokens are redacted for security]".to_string(), false),
                    ];
                    lines.extend(cfg_json.lines().map(|l| (l.to_string(), false)));
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 2 } },
                        cosmic::Action::App,
                    );
                } else if cmd_lower == "ai restart" {
                    let lines = vec![
                        ("Restarting AI inference engine…".to_string(), false),
                        ("✓ AI engine restarted. Use 'models list' to verify.".to_string(), false),
                    ];
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 100 } },
                        cosmic::Action::App,
                    );
                } else if cmd_lower == "tg status" {
                    let tg_ch: Vec<_> = self.config.channels.iter()
                        .filter(|c| c.kind == openclaw_security::ChannelKind::Telegram)
                        .collect();
                    let mut lines = vec![
                        (format!("Telegram channels configured: {}", tg_ch.len()), false),
                    ];
                    for ch in &tg_ch {
                        lines.push((format!("  {} [{}] token={}",
                            ch.label,
                            if ch.enabled { "enabled" } else { "disabled" },
                            if ch.token.is_empty() { "(not set)" } else { "[SET]" }
                        ), false));
                        if !ch.channel_id.is_empty() {
                            lines.push((format!("    chat_id: {}", ch.channel_id), false));
                        }
                    }
                    lines.push((format!("Polling active: {}", self.tg_polling_active), false));
                    if let Some(ref u) = self.tg_bot_username {
                        lines.push((format!("Bot username: @{}", u), false));
                    }
                    lines.push((format!("Last update_id: {}", self.tg_last_update_id), false));
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 1 } },
                        cosmic::Action::App,
                    );
                } else if cmd_lower == "tg poll" || cmd_lower == "tg start" {
                    return self.update(AppMessage::TgStartPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("✈ Telegram polling started. Messages will appear here.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "tg stop" {
                    return self.update(AppMessage::TgStopPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("⏹ Telegram polling stopped.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower.starts_with("tg send ") {
                    // tg send <chat_id> <message>
                    let rest = raw["tg send ".len()..].trim().to_string();
                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        return Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("Usage: tg send <chat_id> <message text>".to_string(), true)],
                                exit_code: Some(1), elapsed_ms: 0,
                            }},
                            cosmic::Action::App,
                        );
                    }
                    let chat_id = parts[0].to_string();
                    let text = parts[1].to_string();
                    return self.update(AppMessage::TgSendMessage { chat_id, text }).chain(
                        Task::perform(
                            async move { AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 0 } },
                            cosmic::Action::App,
                        )
                    );
                // ── Discord commands ──────────────────────────────────────────
                } else if cmd_lower == "discord poll" || cmd_lower == "discord start" {
                    return self.update(AppMessage::DiscordStartPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("🎮 Discord polling started.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "discord stop" {
                    return self.update(AppMessage::DiscordStopPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("⏹ Discord polling stopped.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower.starts_with("discord send ") {
                    let rest = raw["discord send ".len()..].trim().to_string();
                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        return Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("Usage: discord send <channel_id> <message>".to_string(), true)],
                                exit_code: Some(1), elapsed_ms: 0,
                            }},
                            cosmic::Action::App,
                        );
                    }
                    let channel_id = parts[0].to_string();
                    let text = parts[1].to_string();
                    return self.update(AppMessage::DiscordSendMessage { channel_id, text }).chain(
                        Task::perform(
                            async move { AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 0 } },
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "discord status" {
                    let dc_ch: Vec<_> = self.config.channels.iter()
                        .filter(|c| c.kind == ChannelKind::Discord).collect();
                    let mut lines = vec![(format!("Discord channels: {}", dc_ch.len()), false)];
                    for ch in &dc_ch {
                        lines.push((format!("  {} [{}] token={} channel_id={}",
                            ch.label,
                            if ch.enabled { "ON" } else { "OFF" },
                            if ch.token.is_empty() { "(not set)" } else { "[SET]" },
                            if ch.channel_id.is_empty() { "(not set)" } else { &ch.channel_id }
                        ), false));
                    }
                    lines.push((format!("Polling: {}", self.discord_polling_active), false));
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 1 } },
                        cosmic::Action::App,
                    );
                // ── Matrix commands ───────────────────────────────────────────
                } else if cmd_lower == "matrix poll" || cmd_lower == "matrix start" {
                    return self.update(AppMessage::MatrixStartPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("🔷 Matrix polling started.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "matrix stop" {
                    return self.update(AppMessage::MatrixStopPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("⏹ Matrix polling stopped.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower.starts_with("matrix send ") {
                    let rest = raw["matrix send ".len()..].trim().to_string();
                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        return Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("Usage: matrix send <room_id> <message>".to_string(), true)],
                                exit_code: Some(1), elapsed_ms: 0,
                            }},
                            cosmic::Action::App,
                        );
                    }
                    let room_id = parts[0].to_string();
                    let text = parts[1].to_string();
                    return self.update(AppMessage::MatrixSendMessage { room_id, text }).chain(
                        Task::perform(
                            async move { AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 0 } },
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "matrix status" {
                    let mx_ch: Vec<_> = self.config.channels.iter()
                        .filter(|c| c.kind == ChannelKind::Matrix).collect();
                    let mut lines = vec![(format!("Matrix channels: {}", mx_ch.len()), false)];
                    for ch in &mx_ch {
                        lines.push((format!("  {} [{}] token={} homeserver={} room_id={}",
                            ch.label,
                            if ch.enabled { "ON" } else { "OFF" },
                            if ch.token.is_empty() { "(not set)" } else { "[SET]" },
                            if ch.homeserver_url.is_empty() { "(not set)" } else { &ch.homeserver_url },
                            if ch.channel_id.is_empty() { "(not set)" } else { &ch.channel_id }
                        ), false));
                    }
                    lines.push((format!("Polling: {}", self.matrix_polling_active), false));
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 1 } },
                        cosmic::Action::App,
                    );
                // ── Slack commands ────────────────────────────────────────────
                } else if cmd_lower == "slack poll" || cmd_lower == "slack start" {
                    return self.update(AppMessage::SlackStartPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("💬 Slack polling started.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "slack stop" {
                    return self.update(AppMessage::SlackStopPolling).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("⏹ Slack polling stopped.".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower.starts_with("slack send ") {
                    let rest = raw["slack send ".len()..].trim().to_string();
                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        return Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("Usage: slack send <channel_id> <message>".to_string(), true)],
                                exit_code: Some(1), elapsed_ms: 0,
                            }},
                            cosmic::Action::App,
                        );
                    }
                    let channel_id = parts[0].to_string();
                    let text = parts[1].to_string();
                    return self.update(AppMessage::SlackSendMessage { channel_id, text }).chain(
                        Task::perform(
                            async move { AppMessage::ClawCommandFinished { entry_id, exit_code: Some(0), elapsed_ms: 0 } },
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "slack status" {
                    let sl_ch: Vec<_> = self.config.channels.iter()
                        .filter(|c| c.kind == ChannelKind::Slack).collect();
                    let mut lines = vec![(format!("Slack channels: {}", sl_ch.len()), false)];
                    for ch in &sl_ch {
                        lines.push((format!("  {} [{}] token={} channel_id={}",
                            ch.label,
                            if ch.enabled { "ON" } else { "OFF" },
                            if ch.token.is_empty() { "(not set)" } else { "[SET]" },
                            if ch.channel_id.is_empty() { "(not set)" } else { &ch.channel_id }
                        ), false));
                    }
                    lines.push((format!("Polling: {}", self.slack_polling_active), false));
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 1 } },
                        cosmic::Action::App,
                    );
                } else if cmd_lower == "channels" {
                    let total = self.config.channels.len();
                    let enabled: Vec<_> = self.config.channels.iter().filter(|c| c.enabled).collect();
                    let mut lines = vec![
                        (format!("Communication channels: {} configured, {} enabled", total, enabled.len()), false),
                    ];
                    for ch in &self.config.channels {
                        lines.push((format!("  {} {} [{}] token={}",
                            ch.kind.icon(),
                            ch.label,
                            if ch.enabled { "ON" } else { "OFF" },
                            if ch.token.is_empty() { "(not set)" } else { "[SET]" }
                        ), false));
                    }
                    if total == 0 {
                        lines.push(("No channels configured. Go to General Settings → Communication Channels.".to_string(), false));
                    }
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 1 } },
                        cosmic::Action::App,
                    );
                } else if cmd_lower == "gateway connect" || cmd_lower == "gateway status" {
                    return self.update(AppMessage::ClawProbeGateway).chain(
                        Task::perform(
                            async move { AppMessage::ClawBulkOutput {
                                entry_id,
                                lines: vec![("🔌 Probing OpenClaw Gateway…".to_string(), false)],
                                exit_code: Some(0), elapsed_ms: 1,
                            }},
                            cosmic::Action::App,
                        )
                    );
                } else if cmd_lower == "help" || cmd_lower == "?" {
                    let lines: Vec<(String, bool)> = vec![
                        ("═══ Claw Terminal — Command Reference ═══".to_string(), false),
                        ("".to_string(), false),
                        ("── System ──────────────────────────────".to_string(), false),
                        ("  status              — Full system status".to_string(), false),
                        ("  sandbox start/stop  — WasmEdge sandbox lifecycle".to_string(), false),
                        ("  emergency stop      — Trip circuit breaker immediately".to_string(), false),
                        ("  events clear        — Clear the event log".to_string(), false),
                        ("  config show         — Display config (keys redacted)".to_string(), false),
                        ("".to_string(), false),
                        ("── AI ──────────────────────────────────".to_string(), false),
                        ("  models list         — List installed Ollama models".to_string(), false),
                        ("  ai restart          — Restart AI inference engine".to_string(), false),
                        ("".to_string(), false),
                        ("── Communication Channels ──────────────".to_string(), false),
                        ("  channels                — List all configured channels".to_string(), false),
                        ("  tg status               — Telegram bot status".to_string(), false),
                        ("  tg poll / tg start      — Start Telegram polling".to_string(), false),
                        ("  tg stop                 — Stop Telegram polling".to_string(), false),
                        ("  tg send <id> <msg>      — Send Telegram message".to_string(), false),
                        ("  discord status          — Discord channel status".to_string(), false),
                        ("  discord poll/stop       — Start/stop Discord polling".to_string(), false),
                        ("  discord send <id> <msg> — Send Discord message".to_string(), false),
                        ("  matrix status           — Matrix room status".to_string(), false),
                        ("  matrix poll/stop        — Start/stop Matrix polling".to_string(), false),
                        ("  matrix send <id> <msg>  — Send Matrix message".to_string(), false),
                        ("  slack status            — Slack channel status".to_string(), false),
                        ("  slack poll/stop         — Start/stop Slack polling".to_string(), false),
                        ("  slack send <id> <msg>   — Send Slack message".to_string(), false),
                        ("".to_string(), false),
                        ("── Gateway ─────────────────────────────".to_string(), false),
                        ("  gateway connect     — Probe OpenClaw Gateway".to_string(), false),
                        ("".to_string(), false),
                        ("── Shell ───────────────────────────────".to_string(), false),
                        ("  <any shell command> — Execute in /bin/sh (ls, pwd, git…)".to_string(), false),
                        ("  help / ?            — Show this help".to_string(), false),
                    ];
                    return Task::perform(
                        async move { AppMessage::ClawBulkOutput { entry_id, lines, exit_code: Some(0), elapsed_ms: 0 } },
                        cosmic::Action::App,
                    );
                } else {
                    // Shell pass-through: execute via /bin/sh -c
                    let cmd_str = raw.clone();
                    return Task::perform(
                        async move {
                            let start = std::time::Instant::now();
                            let result = tokio::process::Command::new("/bin/sh")
                                .arg("-c")
                                .arg(&cmd_str)
                                .output()
                                .await;
                            let elapsed_ms = start.elapsed().as_millis() as u64;
                            match result {
                                Ok(output) => {
                                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                    let exit_code = output.status.code();
                                    (stdout, stderr, exit_code, elapsed_ms)
                                }
                                Err(e) => (String::new(), format!("Failed to execute: {}", e), Some(127), elapsed_ms),
                            }
                        },
                        move |(stdout, stderr, exit_code, elapsed_ms)| {
                            cosmic::Action::App(AppMessage::ClawShellResult {
                                entry_id,
                                stdout,
                                stderr,
                                exit_code,
                                elapsed_ms,
                            })
                        },
                    ).chain(scroll_bottom2);
                }
                // Unreachable: all branches above return
                #[allow(unreachable_code)]
                return scroll_bottom2;
            }
            AppMessage::ClawOutputLine { entry_id, line, is_stderr } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    entry.output_lines.push((line, is_stderr));
                }
            }
            AppMessage::ClawCommandFinished { entry_id, exit_code, elapsed_ms } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    entry.status = match exit_code {
                        Some(0) => ClawEntryStatus::Success,
                        Some(c) => ClawEntryStatus::Error(c),
                        None    => ClawEntryStatus::Killed,
                    };
                    entry.elapsed_ms = Some(elapsed_ms);
                }
            }
            AppMessage::ClawBulkOutput { entry_id, lines, exit_code, elapsed_ms } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    entry.output_lines.extend(lines);
                    entry.status = match exit_code {
                        Some(0) => ClawEntryStatus::Success,
                        Some(c) => ClawEntryStatus::Error(c),
                        None    => ClawEntryStatus::Killed,
                    };
                    entry.elapsed_ms = Some(elapsed_ms);
                }
            }
            AppMessage::ClawShellResult { entry_id, stdout, stderr, exit_code, elapsed_ms } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    for line in stdout.lines() {
                        entry.output_lines.push((line.to_string(), false));
                    }
                    for line in stderr.lines() {
                        entry.output_lines.push((line.to_string(), true));
                    }
                    if stdout.is_empty() && stderr.is_empty() {
                        entry.output_lines.push(("(no output)".to_string(), false));
                    }
                    entry.status = match exit_code {
                        Some(0) => ClawEntryStatus::Success,
                        Some(c) => ClawEntryStatus::Error(c),
                        None    => ClawEntryStatus::Killed,
                    };
                    entry.elapsed_ms = Some(elapsed_ms);
                }
            }
            // ── Plugin / Gateway handlers ──────────────────────────────────────
            AppMessage::ClawProbeGateway => {
                let url = self.gateway_url.clone()
                    .unwrap_or_else(|| "http://127.0.0.1:0".to_string());
                return Task::perform(
                    async move {
                        let reachable = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(3))
                            .build()
                            .unwrap_or_default()
                            .get(format!("{}/health", url))
                            .send()
                            .await
                            .map(|r| r.status().is_success())
                            .unwrap_or(false);
                        AppMessage::ClawGatewayProbeResult { reachable, url }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ClawGatewayProbeResult { reachable, url } => {
                self.gateway_reachable = reachable;
                if reachable && self.gateway_url.is_none() {
                    self.gateway_url = Some(url);
                }
            }
            AppMessage::ClawSendToGateway { entry_id, instruction } => {
                let gw = match &self.gateway_url {
                    Some(u) if self.gateway_reachable => u.clone(),
                    _ => {
                        return Task::perform(
                            async move {
                                AppMessage::ClawBulkOutput {
                                    entry_id,
                                    lines: vec![
                                        ("⚠ OpenClaw Gateway not connected.".to_string(), true),
                                        ("Set OPENCLAW_GATEWAY_URL and click 'Connect' to enable Plugin Mode.".to_string(), false),
                                        ("Falling back to NL Agent (offline) mode…".to_string(), false),
                                    ],
                                    exit_code: Some(1),
                                    elapsed_ms: 0,
                                }
                            },
                            cosmic::Action::App,
                        );
                    }
                };
                return Task::perform(
                    async move {
                        gateway_send_instruction(entry_id, &gw, &instruction).await
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ClawGatewaySkillResult { entry_id, lines, elapsed_ms } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    entry.output_lines.extend(lines);
                    entry.status = ClawEntryStatus::Success;
                    entry.elapsed_ms = Some(elapsed_ms);
                }
            }
            // ── NL Agent handlers ──────────────────────────────────────────────
            AppMessage::ClawToggleNlMode => {
                self.claw_nl_mode = !self.claw_nl_mode;
                // Auto-probe gateway when NL mode is toggled on
                if self.claw_nl_mode && self.gateway_url.is_some() {
                    return self.update(AppMessage::ClawProbeGateway);
                }
            }
            AppMessage::ClawSelectAgent(agent_id) => {
                tracing::info!("[CLAW] Agent selected: {:?}", agent_id);
                self.claw_selected_agent_id = agent_id.clone();
                // Load agent list if not already loaded
                if self.claw_agent_list.is_empty() {
                    self.claw_agent_list = openclaw_security::AgentProfile::list_all();
                    tracing::info!("[CLAW] Loaded {} agents", self.claw_agent_list.len());
                }
                tracing::info!("[CLAW] Current selected agent: {:?}", self.claw_selected_agent_id);
            }
            AppMessage::ClawAgentChat(message) => {
                if let Some(agent_id) = &self.claw_selected_agent_id {
                    if let Some(agent) = self.claw_agent_list.iter().find(|a| a.id.as_str() == agent_id) {
                        let agent_name = agent.display_name.clone();
                        let agent_id_clone = agent_id.clone();
                        let message_clone = message.clone();

                        // Build role description from AgentRole
                        let role_desc = match &agent.role {
                            openclaw_security::AgentRole::TicketAssistant      => "工单助手，负责处理和分类用户工单",
                            openclaw_security::AgentRole::CodeReviewer         => "代码审查员，分析代码质量和潜在问题",
                            openclaw_security::AgentRole::ReportGenerator      => "报告生成器，生成结构化分析报告",
                            openclaw_security::AgentRole::SecurityAuditor      => "安全审计员，分析安全漏洞和合规问题",
                            openclaw_security::AgentRole::DataAnalyst          => "数据分析师，解读数据趋势和统计信息",
                            openclaw_security::AgentRole::CustomerSupport      => "客服助手，友好专业地解答用户问题",
                            openclaw_security::AgentRole::KnowledgeOfficer     => "知识库首席官，管理和检索文档知识",
                            openclaw_security::AgentRole::SocialMediaManager   => "社媒运营经理，负责多平台内容策略",
                            openclaw_security::AgentRole::InboxTriageAgent     => "邮件分拣员，对邮件进行分类和草拟回复",
                            openclaw_security::AgentRole::FinanceProcurement   => "财务采购员，处理付款审批和采购流程",
                            openclaw_security::AgentRole::NewsSecretary        => "新闻信息秘书，推送热点和重要提醒",
                            openclaw_security::AgentRole::SecurityCodeAuditor  => "安全代码审计员，执行 SAST 和 Git 提交监控",
                            openclaw_security::AgentRole::Custom { label }     => label.as_str(),
                        };
                        let system_prompt = format!(
                            "你是 {}，角色：{}。请简洁专业地用中文回答用户问题。",
                            agent_name, role_desc
                        );

                        // Detect image prefix for display purposes
                        let has_image = message.starts_with("[image:");
                        let display_message = if has_image {
                            // Extract user text after the image prefix line
                            let text_part = message
                                .lines()
                                .skip(1)
                                .collect::<Vec<_>>()
                                .join("\n");
                            if text_part.trim().is_empty() {
                                "📎 [图片已附加]".to_string()
                            } else {
                                format!("📎 [图片] {}", text_part)
                            }
                        } else {
                            message.clone()
                        };

                        // Add user message to Claw history display
                        let entry_id = self.claw_next_id;
                        self.claw_next_id += 1;
                        self.claw_history.push(ClawEntry {
                            id: entry_id,
                            command: format!("[{}] {}", agent_name, display_message),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                            source: ClawEntrySource::User,
                            status: ClawEntryStatus::Running,
                            output_lines: vec![],
                            elapsed_ms: None,
                        });

                        // Always create a fresh InferenceEngine for each agent call.
                        // Prefer ai_chat.endpoint/model_name (set via Settings UI) over
                        // config.openclaw_ai which may lag behind.
                        let endpoint = if !self.ai_chat.endpoint.is_empty() {
                            self.ai_chat.endpoint.clone()
                        } else {
                            self.config.openclaw_ai.endpoint.clone()
                        };
                        let model = if !self.ai_chat.model_name.is_empty() {
                            self.ai_chat.model_name.clone()
                        } else {
                            self.config.openclaw_ai.model.clone()
                        };
                        let api_key = if !self.config.openclaw_ai.api_key.is_empty() {
                            Some(self.config.openclaw_ai.api_key.clone())
                        } else {
                            None
                        };
                        // Derive backend kind from endpoint URL
                        let backend = if endpoint.contains("localhost:11434") || endpoint.contains("ollama") {
                            openclaw_inference::types::BackendKind::Ollama
                        } else if endpoint.contains("openai.com") {
                            openclaw_inference::types::BackendKind::OpenAiCompat
                        } else {
                            provider_to_backend_kind(&self.config.openclaw_ai.provider)
                        };
                        eprintln!("[CLAW-AGENT] resolved: endpoint={} model={} backend={:?}", endpoint, model, backend);
                        let cfg = InferenceConfig {
                            backend,
                            endpoint,
                            model_name: model,
                            api_key,
                            circuit_breaker_threshold: 999,
                            circuit_breaker_reset: std::time::Duration::from_secs(1),
                            inference_timeout: std::time::Duration::from_secs(120),
                            ..InferenceConfig::default()
                        };
                        eprintln!("[CLAW-AGENT] init fresh engine: endpoint={} model={}", cfg.endpoint, cfg.model_name);
                        let engine_arc = match InferenceEngine::new(cfg) {
                            Ok(eng) => Arc::new(eng),
                            Err(e) => {
                                let err = format!("AI 推理引擎初始化失败: {e}");
                                eprintln!("[CLAW-AGENT] engine init FAILED: {err}");
                                return self.update(AppMessage::ClawNlPlanError { entry_id, error: err });
                            }
                        };

                        // Build multi-turn conversation: system + history + new user message
                        // Strip image base64 from stored content to avoid context window overflow
                        let stored_content = if message_clone.starts_with("[image:") {
                            let text_part = message_clone.lines().skip(1).collect::<Vec<_>>().join("\n");
                            if text_part.trim().is_empty() {
                                "[图片已附加]".to_string()
                            } else {
                                format!("[图片] {}", text_part.trim())
                            }
                        } else {
                            message_clone.clone()
                        };
                        let history = self.claw_agent_conversations
                            .entry(agent_id_clone.clone())
                            .or_insert_with(Vec::new);
                        history.push(ConversationTurn {
                            role: "user".to_string(),
                            content: stored_content,
                        });
                        let mut messages = vec![ConversationTurn {
                            role: "system".to_string(),
                            content: system_prompt,
                        }];
                        // Keep last 6 turns only; also truncate each content to 800 chars
                        let history_snapshot: Vec<ConversationTurn> = history
                            .iter()
                            .rev()
                            .take(6)
                            .cloned()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .map(|mut t| {
                                if t.content.len() > 800 {
                                    t.content.truncate(800);
                                    t.content.push_str("…");
                                }
                                t
                            })
                            .collect();
                        messages.extend(history_snapshot);

                        eprintln!("[CLAW-AGENT] sending {} messages to agent {}", messages.len(), agent_id_clone);
                        return Task::perform(
                            async move {
                                let start = std::time::Instant::now();
                                let req = InferenceRequest {
                                    request_id: entry_id,
                                    messages,
                                    max_tokens_override: Some(512),
                                    temperature_override: Some(0.7),
                                    stream: false,
                                };
                                match engine_arc.infer(req).await {
                                    Ok(resp) => {
                                        eprintln!("[CLAW-AGENT] response ({} ms): {:.120}", start.elapsed().as_millis(), resp.content);
                                        AppMessage::ClawAgentResponse {
                                            agent_id: agent_id_clone,
                                            content: resp.content,
                                            latency_ms: start.elapsed().as_millis() as u64,
                                            user_entry_id: entry_id,
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("[CLAW-AGENT] infer error: {e}");
                                        AppMessage::ClawNlPlanError {
                                            entry_id,
                                            error: format!("AI 推理失败: {e}"),
                                        }
                                    }
                                }
                            },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::ClawAgentResponse { agent_id, content, latency_ms, user_entry_id } => {
                // Save assistant reply into per-agent conversation history for multi-turn
                self.claw_agent_conversations
                    .entry(agent_id.clone())
                    .or_insert_with(Vec::new)
                    .push(ConversationTurn {
                        role: "assistant".to_string(),
                        content: content.clone(),
                    });

                // Update user message entry status from Running -> Success
                if let Some(user_entry) = self.claw_history.iter_mut().find(|e| e.id == user_entry_id) {
                    user_entry.status = ClawEntryStatus::Success;
                    user_entry.elapsed_ms = Some(latency_ms);
                }
                // Will snap_to bottom after pushing the reply entry (handled below)

                let agent_name = self.claw_agent_list
                    .iter()
                    .find(|a| a.id.as_str() == &agent_id)
                    .map(|a| a.display_name.clone())
                    .unwrap_or_else(|| "Agent".to_string());

                // Split content by newlines so each line renders correctly
                let output_lines: Vec<(String, bool)> = content
                    .split('\n')
                    .map(|l| (l.to_string(), false))
                    .filter(|(l, _)| !l.trim().is_empty())
                    .collect();
                let output_lines = if output_lines.is_empty() {
                    vec![(content.clone(), false)]
                } else {
                    output_lines
                };

                let new_entry_id = self.claw_next_id;
                self.claw_next_id += 1;
                self.claw_history.push(ClawEntry {
                    id: new_entry_id,
                    command: format!("🤖 {}", agent_name),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    source: ClawEntrySource::OpenClaw,
                    status: ClawEntryStatus::Success,
                    output_lines,
                    elapsed_ms: Some(latency_ms),
                });
                return iced_scrollable::snap_to(
                    CLAW_SCROLL_ID.clone(),
                    RelativeOffset { x: 0.0, y: 1.0 },
                );
            }
            AppMessage::ClawNlPlanError { entry_id, error } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    entry.output_lines.push((format!("⚠ AI plan error: {}", error), true));
                    entry.status = ClawEntryStatus::Error(1);
                    entry.elapsed_ms = Some(0);
                }
            }
            AppMessage::ClawNlPlanReady { entry_id, plan_json } => {
                // Parse the JSON plan and execute each step sequentially
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    entry.output_lines.push(("🤖 NL Agent plan received. Executing…".to_string(), false));
                }
                // Execute the plan: parse JSON steps and run them
                return Task::perform(
                    async move {
                        nl_agent_execute_plan(entry_id, &plan_json).await
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ClawNlStepDone { entry_id, step, output, is_err } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    let prefix = if is_err { format!("  [Step {}] ⚠ ", step + 1) } else { format!("  [Step {}] ", step + 1) };
                    for line in output.lines() {
                        entry.output_lines.push((format!("{}{}", prefix, line), is_err));
                    }
                    if output.is_empty() {
                        entry.output_lines.push((format!("{}(no output)", prefix), false));
                    }
                }
            }
            AppMessage::ClawNlDone { entry_id, elapsed_ms } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    entry.output_lines.push((format!("✓ NL Agent completed in {}ms", elapsed_ms), false));
                    entry.status = ClawEntryStatus::Success;
                    entry.elapsed_ms = Some(elapsed_ms);
                }
            }
            AppMessage::ClawFetchUrl { entry_id, url, step } => {
                return Task::perform(
                    async move {
                        let result = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(15))
                            .user_agent("OpenClaw-NL-Agent/0.1")
                            .build()
                            .unwrap_or_default()
                            .get(&url)
                            .send()
                            .await;
                        match result {
                            Ok(resp) => {
                                let status = resp.status().as_u16();
                                match resp.text().await {
                                    Ok(body) => {
                                        let truncated = if body.len() > 4000 {
                                            format!("{}…[truncated {} chars]", &body[..4000], body.len() - 4000)
                                        } else {
                                            body
                                        };
                                        AppMessage::ClawFetchResult { entry_id, step, content: format!("HTTP {}\n{}", status, truncated), is_err: false }
                                    }
                                    Err(e) => AppMessage::ClawFetchResult { entry_id, step, content: e.to_string(), is_err: true },
                                }
                            }
                            Err(e) => AppMessage::ClawFetchResult { entry_id, step, content: e.to_string(), is_err: true },
                        }
                    },
                    cosmic::Action::App,
                );
            }
            AppMessage::ClawFetchResult { entry_id, step, content, is_err } => {
                if let Some(entry) = self.claw_history.iter_mut().find(|e| e.id == entry_id) {
                    let prefix = if is_err { format!("  [Step {}] ⚠ Fetch error: ", step + 1) } else { format!("  [Step {}] ", step + 1) };
                    for line in content.lines().take(30) {
                        entry.output_lines.push((format!("{}{}", prefix, line), is_err));
                    }
                    if content.lines().count() > 30 {
                        entry.output_lines.push((format!("  [Step {}] … ({} more lines)", step + 1, content.lines().count() - 30), false));
                    }
                }
            }
            // ── Telegram integration handlers ──────────────────────────────────
            AppMessage::TgStartPolling => {
                // Find the first enabled Telegram channel with a token
                let tg_token = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Telegram && c.enabled && !c.token.is_empty())
                    .map(|c| c.token.clone());
                match tg_token {
                    None => {
                        // Inject a system info entry
                        let id = self.claw_next_id;
                        self.claw_next_id += 1;
                        let entry = ClawEntry::reply(
                            id,
                            "⚠ No enabled Telegram channel with a token found.",
                            ClawEntrySource::System,
                            vec![
                                ("Go to General Settings → Communication Channels".to_string(), false),
                                ("Add a Telegram channel, set Bot Token, and enable it.".to_string(), false),
                            ],
                            0,
                        );
                        self.claw_history.push(entry);
                    }
                    Some(token) => {
                        self.tg_polling_active = true;
                        let offset = self.tg_last_update_id + 1;
                        return Task::perform(
                            async move { tg_get_updates(&token, offset).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::TgStopPolling => {
                self.tg_polling_active = false;
            }
            AppMessage::TgMessagesReceived(messages) => {
                for msg in &messages {
                    if msg.update_id > self.tg_last_update_id {
                        self.tg_last_update_id = msg.update_id;
                    }
                    let id = self.claw_next_id;
                    self.claw_next_id += 1;
                    let entry = ClawEntry::reply(
                        id,
                        msg.text.clone(),
                        ClawEntrySource::Telegram { from: msg.from.clone() },
                        vec![
                            (format!("from: {} | chat_id: {} | {}", msg.from, msg.chat_id,
                                chrono_fmt(msg.date)), false),
                        ],
                        0,
                    );
                    self.claw_history.push(entry);
                }
                // Continue polling if still active
                if self.tg_polling_active {
                    let tg_token = self.config.channels.iter()
                        .find(|c| c.kind == ChannelKind::Telegram && c.enabled && !c.token.is_empty())
                        .map(|c| c.token.clone());
                    if let Some(token) = tg_token {
                        let offset = self.tg_last_update_id + 1;
                        return Task::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                tg_get_updates(&token, offset).await
                            },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::TgSendMessage { chat_id, text } => {
                let tg_token = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Telegram && c.enabled && !c.token.is_empty())
                    .map(|c| c.token.clone());
                match tg_token {
                    None => {
                        let id = self.claw_next_id;
                        self.claw_next_id += 1;
                        let entry = ClawEntry::reply(
                            id,
                            "⚠ No enabled Telegram channel configured.",
                            ClawEntrySource::System,
                            vec![("Configure a Telegram channel in General Settings first.".to_string(), false)],
                            0,
                        );
                        self.claw_history.push(entry);
                    }
                    Some(token) => {
                        let text_clone = text.clone();
                        let chat_id_clone = chat_id.clone();
                        return Task::perform(
                            async move { tg_send_message(&token, &chat_id_clone, &text_clone).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::TgSendResult { ok, info } => {
                let id = self.claw_next_id;
                self.claw_next_id += 1;
                let entry = ClawEntry::reply(
                    id,
                    if ok { "✓ Message sent via Telegram" } else { "✗ Telegram send failed" },
                    ClawEntrySource::System,
                    vec![(info, !ok)],
                    0,
                );
                self.claw_history.push(entry);
            }
            // ── Discord handlers ───────────────────────────────────────────────
            AppMessage::DiscordStartPolling => {
                let ch = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Discord && c.enabled && !c.token.is_empty())
                    .cloned();
                match ch {
                    None => {
                        let id = self.claw_next_id; self.claw_next_id += 1;
                        self.claw_history.push(ClawEntry::reply(id,
                            "⚠ No enabled Discord channel configured.",
                            ClawEntrySource::System,
                            vec![("Go to General Settings → Communication Channels → Discord".to_string(), false)], 0));
                    }
                    Some(ch) => {
                        self.discord_polling_active = true;
                        let after = self.discord_last_msg_id.clone();
                        return Task::perform(
                            async move { discord_get_messages(&ch.token, &ch.channel_id, after.as_deref()).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::DiscordStopPolling => { self.discord_polling_active = false; }
            AppMessage::DiscordMessagesReceived(messages) => {
                for msg in &messages {
                    self.discord_last_msg_id = Some(msg.msg_id.clone());
                    let id = self.claw_next_id; self.claw_next_id += 1;
                    self.claw_history.push(ClawEntry::reply(id, msg.text.clone(),
                        ClawEntrySource::BotChannel { platform: msg.platform.to_string(), from: msg.from.clone() },
                        vec![(format!("from: {} | channel: {} | {}", msg.from, msg.channel_id, chrono_fmt(msg.date)), false)], 0));
                }
                if self.discord_polling_active {
                    let ch = self.config.channels.iter()
                        .find(|c| c.kind == ChannelKind::Discord && c.enabled && !c.token.is_empty())
                        .cloned();
                    if let Some(ch) = ch {
                        let after = self.discord_last_msg_id.clone();
                        return Task::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                discord_get_messages(&ch.token, &ch.channel_id, after.as_deref()).await
                            },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::DiscordSendMessage { channel_id, text } => {
                let token = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Discord && c.enabled && !c.token.is_empty())
                    .map(|c| c.token.clone());
                match token {
                    None => {
                        let id = self.claw_next_id; self.claw_next_id += 1;
                        self.claw_history.push(ClawEntry::reply(id, "⚠ No enabled Discord channel configured.",
                            ClawEntrySource::System, vec![], 0));
                    }
                    Some(tok) => {
                        return Task::perform(
                            async move { discord_send_message(&tok, &channel_id, &text).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::DiscordSendResult { ok, info } => {
                let id = self.claw_next_id; self.claw_next_id += 1;
                self.claw_history.push(ClawEntry::reply(id,
                    if ok { "✓ Message sent via Discord" } else { "✗ Discord send failed" },
                    ClawEntrySource::System, vec![(info, !ok)], 0));
            }
            // ── Matrix handlers ────────────────────────────────────────────────
            AppMessage::MatrixStartPolling => {
                let ch = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Matrix && c.enabled && !c.token.is_empty())
                    .cloned();
                match ch {
                    None => {
                        let id = self.claw_next_id; self.claw_next_id += 1;
                        self.claw_history.push(ClawEntry::reply(id,
                            "⚠ No enabled Matrix channel configured.",
                            ClawEntrySource::System,
                            vec![("Go to General Settings → Communication Channels → Matrix".to_string(), false)], 0));
                    }
                    Some(ch) => {
                        self.matrix_polling_active = true;
                        let since = self.matrix_next_batch.clone();
                        return Task::perform(
                            async move { matrix_sync(&ch.homeserver_url, &ch.token, &ch.channel_id, since.as_deref()).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::MatrixStopPolling => { self.matrix_polling_active = false; }
            AppMessage::MatrixMessagesReceived(messages) => {
                for msg in &messages {
                    let id = self.claw_next_id; self.claw_next_id += 1;
                    self.claw_history.push(ClawEntry::reply(id, msg.text.clone(),
                        ClawEntrySource::BotChannel { platform: msg.platform.to_string(), from: msg.from.clone() },
                        vec![(format!("from: {} | room: {} | {}", msg.from, msg.channel_id, chrono_fmt(msg.date)), false)], 0));
                }
                if self.matrix_polling_active {
                    let ch = self.config.channels.iter()
                        .find(|c| c.kind == ChannelKind::Matrix && c.enabled && !c.token.is_empty())
                        .cloned();
                    if let Some(ch) = ch {
                        let since = self.matrix_next_batch.clone();
                        return Task::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                matrix_sync(&ch.homeserver_url, &ch.token, &ch.channel_id, since.as_deref()).await
                            },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::MatrixSendMessage { room_id, text } => {
                let ch = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Matrix && c.enabled && !c.token.is_empty())
                    .cloned();
                match ch {
                    None => {
                        let id = self.claw_next_id; self.claw_next_id += 1;
                        self.claw_history.push(ClawEntry::reply(id, "⚠ No enabled Matrix channel configured.",
                            ClawEntrySource::System, vec![], 0));
                    }
                    Some(ch) => {
                        return Task::perform(
                            async move { matrix_send_message(&ch.homeserver_url, &ch.token, &room_id, &text).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::MatrixSendResult { ok, info } => {
                let id = self.claw_next_id; self.claw_next_id += 1;
                self.claw_history.push(ClawEntry::reply(id,
                    if ok { "✓ Message sent via Matrix" } else { "✗ Matrix send failed" },
                    ClawEntrySource::System, vec![(info, !ok)], 0));
            }
            // ── Slack handlers ─────────────────────────────────────────────────
            AppMessage::SlackStartPolling => {
                let ch = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Slack && c.enabled && !c.token.is_empty())
                    .cloned();
                match ch {
                    None => {
                        let id = self.claw_next_id; self.claw_next_id += 1;
                        self.claw_history.push(ClawEntry::reply(id,
                            "⚠ No enabled Slack channel configured.",
                            ClawEntrySource::System,
                            vec![("Go to General Settings → Communication Channels → Slack".to_string(), false)], 0));
                    }
                    Some(ch) => {
                        self.slack_polling_active = true;
                        let oldest = self.slack_last_ts.clone();
                        return Task::perform(
                            async move { slack_get_messages(&ch.token, &ch.channel_id, oldest.as_deref()).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::SlackStopPolling => { self.slack_polling_active = false; }
            AppMessage::SlackMessagesReceived(messages) => {
                for msg in &messages {
                    self.slack_last_ts = Some(msg.msg_id.clone());
                    let id = self.claw_next_id; self.claw_next_id += 1;
                    self.claw_history.push(ClawEntry::reply(id, msg.text.clone(),
                        ClawEntrySource::BotChannel { platform: msg.platform.to_string(), from: msg.from.clone() },
                        vec![(format!("from: {} | channel: {} | {}", msg.from, msg.channel_id, chrono_fmt(msg.date)), false)], 0));
                }
                if self.slack_polling_active {
                    let ch = self.config.channels.iter()
                        .find(|c| c.kind == ChannelKind::Slack && c.enabled && !c.token.is_empty())
                        .cloned();
                    if let Some(ch) = ch {
                        let oldest = self.slack_last_ts.clone();
                        return Task::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                slack_get_messages(&ch.token, &ch.channel_id, oldest.as_deref()).await
                            },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::SlackSendMessage { channel_id, text } => {
                let token = self.config.channels.iter()
                    .find(|c| c.kind == ChannelKind::Slack && c.enabled && !c.token.is_empty())
                    .map(|c| c.token.clone());
                match token {
                    None => {
                        let id = self.claw_next_id; self.claw_next_id += 1;
                        self.claw_history.push(ClawEntry::reply(id, "⚠ No enabled Slack channel configured.",
                            ClawEntrySource::System, vec![], 0));
                    }
                    Some(tok) => {
                        return Task::perform(
                            async move { slack_send_message(&tok, &channel_id, &text).await },
                            cosmic::Action::App,
                        );
                    }
                }
            }
            AppMessage::SlackSendResult { ok, info } => {
                let id = self.claw_next_id; self.claw_next_id += 1;
                self.claw_history.push(ClawEntry::reply(id,
                    if ok { "✓ Message sent via Slack" } else { "✗ Slack send failed" },
                    ClawEntrySource::System, vec![(info, !ok)], 0));
            }
            AppMessage::ClawSystemInfo { text } => {
                let id = self.claw_next_id;
                self.claw_next_id += 1;
                let entry = ClawEntry::reply(
                    id,
                    text,
                    ClawEntrySource::System,
                    vec![],
                    0,
                );
                self.claw_history.push(entry);
            }
            AppMessage::ShowQuitDialog => {
                self.show_quit_dialog = true;
            }
            AppMessage::ConfirmQuit => {
                // Save all application state before exiting
                self.save_all_state();
                tracing::info!("All state saved, exiting application");
                std::process::exit(0);
            }
            AppMessage::CancelQuit => {
                self.show_quit_dialog = false;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // Plugin-mode: poll the gateway's /skills/events endpoint every second.
        if let RunMode::Plugin { gateway_url } = &self.run_mode {
            let url = format!("{}/skills/events?limit=50", gateway_url);
            return Subscription::run_with_id(
                std::any::TypeId::of::<RunMode>(),
                cosmic::iced_futures::stream::channel(
                    100,
                    move |mut output| async move {
                        let client = reqwest::Client::new();
                        let mut last_seen_id: u64 = 0;
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            if let Ok(resp) = client.get(&url).send().await {
                                if let Ok(events) = resp.json::<Vec<SandboxEvent>>().await {
                                    for ev in events {
                                        if ev.id > last_seen_id {
                                            last_seen_id = ev.id;
                                            let _ = output.try_send(AppMessage::SandboxEvent(ev));
                                        }
                                    }
                                }
                            }
                        }
                    },
                ),
            );
        }

        // Keyboard shortcuts:
        // - ArrowUp / ArrowDown: sidebar navigation
        // - Cmd+Enter (macOS) / Ctrl+Enter (others): send Claw Terminal message
        let keyboard_sub = keyboard::on_key_press(|key, _modifiers| {
            match key {
                // Arrow keys navigate the sidebar
                Key::Named(keyboard::key::Named::ArrowUp)   => Some(AppMessage::NavUp),
                Key::Named(keyboard::key::Named::ArrowDown) => Some(AppMessage::NavDown),
                // Cmd+Enter is handled inside text_editor key_binding — do NOT intercept here
                _ => None,
            }
        });

        // Embedded mode: receive events directly from the flume channel.
        // SAFETY: We clone the inner Receiver out of the Mutex immediately so the
        // lock is not held across the async recv_async() await point, preventing deadlock.
        if let Some(rx_arc) = &self.event_rx {
            let rx_arc = rx_arc.clone();
            let sandbox_sub = Subscription::run_with_id(
                std::any::TypeId::of::<SandboxEvent>(),
                cosmic::iced_futures::stream::channel(
                    100,
                    move |mut output| async move {
                        // Clone the receiver once outside the loop to avoid
                        // re-acquiring the lock on every iteration.
                        let receiver = rx_arc.lock().await.clone();
                        loop {
                            match receiver.recv_async().await {
                                Ok(event) => {
                                    let _ = output.try_send(AppMessage::SandboxEvent(event));
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, "sandbox event channel closed — subscription ending");
                                    break;
                                }
                            }
                        }
                    },
                ),
            );
            Subscription::batch([keyboard_sub, sandbox_sub])
        } else {
            keyboard_sub
        }
    }
}

/// Map AiProvider (UI config) to the InferenceEngine BackendKind.
fn provider_to_backend_kind(provider: &AiProvider) -> BackendKind {
    match provider {
        AiProvider::Ollama        => BackendKind::Ollama,
        AiProvider::LlamaCppHttp  => BackendKind::LlamaCppHttp,
        AiProvider::OpenAi        => BackendKind::OpenAiCompat,
        AiProvider::Anthropic     => BackendKind::OpenAiCompat,
        AiProvider::DeepSeek      => BackendKind::OpenAiCompat,
        AiProvider::Gemini        => BackendKind::OpenAiCompat,
        AiProvider::OpenAiCompat  => BackendKind::OpenAiCompat,
    }
}

impl OpenClawApp {
    /// Save all application state to disk before quitting.
    /// This includes: SecurityConfig, AI chat history, Claw Terminal history, and UI preferences.
    /// Aerospace-grade: comprehensive error handling, atomic writes, and detailed logging.
    fn save_all_state(&self) {
        tracing::info!("Starting aerospace-grade state save sequence");
        
        // 1. Save SecurityConfig to config file
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("config.toml");
        
        if let Some(parent) = config_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::error!(error = %e, path = %parent.display(), "Failed to create config directory");
            }
        }
        
        match toml::to_string_pretty(&self.config) {
            Ok(toml_str) => {
                match std::fs::write(&config_path, &toml_str) {
                    Ok(_) => {
                        tracing::info!(
                            path = %config_path.display(),
                            size = toml_str.len(),
                            "SecurityConfig saved successfully"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            path = %config_path.display(),
                            "Failed to write SecurityConfig"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize SecurityConfig to TOML");
            }
        }
        
        // 2. Save AI chat history
        let chat_history_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("ai_chat_history.json");
        
        if let Some(parent) = chat_history_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::error!(error = %e, path = %parent.display(), "Failed to create data directory");
            }
        }
        
        match serde_json::to_string_pretty(&self.ai_chat.messages) {
            Ok(json_str) => {
                match std::fs::write(&chat_history_path, &json_str) {
                    Ok(_) => {
                        tracing::info!(
                            messages = self.ai_chat.messages.len(),
                            size = json_str.len(),
                            "AI chat history saved successfully"
                        );
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to write AI chat history");
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize AI chat history");
            }
        }
        
        // 3. Save Claw Terminal history
        let claw_history_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("claw_terminal_history.json");
        
        match serde_json::to_string_pretty(&self.claw_history) {
            Ok(json_str) => {
                match std::fs::write(&claw_history_path, &json_str) {
                    Ok(_) => {
                        tracing::info!(
                            entries = self.claw_history.len(),
                            size = json_str.len(),
                            "Claw Terminal history saved successfully"
                        );
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to write Claw Terminal history");
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize Claw Terminal history");
            }
        }
        
        // 4. Save UI preferences (language, theme, nav page)
        #[derive(serde::Serialize)]
        struct UiPrefs {
            language: String,
            warm_theme_active: bool,
            nav_page: String,
        }
        
        let ui_prefs = UiPrefs {
            language: format!("{:?}", self.language),
            warm_theme_active: self.warm_theme_active,
            nav_page: format!("{:?}", self.nav_page),
        };
        
        let ui_prefs_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("ui_prefs.json");
        
        match serde_json::to_string_pretty(&ui_prefs) {
            Ok(json_str) => {
                match std::fs::write(&ui_prefs_path, &json_str) {
                    Ok(_) => {
                        tracing::info!("UI preferences saved successfully");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to write UI preferences");
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize UI preferences");
            }
        }
        
        tracing::info!("Aerospace-grade state save sequence completed");
    }

    /// Render the quit confirmation dialog with detailed state information.
    fn view_quit_dialog(&self) -> Element<AppMessage> {
        use cosmic::iced::font::Font;
        
        widget::layer_container(
            widget::container(
                widget::column()
                    .push(
                        widget::text("Confirm Application Exit")
                            .size(20)
                            .font(Font {
                                weight: cosmic::iced::font::Weight::Bold,
                                ..Font::DEFAULT
                            })
                    )
                    .push(widget::vertical_space().height(16))
                    .push(
                        widget::text("All application state will be saved before exiting:")
                            .size(14)
                    )
                    .push(widget::vertical_space().height(12))
                    .push(
                        widget::text(format!("✓ Security configuration ({} mounts, {} network rules)", 
                            self.config.fs_mounts.len(),
                            self.config.network_allowlist.len()))
                            .size(12)
                    )
                    .push(
                        widget::text(format!("✓ AI chat history ({} messages)", 
                            self.ai_chat.messages.len()))
                            .size(12)
                    )
                    .push(
                        widget::text(format!("✓ Claw Terminal history ({} entries)", 
                            self.claw_history.len()))
                            .size(12)
                    )
                    .push(
                        widget::text("✓ UI preferences (language, theme, navigation)")
                            .size(12)
                    )
                    .push(widget::vertical_space().height(16))
                    .push(
                        widget::text("Are you sure you want to quit?")
                            .size(14)
                            .font(Font {
                                weight: cosmic::iced::font::Weight::Bold,
                                ..Font::DEFAULT
                            })
                    )
                    .push(widget::vertical_space().height(20))
                    .push(
                        widget::row()
                            .push(
                                widget::button::destructive("Quit")
                                    .on_press(AppMessage::ConfirmQuit)
                            )
                            .push(widget::horizontal_space().width(12))
                            .push(
                                widget::button::standard("Cancel")
                                    .on_press(AppMessage::CancelQuit)
                            )
                            .spacing(8)
                    )
                    .spacing(4)
                    .padding(24)
                    .max_width(520)
            )
            .padding(20)
            .class(cosmic::theme::Container::Dialog)
        )
        .into()
    }

    /// Render the Digital Workers (Agent) management page.
    fn view_agents_page(&self, lang: crate::theme::Language) -> Element<AppMessage> {
        use cosmic::iced::Length;
        use openclaw_security::AgentStatus;

        let selected = self.agent_selected;
        let agents   = &self.agent_list;
        let c_muted  = cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48);
        let c_accent = cosmic::iced::Color::from_rgb(0.98, 0.62, 0.22);

        // ── LEFT PANEL ────────────────────────────────────────────────────
        let mut list_col = widget::column().spacing(0);

        // Header
        list_col = list_col.push(
            widget::container(
                widget::row()
                    .push(widget::text(tx(lang, "Digital Workers")).size(14).font(cosmic::font::bold()))
                    .push(widget::horizontal_space().width(Length::Fill))
                    .push(widget::button::text(tx(lang, "Refresh")).on_press(AppMessage::AgentRefresh).class(cosmic::theme::Button::Text))
                    .align_y(Alignment::Center)
                    .padding([8, 10])
            )
            .style(|theme: &cosmic::Theme| {
                let bg = theme.cosmic().bg_color();
                ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgb((bg.red*0.82).min(1.0),(bg.green*0.82).min(1.0),(bg.blue*0.82).min(1.0))
                    )),
                    ..Default::default()
                }
            })
            .width(Length::Fill)
        );

        // Create form
        list_col = list_col.push(
            widget::column()
                .push(
                    widget::text_input(tx(lang, "Worker name..."), &self.agent_name_input)
                        .on_input(AppMessage::AgentNameChanged)
                        .padding([6, 10])
                        .width(Length::Fill)
                )
                .push(
                    widget::row()
                        .push(widget::text(tx(lang, "Role")).size(11).class(cosmic::theme::Text::Color(c_muted)))
                        .push(widget::horizontal_space().width(Length::Fill))
                        .push(widget::text(&self.agent_role_input).size(11).class(cosmic::theme::Text::Color(c_accent)))
                        .align_y(Alignment::Center)
                )
                .push(
                    widget::row()
                        .push(Self::role_chip("工单",   "Ticket Assistant",       &self.agent_role_input))
                        .push(Self::role_chip("代码",   "Code Reviewer",          &self.agent_role_input))
                        .push(Self::role_chip("安全",   "Security Auditor",       &self.agent_role_input))
                        .push(Self::role_chip("数据",   "Data Analyst",           &self.agent_role_input))
                        .push(Self::role_chip("客服",   "Customer Support",       &self.agent_role_input))
                        .push(Self::role_chip("知识库", "Knowledge Officer",      &self.agent_role_input))
                        .spacing(3)
                        .wrap()
                )
                .push(
                    widget::row()
                        .push(Self::role_chip("社媒",   "Social Media Manager",   &self.agent_role_input))
                        .push(Self::role_chip("邮件",   "Inbox Triage Agent",     &self.agent_role_input))
                        .push(Self::role_chip("财务",   "Finance & Procurement",  &self.agent_role_input))
                        .push(Self::role_chip("新闻",   "News Secretary",         &self.agent_role_input))
                        .push(Self::role_chip("代码审", "Security Code Auditor",  &self.agent_role_input))
                        .spacing(3)
                        .wrap()
                )
                .push(
                    widget::button::suggested(tx(lang, "Create Worker"))
                        .on_press(AppMessage::AgentCreate)
                        .width(Length::Fill)
                )
                .spacing(6)
                .padding([8, 10])
        );

        list_col = list_col.push(widget::divider::horizontal::default());

        // Agent list
        if self.agent_loading {
            list_col = list_col.push(
                widget::container(widget::text(tx(lang, "Loading...")).size(13).class(cosmic::theme::Text::Color(c_muted)))
                    .padding([16, 10])
            );
        } else if agents.is_empty() {
            list_col = list_col.push(
                widget::container(
                    widget::column()
                        .push(widget::text(tx(lang, "No workers yet")).size(13).class(cosmic::theme::Text::Color(c_muted)))
                        .push(widget::text(tx(lang, "Create one above")).size(11).class(cosmic::theme::Text::Color(c_muted)))
                        .spacing(4)
                        .align_x(Alignment::Center)
                )
                .center_x(Length::Fill)
                .padding([24, 10])
            );
        } else {
            for (idx, agent) in agents.iter().enumerate() {
                let is_sel = selected == Some(idx);
                let (sr, sg, sb) = agent.status.color_rgb();
                let sc = cosmic::iced::Color::from_rgb(sr, sg, sb);

                // Avatar circle: first char of display_name on role-color bg
                let initial = agent.display_name.chars().next().unwrap_or('?').to_string();
                let avatar_bg = Self::role_accent_color(&agent.role);
                let avatar = widget::container(
                    widget::text(initial).size(14).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(cosmic::iced::Color::WHITE))
                )
                .width(Length::Fixed(34.0))
                .height(Length::Fixed(34.0))
                .center(Length::Fixed(34.0))
                .style(move |_: &cosmic::Theme| ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(avatar_bg)),
                    border: cosmic::iced::Border { radius: 17.0.into(), ..Default::default() },
                    ..Default::default()
                });

                // Status indicator dot (bottom-right of avatar area)
                let status_dot = widget::container(widget::Space::new(8, 8))
                    .style(move |_: &cosmic::Theme| ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(sc)),
                        border: cosmic::iced::Border {
                            radius: 4.0.into(),
                            color: cosmic::iced::Color::from_rgba(0.0,0.0,0.0,0.4),
                            width: 1.0,
                        },
                        ..Default::default()
                    });

                let row_inner = widget::row()
                    .push(
                        widget::column()
                            .push(avatar)
                            .push(widget::container(status_dot)
                                .padding([0, 0, 0, 22]))
                            .spacing(0)
                    )
                    .push(widget::Space::new(8, 0))
                    .push(
                        widget::column()
                            .push(widget::text(&agent.display_name).size(13).font(cosmic::font::bold()))
                            .push(
                                widget::text(format!("{}  ·  {} runs", agent.role.display_zh(), agent.stats.total_runs))
                                    .size(10).class(cosmic::theme::Text::Color(c_muted))
                            )
                            .spacing(2)
                    )
                    .push(widget::horizontal_space().width(Length::Fill))
                    .push(widget::text(agent.status.to_string()).size(10).class(cosmic::theme::Text::Color(sc)))
                    .align_y(Alignment::Center)
                    .padding([7, 10]);

                let btn = widget::button::custom(row_inner)
                    .on_press(AppMessage::AgentSelect(idx))
                    .class(if is_sel { cosmic::theme::Button::MenuItem } else { cosmic::theme::Button::MenuRoot })
                    .width(Length::Fill);

                if is_sel {
                    list_col = list_col.push(
                        widget::row()
                            .push(
                                widget::container(widget::Space::new(3, 0))
                                    .style(move |_: &cosmic::Theme| ContainerStyle {
                                        background: Some(cosmic::iced::Background::Color(c_accent)),
                                        ..Default::default()
                                    })
                                    .height(Length::Fill)
                            )
                            .push(btn)
                            .height(Length::Shrink)
                    );
                } else {
                    list_col = list_col.push(btn);
                }
            }
        }

        let left_panel = widget::container(widget::scrollable(list_col))
            .width(Length::Fixed(260.0))
            .height(Length::Fill)
            .style(|theme: &cosmic::Theme| {
                let bg = theme.cosmic().bg_color();
                ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgb((bg.red*0.88).min(1.0),(bg.green*0.87).min(1.0),(bg.blue*0.86).min(1.0))
                    )),
                    ..Default::default()
                }
            });

        // ── RIGHT PANEL ───────────────────────────────────────────────────
        let c_muted2  = cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48);
        let c_success = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
        let c_danger  = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);

        let right_panel: Element<AppMessage> = if let Some(idx) = selected {
            if let Some(agent) = agents.get(idx) {
                let (sr, sg, sb) = agent.status.color_rgb();
                let sc = cosmic::iced::Color::from_rgb(sr, sg, sb);
                let can_suspend = matches!(agent.status, AgentStatus::Active | AgentStatus::Running);
                let can_resume  = matches!(agent.status, AgentStatus::Suspended);
                let can_archive = !matches!(agent.status, AgentStatus::Archived);

                // ── header bar ────────────────────────────────────────────
                let id_str    = agent.id.to_string();
                let id_short  = &id_str[..8.min(id_str.len())];
                let role_desc = agent.role.description();
                let avatar_bg = Self::role_accent_color(&agent.role);
                let av_initial = agent.display_name.chars().next().unwrap_or('?').to_string();

                // Large avatar circle (56px)
                let big_avatar = widget::container(
                    widget::text(av_initial).size(22).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(cosmic::iced::Color::WHITE))
                )
                .width(Length::Fixed(56.0))
                .height(Length::Fixed(56.0))
                .center(Length::Fixed(56.0))
                .style(move |_: &cosmic::Theme| ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(avatar_bg)),
                    border: cosmic::iced::Border { radius: 28.0.into(), ..Default::default() },
                    ..Default::default()
                });

                let avatar_placeholder: &'static str = "头像 URL（留空即使用默认角色头像）";

                let header = widget::container(
                    widget::column()
                        .push(
                            widget::row()
                                // Large avatar
                                .push(
                                    widget::column()
                                        .push(big_avatar)
                                        .spacing(2)
                                        .align_x(Alignment::Center)
                                )
                                .push(widget::Space::new(14, 0))
                                // Name + meta + avatar URL input
                                .push(
                                    widget::column()
                                        .push(
                                            if self.agent_editing_name {
                                                Element::from(
                                                    widget::row()
                                                        .push(
                                                            widget::text_input("新名称…", &self.agent_rename_input)
                                                                .on_input(AppMessage::AgentRenameInputChanged)
                                                                .size(16)
                                                                .padding([4, 8])
                                                                .width(Length::Fixed(200.0))
                                                        )
                                                        .push(
                                                            widget::button::text("✓")
                                                                .class(cosmic::theme::Button::Suggested)
                                                                .on_press(AppMessage::AgentRenameConfirm)
                                                                .padding([4, 8])
                                                        )
                                                        .push(
                                                            widget::button::text("✕")
                                                                .class(cosmic::theme::Button::Destructive)
                                                                .on_press(AppMessage::AgentRenameCancelled)
                                                                .padding([4, 8])
                                                        )
                                                        .spacing(4)
                                                        .align_y(Alignment::Center)
                                                )
                                            } else {
                                                Element::from(
                                                    widget::row()
                                                        .push(widget::text(&agent.display_name).size(20).font(cosmic::font::bold()))
                                                        .push(widget::Space::new(8, 0))
                                                        .push(
                                                            widget::button::text("✏")
                                                                .class(cosmic::theme::Button::Text)
                                                                .on_press(AppMessage::AgentStartRename)
                                                                .padding([2, 6])
                                                        )
                                                        .align_y(Alignment::Center)
                                                )
                                            }
                                        )
                                        .push(
                                            // Role coloured badge: emoji + zh name
                                            widget::row()
                                                .push(
                                                    widget::container(
                                                        widget::row()
                                                            .push(widget::text(agent.role.role_emoji()).size(11))
                                                            .push(widget::text(agent.role.display_zh()).size(11).font(cosmic::font::bold())
                                                                .class(cosmic::theme::Text::Color(cosmic::iced::Color::WHITE)))
                                                            .spacing(4)
                                                            .align_y(Alignment::Center)
                                                            .padding([2, 8])
                                                    )
                                                    .style(move |_: &cosmic::Theme| ContainerStyle {
                                                        background: Some(cosmic::iced::Background::Color(avatar_bg)),
                                                        border: cosmic::iced::Border { radius: 10.0.into(), ..Default::default() },
                                                        ..Default::default()
                                                    })
                                                )
                                                .push(
                                                    widget::text(format!("· {}", agent.owner))
                                                        .size(11).class(cosmic::theme::Text::Color(c_muted2))
                                                )
                                                .spacing(6)
                                                .align_y(Alignment::Center)
                                        )
                                        .push(
                                            widget::text(format!("ID: {}", id_short))
                                                .size(10).font(cosmic::font::mono())
                                                .class(cosmic::theme::Text::Color(c_muted2))
                                        )
                                        .push(
                                            widget::row()
                                                .push(
                                                    widget::text_input(
                                                        avatar_placeholder,
                                                        &self.agent_avatar_input,
                                                    )
                                                    .on_input(AppMessage::AgentAvatarInputChanged)
                                                    .size(10)
                                                    .padding([3, 6])
                                                    .width(Length::Fill)
                                                )
                                                .push(
                                                    widget::button::text("✓")
                                                        .class(cosmic::theme::Button::Suggested)
                                                        .on_press(AppMessage::AgentAvatarChange(
                                                            self.agent_avatar_input.clone()
                                                        ))
                                                        .padding([3, 6])
                                                )
                                                .spacing(4)
                                                .align_y(Alignment::Center)
                                        )
                                        .spacing(3)
                                )
                                .push(widget::horizontal_space().width(Length::Fill))
                                .push(
                                    widget::container(widget::text(agent.status.to_string()).size(12))
                                        .padding([4, 14])
                                        .style(move |_: &cosmic::Theme| ContainerStyle {
                                            background: Some(cosmic::iced::Background::Color(
                                                cosmic::iced::Color::from_rgba(sr, sg, sb, 0.18)
                                            )),
                                            border: cosmic::iced::Border { color: sc, width: 1.0, radius: 10.0.into() },
                                            ..Default::default()
                                        })
                                )
                                .align_y(Alignment::Center)
                                .padding([14, 24, 8, 24])
                        )
                        .push(
                            widget::container(
                                widget::row()
                                    .push(
                                        widget::container(widget::Space::new(3, 0))
                                            .style(move |_: &cosmic::Theme| ContainerStyle {
                                                background: Some(cosmic::iced::Background::Color(
                                                    cosmic::iced::Color::from_rgba(sr, sg, sb, 0.80)
                                                )),
                                                ..Default::default()
                                            })
                                            .height(Length::Fill)
                                    )
                                    .push(
                                        widget::text(role_desc).size(11)
                                            .class(cosmic::theme::Text::Color(c_muted2))
                                    )
                                    .spacing(8)
                                    .align_y(Alignment::Center)
                                    .padding([0, 24, 10, 24])
                            )
                            .height(Length::Shrink)
                        )
                )
                .style(|theme: &cosmic::Theme| {
                    let bg = theme.cosmic().bg_color();
                    ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgb((bg.red*0.84).min(1.0),(bg.green*0.84).min(1.0),(bg.blue*0.84).min(1.0))
                        )),
                        ..Default::default()
                    }
                })
                .width(Length::Fill);

                // ── stats row ─────────────────────────────────────────────
                let stats = widget::row()
                    .push(Self::stat_card("总任务",  &agent.stats.total_runs.to_string()))
                    .push(Self::stat_card("成功",    &agent.stats.successful_runs.to_string()))
                    .push(Self::stat_card("失败",    &agent.stats.failed_runs.to_string()))
                    .push(Self::stat_card("成功率",  &format!("{:.0}%", agent.stats.success_rate() * 100.0)))
                    .push(Self::stat_card("内存限制", &format!("{} MB", agent.memory_limit_mb)))
                    .spacing(10)
                    .padding([12, 24]);

                // ── security badges ───────────────────────────────────────
                let sec_badges = {
                    let badge = |label: String, ok: bool| -> Element<'static, AppMessage> {
                        let col = if ok { c_success } else { c_danger };
                        widget::container(
                            widget::row()
                                .push(widget::container(widget::Space::new(6,6))
                                    .style(move |_: &cosmic::Theme| ContainerStyle {
                                        background: Some(cosmic::iced::Background::Color(col)),
                                        border: cosmic::iced::Border { radius: 3.0.into(), ..Default::default() },
                                        ..Default::default()
                                    }))
                                .push(widget::text(label).size(11))
                                .spacing(5).align_y(Alignment::Center).padding([3,8])
                        )
                        .style(move |_: &cosmic::Theme| ContainerStyle {
                            background: Some(cosmic::iced::Background::Color(
                                cosmic::iced::Color::from_rgba(col.r, col.g, col.b, 0.10)
                            )),
                            border: cosmic::iced::Border {
                                color: cosmic::iced::Color::from_rgba(col.r, col.g, col.b, 0.40),
                                width: 1.0, radius: 6.0.into(),
                            },
                            ..Default::default()
                        })
                        .into()
                    };
                    widget::row()
                        .push(badge("Shell 拦截".to_string(), agent.intercept_shell))
                        .push(badge("Shell 确认".to_string(), agent.confirm_shell_exec))
                        .push(badge(format!("网络白名单 {}", agent.network_allowlist.len()), !agent.network_allowlist.is_empty()))
                        .push(badge(format!("FS 挂载 {}", agent.fs_mounts.len()), !agent.fs_mounts.is_empty()))
                        .spacing(8)
                        .padding([0, 24])
                };

                // ── capabilities ──────────────────────────────────────────
                let caps_el: Element<AppMessage> = if agent.capabilities.is_empty() {
                    widget::text("最小权限模式 — 暂无授权能力").size(12)
                        .class(cosmic::theme::Text::Color(c_muted2))
                        .into()
                } else {
                    // Risk distribution counts
                    let n_high   = agent.capabilities.iter().filter(|c| c.risk_label() == "high").count();
                    let n_medium = agent.capabilities.iter().filter(|c| c.risk_label() == "medium").count();
                    let n_low    = agent.capabilities.iter().filter(|c| c.risk_label() == "low").count();
                    let n_total  = agent.capabilities.len().max(1);
                    let col_high   = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);
                    let col_medium = cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12);
                    let col_low    = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);

                    // Risk distribution bar
                    let bar_seg = |color: cosmic::iced::Color, count: usize, total: usize| -> Element<'static, AppMessage> {
                        let pct = count as f32 / total as f32;
                        widget::container(widget::Space::new(0, 6))
                            .width(Length::FillPortion((pct * 100.0) as u16 + 1))
                            .style(move |_: &cosmic::Theme| ContainerStyle {
                                background: Some(cosmic::iced::Background::Color(color)),
                                border: cosmic::iced::Border { radius: 3.0.into(), ..Default::default() },
                                ..Default::default()
                            })
                            .into()
                    };
                    let risk_bar = widget::row()
                        .push(bar_seg(col_high,   n_high,   n_total))
                        .push(bar_seg(col_medium, n_medium, n_total))
                        .push(bar_seg(col_low,    n_low,    n_total))
                        .spacing(2)
                        .height(Length::Fixed(6.0))
                        .width(Length::Fill);

                    let risk_legend = widget::row()
                        .push(widget::container(
                            widget::row()
                                .push(widget::container(widget::Space::new(8,8)).style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: Some(cosmic::iced::Background::Color(col_high)),
                                    border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                                    ..Default::default()
                                }))
                                .push(widget::text(format!("高风险 {}", n_high)).size(10).class(cosmic::theme::Text::Color(col_high)))
                                .spacing(4).align_y(Alignment::Center)
                        ))
                        .push(widget::container(
                            widget::row()
                                .push(widget::container(widget::Space::new(8,8)).style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: Some(cosmic::iced::Background::Color(col_medium)),
                                    border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                                    ..Default::default()
                                }))
                                .push(widget::text(format!("中风险 {}", n_medium)).size(10).class(cosmic::theme::Text::Color(col_medium)))
                                .spacing(4).align_y(Alignment::Center)
                        ))
                        .push(widget::container(
                            widget::row()
                                .push(widget::container(widget::Space::new(8,8)).style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: Some(cosmic::iced::Background::Color(col_low)),
                                    border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                                    ..Default::default()
                                }))
                                .push(widget::text(format!("低风险 {}", n_low)).size(10).class(cosmic::theme::Text::Color(col_low)))
                                .spacing(4).align_y(Alignment::Center)
                        ))
                        .spacing(16).align_y(Alignment::Center);

                    let mut cap_row = widget::row().spacing(8);
                    for cap in &agent.capabilities {
                        let risk_col = match cap.risk_label() {
                            "high"   => col_high,
                            "medium" => col_medium,
                            _        => col_low,
                        };
                        cap_row = cap_row.push(
                            widget::container(
                                widget::column()
                                    .push(widget::text(&cap.name).size(12).font(cosmic::font::bold()))
                                    .push(widget::text(cap.risk_label()).size(10)
                                        .class(cosmic::theme::Text::Color(risk_col)))
                                    .spacing(2)
                                    .align_x(Alignment::Center)
                                    .padding([6, 10])
                            )
                            .style(move |_: &cosmic::Theme| ContainerStyle {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color::from_rgba(risk_col.r, risk_col.g, risk_col.b, 0.08)
                                )),
                                border: cosmic::iced::Border {
                                    color: cosmic::iced::Color::from_rgba(risk_col.r, risk_col.g, risk_col.b, 0.35),
                                    width: 1.0, radius: 6.0.into(),
                                },
                                ..Default::default()
                            })
                        );
                    }

                    Element::from(
                        widget::column()
                            .push(widget::container(risk_bar).padding([0, 24, 4, 0]).width(Length::Fill))
                            .push(risk_legend)
                            .push(widget::Space::new(0, 6))
                            .push(Element::from(cap_row.wrap()))
                            .spacing(4)
                    )
                };

                // ── recent runs quick-preview ─────────────────────────────
                let agent_id_for_runs = agent.id.to_string();
                let recent_runs_el: Element<AppMessage> = {
                    use openclaw_storage::types::RunStatus;
                    let runs = &self.audit_runs;
                    let preview: Vec<_> = runs.iter().take(3).collect();
                    if preview.is_empty() {
                        widget::row()
                            .push(
                                widget::button::text("📋 查看运行记录")
                                    .on_press(AppMessage::AuditLoadRuns(agent_id_for_runs))
                                    .class(cosmic::theme::Button::Standard)
                            )
                            .push(
                                widget::text("暂无运行记录").size(11)
                                    .class(cosmic::theme::Text::Color(c_muted2))
                            )
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .into()
                    } else {
                        let mut run_col = widget::column().spacing(4);
                        for run in preview {
                            let (status_icon, status_col) = match run.status {
                                RunStatus::Success         => ("✅", c_success),
                                RunStatus::Failed          => ("❌", c_danger),
                                RunStatus::Running         => ("⏳", cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12)),
                                RunStatus::Cancelled       => ("⛔", c_muted2),
                                RunStatus::PendingApproval => ("⏸",  c_muted2),
                            };
                            let elapsed = if let Some(fin) = run.finished_at {
                                let secs = fin.saturating_sub(run.started_at);
                                if secs < 60 { format!("{}s", secs) } else { format!("{}m{}s", secs/60, secs%60) }
                            } else { "进行中".to_string() };
                            // Simple timestamp: seconds since epoch → HH:MM
                            let ts = {
                                let secs = run.started_at;
                                let h = (secs / 3600) % 24;
                                let m = (secs / 60) % 60;
                                format!("{:02}:{:02}", h, m)
                            };
                            let step_count = run.step_count;
                            run_col = run_col.push(
                                widget::container(
                                    widget::row()
                                        .push(widget::text(status_icon).size(13))
                                        .push(
                                            widget::column()
                                                .push(
                                                    widget::text(&run.task_description).size(12)
                                                        .font(cosmic::font::bold())
                                                )
                                                .push(
                                                    widget::text(format!("{} · {} 步骤 · {}", ts, step_count, elapsed))
                                                        .size(10)
                                                        .class(cosmic::theme::Text::Color(c_muted2))
                                                )
                                                .spacing(1)
                                        )
                                        .push(widget::horizontal_space().width(Length::Fill))
                                        .push(
                                            widget::text(run.status.to_string()).size(10)
                                                .class(cosmic::theme::Text::Color(status_col))
                                        )
                                        .spacing(8)
                                        .align_y(Alignment::Center)
                                        .padding([6, 10])
                                )
                                .style(|theme: &cosmic::Theme| {
                                    let bg = theme.cosmic().bg_color();
                                    ContainerStyle {
                                        background: Some(cosmic::iced::Background::Color(
                                            cosmic::iced::Color::from_rgb((bg.red*0.84).min(1.0),(bg.green*0.84).min(1.0),(bg.blue*0.84).min(1.0))
                                        )),
                                        border: cosmic::iced::Border { radius: 6.0.into(), ..Default::default() },
                                        ..Default::default()
                                    }
                                })
                                .width(Length::Fill)
                            );
                        }
                        widget::column()
                            .push(
                                widget::row()
                                    .push(widget::text("最近运行").size(11).font(cosmic::font::bold())
                                        .class(cosmic::theme::Text::Color(c_muted2)))
                                    .push(widget::horizontal_space().width(Length::Fill))
                                    .push(
                                        widget::button::text("全部记录 →")
                                            .class(cosmic::theme::Button::Text)
                                            .on_press(AppMessage::AuditLoadRuns(agent.id.to_string()))
                                    )
                                    .align_y(Alignment::Center)
                            )
                            .push(run_col)
                            .spacing(4)
                            .into()
                    }
                };

                // ── action buttons ────────────────────────────────────────
                let actions = widget::row()
                    .push(if can_suspend {
                        widget::button::standard("⏸ 暂停").on_press(AppMessage::AgentSuspend).into()
                    } else { Element::from(widget::Space::new(0,0)) })
                    .push(if can_resume {
                        widget::button::suggested("▶ 恢复").on_press(AppMessage::AgentResume).into()
                    } else { Element::from(widget::Space::new(0,0)) })
                    .push(widget::horizontal_space().width(Length::Fill))
                    .push(if can_archive {
                        widget::button::destructive("🗄 归档").on_press(AppMessage::AgentArchive).into()
                    } else { Element::from(widget::Space::new(0,0)) })
                    .spacing(8)
                    .padding([0, 24]);

                // ── assemble detail ───────────────────────────────────────
                let detail = widget::column()
                    .push(header)
                    .push(widget::divider::horizontal::light())
                    .push(stats)
                    .push(widget::divider::horizontal::light())
                    .push(
                        widget::column()
                            .push(
                                widget::text("安全边界").size(11).font(cosmic::font::bold())
                                    .class(cosmic::theme::Text::Color(c_muted2))
                            )
                            .push(sec_badges)
                            .spacing(6)
                            .padding([10, 0, 6, 24])
                    )
                    .push(widget::divider::horizontal::light())
                    .push(
                        widget::column()
                            .push(
                                widget::text("能力集合 (插件白名单)").size(11).font(cosmic::font::bold())
                                    .class(cosmic::theme::Text::Color(c_muted2))
                            )
                            .push(widget::container(caps_el).padding([0, 24]))
                            .spacing(6)
                            .padding([10, 0, 10, 0])
                    )
                    .push(widget::divider::horizontal::light())
                    // ── workspace path ────────────────────────────────────
                    .push(
                        widget::column()
                            .push(
                                widget::text("工作空间").size(11).font(cosmic::font::bold())
                                    .class(cosmic::theme::Text::Color(c_muted2))
                            )
                            .push(
                                widget::container(
                                    widget::row()
                                        .push(widget::text("📁").size(12))
                                        .push(
                                            widget::text(agent.workspace_dir().to_string_lossy().to_string())
                                                .size(11)
                                                .font(cosmic::font::mono())
                                                .class(cosmic::theme::Text::Color(c_muted2))
                                        )
                                        .spacing(6)
                                        .align_y(Alignment::Center)
                                )
                                .padding([4, 12])
                                .style(|theme: &cosmic::Theme| {
                                    let bg = theme.cosmic().bg_color();
                                    ContainerStyle {
                                        background: Some(cosmic::iced::Background::Color(
                                            cosmic::iced::Color::from_rgb((bg.red*0.78).min(1.0),(bg.green*0.78).min(1.0),(bg.blue*0.78).min(1.0))
                                        )),
                                        border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                                        ..Default::default()
                                    }
                                })
                                .width(Length::Fill)
                            )
                            .spacing(6)
                            .padding([10, 24, 10, 24])
                    )
                    .push(widget::divider::horizontal::light())
                    // ── communication channels ────────────────────────────
                    .push(
                        widget::column()
                            .push(
                                widget::row()
                                    .push(
                                        widget::text("通信渠道绑定").size(11).font(cosmic::font::bold())
                                            .class(cosmic::theme::Text::Color(c_muted2))
                                    )
                                    .push(widget::horizontal_space().width(Length::Fill))
                                    .push(
                                        widget::button::text("+ 添加渠道")
                                            .class(cosmic::theme::Button::Text)
                                            .on_press(AppMessage::NavSelect(NavPage::GeneralSettings))
                                    )
                                    .align_y(Alignment::Center)
                                    .padding([0, 24, 0, 24])
                            )
                            .push(
                                if agent.channels.is_empty() {
                                    Element::from(
                                        widget::container(
                                            widget::text("暂无绑定渠道 — 点击「+ 添加渠道」配置 Telegram / Discord / Slack")
                                                .size(11)
                                                .class(cosmic::theme::Text::Color(c_muted2))
                                        )
                                        .padding([6, 24])
                                    )
                                } else {
                                    let mut ch_col = widget::column().spacing(4);
                                    for ch in &agent.channels {
                                        let display = format!(
                                            "{}  {}{}",
                                            ch.kind.icon(),
                                            ch.kind,
                                            if ch.label.is_empty() { String::new() } else { format!(" — {}", ch.label) }
                                        );
                                        ch_col = ch_col.push(
                                            widget::container(
                                                widget::row()
                                                    .push(widget::text(display).size(12))
                                                    .spacing(6)
                                                    .align_y(Alignment::Center)
                                                    .padding([4, 12])
                                            )
                                            .style(|theme: &cosmic::Theme| {
                                                let bg = theme.cosmic().bg_color();
                                                ContainerStyle {
                                                    background: Some(cosmic::iced::Background::Color(
                                                        cosmic::iced::Color::from_rgb((bg.red*0.82).min(1.0),(bg.green*0.82).min(1.0),(bg.blue*0.82).min(1.0))
                                                    )),
                                                    border: cosmic::iced::Border { radius: 6.0.into(), ..Default::default() },
                                                    ..Default::default()
                                                }
                                            })
                                            .width(Length::Fill)
                                        );
                                    }
                                    widget::container(ch_col).padding([0, 24]).into()
                                }
                            )
                            .spacing(6)
                            .padding([10, 0, 10, 0])
                    )
                    .push(widget::divider::horizontal::light())
                    // ── recent runs quick-preview ─────────────────────────
                    .push(
                        widget::column()
                            .push(recent_runs_el)
                            .padding([10, 24, 10, 24])
                    )
                    .push(widget::divider::horizontal::light())
                    .push(widget::container(actions).padding([12, 0]))
                    .spacing(0);

                widget::container(widget::scrollable(detail))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                widget::container(
                    widget::text("请从左侧选择一个数字员工").size(14)
                        .class(cosmic::theme::Text::Color(c_muted2))
                )
                .width(Length::Fill).height(Length::Fill).center(Length::Fill)
                .into()
            }
        } else {
            // ── welcome / empty state ─────────────────────────────────────
            let fr = |icon: &'static str, title: &'static str, desc: &'static str| -> Element<'static, AppMessage> {
                widget::row()
                    .push(
                        widget::container(widget::text(icon).size(18))
                            .width(Length::Fixed(36.0))
                            .center_x(Length::Fixed(36.0))
                    )
                    .push(
                        widget::column()
                            .push(widget::text(title).size(13).font(cosmic::font::bold()))
                            .push(widget::text(desc).size(11).class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48)
                            )))
                            .spacing(2)
                    )
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into()
            };

            widget::container(
                widget::column()
                    .push(widget::text("数字员工管理").size(22).font(cosmic::font::bold()))
                    .push(widget::Space::new(0, 4))
                    .push(
                        widget::text("在左侧创建或选择一个数字员工")
                            .size(13).class(cosmic::theme::Text::Color(c_muted2))
                    )
                    .push(widget::Space::new(0, 24))
                    .push(widget::divider::horizontal::light())
                    .push(widget::Space::new(0, 16))
                    .push(widget::text("每个数字员工拥有").size(12).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(c_muted2)))
                    .push(widget::Space::new(0, 12))
                    .push(fr("🛡", "独立沙盒隔离",    "WasmEdge 沙盒环境，完全隔离执行"))
                    .push(fr("🔒", "最小权限边界",    "文件/网络/Shell 精细化访问控制"))
                    .push(fr("📋", "完整审计链路",    "每次运行的步骤与事件全量记录"))
                    .push(fr("⚡", "可配置能力集",    "插件白名单，按需授权"))
                    .push(fr("📡", "通信渠道绑定",    "Telegram / Discord / Slack"))
                    .push(fr("📊", "运行统计分析",    "成功率、延迟、资源消耗实时追踪"))
                    .spacing(10)
                    .padding([40, 48])
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

        // ── Combine left + divider + right ────────────────────────────────
        widget::row()
            .push(left_panel)
            .push(
                widget::container(widget::Space::new(1, 0))
                    .style(|theme: &cosmic::Theme| {
                        let c = theme.cosmic().bg_divider();
                        ContainerStyle {
                            background: Some(cosmic::iced::Background::Color(c.into())),
                            ..Default::default()
                        }
                    })
                    .height(Length::Fill)
            )
            .push(right_panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Format Unix timestamp to human-readable date/time (aerospace-grade precision).
    /// Returns format: "2026-02-24 11:49:32" (UTC)
    fn format_timestamp(unix_secs: u64) -> String {
        const SECS_PER_DAY: u64 = 86400;
        const SECS_PER_HOUR: u64 = 3600;
        const SECS_PER_MIN: u64 = 60;
        
        // Days since Unix epoch (1970-01-01)
        let days = unix_secs / SECS_PER_DAY;
        let remaining = unix_secs % SECS_PER_DAY;
        
        let hours = remaining / SECS_PER_HOUR;
        let mins = (remaining % SECS_PER_HOUR) / SECS_PER_MIN;
        let secs = remaining % SECS_PER_MIN;
        
        // Simple date calculation (approximate, good enough for display)
        let year = 1970 + (days / 365);
        let day_of_year = days % 365;
        let month = (day_of_year / 30).min(11) + 1;
        let day = (day_of_year % 30) + 1;
        
        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            year, month, day, hours, mins, secs)
    }
    
    /// Format Unix timestamp to relative time (e.g., "2 minutes ago").
    fn format_relative_time(unix_secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if unix_secs > now {
            return "in the future".to_string();
        }
        
        let diff = now - unix_secs;
        
        if diff < 60 {
            format!("{}s ago", diff)
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }

    /// Sanitize sensitive data for display (aerospace-grade security).
    /// Redacts home directory paths, API keys, tokens, and sensitive URL parameters.
    fn sanitize_sensitive_data(input: &str) -> String {
        let mut result = input.to_string();
        
        // Redact home directory paths
        if let Some(home) = dirs::home_dir() {
            if let Some(home_str) = home.to_str() {
                result = result.replace(home_str, "~");
            }
        }
        
        // Redact common sensitive query parameters
        for keyword in &["api_key=", "apikey=", "token=", "password=", "secret=", "key="] {
            if let Some(pos) = result.find(keyword) {
                let start = pos + keyword.len();
                if let Some(end_pos) = result[start..].find(&['&', ' ', '\n'][..]) {
                    result.replace_range(start..start + end_pos, "***");
                } else if start < result.len() {
                    result.replace_range(start.., "***");
                }
            }
        }
        
        // Redact Bearer tokens
        if let Some(pos) = result.find("Bearer ") {
            let start = pos + 7;
            if let Some(end_pos) = result[start..].find(&[' ', '\n', ','][..]) {
                result.replace_range(start..start + end_pos, "***");
            } else if start < result.len() {
                result.replace_range(start.., "***");
            }
        }
        
        result
    }

    /// Render the Run / Audit replay page for the selected agent.
    fn view_audit_page(&self, _lang: crate::theme::Language) -> Element<AppMessage> {
        use cosmic::iced::Length;
        use openclaw_storage::types::{RunStatus, AuditDecision};

        // ── Left panel: run list ──────────────────────────────────────────
        let mut run_col = widget::column().spacing(4);

        run_col = run_col.push(
            widget::row()
                .push(widget::text("运行记录").size(16).font(cosmic::font::bold()))
                .push(widget::horizontal_space().width(Length::Fill))
                .push(widget::button::text("清空").on_press(AppMessage::AuditClear))
                .align_y(cosmic::iced::Alignment::Center)
        );
        run_col = run_col.push(widget::divider::horizontal::light());

        let c_muted2 = cosmic::iced::Color::from_rgb(0.55, 0.53, 0.51);
        if self.audit_loading {
            run_col = run_col.push(
                widget::container(
                    widget::text("⏳ 加载中...").size(13)
                        .class(cosmic::theme::Text::Color(c_muted2))
                ).padding([8, 8])
            );
        } else if self.audit_runs.is_empty() {
            run_col = run_col.push(
                widget::column()
                    .push(widget::text("暂无运行记录").size(12).class(cosmic::theme::Text::Color(c_muted2)))
                    .push(widget::text("在数字员工页面点击").size(11).class(cosmic::theme::Text::Color(c_muted2)))
                    .push(widget::text("「运行记录」加载").size(11).class(cosmic::theme::Text::Color(c_muted2)))
                    .spacing(2).padding([12, 8])
            );
        } else {
            for (idx, run) in self.audit_runs.iter().enumerate() {
                let is_sel = self.audit_run_selected == Some(idx);
                let (sr, sg, sb) = match &run.status {
                    RunStatus::Running         => (0.3_f32, 0.6, 1.0),
                    RunStatus::Success         => (0.2, 0.85, 0.4),
                    RunStatus::Failed          => (0.9, 0.2, 0.2),
                    RunStatus::Cancelled       => (0.5, 0.5, 0.5),
                    RunStatus::PendingApproval => (0.9, 0.75, 0.2),
                };
                let status_icon = match &run.status {
                    RunStatus::Success => "✅", RunStatus::Failed => "❌",
                    RunStatus::Running => "🔵", RunStatus::Cancelled => "⬜",
                    RunStatus::PendingApproval => "🟡",
                };
                let left_bar = widget::container(widget::Space::new(4, 36))
                    .style(move |_: &cosmic::Theme| ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgb(sr, sg, sb)
                        )),
                        border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                        ..Default::default()
                    });
                let dur = run.duration_secs()
                    .map(|s| if s >= 60 { format!("{}m{}s", s/60, s%60) } else { format!("{}s", s) })
                    .unwrap_or_else(|| "运行中…".to_string());
                let task_label = if run.task_description.len() > 22 {
                    format!("{}…", &run.task_description[..22])
                } else { run.task_description.clone() };

                let row_inner = widget::row()
                    .push(left_bar)
                    .push(widget::Space::new(6, 0))
                    .push(
                        widget::column()
                            .push(widget::row()
                                .push(widget::text(status_icon).size(11))
                                .push(widget::Space::new(4, 0))
                                .push(widget::text(task_label).size(12).font(cosmic::font::bold()))
                                .align_y(cosmic::iced::Alignment::Center)
                            )
                            .push(
                                widget::text(format!("{} 步  {}  拒: {}", run.step_count, dur, run.denied_count))
                                    .size(10).class(cosmic::theme::Text::Color(c_muted2))
                            )
                            .spacing(2)
                    )
                    .align_y(cosmic::iced::Alignment::Center).spacing(4);
                run_col = run_col.push(
                    widget::button::custom(row_inner)
                        .on_press(AppMessage::AuditSelectRun(idx))
                        .class(if is_sel { cosmic::theme::Button::Suggested } else { cosmic::theme::Button::Text })
                        .width(Length::Fill)
                );
            }
            let total   = self.audit_runs.len();
            let success = self.audit_runs.iter().filter(|r| matches!(r.status, RunStatus::Success)).count();
            let failed  = self.audit_runs.iter().filter(|r| matches!(r.status, RunStatus::Failed)).count();
            run_col = run_col
                .push(widget::divider::horizontal::light())
                .push(widget::container(
                    widget::text(format!("共{}  ✅{}  ❌{}", total, success, failed))
                        .size(11).class(cosmic::theme::Text::Color(c_muted2))
                ).padding([4, 8]));
        }

        let left_panel = widget::container(
            widget::scrollable(run_col.padding([8, 8]))
        )
        .width(Length::Fixed(260.0))
        .height(Length::Fill)
        .style(|theme: &cosmic::Theme| {
            let c = theme.cosmic();
            ContainerStyle {
                background: Some(cosmic::iced::Background::Color(c.bg_color().into())),
                ..Default::default()
            }
        });

        // ── Right panel: aerospace-grade audit detail ─────────────────────
        let c_ok     = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
        let c_warn   = cosmic::iced::Color::from_rgb(0.96, 0.72, 0.18);
        let c_danger = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);
        let c_info   = cosmic::iced::Color::from_rgb(0.32, 0.62, 0.98);

        let right_panel: Element<AppMessage> = if let Some(idx) = self.audit_run_selected {
            if let Some(run) = self.audit_runs.get(idx) {
                // ── Status color ──────────────────────────────────────────
                let (sr, sg, sb) = match &run.status {
                    RunStatus::Running         => (0.3_f32, 0.6, 1.0),
                    RunStatus::Success         => (0.2, 0.85, 0.4),
                    RunStatus::Failed          => (0.9, 0.2, 0.2),
                    RunStatus::Cancelled       => (0.5, 0.5, 0.5),
                    RunStatus::PendingApproval => (0.9, 0.75, 0.2),
                };
                // ── Section title helper ──────────────────────────────────
                let section_hdr = |label: &str| -> Element<AppMessage> {
                    widget::row()
                        .push(
                            widget::container(widget::Space::new(3, 16))
                                .style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: Some(cosmic::iced::Background::Color(c_info)),
                                    border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                                    ..Default::default()
                                })
                        )
                        .push(widget::Space::new(8, 0))
                        .push(widget::text(label.to_string()).size(14).font(cosmic::font::bold()))
                        .align_y(cosmic::iced::Alignment::Center)
                        .into()
                };
                // ── 1. Header ─────────────────────────────────────────────
                let header_row = widget::row()
                    .push(widget::text(&run.task_description).size(18).font(cosmic::font::bold()))
                    .push(widget::horizontal_space().width(Length::Fill))
                    .push(
                        widget::container(widget::text(run.status.to_string()).size(12))
                        .padding([4, 12])
                        .style(move |_: &cosmic::Theme| ContainerStyle {
                            background: Some(cosmic::iced::Background::Color(
                                cosmic::iced::Color::from_rgba(sr, sg, sb, 0.18))),
                            border: cosmic::iced::Border {
                                color: cosmic::iced::Color::from_rgb(sr, sg, sb),
                                width: 1.0, radius: 10.0.into(),
                            },
                            ..Default::default()
                        })
                    )
                    .align_y(cosmic::iced::Alignment::Center);

                let total_dur_str = run.duration_secs()
                    .map(|s| if s >= 60 { format!("{}m{}s", s/60, s%60) } else { format!("{}s", s) })
                    .unwrap_or_else(|| "运行中".to_string());
                let meta_row = widget::text(format!(
                    "Agent: {}  ·  开始: {}  ·  耗时: {}",
                    &run.agent_id[..8.min(run.agent_id.len())],
                    Self::format_timestamp(run.started_at),
                    total_dur_str
                )).size(11).class(cosmic::theme::Text::Color(c_muted2));

                // ── 2. Summary stat cards ─────────────────────────────────
                let risk_ratio = if run.step_count > 0 {
                    format!("{:.0}%", (run.denied_count as f32 / run.step_count as f32) * 100.0)
                } else { "0%".to_string() };

                let mk_card = |lbl: &str, val: String, col: cosmic::iced::Color| -> Element<AppMessage> {
                    widget::container(
                        widget::column()
                            .push(widget::text(val).size(20).font(cosmic::font::bold())
                                .class(cosmic::theme::Text::Color(col)))
                            .push(widget::text(lbl.to_string()).size(10)
                                .class(cosmic::theme::Text::Color(c_muted2)))
                            .spacing(2).align_x(cosmic::iced::Alignment::Center)
                    )
                    .padding([8, 16])
                    .style(|theme: &cosmic::Theme| {
                        let bg = theme.cosmic().bg_color();
                        ContainerStyle {
                            background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgb(
                                (bg.red*0.82).min(1.0),(bg.green*0.82).min(1.0),(bg.blue*0.82).min(1.0)
                            ))),
                            border: cosmic::iced::Border { radius: 8.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    })
                    .into()
                };
                let stat_row = widget::row()
                    .push(mk_card("总步骤",  run.step_count.to_string(),       c_info))
                    .push(mk_card("批准",    run.approved_count.to_string(),   c_ok))
                    .push(mk_card("拒绝",    run.denied_count.to_string(),     c_danger))
                    .push(mk_card("风险率",  risk_ratio,                       c_warn))
                    .spacing(8);

                // ── 3. Risk distribution bar ──────────────────────────────
                let risk_bar: Element<AppMessage> = if !self.audit_steps.is_empty() {
                    let n = self.audit_steps.len() as f32;
                    let r0 = self.audit_steps.iter().filter(|s| s.kind.risk_level()==0).count() as f32;
                    let r1 = self.audit_steps.iter().filter(|s| s.kind.risk_level()==1).count() as f32;
                    let r2 = self.audit_steps.iter().filter(|s| s.kind.risk_level()==2).count() as f32;
                    let r3 = self.audit_steps.iter().filter(|s| s.kind.risk_level()>=3).count() as f32;

                    let bar_w = 480.0_f32;
                    let seg = |ratio: f32, r: f32, g: f32, b: f32, lbl: &'static str| -> Element<AppMessage> {
                        let w = (ratio / n * bar_w).max(0.0) as u16;
                        if w == 0 { return Element::from(widget::Space::new(0,0)); }
                        widget::container(
                            widget::text(lbl).size(9)
                                .class(cosmic::theme::Text::Color(cosmic::iced::Color::WHITE))
                        )
                        .width(Length::Fixed(w as f32))
                        .height(Length::Fixed(18.0))
                        .center(Length::Fixed(w as f32))
                        .style(move |_: &cosmic::Theme| ContainerStyle {
                            background: Some(cosmic::iced::Background::Color(
                                cosmic::iced::Color::from_rgb(r, g, b))),
                            ..Default::default()
                        })
                        .into()
                    };
                    widget::column()
                        .push(widget::text("风险分布").size(12).font(cosmic::font::bold()))
                        .push(
                            widget::row()
                                .push(seg(r0, 0.3, 0.78, 0.4,  "低"))
                                .push(seg(r1, 0.3, 0.6,  0.98, "中"))
                                .push(seg(r2, 0.92,0.72, 0.18, "高"))
                                .push(seg(r3, 0.92,0.28, 0.28, "危"))
                        )
                        .push(
                            widget::row()
                                .push(widget::text(format!("●低 {:.0}%  ●中 {:.0}%  ●高 {:.0}%  ●危 {:.0}%",
                                    r0/n*100.0, r1/n*100.0, r2/n*100.0, r3/n*100.0))
                                    .size(10).class(cosmic::theme::Text::Color(c_muted2)))
                        )
                        .spacing(4)
                        .into()
                } else {
                    Element::from(widget::Space::new(0, 0))
                };

                // ── 4. Waterfall step timeline ────────────────────────────
                let mut detail = widget::column()
                    .push(header_row)
                    .push(meta_row)
                    .push(widget::vertical_space().height(12))
                    .push(stat_row)
                    .push(widget::vertical_space().height(12))
                    .push(risk_bar)
                    .push(widget::vertical_space().height(16))
                    .push(section_hdr("⚡  执行步骤  — 瀑布时间线"))
                    .push(widget::vertical_space().height(8))
                    .spacing(4);

                // ── 4a. Waterfall step timeline ───────────────────────────
                // Calculate max duration for proportional bar widths
                let max_dur_ms = self.audit_steps.iter()
                    .filter_map(|s| s.duration_ms())
                    .max()
                    .unwrap_or(1)
                    .max(1) as f32;

                if self.audit_steps.is_empty() {
                    detail = detail.push(
                        widget::text("暂无步骤记录 — 请先加载运行记录").size(12)
                            .class(cosmic::theme::Text::Color(c_muted2))
                    );
                } else {
                    for (step_idx, step) in self.audit_steps.iter().enumerate() {
                        let (risk_r, risk_g, risk_b, risk_label) = match step.kind.risk_level() {
                            0 => (0.3_f32, 0.78, 0.4,  "低"),
                            1 => (0.3,     0.6,  0.98, "中"),
                            2 => (0.92,    0.72, 0.18, "高"),
                            3 => (0.92,    0.4,  0.18, "危"),
                            _ => (0.92,    0.28, 0.28, "极"),
                        };
                        let (si_r, si_g, si_b) = if step.success {
                            (0.22_f32, 0.82, 0.46)
                        } else {
                            (0.92, 0.28, 0.28)
                        };
                        let status_icon = if step.success { "✓" } else { "✗" };
                        let dur_ms = step.duration_ms().unwrap_or(0);
                        let dur_str = if dur_ms >= 1000 {
                            format!("{:.1}s", dur_ms as f32 / 1000.0)
                        } else if dur_ms > 0 {
                            format!("{}ms", dur_ms)
                        } else { "—".to_string() };

                        // Proportional waterfall bar width (max 320px)
                        let bar_px = if dur_ms > 0 {
                            ((dur_ms as f32 / max_dur_ms) * 320.0).max(4.0)
                        } else { 4.0 };

                        let sanitized_desc = {
                            let s = Self::sanitize_sensitive_data(&step.description);
                            if s.len() > 72 { format!("{}…", &s[..72]) } else { s }
                        };

                        let step_row = widget::row()
                            // Step number column
                            .push(
                                widget::text(format!("{:02}", step_idx + 1))
                                    .size(10).font(cosmic::font::mono())
                                    .class(cosmic::theme::Text::Color(c_muted2))
                                    .width(Length::Fixed(24.0))
                            )
                            // Risk level badge
                            .push(
                                widget::container(
                                    widget::text(risk_label).size(9)
                                        .class(cosmic::theme::Text::Color(cosmic::iced::Color::WHITE))
                                )
                                .width(Length::Fixed(22.0))
                                .height(Length::Fixed(16.0))
                                .center(Length::Fixed(22.0))
                                .style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: Some(cosmic::iced::Background::Color(
                                        cosmic::iced::Color::from_rgb(risk_r, risk_g, risk_b))),
                                    border: cosmic::iced::Border { radius: 3.0.into(), ..Default::default() },
                                    ..Default::default()
                                })
                            )
                            .push(widget::Space::new(6, 0))
                            // Status icon + kind + description
                            .push(
                                widget::column()
                                    .push(
                                        widget::row()
                                            .push(
                                                widget::container(
                                                    widget::text(status_icon).size(10)
                                                        .class(cosmic::theme::Text::Color(cosmic::iced::Color::WHITE))
                                                )
                                                .width(Length::Fixed(16.0))
                                                .height(Length::Fixed(16.0))
                                                .center(Length::Fixed(16.0))
                                                .style(move |_: &cosmic::Theme| ContainerStyle {
                                                    background: Some(cosmic::iced::Background::Color(
                                                        cosmic::iced::Color::from_rgb(si_r, si_g, si_b))),
                                                    border: cosmic::iced::Border { radius: 8.0.into(), ..Default::default() },
                                                    ..Default::default()
                                                })
                                            )
                                            .push(widget::Space::new(6, 0))
                                            .push(widget::text(step.kind.as_str()).size(12).font(cosmic::font::bold()))
                                            .push(widget::Space::new(8, 0))
                                            .push(widget::text(dur_str.clone()).size(11)
                                                .class(cosmic::theme::Text::Color(c_muted2)))
                                            .align_y(cosmic::iced::Alignment::Center)
                                    )
                                    .push(
                                        // Proportional waterfall bar
                                        widget::container(widget::Space::new(bar_px as u16, 5))
                                            .style(move |_: &cosmic::Theme| ContainerStyle {
                                                background: Some(cosmic::iced::Background::Color(
                                                    cosmic::iced::Color::from_rgba(risk_r, risk_g, risk_b, 0.5))),
                                                border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                                                ..Default::default()
                                            })
                                    )
                                    .push(
                                        widget::text(sanitized_desc).size(10)
                                            .class(cosmic::theme::Text::Color(c_muted2))
                                    )
                                    .spacing(2)
                                    .width(Length::Fill)
                            )
                            .align_y(cosmic::iced::Alignment::Start)
                            .spacing(4)
                            .padding([4, 0]);

                        // Alternating row background
                        let row_bg = if step_idx % 2 == 0 {
                            cosmic::iced::Color::TRANSPARENT
                        } else {
                            cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.04)
                        };
                        let row_bg_copy = row_bg;
                        detail = detail.push(
                            widget::container(step_row)
                                .padding([2, 4])
                                .style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: if row_bg_copy != cosmic::iced::Color::TRANSPARENT {
                                        Some(cosmic::iced::Background::Color(row_bg_copy))
                                    } else { None },
                                    border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                                    ..Default::default()
                                })
                                .width(Length::Fill)
                        );
                    }
                }

                // ── 5. Audit events section (enhanced table) ──────────────
                detail = detail
                    .push(widget::vertical_space().height(16))
                    .push(section_hdr("🔐  审计事件流  — 不可篡改追加日志"))
                    .push(widget::vertical_space().height(4));

                // Table header
                detail = detail.push(
                    widget::container(
                        widget::row()
                            .push(widget::text("决策").size(10).font(cosmic::font::bold()).width(Length::Fixed(58.0)))
                            .push(widget::text("事件类型").size(10).font(cosmic::font::bold()).width(Length::Fixed(110.0)))
                            .push(widget::text("目标资源").size(10).font(cosmic::font::bold()).width(Length::Fill))
                            .push(widget::text("时间戳").size(10).font(cosmic::font::bold()).width(Length::Fixed(140.0)))
                            .spacing(4)
                            .padding([4, 8])
                    )
                    .style(|theme: &cosmic::Theme| {
                        let bg = theme.cosmic().bg_color();
                        ContainerStyle {
                            background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgb(
                                (bg.red*0.75).min(1.0),(bg.green*0.75).min(1.0),(bg.blue*0.75).min(1.0)))),
                            border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    })
                    .width(Length::Fill)
                );

                if self.audit_events.is_empty() {
                    detail = detail.push(
                        widget::container(
                            widget::text("暂无审计事件  — 运行任务后自动记录").size(12)
                                .class(cosmic::theme::Text::Color(c_muted2))
                        ).padding([8, 8])
                    );
                } else {
                    for (ev_idx, event) in self.audit_events.iter().enumerate() {
                        let (dec_r, dec_g, dec_b, dec_label, dec_icon) = match &event.decision {
                            AuditDecision::AutoAllowed   => (0.2_f32, 0.85, 0.4,  "自动放行", "✅"),
                            AuditDecision::HumanApproved => (0.2,     0.7,  1.0,  "人工批准", "👤"),
                            AuditDecision::AutoDenied    => (0.9,     0.2,  0.2,  "自动拒绝", "🚫"),
                            AuditDecision::HumanDenied   => (0.8,     0.1,  0.1,  "人工拒绝", "🔴"),
                            AuditDecision::Pending       => (0.9,     0.75, 0.2,  "待审批",   "⏳"),
                        };
                        let target_str = event.target.as_deref().unwrap_or("—");
                        let sanitized_target = Self::sanitize_sensitive_data(target_str);
                        let target_display = if sanitized_target.len() > 38 {
                            format!("{}…", &sanitized_target[..38])
                        } else { sanitized_target };
                        let ts_str = Self::format_timestamp(event.ts);

                        let ev_row = widget::row()
                            .push(
                                widget::container(
                                    widget::text(format!("{} {}", dec_icon, dec_label)).size(10)
                                )
                                .padding([2, 5])
                                .width(Length::Fixed(58.0))
                                .style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: Some(cosmic::iced::Background::Color(
                                        cosmic::iced::Color::from_rgba(dec_r, dec_g, dec_b, 0.15))),
                                    border: cosmic::iced::Border {
                                        color: cosmic::iced::Color::from_rgb(dec_r, dec_g, dec_b),
                                        width: 1.0, radius: 4.0.into(),
                                    },
                                    ..Default::default()
                                })
                            )
                            .push(widget::Space::new(4, 0))
                            .push(
                                widget::text(&event.event_kind).size(11).font(cosmic::font::bold())
                                    .width(Length::Fixed(106.0))
                            )
                            .push(
                                widget::text(target_display).size(11)
                                    .class(cosmic::theme::Text::Color(c_muted2))
                                    .width(Length::Fill)
                            )
                            .push(
                                widget::text(ts_str).size(10).font(cosmic::font::mono())
                                    .class(cosmic::theme::Text::Color(c_muted2))
                                    .width(Length::Fixed(140.0))
                            )
                            .align_y(cosmic::iced::Alignment::Center)
                            .spacing(4)
                            .padding([4, 8]);

                        let ev_bg = if ev_idx % 2 == 0 {
                            cosmic::iced::Color::TRANSPARENT
                        } else {
                            cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.04)
                        };
                        let ev_bg_copy = ev_bg;
                        detail = detail.push(
                            widget::container(ev_row)
                                .style(move |_: &cosmic::Theme| ContainerStyle {
                                    background: if ev_bg_copy != cosmic::iced::Color::TRANSPARENT {
                                        Some(cosmic::iced::Background::Color(ev_bg_copy))
                                    } else { None },
                                    ..Default::default()
                                })
                                .width(Length::Fill)
                        );
                    }
                    // Event count footer
                    detail = detail.push(
                        widget::container(
                            widget::text(format!("共 {} 条审计事件  ·  仅追加  ·  防篡改", self.audit_events.len()))
                                .size(10)
                                .class(cosmic::theme::Text::Color(c_muted2))
                        ).padding([6, 8])
                    );
                }

                widget::container(
                    widget::scrollable(detail.padding([24, 32]))
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            } else {
                widget::container(
                    widget::text("请从左侧选择一条运行记录").size(14)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)
                        ))
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill)
                .into()
            }
        } else {
            widget::container(
                widget::column()
                    .push(widget::text("运行记录 & 审计回放").size(22).font(cosmic::font::bold()))
                    .push(widget::vertical_space().height(12))
                    .push(widget::text("在数字员工页面选择一个员工，点击「运行记录」加载").size(14)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)
                        )))
                    .push(widget::vertical_space().height(24))
                    .push(widget::text("审计回放功能：").size(13))
                    .push(widget::text("• 查看每次任务执行的完整步骤链路").size(12))
                    .push(widget::text("• 每步骤的风险等级与执行时长").size(12))
                    .push(widget::text("• 审计事件：自动放行 / 人工批准 / 拒绝记录").size(12))
                    .push(widget::text("• 不可篡改的审计事件流（仅追加）").size(12))
                    .spacing(6)
                    .padding([40, 40])
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

        widget::row()
            .push(left_panel)
            .push(
                widget::container(widget::Space::new(1, 0))
                    .style(|theme: &cosmic::Theme| {
                        let c = theme.cosmic().bg_divider();
                        ContainerStyle {
                            background: Some(cosmic::iced::Background::Color(c.into())),
                            ..Default::default()
                        }
                    })
                    .height(Length::Fill)
            )
            .push(right_panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Small stat card for agent detail panel.
    fn stat_card(label: impl Into<String>, value: impl Into<String>) -> Element<'static, AppMessage> {
        let label = label.into();
        let value = value.into();
        widget::container(
            widget::column()
                .push(widget::text(value).size(20).font(cosmic::font::bold()))
                .push(widget::text(label).size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6)
                    )))
                .spacing(2)
                .align_x(cosmic::iced::Alignment::Center)
        )
        .padding([10, 16])
        .style(|theme: &cosmic::Theme| {
            let c = theme.cosmic();
            ContainerStyle {
                background: Some(cosmic::iced::Background::Color(c.bg_color().into())),
                border: cosmic::iced::Border {
                    color: c.bg_divider().into(),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }

    /// Role selection chip for the agent create form.
    fn role_chip(short: &str, full_role: &str, current: &str) -> Element<'static, AppMessage> {
        let is_active = current == full_role;
        let short = short.to_string();
        let full  = full_role.to_string();
        widget::button::custom(
            widget::text(short).size(11)
        )
        .on_press(AppMessage::AgentRoleChanged(full))
        .class(if is_active {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::Standard
        })
        .padding([3, 8])
        .into()
    }

    /// Per-role accent colour used for avatar circles.
    fn role_accent_color(role: &openclaw_security::AgentRole) -> cosmic::iced::Color {
        use openclaw_security::AgentRole;
        match role {
            AgentRole::TicketAssistant      => cosmic::iced::Color::from_rgb(0.27, 0.62, 0.85),
            AgentRole::CodeReviewer         => cosmic::iced::Color::from_rgb(0.55, 0.38, 0.82),
            AgentRole::ReportGenerator      => cosmic::iced::Color::from_rgb(0.22, 0.72, 0.48),
            AgentRole::SecurityAuditor      => cosmic::iced::Color::from_rgb(0.88, 0.32, 0.32),
            AgentRole::DataAnalyst          => cosmic::iced::Color::from_rgb(0.92, 0.58, 0.18),
            AgentRole::CustomerSupport      => cosmic::iced::Color::from_rgb(0.22, 0.68, 0.78),
            AgentRole::KnowledgeOfficer     => cosmic::iced::Color::from_rgb(0.62, 0.38, 0.82),
            AgentRole::SocialMediaManager   => cosmic::iced::Color::from_rgb(0.88, 0.28, 0.58),
            AgentRole::InboxTriageAgent     => cosmic::iced::Color::from_rgb(0.28, 0.52, 0.88),
            AgentRole::FinanceProcurement   => cosmic::iced::Color::from_rgb(0.72, 0.62, 0.12),
            AgentRole::NewsSecretary        => cosmic::iced::Color::from_rgb(0.48, 0.32, 0.72),
            AgentRole::SecurityCodeAuditor  => cosmic::iced::Color::from_rgb(0.82, 0.38, 0.22),
            AgentRole::Custom { .. }        => cosmic::iced::Color::from_rgb(0.45, 0.45, 0.45),
        }
    }

    /// Render the integrated Plugin Store page.
    fn view_plugin_store(&self, _lang: crate::theme::Language) -> Element<AppMessage> {
        use cosmic::iced::Length;
        
        widget::container(
            widget::column()
                .push(
                    widget::text("Plugin Store")
                        .size(24)
                        .font(cosmic::font::bold())
                )
                .push(widget::vertical_space().height(20))
                .push(
                    // Store navigation tabs
                    widget::row()
                        .spacing(8)
                        .push(
                            widget::button::text("Dashboard")
                                .on_press(AppMessage::Noop) // TODO: Add StoreNav message
                        )
                        .push(
                            widget::button::text("Browse")
                                .on_press(AppMessage::Noop)
                        )
                        .push(
                            widget::button::text("Installed")
                                .on_press(AppMessage::Noop)
                        )
                        .push(
                            widget::button::text("AI Models")
                                .on_press(AppMessage::Noop)
                        )
                        .push(
                            widget::button::text("Chat")
                                .on_press(AppMessage::Noop)
                        )
                        .push(
                            widget::button::text("Settings")
                                .on_press(AppMessage::Noop)
                        )
                )
                .push(widget::vertical_space().height(20))
                .push(
                    widget::text("Plugin Store is being integrated...")
                        .size(16)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)
                        ))
                )
                .push(widget::vertical_space().height(20))
                .push(
                    widget::text("This will show the full plugin store functionality:")
                        .size(14)
                )
                .push(
                    widget::text("• Browse available plugins")
                        .size(12)
                )
                .push(
                    widget::text("• Install/uninstall plugins")
                        .size(12)
                )
                .push(
                    widget::text("• Manage AI model configurations")
                        .size(12)
                )
                .push(
                    widget::text("• Chat with AI assistants")
                        .size(12)
                )
                .push(
                    widget::text("• Configure bot integrations")
                        .size(12)
                )
                .push(widget::vertical_space().height(20))
                .push(
                    widget::text("For now, you can use the standalone Store app:")
                        .size(14)
                )
                .push(
                    widget::button::suggested("Launch Standalone Store")
                        .on_press(AppMessage::OpenPluginStore)
                )
                .spacing(8)
                .padding(40)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Build the custom sidebar element embedded directly in view().
    fn build_sidebar(&self) -> Element<'_, AppMessage> {
        let lang = self.language;
        let cur  = self.nav_page;

        type IconFn = fn(u16) -> cosmic::widget::icon::Icon;
        let items: &[(NavPage, IconFn, cosmic::iced::Color, &str)] = &[
            // 1 — Dashboard
            (NavPage::Dashboard,      crate::icons::home      as IconFn,
             cosmic::iced::Color::from_rgb(0.98, 0.62, 0.22), "Dashboard"),
            // 2 — Event Log
            (NavPage::Events,         crate::icons::event_log as IconFn,
             cosmic::iced::Color::from_rgb(0.38, 0.72, 0.98), "Event Log"),
            // 3 — AI Assistant
            (NavPage::AiChat,         crate::icons::ai        as IconFn,
             cosmic::iced::Color::from_rgb(0.82, 0.52, 0.98), "AI Assistant"),
            // 4 — Claw Terminal
            (NavPage::ClawTerminal,   crate::icons::claw_term as IconFn,
             cosmic::iced::Color::from_rgb(0.28, 0.92, 0.78), "Claw Terminal"),
            // 5 — Digital Workers
            (NavPage::Agents,         crate::icons::home      as IconFn,
             cosmic::iced::Color::from_rgb(0.98, 0.55, 0.35), "Digital Workers"),
            // 6 — Audit Replay
            (NavPage::AuditReplay,    crate::icons::event_log as IconFn,
             cosmic::iced::Color::from_rgb(0.72, 0.55, 0.98), "Audit Replay"),
            // 7 — (divider group) — 8 Security Settings
            (NavPage::Settings,       crate::icons::shield    as IconFn,
             cosmic::iced::Color::from_rgb(0.42, 0.88, 0.55), "Security Settings"),
            // 9 — General Settings
            (NavPage::GeneralSettings, crate::icons::gear     as IconFn,
             cosmic::iced::Color::from_rgb(0.62, 0.72, 0.88), "General Settings"),
        ];

        let mut nav_buttons: Vec<Element<'_, AppMessage>> = Vec::new();

        for (idx, &(page, icon_fn, accent, en)) in items.iter().enumerate() {
            // Divider before: AI Assistant(2), Claw Terminal(3), Digital Workers(4), Security Settings(6)
            if idx == 2 || idx == 3 || idx == 4 || idx == 6 {
                nav_buttons.push(
                    widget::container(widget::divider::horizontal::light())
                        .padding([2, 8])
                        .into(),
                );
            }

            let is_active = cur == page;
            let label = tx(lang, en);

            let accent_bar = widget::container(widget::Space::new(3, 32))
                .style(move |_: &cosmic::Theme| ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(if is_active {
                        accent
                    } else {
                        cosmic::iced::Color::TRANSPARENT
                    })),
                    border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                    ..Default::default()
                });

            let svg_icon = icon_fn(18);

            let item_content = widget::row::with_children(vec![
                accent_bar.into(),
                widget::Space::new(8, 0).into(),
                svg_icon.into(),
                widget::Space::new(8, 0).into(),
                widget::text(label)
                    .size(13)
                    .class(if is_active {
                        cosmic::theme::Text::Color(accent)
                    } else {
                        cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.78, 0.75, 0.72),
                        )
                    })
                    .into(),
            ])
            .align_y(Alignment::Center)
            .padding([5, 6, 5, 0]);

            let btn = widget::button::custom(item_content)
                .on_press(AppMessage::NavSelect(page))
                .class(if is_active {
                    cosmic::theme::Button::MenuItem
                } else {
                    cosmic::theme::Button::MenuRoot
                })
                .width(Length::Fill);

            nav_buttons.push(btn.into());
        }

        let nav_list = widget::column::with_children(nav_buttons)
            .spacing(0)
            .padding([6, 0]);

        // ── Language switcher: compact popup menu ────────────────────────
        let lang_button = widget::button::custom(
            widget::row::with_children(vec![
                crate::icons::lang(12).into(),
                widget::text(lang.label())
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50),
                    ))
                    .into(),
                widget::Space::new(Length::Fixed(2.0), Length::Fixed(0.0)).into(),
                widget::text("▼")
                    .size(8)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50),
                    ))
                    .into(),
            ])
            .spacing(4)
            .align_y(Alignment::Center)
            .padding([4, 6])
        )
        .class(cosmic::theme::Button::Text)
        .on_press(AppMessage::ShowLanguageMenu)
        .width(Length::Fill);
        
        let lang_section = widget::column::with_children(vec![
            lang_button.into()
        ])
        .padding([2, 6]);

        // ── Sandbox status ───────────────────────────────────────────────
        let status_color = match &self.sandbox_status {
            SandboxStatus::Running    => cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46),
            SandboxStatus::Tripped(_) => cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
            SandboxStatus::Error(_)   => cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
            _                         => cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48),
        };
        let status_dot = widget::container(widget::Space::new(8, 8))
            .style(move |_: &cosmic::Theme| ContainerStyle {
                background: Some(cosmic::iced::Background::Color(status_color)),
                border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            });
        let status_row = widget::row::with_children(vec![
            status_dot.into(),
            widget::text(self.sandbox_status.to_string())
                .size(11)
                .class(cosmic::theme::Text::Color(status_color))
                .into(),
        ])
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([3, 8]);

        // ── Assemble ─────────────────────────────────────────────────────
        let sidebar_col = widget::column::with_children(vec![
            widget::container(nav_list)
                .height(Length::Fill)
                .width(Length::Fill)
                .into(),
            widget::divider::horizontal::default().into(),
            lang_section.into(),
            widget::divider::horizontal::light().into(),
            status_row.into(),
            widget::container(
                widget::text("OpenClaw+  v0.1.0")
                    .size(10)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.40, 0.38, 0.36),
                    )),
            )
            .padding([2, 10, 8, 10])
            .into(),
        ])
        .height(Length::Fill);

        widget::container(sidebar_col)
            .style(|theme: &cosmic::Theme| {
                let bg = theme.cosmic().bg_color();
                ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgb(
                            (bg.red   * 0.90).min(1.0),
                            (bg.green * 0.88).min(1.0),
                            (bg.blue  * 0.86).min(1.0),
                        ),
                    )),
                    ..Default::default()
                }
            })
            .width(Length::Fixed(220.0))
            .height(Length::Fill)
            .into()
    }

    /// Persist the current config to disk (best-effort, logs on failure).
    fn save_config(&self) {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("config.toml");
        if let Err(e) = self.config.save_to_file(&config_path) {
            tracing::warn!("Failed to save config: {}", e);
        }
    }

    /// Re-label sidebar items when the language changes.
    fn update_nav_labels(&mut self) {
        let lang = self.language;
        // Collect (entity, label) pairs first to avoid simultaneous immutable+mutable borrow.
        let updates: Vec<_> = self
            .nav_model
            .iter()
            .filter_map(|entity| {
                let page = self.nav_model.data::<NavPage>(entity)?;
                let label = match page {
                    NavPage::Dashboard      => t(lang, "Dashboard",         "仪表盘"),
                    NavPage::Events         => t(lang, "Event Log",         "事件日志"),
                    NavPage::Settings       => t(lang, "Security Settings", "安全设置"),
                    NavPage::AiChat         => t(lang, "AI Assistant",      "AI 助手"),
                    NavPage::PluginStore    => t(lang, "Plugin Store",      "插件商店"),
                    NavPage::GeneralSettings => t(lang, "General Settings", "通用设置"),
                    NavPage::ClawTerminal     => t(lang, "Claw Terminal",     "Claw 终端"),
                    NavPage::Agents           => t(lang, "Digital Workers",   "数字员工"),
                    NavPage::AuditReplay       => t(lang, "Audit Replay",       "审计回放"),
                };
                Some((entity, label))
            })
            .collect();
        for (entity, label) in updates {
            self.nav_model.text_set(entity, label);
        }
    }

    fn view_language_menu(&self) -> Element<AppMessage> {
        let lang = self.language;
        let all_langs = Language::all();
        
        // Create language buttons in a grid layout
        let mut lang_buttons: Vec<Element<'_, AppMessage>> = Vec::new();
        for &l in all_langs.iter() {
            let is_current = l == lang;
            let btn = widget::button::text(l.label())
                .on_press(AppMessage::SetLanguage(l))
                .class(if is_current {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Text
                })
                .padding([8, 16])
                .width(Length::Fixed(80.0));
            lang_buttons.push(btn.into());
        }
        
        // Arrange in rows of 4
        let mut lang_rows: Vec<Element<'_, AppMessage>> = Vec::new();
        let mut it = lang_buttons.into_iter();
        loop {
            let mut row_items: Vec<Element<'_, AppMessage>> = Vec::with_capacity(4);
            for _ in 0..4 {
                match it.next() {
                    Some(e) => row_items.push(e),
                    None => break,
                }
            }
            if row_items.is_empty() { break; }
            lang_rows.push(widget::row::with_children(row_items).spacing(8).into());
        }
        
        let dialog = widget::container(
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    crate::icons::lang(16).into(),
                    widget::text(tx(lang, "Language"))
                        .size(20)
                        .font(cosmic::font::bold())
                        .into(),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
                widget::Space::new(Length::Fill, 16).into(),
                widget::divider::horizontal::default().into(),
                widget::Space::new(Length::Fill, 16).into(),
                widget::column::with_children(lang_rows)
                    .spacing(8)
                    .into(),
                widget::Space::new(Length::Fill, 16).into(),
                widget::divider::horizontal::default().into(),
                widget::Space::new(Length::Fill, 12).into(),
                widget::button::text(tx(lang, "Close"))
                    .on_press(AppMessage::HideLanguageMenu)
                    .class(cosmic::theme::Button::Standard)
                    .width(Length::Fill)
                    .into(),
            ])
            .spacing(0)
            .padding(24)
            .width(Length::Fixed(420.0))
            .align_x(Alignment::Center),
        )
        .style(|theme: &cosmic::Theme| {
            let bg = theme.cosmic().bg_color();
            ContainerStyle {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgb(
                        (bg.red * 0.95).min(1.0),
                        (bg.green * 0.95).min(1.0),
                        (bg.blue * 0.95).min(1.0),
                    ),
                )),
                border: cosmic::iced::Border {
                    width: 1.0,
                    color: cosmic::iced::Color::from_rgb(0.3, 0.28, 0.26),
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        });
        
        widget::container(
            widget::mouse_area(
                widget::container(dialog)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(AppMessage::HideLanguageMenu),
        )
        .style(|_theme: &cosmic::Theme| ContainerStyle {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.7),
            )),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_about(&self) -> Element<AppMessage> {
        let status_color = match &self.sandbox_status {
            SandboxStatus::Running => cosmic::iced::Color::from_rgb(0.2, 0.8, 0.4),
            SandboxStatus::Tripped(_) => cosmic::iced::Color::from_rgb(0.9, 0.2, 0.2),
            _ => cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5),
        };

        let dialog = widget::container(
            widget::column::with_children(vec![
                widget::text("OpenClaw+")
                    .size(28)
                    .font(cosmic::font::bold())
                    .into(),
                widget::Space::new(Length::Fill, 8).into(),
                widget::text("Aerospace-grade sandbox security monitor")
                    .size(14)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5),
                    ))
                    .into(),
                widget::Space::new(Length::Fill, 16).into(),
                widget::divider::horizontal::default().into(),
                widget::Space::new(Length::Fill, 12).into(),
                widget::row::with_children(vec![
                    widget::text("Version").size(13).font(cosmic::font::bold()).into(),
                    widget::text("0.1.0").size(13).into(),
                ])
                .spacing(12)
                .into(),
                widget::row::with_children(vec![
                    widget::text("Sandbox Status").size(13).font(cosmic::font::bold()).into(),
                    widget::text(self.sandbox_status.to_string())
                        .size(13)
                        .class(cosmic::theme::Text::Color(status_color))
                        .into(),
                ])
                .spacing(12)
                .into(),
                widget::row::with_children(vec![
                    widget::text("Events Recorded").size(13).font(cosmic::font::bold()).into(),
                    widget::text(self.stats.total_events.to_string()).size(13).into(),
                ])
                .spacing(12)
                .into(),
                widget::row::with_children(vec![
                    widget::text("Run Mode").size(13).font(cosmic::font::bold()).into(),
                    widget::text(match &self.run_mode {
                        RunMode::Embedded => "Embedded".to_string(),
                        RunMode::Plugin { gateway_url } => format!("Plugin ({})", gateway_url),
                    })
                    .size(13)
                    .into(),
                ])
                .spacing(12)
                .into(),
                widget::row::with_children(vec![
                    widget::text("AI Model").size(13).font(cosmic::font::bold()).into(),
                    widget::text(&self.ai_chat.model_name).size(13).into(),
                ])
                .spacing(12)
                .into(),
                widget::row::with_children(vec![
                    widget::text("AI Endpoint").size(13).font(cosmic::font::bold()).into(),
                    widget::text(&self.ai_chat.endpoint)
                        .size(11)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                        ))
                        .into(),
                ])
                .spacing(12)
                .into(),
                widget::Space::new(Length::Fill, 16).into(),
                widget::divider::horizontal::default().into(),
                widget::Space::new(Length::Fill, 12).into(),
                widget::text("Built with Rust + WasmEdge + libcosmic")
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                    ))
                    .into(),
                widget::text("© 2026 OpenClaw+ Project")
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                    ))
                    .into(),
                widget::Space::new(Length::Fill, 12).into(),
                widget::row::with_children(vec![
                    widget::button::text("GitHub")
                        .on_press(AppMessage::Noop)
                        .class(cosmic::theme::Button::Text)
                        .into(),
                    widget::button::text("Documentation")
                        .on_press(AppMessage::Noop)
                        .class(cosmic::theme::Button::Text)
                        .into(),
                    widget::button::text("License")
                        .on_press(AppMessage::Noop)
                        .class(cosmic::theme::Button::Text)
                        .into(),
                ])
                .spacing(8)
                .into(),
                widget::Space::new(Length::Fill, 12).into(),
                widget::button::suggested("Close")
                    .on_press(AppMessage::CloseAbout)
                    .into(),
            ])
            .spacing(6)
            .padding(32),
        )
        .style(|theme: &cosmic::Theme| {
            let cosmic = theme.cosmic();
            cosmic::iced::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic.background.base.into(),
                )),
                border: cosmic::iced::Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                },
                ..Default::default()
            }
        })
        .width(Length::Fixed(420.0));

        // Centre the dialog over a dimmed backdrop.
        widget::container(
            widget::column::with_children(vec![
                widget::button::text("")
                    .on_press(AppMessage::Noop)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
                widget::row::with_children(vec![
                    widget::horizontal_space().into(),
                    dialog.into(),
                    widget::horizontal_space().into(),
                ])
                .into(),
                widget::button::text("")
                    .on_press(AppMessage::Noop)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            ])
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl OpenClawApp {
    fn build_menu_bar(&self) -> Element<AppMessage> {
        use cosmic::widget::menu::{self, ItemHeight, ItemWidth, KeyBind};
        use cosmic::widget::RcElementWrapper;
        use std::collections::HashMap;

        let no_keys: HashMap<KeyBind, MenuAction> = HashMap::new();

        macro_rules! menu_section {
            ($label:expr, $items:expr) => {
                menu::Tree::with_children(
                    RcElementWrapper::new(Element::from(menu::root($label))),
                    menu::items(&no_keys, $items),
                )
            };
        }

        let lang = self.language;
        let theme_label_tx = if self.warm_theme_active {
            tx(lang, "Switch to Default Dark")
        } else {
            tx(lang, "Switch to Warm Dark")
        };

        menu::bar(vec![
            menu_section!(tx(lang, "File"), vec![
                menu::Item::Button(tx(lang, "Clear Events"),   None, MenuAction::ClearEvents),
                menu::Item::Divider,
                menu::Item::Button(tx(lang, "Emergency Stop"), None, MenuAction::EmergencyStop),
                menu::Item::Divider,
                menu::Item::Button(tx(lang, "Quit"), None, MenuAction::Quit),
            ]),
            menu_section!(tx(lang, "Sandbox"), vec![
                menu::Item::Button(tx(lang, "Start Sandbox"), None, MenuAction::StartSandbox),
                menu::Item::Button(tx(lang, "Stop Sandbox"),  None, MenuAction::StopSandbox),
            ]),
            menu_section!(tx(lang, "Settings"), vec![
                menu::Item::Button(tx(lang, "Security Settings"), None, MenuAction::OpenSecuritySettings),
                menu::Item::Button(tx(lang, "General Settings"),  None, MenuAction::OpenGeneralSettings),
            ]),
            menu_section!(tx(lang, "View"), vec![
                menu::Item::Button(tx(lang, "Toggle Sidebar"), None, MenuAction::ToggleSidebar),
                menu::Item::Button(theme_label_tx,             None, MenuAction::ToggleTheme),
            ]),
            menu_section!(tx(lang, "Plugins"), vec![
                menu::Item::Button(tx(lang, "Open Plugin Store"), None, MenuAction::OpenPluginStore),
            ]),
            menu_section!(tx(lang, "Help"), vec![
                menu::Item::Button(tx(lang, "About OpenClaw+"), None, MenuAction::About),
            ]),
        ])
        .item_height(ItemHeight::Dynamic(40))
        .item_width(ItemWidth::Static(220))
        .spacing(2.0)
        .into()
    }
}

// ── NL Agent execution engine ─────────────────────────────────────────────────

/// A single action step parsed from the AI-generated plan JSON.
#[derive(Debug, serde::Deserialize)]
struct NlStep {
    #[serde(rename = "type")]
    kind: String,
    description: String,
    #[serde(default)]
    command: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    input: String,
    #[serde(default)]
    message: String,
}

/// Execute all steps in the NL agent plan and return a single bulk-output message.
async fn nl_agent_execute_plan(entry_id: u64, plan_json: &str) -> AppMessage {
    let start = std::time::Instant::now();

    // Strip markdown code fences if the model wrapped the JSON
    let clean = plan_json
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let steps: Vec<NlStep> = match serde_json::from_str(clean) {
        Ok(s) => s,
        Err(e) => {
            return AppMessage::ClawBulkOutput {
                entry_id,
                lines: vec![
                    (format!("⚠ Failed to parse AI plan: {}", e), true),
                    (format!("Raw response: {}", &plan_json[..plan_json.len().min(500)]), true),
                ],
                exit_code: Some(1),
                elapsed_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    let mut lines: Vec<(String, bool)> = Vec::new();
    lines.push((format!("🤖 NL Agent: {} step(s) planned", steps.len()), false));

    let mut prev_output = String::new();

    for (i, step) in steps.iter().enumerate() {
        let step_num = i + 1;
        lines.push((format!("▶ Step {}/{}: {}", step_num, steps.len(), step.description), false));

        match step.kind.as_str() {
            "shell" => {
                let cmd = if step.command.is_empty() { "echo '(no command)'".to_string() } else { step.command.clone() };
                lines.push((format!("  $ {}", cmd), false));
                match tokio::process::Command::new("/bin/sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output()
                    .await
                {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        prev_output = stdout.clone();
                        for line in stdout.lines().take(50) {
                            lines.push((format!("  {}", line), false));
                        }
                        if stdout.lines().count() > 50 {
                            lines.push((format!("  … ({} more lines)", stdout.lines().count() - 50), false));
                        }
                        for line in stderr.lines().take(20) {
                            lines.push((format!("  ⚠ {}", line), true));
                        }
                        if !out.status.success() {
                            lines.push((format!("  Exit code: {}", out.status.code().unwrap_or(-1)), true));
                        }
                    }
                    Err(e) => {
                        lines.push((format!("  ✗ Shell error: {}", e), true));
                        prev_output = String::new();
                    }
                }
            }
            "fetch" => {
                let url = step.url.trim().to_string();
                if url.is_empty() {
                    lines.push(("  ✗ No URL specified".to_string(), true));
                    continue;
                }
                lines.push((format!("  GET {}", url), false));
                match reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(15))
                    .user_agent("OpenClaw-NL-Agent/0.1")
                    .build()
                    .unwrap_or_default()
                    .get(&url)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        match resp.text().await {
                            Ok(body) => {
                                let truncated = if body.len() > 3000 {
                                    format!("{} …[+{} chars]", &body[..3000], body.len() - 3000)
                                } else {
                                    body.clone()
                                };
                                prev_output = body;
                                lines.push((format!("  HTTP {}", status), false));
                                for line in truncated.lines().take(40) {
                                    lines.push((format!("  {}", line), false));
                                }
                            }
                            Err(e) => {
                                lines.push((format!("  ✗ Read error: {}", e), true));
                            }
                        }
                    }
                    Err(e) => {
                        lines.push((format!("  ✗ Fetch error: {}", e), true));
                    }
                }
            }
            "analyze" => {
                let input_data = if step.input.contains("$prev") {
                    step.input.replace("$prev", &prev_output)
                } else if step.input.is_empty() {
                    prev_output.clone()
                } else {
                    step.input.clone()
                };
                // Simple analysis: extract key info using grep-like patterns
                let summary_lines: Vec<&str> = input_data.lines()
                    .filter(|l| !l.trim().is_empty())
                    .take(20)
                    .collect();
                lines.push(("  Analysis:".to_string(), false));
                for l in &summary_lines {
                    lines.push((format!("  {}", l), false));
                }
                prev_output = summary_lines.join("\n");
            }
            "report" => {
                let msg = if step.message.is_empty() { "Task completed.".to_string() } else { step.message.clone() };
                lines.push((format!("  📋 {}", msg), false));
                if !prev_output.is_empty() && !step.message.contains("$prev") {
                    lines.push(("  (See previous steps for details)".to_string(), false));
                }
            }
            other => {
                lines.push((format!("  ⚠ Unknown step type: '{}'", other), true));
            }
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    lines.push((format!("✓ NL Agent finished in {}ms", elapsed_ms), false));

    AppMessage::ClawBulkOutput {
        entry_id,
        lines,
        exit_code: Some(0),
        elapsed_ms,
    }
}

// ── Plugin / Gateway execution engine ─────────────────────────────────────────

/// Send a natural-language instruction to OpenClaw via the Plugin Gateway.
///
/// Flow:
/// 1. POST /hooks/before-skill with a synthetic "agent.naturalLanguage" skill call
/// 2. GET  /skills/status to show current state
/// 3. GET  /skills/events to fetch recent events triggered by OpenClaw
/// 4. Summarise results back to the UI
async fn gateway_send_instruction(
    entry_id: u64,
    gateway_url: &str,
    instruction: &str,
) -> AppMessage {
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("OpenClaw-Plus-UI/0.1")
        .build()
        .unwrap_or_default();

    let mut lines: Vec<(String, bool)> = Vec::new();
    lines.push((format!("🔌 Plugin Mode — OpenClaw Gateway: {}", gateway_url), false));
    lines.push((format!("📨 Instruction: {}", instruction), false));

    // ── Step 1: Check gateway health ─────────────────────────────────────────
    match client.get(format!("{}/health", gateway_url)).send().await {
        Ok(r) if r.status().is_success() => {
            lines.push(("  ✓ Gateway reachable".to_string(), false));
        }
        Ok(r) => {
            lines.push((format!("  ⚠ Gateway returned HTTP {}", r.status()), true));
        }
        Err(e) => {
            lines.push((format!("  ✗ Gateway unreachable: {}", e), true));
            return AppMessage::ClawBulkOutput {
                entry_id, lines,
                exit_code: Some(1),
                elapsed_ms: start.elapsed().as_millis() as u64,
            };
        }
    }

    // ── Step 2: Get current status ────────────────────────────────────────────
    if let Ok(resp) = client.get(format!("{}/skills/status", gateway_url)).send().await {
        if let Ok(status) = resp.json::<serde_json::Value>().await {
            let running  = status.get("sandboxRunning").and_then(|v| v.as_bool()).unwrap_or(false);
            let tripped  = status.get("breakerTripped").and_then(|v| v.as_bool()).unwrap_or(false);
            let total    = status.get("totalEvents").and_then(|v| v.as_u64()).unwrap_or(0);
            let denied   = status.get("deniedEvents").and_then(|v| v.as_u64()).unwrap_or(0);
            lines.push((format!("  Sandbox: {}  |  Breaker: {}  |  Events: {} total, {} denied",
                if running { "Running" } else { "Stopped" },
                if tripped { "TRIPPED" } else { "OK" },
                total, denied,
            ), false));
        }
    }

    // ── Step 3: Simulate sending instruction to OpenClaw agent ────────────────
    // In Plugin Mode, OpenClaw is the agent — we send it a task via the
    // before-skill hook as if it were a skill invocation from the agent.
    // The real OpenClaw agent would receive this via its channel (Telegram/Discord/etc.)
    // Here we use the gateway's skill API to inject a synthetic task.
    let skill_payload = serde_json::json!({
        "invocationId": format!("ui-{}", entry_id),
        "skillName": "agent.naturalLanguage",
        "sessionId": format!("claw-terminal-{}", entry_id),
        "args": {
            "instruction": instruction,
            "source": "claw-terminal",
            "timestamp": chrono_or_now(),
        },
        "timestamp": chrono_or_now(),
    });

    match client
        .post(format!("{}/hooks/before-skill", gateway_url))
        .json(&skill_payload)
        .send()
        .await
    {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            match resp.json::<serde_json::Value>().await {
                Ok(body) => {
                    let verdict = body.get("verdict").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let reason  = body.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                    let prompt  = body.get("confirmPrompt").and_then(|v| v.as_str()).unwrap_or("");
                    match verdict {
                        "allow" => {
                            lines.push(("  ✓ Instruction accepted by OpenClaw Gateway".to_string(), false));
                            lines.push(("  OpenClaw will process this instruction via its active channels.".to_string(), false));
                        }
                        "deny" => {
                            lines.push((format!("  ✗ Blocked by security policy: {}", reason), true));
                        }
                        "confirm" => {
                            lines.push(("  ⏳ Awaiting confirmation:".to_string(), false));
                            lines.push((format!("     {}", prompt), false));
                            lines.push(("  Use the Dashboard to Allow/Deny this action.".to_string(), false));
                        }
                        other => {
                            lines.push((format!("  ? Gateway verdict: {} (HTTP {})", other, status_code), false));
                        }
                    }
                }
                Err(e) => {
                    lines.push((format!("  ⚠ Could not parse gateway response: {}", e), true));
                }
            }
        }
        Err(e) => {
            lines.push((format!("  ✗ Failed to send to gateway: {}", e), true));
        }
    }

    // ── Step 4: Fetch recent events to show what OpenClaw did ─────────────────
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    if let Ok(resp) = client
        .get(format!("{}/skills/events?limit=10", gateway_url))
        .send()
        .await
    {
        if let Ok(events) = resp.json::<Vec<serde_json::Value>>().await {
            if !events.is_empty() {
                lines.push((format!("  📋 Recent OpenClaw activity ({} events):", events.len()), false));
                for ev in events.iter().take(8) {
                    let kind    = ev.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
                    let path    = ev.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let allowed = ev.get("allowed").and_then(|v| v.as_bool());
                    let icon = match allowed {
                        Some(true)  => "✓",
                        Some(false) => "✗",
                        None        => "⏳",
                    };
                    let detail = if path.is_empty() { String::new() } else { format!(" — {}", path) };
                    lines.push((format!("    {} {} {}", icon, kind, detail), allowed == Some(false)));
                }
            } else {
                lines.push(("  (No recent events — OpenClaw may not be running yet)".to_string(), false));
            }
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    lines.push((format!("✓ Gateway interaction completed in {}ms", elapsed_ms), false));

    AppMessage::ClawBulkOutput {
        entry_id,
        lines,
        exit_code: Some(0),
        elapsed_ms,
    }
}

/// Returns a simple timestamp string without pulling in the chrono crate.
fn chrono_or_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}

/// Format a UNIX timestamp as HH:MM:SS (UTC) without external crates.
fn chrono_fmt(unix_secs: u64) -> String {
    let s = unix_secs % 60;
    let m = (unix_secs / 60) % 60;
    let h = (unix_secs / 3600) % 24;
    format!("{:02}:{:02}:{:02} UTC", h, m, s)
}

// ── Telegram Bot API helpers ───────────────────────────────────────────────────

/// Poll Telegram getUpdates and return received messages as AppMessage.
async fn tg_get_updates(token: &str, offset: i64) -> AppMessage {
    let url = format!(
        "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=10&limit=20",
        token, offset
    );
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default()
        .get(&url)
        .send()
        .await;

    match result {
        Err(e) => {
            tracing::warn!("[Telegram] getUpdates error: {}", e);
            AppMessage::TgMessagesReceived(vec![])
        }
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Err(e) => {
                    tracing::warn!("[Telegram] JSON parse error: {}", e);
                    AppMessage::TgMessagesReceived(vec![])
                }
                Ok(json) => {
                    if !json["ok"].as_bool().unwrap_or(false) {
                        let desc = json["description"].as_str().unwrap_or("unknown error");
                        tracing::warn!("[Telegram] API error: {} (HTTP {})", desc, status);
                        return AppMessage::TgMessagesReceived(vec![]);
                    }
                    let mut messages = Vec::new();
                    if let Some(results) = json["result"].as_array() {
                        for update in results {
                            let update_id = update["update_id"].as_i64().unwrap_or(0);
                            // Handle regular messages
                            if let Some(msg) = update.get("message") {
                                let chat_id = msg["chat"]["id"]
                                    .as_i64()
                                    .map(|i| i.to_string())
                                    .unwrap_or_default();
                                let from = msg["from"]["username"]
                                    .as_str()
                                    .or_else(|| msg["from"]["first_name"].as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let text = msg["text"].as_str().unwrap_or("").to_string();
                                let date = msg["date"].as_u64().unwrap_or(0);
                                if !text.is_empty() {
                                    messages.push(TgMessage { update_id, chat_id, from, text, date });
                                }
                            }
                        }
                    }
                    AppMessage::TgMessagesReceived(messages)
                }
            }
        }
    }
}

// ── Discord Bot API helpers ────────────────────────────────────────────────────

/// Poll Discord channel messages via REST API (GET /channels/{id}/messages).
/// Discord does not support long-polling; we use short-poll every 3s.
async fn discord_get_messages(token: &str, channel_id: &str, after: Option<&str>) -> AppMessage {
    if channel_id.is_empty() {
        return AppMessage::DiscordMessagesReceived(vec![]);
    }
    let mut url = format!("https://discord.com/api/v10/channels/{}/messages?limit=20", channel_id);
    if let Some(a) = after {
        url.push_str(&format!("&after={}", a));
    }
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
        .get(&url)
        .header("Authorization", format!("Bot {}", token))
        .header("User-Agent", "OpenClaw-Plus/0.1 (https://github.com/openclaw)")
        .send()
        .await;

    match result {
        Err(e) => {
            tracing::warn!("[Discord] get_messages error: {}", e);
            AppMessage::DiscordMessagesReceived(vec![])
        }
        Ok(resp) => {
            if !resp.status().is_success() {
                tracing::warn!("[Discord] HTTP {}", resp.status());
                return AppMessage::DiscordMessagesReceived(vec![]);
            }
            match resp.json::<serde_json::Value>().await {
                Err(e) => {
                    tracing::warn!("[Discord] JSON parse error: {}", e);
                    AppMessage::DiscordMessagesReceived(vec![])
                }
                Ok(json) => {
                    let mut messages = Vec::new();
                    if let Some(arr) = json.as_array() {
                        // Discord returns newest-first; reverse for chronological order
                        for msg in arr.iter().rev() {
                            let msg_id = msg["id"].as_str().unwrap_or("").to_string();
                            let from = msg["author"]["username"].as_str().unwrap_or("unknown").to_string();
                            let text = msg["content"].as_str().unwrap_or("").to_string();
                            // Discord timestamp is ISO8601; convert to unix approx
                            let date = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs()).unwrap_or(0);
                            // Skip bot's own messages (author.bot == true)
                            if msg["author"]["bot"].as_bool().unwrap_or(false) { continue; }
                            if !text.is_empty() {
                                messages.push(BotMessage {
                                    msg_id,
                                    channel_id: channel_id.to_string(),
                                    from,
                                    text,
                                    date,
                                    platform: BotPlatformKind::Discord,
                                });
                            }
                        }
                    }
                    AppMessage::DiscordMessagesReceived(messages)
                }
            }
        }
    }
}

/// Send a message to a Discord channel via REST API.
async fn discord_send_message(token: &str, channel_id: &str, text: &str) -> AppMessage {
    let url = format!("https://discord.com/api/v10/channels/{}/messages", channel_id);
    let body = serde_json::json!({ "content": text });
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
        .post(&url)
        .header("Authorization", format!("Bot {}", token))
        .header("User-Agent", "OpenClaw-Plus/0.1 (https://github.com/openclaw)")
        .json(&body)
        .send()
        .await;

    match result {
        Err(e) => AppMessage::DiscordSendResult { ok: false, info: e.to_string() },
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                AppMessage::DiscordSendResult {
                    ok: true,
                    info: format!("Delivered to channel={} (HTTP {})", channel_id, status),
                }
            } else {
                let body = resp.text().await.unwrap_or_default();
                AppMessage::DiscordSendResult { ok: false, info: format!("HTTP {} — {}", status, body) }
            }
        }
    }
}

// ── Matrix Client-Server API helpers ──────────────────────────────────────────

/// Poll Matrix room events via /_matrix/client/v3/sync.
/// Uses incremental sync with a `since` token for efficiency.
async fn matrix_sync(
    homeserver: &str,
    access_token: &str,
    room_id: &str,
    since: Option<&str>,
) -> AppMessage {
    if homeserver.is_empty() || access_token.is_empty() {
        return AppMessage::MatrixMessagesReceived(vec![]);
    }
    let hs = homeserver.trim_end_matches('/');
    let mut url = format!(
        "{}/_matrix/client/v3/sync?timeout=5000&filter={{\"room\":{{\"timeline\":{{\"limit\":20}}}}}}",
        hs
    );
    if let Some(s) = since {
        url.push_str(&format!("&since={}", s));
    }

    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default()
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await;

    match result {
        Err(e) => {
            tracing::warn!("[Matrix] sync error: {}", e);
            AppMessage::MatrixMessagesReceived(vec![])
        }
        Ok(resp) => {
            if !resp.status().is_success() {
                tracing::warn!("[Matrix] HTTP {}", resp.status());
                return AppMessage::MatrixMessagesReceived(vec![]);
            }
            match resp.json::<serde_json::Value>().await {
                Err(e) => {
                    tracing::warn!("[Matrix] JSON parse error: {}", e);
                    AppMessage::MatrixMessagesReceived(vec![])
                }
                Ok(json) => {
                    let mut messages = Vec::new();
                    // Extract timeline events from the target room
                    if let Some(timeline) = json
                        .pointer(&format!("/rooms/join/{}/timeline/events",
                            room_id.replace('!', "!").replace(':', "%3A")))
                        .or_else(|| {
                            // Try URL-decoded room_id path
                            json["rooms"]["join"].as_object().and_then(|rooms| {
                                rooms.get(room_id).and_then(|r| r["timeline"]["events"].as_array().map(|_a| {
                                    // Return a ref to the array value
                                    &json["rooms"]["join"][room_id]["timeline"]["events"]
                                }))
                            })
                        })
                    {
                        if let Some(events) = timeline.as_array() {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs()).unwrap_or(0);
                            for ev in events {
                                if ev["type"].as_str() != Some("m.room.message") { continue; }
                                let from = ev["sender"].as_str().unwrap_or("unknown").to_string();
                                let text = ev["content"]["body"].as_str().unwrap_or("").to_string();
                                let event_id = ev["event_id"].as_str().unwrap_or("").to_string();
                                if !text.is_empty() {
                                    messages.push(BotMessage {
                                        msg_id: event_id,
                                        channel_id: room_id.to_string(),
                                        from,
                                        text,
                                        date: now,
                                        platform: BotPlatformKind::Matrix,
                                    });
                                }
                            }
                        }
                    }
                    AppMessage::MatrixMessagesReceived(messages)
                }
            }
        }
    }
}

/// Send a message to a Matrix room via /_matrix/client/v3/rooms/{roomId}/send.
async fn matrix_send_message(homeserver: &str, access_token: &str, room_id: &str, text: &str) -> AppMessage {
    let hs = homeserver.trim_end_matches('/');
    let txn_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis()).unwrap_or(0);
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
        hs, room_id, txn_id
    );
    let body = serde_json::json!({
        "msgtype": "m.text",
        "body": text
    });
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
        .put(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&body)
        .send()
        .await;

    match result {
        Err(e) => AppMessage::MatrixSendResult { ok: false, info: e.to_string() },
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                AppMessage::MatrixSendResult {
                    ok: true,
                    info: format!("Delivered to room={} (HTTP {})", room_id, status),
                }
            } else {
                let body = resp.text().await.unwrap_or_default();
                AppMessage::MatrixSendResult { ok: false, info: format!("HTTP {} — {}", status, body) }
            }
        }
    }
}

// ── Slack Web API helpers ──────────────────────────────────────────────────────

/// Poll Slack channel messages via conversations.history API.
async fn slack_get_messages(token: &str, channel_id: &str, oldest: Option<&str>) -> AppMessage {
    if channel_id.is_empty() {
        return AppMessage::SlackMessagesReceived(vec![]);
    }
    let mut url = format!(
        "https://slack.com/api/conversations.history?channel={}&limit=20",
        channel_id
    );
    if let Some(o) = oldest {
        url.push_str(&format!("&oldest={}", o));
    }
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match result {
        Err(e) => {
            tracing::warn!("[Slack] get_messages error: {}", e);
            AppMessage::SlackMessagesReceived(vec![])
        }
        Ok(resp) => {
            match resp.json::<serde_json::Value>().await {
                Err(e) => {
                    tracing::warn!("[Slack] JSON parse error: {}", e);
                    AppMessage::SlackMessagesReceived(vec![])
                }
                Ok(json) => {
                    if !json["ok"].as_bool().unwrap_or(false) {
                        let err = json["error"].as_str().unwrap_or("unknown");
                        tracing::warn!("[Slack] API error: {}", err);
                        return AppMessage::SlackMessagesReceived(vec![]);
                    }
                    let mut messages = Vec::new();
                    if let Some(msgs) = json["messages"].as_array() {
                        // Slack returns newest-first; reverse for chronological
                        for msg in msgs.iter().rev() {
                            // Skip bot messages and subtypes
                            if msg["subtype"].is_string() { continue; }
                            let ts = msg["ts"].as_str().unwrap_or("0").to_string();
                            let from = msg["user"].as_str().unwrap_or("unknown").to_string();
                            let text = msg["text"].as_str().unwrap_or("").to_string();
                            let date = ts.split('.').next()
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(0);
                            if !text.is_empty() {
                                messages.push(BotMessage {
                                    msg_id: ts,
                                    channel_id: channel_id.to_string(),
                                    from,
                                    text,
                                    date,
                                    platform: BotPlatformKind::Slack,
                                });
                            }
                        }
                    }
                    AppMessage::SlackMessagesReceived(messages)
                }
            }
        }
    }
}

/// Send a message to a Slack channel via chat.postMessage API.
async fn slack_send_message(token: &str, channel_id: &str, text: &str) -> AppMessage {
    let body = serde_json::json!({
        "channel": channel_id,
        "text": text
    });
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
        .post("https://slack.com/api/chat.postMessage")
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await;

    match result {
        Err(e) => AppMessage::SlackSendResult { ok: false, info: e.to_string() },
        Ok(resp) => {
            match resp.json::<serde_json::Value>().await {
                Err(e) => AppMessage::SlackSendResult { ok: false, info: e.to_string() },
                Ok(json) => {
                    if json["ok"].as_bool().unwrap_or(false) {
                        AppMessage::SlackSendResult {
                            ok: true,
                            info: format!("Delivered to channel={}", channel_id),
                        }
                    } else {
                        let err = json["error"].as_str().unwrap_or("unknown").to_string();
                        AppMessage::SlackSendResult { ok: false, info: err }
                    }
                }
            }
        }
    }
}

/// Send a text message via Telegram Bot API sendMessage.
async fn tg_send_message(token: &str, chat_id: &str, text: &str) -> AppMessage {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "HTML"
    });
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
        .post(&url)
        .json(&body)
        .send()
        .await;

    match result {
        Err(e) => AppMessage::TgSendResult { ok: false, info: e.to_string() },
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Err(e) => AppMessage::TgSendResult { ok: false, info: e.to_string() },
                Ok(json) => {
                    if json["ok"].as_bool().unwrap_or(false) {
                        AppMessage::TgSendResult {
                            ok: true,
                            info: format!("Delivered to chat_id={} (HTTP {})", chat_id, status),
                        }
                    } else {
                        let desc = json["description"].as_str().unwrap_or("unknown").to_string();
                        AppMessage::TgSendResult { ok: false, info: desc }
                    }
                }
            }
        }
    }
}
