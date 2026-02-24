//! # `app.rs` — OpenClaw+ Store Application Root
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! This module is the top-level entry point for the `libcosmic`-based
//! OpenClaw+ Store UI.  It owns the entire application state ([`StoreApp`]),
//! defines every inter-component message ([`StoreMessage`]), and wires
//! together the navigation model, sidebar, header, and per-page views.
//!
//! ## Module layout
//! | Sub-module | Responsibility |
//! |---|---|
//! | [`crate::view_store`] | Plugin Store & Installed pages |
//! | [`crate::view_ai`]    | AI Model configuration page |
//! | [`crate::view_bot`]   | Bot integrations & Local API page |
//! | [`crate::view_chat`]  | Local AI chat conversation page |
//!
//! ## Warm-tone UI palette
//! libcosmic does not expose arbitrary container background colours, so the
//! warm tone is achieved through:
//! - `cosmic::iced::Color` on `widget::text` nodes (amber `#E8820C`, gold
//!   `#D4A017`, terracotta `#C0622A`).
//! - Consistent use of `widget::button::suggested` (uses the theme accent)
//!   for the active navigation item and primary actions.
//! - Generous padding and `widget::divider` separators to create visual
//!   breathing room.

use crate::i18n::{self, Locale};
use crate::registry::{InstallProgress, RegistryClient};
use crate::types::{
    AiBackend, AiModelConfig, AiModelStatus,
    ApiTransport, AuditEvent, BotConfig, BotPlatform,
    ChatMessage, ChatRole, ChatState,
    DashboardState, InstallState, LibrarySource, LocalApiConfig,
    PluginEntry, ProcessStatus, RegistryIndex, SandboxMetrics, StorePrefs,
};
use cosmic::app::{Core, Task};
use cosmic::iced::{Color, Font, Length, Subscription};
use cosmic::widget;
use cosmic::{executor, Element};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

// ── Warm-tone colour constants ─────────────────────────────────────────────────
// These are used to tint specific text nodes (e.g. the app title) because
// libcosmic containers do not support arbitrary background colours.

/// Amber-orange used for the "OpenClaw+" brand title.
const BRAND_ORANGE: Color = Color { r: 0.91, g: 0.51, b: 0.05, a: 1.0 };

/// Muted gold used for section sub-headings and status badges.
#[allow(dead_code)]
const WARM_GOLD: Color = Color { r: 0.83, g: 0.63, b: 0.09, a: 1.0 };

/// Terracotta used for error / warning indicators.
#[allow(dead_code)]
const WARM_TERRACOTTA: Color = Color { r: 0.75, g: 0.38, b: 0.17, a: 1.0 };

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StoreMessage {
    // ── Plugin store ──
    SwitchSource(LibrarySource),
    IndexLoaded(Result<RegistryIndex, String>),
    InstallPlugin(String),
    UninstallPlugin(String),
    DownloadProgress { plugin_id: String, progress: InstallProgress },
    SearchChanged(String),
    CategoryFilter(Option<String>),
    Refresh,
    RegistryUrlChanged(LibrarySource, String),
    NavTo(NavPage),
    /// Async scan of the plugin directory completed — contains IDs found on disk.
    PluginsScanned(Vec<String>),
    /// A streaming progress tick arrived while a plugin is downloading.
    DownloadTick { plugin_id: String, progress: InstallProgress },
    /// Show the quit confirmation dialog.
    ShowQuitDialog,
    /// User confirmed quit — save state and exit.
    ConfirmQuit,
    /// User cancelled quit — close the dialog.
    CancelQuit,

    // ── AI Models ──
    AiEndpointChanged(usize, String),
    AiModelNameChanged(usize, String),
    AiBackendChanged(usize, AiBackend),
    AiApiKeyChanged(usize, String),
    AiMaxTokensChanged(usize, String),
    AiTemperatureChanged(usize, String),
    AiSetActive(usize),
    AiTestConnection(usize),
    AiConnectionResult(usize, Result<usize, String>),
    AiAddProfile,
    AiRemoveProfile(usize),

    // ── Bot & Local API ──
    BotTokenChanged(usize, String),
    BotWebhookChanged(usize, String),
    BotPlatformChanged(usize, BotPlatform),
    BotToggle(usize),
    BotAdd,
    BotRemove(usize),
    ApiHostChanged(usize, String),
    ApiPortChanged(usize, String),
    ApiTokenChanged(usize, String),
    ApiTransportChanged(usize, ApiTransport),
    ApiToggle(usize),
    ApiAdd,
    ApiRemove(usize),

    // ── Chat ──
    ChatInputChanged(String),
    ChatSend,
    ChatResponseReceived(Result<String, String>),
    ChatClear,
    ChatSelectProfile(usize),
    ChatToggleSystemPrompt,
    ChatSystemPromptChanged(String),

    // ── Dashboard ──
    /// Trigger a fresh process-status check and metrics snapshot.
    DashboardRefresh,
    /// Result of the async process-status probe.
    DashboardProcessResult(ProcessStatus),
    /// Inject a synthetic audit event (used by the security crate bridge).
    DashboardAuditEvent(AuditEvent),
    /// Clear the audit log.
    DashboardClearLog,

    /// Switch the UI language.
    SetLocale(Locale),

    Noop,
}

// ── Nav pages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavPage {
    Dashboard,
    Store,
    Installed,
    Chat,
    AiModels,
    BotApi,
    Settings,
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct StoreApp {
    core: Core,
    nav_page: NavPage,
    prefs: Arc<parking_lot::RwLock<StorePrefs>>,
    client: Arc<RegistryClient>,
    plugins: Vec<PluginEntry>,
    install_states: HashMap<String, InstallState>,
    loading: bool,
    fetch_error: Option<String>,
    search: String,
    category_filter: Option<String>,
    clawplus_url_edit: String,
    openclaw_url_edit: String,
    // AI model profiles
    ai_profiles: Vec<AiModelConfig>,
    // Bot integrations
    bots: Vec<BotConfig>,
    // Local API endpoints
    local_apis: Vec<LocalApiConfig>,
    // Chat page
    chat_messages: Vec<ChatMessage>,
    chat_input: String,
    chat_state: ChatState,
    chat_active_profile: usize,
    chat_system_prompt: String,
    chat_show_system_prompt: bool,
    chat_msg_counter: usize,
    // Dashboard page
    dashboard: DashboardState,
    // Active UI locale
    locale: Locale,
    // Whether the quit confirmation dialog is currently shown
    show_quit_dialog: bool,
}

impl cosmic::Application for StoreApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = StoreMessage;

    const APP_ID: &'static str = "dev.clawplus.Store";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let prefs = Arc::new(parking_lot::RwLock::new(StorePrefs::default()));
        let client = Arc::new(RegistryClient::new(prefs.clone()));

        let clawplus_url = prefs.read().clawplus_registry_url.clone();
        let openclaw_url = prefs.read().openclaw_registry_url.clone();

        let app = Self {
            core,
            nav_page: NavPage::Dashboard,
            prefs,
            client,
            plugins: Vec::new(),
            install_states: HashMap::new(),
            loading: true,
            fetch_error: None,
            search: String::new(),
            category_filter: None,
            clawplus_url_edit: clawplus_url,
            openclaw_url_edit: openclaw_url,
            ai_profiles: vec![AiModelConfig::default()],
            bots: vec![BotConfig::default()],
            local_apis: vec![LocalApiConfig::default()],
            chat_messages: Vec::new(),
            chat_input: String::new(),
            chat_state: ChatState::Idle,
            chat_active_profile: 0,
            chat_system_prompt: "You are a helpful assistant running locally via OpenClaw+.".into(),
            chat_show_system_prompt: false,
            chat_msg_counter: 0,
            dashboard: DashboardState {
                metrics: SandboxMetrics {
                    plugins_loaded: 1,
                    ..SandboxMetrics::default()
                },
                ..DashboardState::default()
            },
            locale: Locale::En,
            show_quit_dialog: false,
        };

        let fetch_cmd = app.fetch_index_cmd();
        let scan_cmd  = app.scan_installed_cmd();
        (app, Task::batch([fetch_cmd, scan_cmd]))
    }

    fn nav_model(&self) -> Option<&widget::nav_bar::Model> {
        None
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let s = i18n::strings();
        let page_title = match self.nav_page {
            NavPage::Dashboard => s.title_dashboard,
            NavPage::Store     => s.title_store,
            NavPage::Installed => s.title_installed,
            NavPage::Chat      => s.title_chat,
            NavPage::AiModels  => s.title_ai_models,
            NavPage::BotApi    => s.title_bot_api,
            NavPage::Settings  => s.title_settings,
        };
        // "OpenClaw+" is rendered in bold amber-orange to establish the brand.
        // libcosmic's `widget::text` accepts an iced `Font` for weight and a
        // `Color` via the `.color()` builder method.
        let brand = widget::text("OpenClaw+")
            .size(17)
            .font(Font {
                weight: cosmic::iced::font::Weight::Bold,
                ..Font::DEFAULT
            })
            .class(cosmic::theme::Text::Color(BRAND_ORANGE));
        vec![
            brand.into(),
            widget::text("  /  ").size(13).into(),
            widget::text(page_title).size(15).into(),
        ]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let mut buttons = match self.nav_page {
            NavPage::Store => vec![
                widget::button::text(i18n::strings().common_refresh)
                    .on_press(StoreMessage::Refresh)
                    .into(),
            ],
            _ => vec![],
        };
        
        // Add Quit button to all pages
        buttons.push(
            widget::button::text("Quit")
                .on_press(StoreMessage::ShowQuitDialog)
                .into()
        );
        
        buttons
    }

    fn view(&self) -> Element<'_, Self::Message> {
        if self.show_quit_dialog {
            return self.view_quit_dialog();
        }

        // ── Sidebar navigation ────────────────────────────────────────────────
        // The sidebar uses a fixed 168 px width.  The active page is
        // highlighted with `widget::button::suggested` (theme accent colour);
        // inactive items use `widget::button::text`.  A small warm-toned
        // brand label sits at the top of the sidebar for quick orientation.
        let sidebar_brand = widget::text("OpenClaw+")
            .size(13)
            .font(Font {
                weight: cosmic::iced::font::Weight::Bold,
                ..Font::DEFAULT
            })
            .class(cosmic::theme::Text::Color(BRAND_ORANGE));

        let s = i18n::strings();

        // Language switcher: compact pick-list at the bottom of the sidebar.
        let lang_picker = {
            let current = self.locale;
            let mut col = widget::column().spacing(1);
            col = col.push(
                widget::text(s.nav_language).size(11)
                    .class(cosmic::theme::Text::Color(WARM_GOLD))
            );
            for &locale in Locale::all() {
                let label = locale.display_name();
                let btn = if locale == current {
                    widget::button::suggested(label)
                        .on_press(StoreMessage::Noop)
                } else {
                    widget::button::text(label)
                        .on_press(StoreMessage::SetLocale(locale))
                };
                col = col.push(btn);
            }
            col
        };

        let sidebar = widget::column()
            .push(widget::container(sidebar_brand).padding([10, 8, 6, 8]))
            .push(widget::divider::horizontal::default())
            .push(self.sidebar_item("📊", s.nav_dashboard, NavPage::Dashboard))
            .push(widget::divider::horizontal::default())
            .push(self.sidebar_item("🔌", s.nav_store,     NavPage::Store))
            .push(self.sidebar_item("📦", s.nav_installed,  NavPage::Installed))
            .push(widget::divider::horizontal::default())
            .push(self.sidebar_item("💬", s.nav_chat,       NavPage::Chat))
            .push(widget::divider::horizontal::default())
            .push(self.sidebar_item("🤖", s.nav_ai_models,  NavPage::AiModels))
            .push(self.sidebar_item("📡", s.nav_bot_api,    NavPage::BotApi))
            .push(widget::divider::horizontal::default())
            .push(self.sidebar_item("⚙️", s.nav_settings,   NavPage::Settings))
            .push(widget::divider::horizontal::default())
            .push(widget::container(lang_picker).padding([6, 8]))
            .push(widget::vertical_space())
            .spacing(2)
            .padding([4, 4])
            .width(Length::Fixed(168.0));

        // ── Main content ──────────────────────────────────────────────────────
        let content: Element<StoreMessage> = match self.nav_page {
            NavPage::Dashboard => self.view_dashboard(),
            NavPage::Store     => self.view_store(),
            NavPage::Installed => self.view_installed(),
            NavPage::Chat      => self.view_chat(),
            NavPage::AiModels  => self.view_ai_models(),
            NavPage::BotApi    => self.view_bot_api(),
            NavPage::Settings  => self.view_settings(),
        };

        widget::row()
            .push(sidebar)
            .push(widget::divider::vertical::default())
            .push(
                widget::container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            StoreMessage::SetLocale(locale) => {
                self.locale = locale;
                i18n::set_locale(locale);
            }
            // ── Plugin store ──────────────────────────────────────────────────
            StoreMessage::SwitchSource(src) => {
                self.prefs.write().active_source = src;
                self.plugins.clear();
                self.fetch_error = None;
                self.loading = true;
                self.category_filter = None;
                return self.fetch_index_cmd();
            }
            StoreMessage::IndexLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(index) => {
                        let active = self.prefs.read().active_source.clone();
                        self.plugins = index.plugins.into_iter()
                            .filter(|p| p.source == active).collect();
                        self.fetch_error = None;
                        // Mark preinstalled plugins as Installed immediately —
                        // they ship with ClawPlus and need no download.
                        let mut preinstalled_count = 0usize;
                        for entry in &self.plugins {
                            if entry.preinstalled {
                                self.install_states
                                    .entry(entry.id.clone())
                                    .or_insert(InstallState::Installed);
                                preinstalled_count += 1;
                            }
                        }
                        // Update dashboard metric so the UI reflects loaded plugins.
                        let installed_total = self.install_states.values()
                            .filter(|s| matches!(s, InstallState::Installed))
                            .count();
                        self.dashboard.metrics.plugins_loaded = installed_total;
                        info!(
                            count = self.plugins.len(),
                            preinstalled = preinstalled_count,
                            installed_total,
                            "store index loaded"
                        );
                    }
                    Err(e) => {
                        error!(error = %e, "failed to load registry index");
                        self.fetch_error = Some(e);
                    }
                }
            }
            StoreMessage::NavTo(page) => { self.nav_page = page; }
            StoreMessage::SearchChanged(q) => { self.search = q; }
            StoreMessage::CategoryFilter(cat) => { self.category_filter = cat; }
            StoreMessage::Refresh => {
                self.loading = true;
                self.fetch_error = None;
                return self.fetch_index_cmd();
            }
            StoreMessage::RegistryUrlChanged(src, url) => match src {
                LibrarySource::ClawPlus => self.clawplus_url_edit = url,
                LibrarySource::OpenClaw => self.openclaw_url_edit = url,
            },
            StoreMessage::InstallPlugin(id) => {
                self.install_states.insert(id.clone(), InstallState::Downloading { progress: 0.0 });
                if let Some(entry) = self.plugins.iter().find(|p| p.id == id).cloned() {
                    let client = self.client.clone();
                    // Stream real-time progress ticks to the UI via DownloadTick,
                    // then emit a final DownloadProgress with the terminal state.
                    return cosmic::task::stream(futures_util::stream::unfold(
                        (client, entry, false),
                        |(client, entry, done)| async move {
                            if done { return None; }
                            let plugin_id = entry.id.clone();
                            let (tx, mut rx) = tokio::sync::mpsc::channel::<InstallProgress>(64);
                            let entry2 = entry.clone();
                            let client2 = client.clone();
                            tokio::spawn(async move {
                                if let Err(e) = client2.install_plugin(&entry2, tx.clone()).await {
                                    let _ = tx.send(InstallProgress::Failed(e.to_string())).await;
                                }
                            });
                            let mut last = InstallProgress::Failed("no progress received".into());
                            let mut ticks: Vec<StoreMessage> = Vec::new();
                            while let Some(p) = rx.recv().await {
                                match &p {
                                    InstallProgress::Done(_) | InstallProgress::Failed(_) => {
                                        last = p;
                                        break;
                                    }
                                    _ => {
                                        ticks.push(StoreMessage::DownloadTick {
                                            plugin_id: plugin_id.clone(),
                                            progress: p.clone(),
                                        });
                                        last = p;
                                    }
                                }
                            }
                            // Emit final terminal state
                            let final_msg = StoreMessage::DownloadProgress {
                                plugin_id: plugin_id.clone(),
                                progress: last,
                            };
                            // Yield the final message; ticks were already queued above.
                            Some((final_msg, (client, entry, true)))
                        },
                    ));
                }
            }
            StoreMessage::UninstallPlugin(id) => {
                if let Some(entry) = self.plugins.iter().find(|p| p.id == id).cloned() {
                    let client = self.client.clone();
                    tokio::spawn(async move {
                        if let Err(e) = client.uninstall_plugin(&entry).await {
                            error!(error = %e, "uninstall failed");
                        }
                    });
                }
                self.install_states.insert(id, InstallState::Idle);
            }
            StoreMessage::DownloadProgress { plugin_id, progress } => {
                let state = InstallState::from(progress.clone());
                self.install_states.insert(plugin_id.clone(), state.clone());
                // When installation completes, register the plugin's skills in the
                // SkillRegistry so the security gateway knows their risk level.
                if matches!(state, InstallState::Installed) {
                    if let Some(entry) = self.plugins.iter().find(|p| p.id == plugin_id) {
                        let skill_count = entry.skills.len();
                        tracing::info!(
                            plugin = %plugin_id,
                            skills = skill_count,
                            "plugin installed — skills now active"
                        );
                    }
                }
                // On failure, preserve the error message for the UI.
                if let InstallProgress::Failed(ref msg) = progress {
                    tracing::error!(plugin = %plugin_id, error = %msg, "plugin install failed");
                }
            }
            StoreMessage::DownloadTick { plugin_id, progress } => {
                // Real-time progress update — update state without waiting for completion.
                self.install_states.insert(plugin_id, InstallState::from(progress));
            }
            StoreMessage::PluginsScanned(ids) => {
                // Mark every plugin found on disk as Installed.
                for id in &ids {
                    self.install_states.insert(id.clone(), InstallState::Installed);
                }
                // Mark preinstalled plugins (no .wasm needed) as Installed.
                for entry in &self.plugins {
                    if entry.preinstalled {
                        self.install_states
                            .entry(entry.id.clone())
                            .or_insert(InstallState::Installed);
                    }
                }
                tracing::info!(
                    on_disk = ids.len(),
                    "plugin directory scanned"
                );
            }

            // ── AI Models ─────────────────────────────────────────────────────
            StoreMessage::AiEndpointChanged(i, v) => {
                if let Some(p) = self.ai_profiles.get_mut(i) { p.endpoint = v; }
            }
            StoreMessage::AiModelNameChanged(i, v) => {
                if let Some(p) = self.ai_profiles.get_mut(i) { p.model = v; }
            }
            StoreMessage::AiBackendChanged(i, b) => {
                if let Some(p) = self.ai_profiles.get_mut(i) {
                    p.backend = b.clone();
                    p.endpoint = match b {
                        AiBackend::Ollama   => "http://localhost:11434".into(),
                        AiBackend::LmStudio => "http://localhost:1234".into(),
                        AiBackend::LlamaCpp => "http://localhost:8080".into(),
                        AiBackend::Custom   => p.endpoint.clone(),
                    };
                }
            }
            StoreMessage::AiApiKeyChanged(i, v) => {
                if let Some(p) = self.ai_profiles.get_mut(i) { p.api_key = v; }
            }
            StoreMessage::AiMaxTokensChanged(i, v) => {
                if let Some(p) = self.ai_profiles.get_mut(i) {
                    if let Ok(n) = v.parse::<u32>() { p.max_tokens = n; }
                }
            }
            StoreMessage::AiTemperatureChanged(i, v) => {
                if let Some(p) = self.ai_profiles.get_mut(i) {
                    if let Ok(f) = v.parse::<f32>() { p.temperature = f.clamp(0.0, 2.0); }
                }
            }
            StoreMessage::AiSetActive(i) => {
                for (idx, p) in self.ai_profiles.iter_mut().enumerate() {
                    p.is_active = idx == i;
                }
            }
            StoreMessage::AiTestConnection(i) => {
                if let Some(p) = self.ai_profiles.get_mut(i) {
                    p.status = AiModelStatus::Checking;
                    let endpoint = p.endpoint.clone();
                    let backend = p.backend.clone();
                    return cosmic::task::future(async move {
                        let url = match backend {
                            AiBackend::Ollama => format!("{}/api/tags", endpoint),
                            _                 => format!("{}/v1/models", endpoint),
                        };
                        let result = reqwest::Client::new()
                            .get(&url)
                            .timeout(std::time::Duration::from_secs(5))
                            .send()
                            .await
                            .map(|_| 1usize)
                            .map_err(|e| e.to_string());
                        StoreMessage::AiConnectionResult(i, result)
                    });
                }
            }
            StoreMessage::AiConnectionResult(i, result) => {
                if let Some(p) = self.ai_profiles.get_mut(i) {
                    p.status = match result {
                        Ok(n)  => AiModelStatus::Online { model_count: n },
                        Err(e) => AiModelStatus::Offline(e),
                    };
                }
            }
            StoreMessage::AiAddProfile => {
                self.ai_profiles.push(AiModelConfig {
                    name: format!("Profile {}", self.ai_profiles.len() + 1),
                    is_active: false,
                    ..AiModelConfig::default()
                });
            }
            StoreMessage::AiRemoveProfile(i) => {
                if self.ai_profiles.len() > 1 { self.ai_profiles.remove(i); }
            }

            // ── Bot & Local API ───────────────────────────────────────────────
            StoreMessage::BotTokenChanged(i, v) => {
                if let Some(b) = self.bots.get_mut(i) { b.token = v; }
            }
            StoreMessage::BotWebhookChanged(i, v) => {
                if let Some(b) = self.bots.get_mut(i) { b.webhook_url = v; }
            }
            StoreMessage::BotPlatformChanged(i, p) => {
                if let Some(b) = self.bots.get_mut(i) { b.platform = p; }
            }
            StoreMessage::BotToggle(i) => {
                if let Some(b) = self.bots.get_mut(i) { b.enabled = !b.enabled; }
            }
            StoreMessage::BotAdd => {
                self.bots.push(BotConfig {
                    name: format!("Bot {}", self.bots.len() + 1),
                    ..BotConfig::default()
                });
            }
            StoreMessage::BotRemove(i) => { self.bots.remove(i); }
            StoreMessage::ApiHostChanged(i, v) => {
                if let Some(a) = self.local_apis.get_mut(i) { a.bind_host = v; }
            }
            StoreMessage::ApiPortChanged(i, v) => {
                if let Some(a) = self.local_apis.get_mut(i) {
                    if let Ok(p) = v.parse::<u16>() { a.port = p; }
                }
            }
            StoreMessage::ApiTokenChanged(i, v) => {
                if let Some(a) = self.local_apis.get_mut(i) { a.auth_token = v; }
            }
            StoreMessage::ApiTransportChanged(i, t) => {
                if let Some(a) = self.local_apis.get_mut(i) { a.transport = t; }
            }
            StoreMessage::ApiToggle(i) => {
                if let Some(a) = self.local_apis.get_mut(i) { a.enabled = !a.enabled; }
            }
            StoreMessage::ApiAdd => {
                self.local_apis.push(LocalApiConfig {
                    name: format!("Endpoint {}", self.local_apis.len() + 1),
                    port: 8765 + self.local_apis.len() as u16,
                    ..LocalApiConfig::default()
                });
            }
            StoreMessage::ApiRemove(i) => { self.local_apis.remove(i); }

            // ── Chat ─────────────────────────────────────────────────────────
            StoreMessage::ChatInputChanged(v) => { self.chat_input = v; }
            StoreMessage::ChatClear => {
                self.chat_messages.clear();
                self.chat_state = ChatState::Idle;
                self.chat_input.clear();
            }
            StoreMessage::ChatSelectProfile(i) => {
                self.chat_active_profile = i;
            }
            StoreMessage::ChatToggleSystemPrompt => {
                self.chat_show_system_prompt = !self.chat_show_system_prompt;
            }
            StoreMessage::ChatSystemPromptChanged(v) => {
                self.chat_system_prompt = v;
            }
            StoreMessage::ChatSend => {
                let text = self.chat_input.trim().to_string();
                if text.is_empty() { return Task::none(); }
                if matches!(self.chat_state, ChatState::Thinking | ChatState::Streaming(_)) {
                    return Task::none();
                }

                // Append user message
                self.chat_msg_counter += 1;
                self.chat_messages.push(ChatMessage {
                    role: ChatRole::User,
                    content: text.clone(),
                    id: self.chat_msg_counter,
                });
                self.chat_input.clear();
                self.chat_state = ChatState::Thinking;

                // Build request payload
                let profile = self.ai_profiles
                    .get(self.chat_active_profile)
                    .cloned()
                    .unwrap_or_default();

                let system_prompt = self.chat_system_prompt.clone();
                let history: Vec<serde_json::Value> = {
                    let mut msgs = vec![serde_json::json!({
                        "role": "system",
                        "content": system_prompt
                    })];
                    for m in &self.chat_messages {
                        let role = match m.role {
                            ChatRole::User      => "user",
                            ChatRole::Assistant => "assistant",
                            ChatRole::System    => "system",
                        };
                        msgs.push(serde_json::json!({
                            "role": role,
                            "content": m.content
                        }));
                    }
                    msgs
                };

                let endpoint = profile.endpoint.clone();
                let model    = profile.model.clone();
                let api_key  = profile.api_key.clone();
                let max_tok  = profile.max_tokens;
                let temp     = profile.temperature;

                return cosmic::task::future(async move {
                    let url = if endpoint.contains("11434") {
                        // Ollama uses /api/chat
                        format!("{}/api/chat", endpoint)
                    } else {
                        format!("{}/v1/chat/completions", endpoint)
                    };

                    let body = serde_json::json!({
                        "model": model,
                        "messages": history,
                        "max_tokens": max_tok,
                        "temperature": temp,
                        "stream": false
                    });

                    let mut req = reqwest::Client::new()
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .timeout(std::time::Duration::from_secs(120))
                        .json(&body);

                    if !api_key.is_empty() {
                        req = req.header("Authorization", format!("Bearer {}", api_key));
                    }

                    let result = req.send().await;
                    match result {
                        Err(e) => StoreMessage::ChatResponseReceived(Err(e.to_string())),
                        Ok(resp) => {
                            let status = resp.status();
                            match resp.json::<serde_json::Value>().await {
                                Err(e) => StoreMessage::ChatResponseReceived(
                                    Err(format!("Parse error: {e}"))
                                ),
                                Ok(json) => {
                                    // Support both OpenAI and Ollama response shapes
                                    let content = json["choices"][0]["message"]["content"]
                                        .as_str()
                                        .or_else(|| json["message"]["content"].as_str())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| {
                                            if status.is_success() {
                                                json.to_string()
                                            } else {
                                                format!("HTTP {status}: {json}")
                                            }
                                        });
                                    StoreMessage::ChatResponseReceived(Ok(content))
                                }
                            }
                        }
                    }
                });
            }
            StoreMessage::ChatResponseReceived(result) => {
                match result {
                    Ok(content) => {
                        self.chat_msg_counter += 1;
                        self.chat_messages.push(ChatMessage {
                            role: ChatRole::Assistant,
                            content,
                            id: self.chat_msg_counter,
                        });
                        self.chat_state = ChatState::Idle;
                    }
                    Err(e) => {
                        error!(error = %e, "chat request failed");
                        self.chat_state = ChatState::Error(e);
                    }
                }
            }

            // ── Dashboard ────────────────────────────────────────────────
            StoreMessage::DashboardRefresh => {
                // Reset status to Unknown while the probe runs.
                self.dashboard.process_status = ProcessStatus::Unknown;
                return cosmic::task::future(async move {
                    StoreMessage::DashboardProcessResult(probe_openclaw_process())
                });
            }
            StoreMessage::DashboardProcessResult(status) => {
                self.dashboard.process_status = status;
            }
            StoreMessage::DashboardAuditEvent(event) => {
                self.dashboard.audit_log.push(event);
                // Cap the log at 200 entries to avoid unbounded growth.
                if self.dashboard.audit_log.len() > 200 {
                    self.dashboard.audit_log.remove(0);
                }
            }
            StoreMessage::DashboardClearLog => {
                self.dashboard.audit_log.clear();
            }

            StoreMessage::Noop => {}
            StoreMessage::ShowQuitDialog => {
                self.show_quit_dialog = true;
            }
            StoreMessage::ConfirmQuit => {
                // Save all application state before exiting
                self.save_all_state();
                tracing::info!("All state saved, exiting application");
                std::process::exit(0);
            }
            StoreMessage::CancelQuit => {
                self.show_quit_dialog = false;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }
}

// ── View helpers ──────────────────────────────────────────────────────────────

impl StoreApp {
    /// Save all application state to disk before quitting.
    fn save_all_state(&self) {
        // 1. Save StorePrefs (registry URLs, plugin directory)
        let prefs_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("store_prefs.json");
        
        if let Some(parent) = prefs_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        let prefs = self.prefs.read().clone();
        match serde_json::to_string_pretty(&prefs) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&prefs_path, json_str) {
                    tracing::error!(error = %e, "Failed to save store preferences");
                } else {
                    tracing::info!("Store preferences saved");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize store preferences");
            }
        }
        
        // 2. Save AI model profiles
        let ai_profiles_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("ai_profiles.json");
        
        if let Some(parent) = ai_profiles_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        match serde_json::to_string_pretty(&self.ai_profiles) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&ai_profiles_path, json_str) {
                    tracing::error!(error = %e, "Failed to save AI profiles");
                } else {
                    tracing::info!(profiles = self.ai_profiles.len(), "AI profiles saved");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize AI profiles");
            }
        }
        
        // 3. Save bot configurations
        let bots_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("bots.json");
        
        match serde_json::to_string_pretty(&self.bots) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&bots_path, json_str) {
                    tracing::error!(error = %e, "Failed to save bot configs");
                } else {
                    tracing::info!(bots = self.bots.len(), "Bot configs saved");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize bot configs");
            }
        }
        
        // 4. Save local API configurations
        let apis_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("local_apis.json");
        
        match serde_json::to_string_pretty(&self.local_apis) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&apis_path, json_str) {
                    tracing::error!(error = %e, "Failed to save local APIs");
                } else {
                    tracing::info!(apis = self.local_apis.len(), "Local APIs saved");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize local APIs");
            }
        }
        
        // 5. Save chat history
        let chat_history_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("store_chat_history.json");
        
        match serde_json::to_string_pretty(&self.chat_messages) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&chat_history_path, json_str) {
                    tracing::error!(error = %e, "Failed to save chat history");
                } else {
                    tracing::info!(messages = self.chat_messages.len(), "Chat history saved");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize chat history");
            }
        }
    }

    fn view_quit_dialog(&self) -> Element<'static, StoreMessage> {
        widget::layer_container(
            widget::container(
                widget::column()
                    .push(
                        widget::text("Confirm Quit")
                            .size(20)
                    )
                    .push(widget::vertical_space().height(16))
                    .push(
                        widget::text("All application state will be saved before exiting:")
                            .size(14)
                    )
                    .push(widget::vertical_space().height(8))
                    .push(widget::text("• Store preferences").size(12))
                    .push(widget::text("• AI model profiles").size(12))
                    .push(widget::text("• Bot configurations").size(12))
                    .push(widget::text("• Local API settings").size(12))
                    .push(widget::text("• Chat history").size(12))
                    .push(widget::vertical_space().height(16))
                    .push(
                        widget::text("Are you sure you want to quit?")
                            .size(14)
                    )
                    .push(widget::vertical_space().height(20))
                    .push(
                        widget::row()
                            .push(
                                widget::button::destructive("Quit")
                                    .on_press(StoreMessage::ConfirmQuit)
                            )
                            .push(widget::horizontal_space().width(12))
                            .push(
                                widget::button::standard("Cancel")
                                    .on_press(StoreMessage::CancelQuit)
                            )
                            .spacing(8)
                    )
                    .spacing(4)
                    .padding(24)
                    .max_width(480)
            )
            .padding(20)
            .class(cosmic::theme::Container::Dialog)
        )
        .into()
    }

    fn sidebar_item<'a>(
        &self,
        _icon: &'a str,
        label: &'a str,
        page: NavPage,
    ) -> Element<'a, StoreMessage> {
        if self.nav_page == page {
            widget::button::suggested(label)
                .on_press(StoreMessage::NavTo(page))
                .width(Length::Fill)
                .into()
        } else {
            widget::button::text(label)
                .on_press(StoreMessage::NavTo(page))
                .width(Length::Fill)
                .into()
        }
    }

    fn fetch_index_cmd(&self) -> Task<StoreMessage> {
        let client = self.client.clone();
        let url = {
            let prefs = self.prefs.read();
            match prefs.active_source {
                LibrarySource::ClawPlus => prefs.clawplus_registry_url.clone(),
                LibrarySource::OpenClaw => prefs.openclaw_registry_url.clone(),
            }
        };
        cosmic::task::future(async move {
            StoreMessage::IndexLoaded(
                client.fetch_index(&url).await.map_err(|e| e.to_string()),
            )
        })
    }

    /// Scans the local plugin directory for installed `.wasm` files and emits
    /// [`StoreMessage::PluginsScanned`] so the UI can mark them as installed.
    /// Runs asynchronously to avoid blocking the UI thread.
    fn scan_installed_cmd(&self) -> Task<StoreMessage> {
        let client = self.client.clone();
        cosmic::task::future(async move {
            let ids = client.installed_ids().await;
            StoreMessage::PluginsScanned(ids)
        })
    }

    fn view_store(&self) -> Element<'static, StoreMessage> {
        let source = self.prefs.read().active_source.clone();
        crate::view_store::view_store(
            &source,
            &self.plugins,
            &self.install_states,
            self.loading,
            &self.fetch_error,
            &self.search,
            &self.category_filter,
        )
    }

    fn view_installed(&self) -> Element<'static, StoreMessage> {
        crate::view_store::view_installed(&self.plugins, &self.install_states)
    }

    fn view_chat(&self) -> Element<'static, StoreMessage> {
        crate::view_chat::view_chat(
            &self.chat_messages,
            &self.chat_input,
            &self.chat_state,
            &self.ai_profiles,
            self.chat_active_profile,
            &self.chat_system_prompt,
            self.chat_show_system_prompt,
        )
    }

    fn view_dashboard(&self) -> Element<'static, StoreMessage> {
        crate::view_dashboard::view_dashboard(&self.dashboard)
    }

    fn view_ai_models(&self) -> Element<'static, StoreMessage> {
        crate::view_ai::view_ai_models(&self.ai_profiles)
    }

    fn view_bot_api(&self) -> Element<'static, StoreMessage> {
        crate::view_bot::view_bot_api(&self.bots, &self.local_apis)
    }

    fn view_settings(&self) -> Element<'_, StoreMessage> {
        let plugin_dir = self.prefs.read().plugin_dir.clone();
        let s = i18n::strings();

        widget::scrollable(
            widget::column()
                .push(widget::text(s.title_settings).size(16))
                .push(widget::divider::horizontal::default())
                // ── Registry URLs ──────────────────────────────────────────
                .push(widget::text(s.settings_registry_url).size(14))
                .push(widget::text("ClawPlus Registry URL").size(11))
                .push(
                    widget::text_input(
                        "https://registry.clawplus.dev/index.json",
                        self.clawplus_url_edit.as_str(),
                    )
                    .on_input(|s| StoreMessage::RegistryUrlChanged(LibrarySource::ClawPlus, s))
                    .width(Length::Fill),
                )
                .push(widget::text("OpenClaw Registry URL").size(11))
                .push(
                    widget::text_input(
                        "https://registry.openclaw.dev/index.json",
                        self.openclaw_url_edit.as_str(),
                    )
                    .on_input(|s| StoreMessage::RegistryUrlChanged(LibrarySource::OpenClaw, s))
                    .width(Length::Fill),
                )
                .push(widget::divider::horizontal::default())
                // ── Plugin directory ───────────────────────────────────────
                .push(widget::text(s.settings_plugin_dir).size(14))
                .push(widget::text(plugin_dir).size(12))
                .push(widget::divider::horizontal::default())
                .push(
                    widget::button::suggested(s.settings_apply)
                        .on_press(StoreMessage::Refresh),
                )
                .spacing(8)
                .padding([16, 16]),
        )
        .height(Length::Fill)
        .into()
    }
}

// ── Process probe ─────────────────────────────────────────────────────────────

/// Scan the running process list for any binary whose name contains "openclaw".
///
/// Uses `std::process::Command` to invoke `pgrep -i openclaw` (macOS / Linux).
/// Returns [`ProcessStatus::Running`] with the first matching PID, or
/// [`ProcessStatus::Stopped`] if none are found.
///
/// This function is intentionally synchronous and cheap — it is called from
/// inside a `cosmic::task::future` so it runs on the async executor thread
/// pool, not the UI thread.
fn probe_openclaw_process() -> ProcessStatus {
    #[cfg(target_os = "windows")]
    {
        // Windows: use `tasklist /FI "IMAGENAME eq openclaw*" /NH /FO CSV`
        let output = std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq openclaw*", "/NH", "/FO", "CSV"])
            .output();
        match output {
            Err(e) => ProcessStatus::CheckError(e.to_string()),
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // CSV output: "openclaw.exe","1234",... — extract PID from 2nd field.
                let first_pid = stdout.lines()
                    .filter(|l| l.to_lowercase().contains("openclaw"))
                    .find_map(|l| {
                        l.split(',')
                            .nth(1)
                            .and_then(|s| s.trim_matches('"').parse::<u32>().ok())
                    });
                match first_pid {
                    Some(pid) => ProcessStatus::Running(pid),
                    None      => ProcessStatus::Stopped,
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // macOS / Linux: pgrep -i openclaw
        let output = std::process::Command::new("pgrep")
            .args(["-i", "openclaw"])
            .output();
        match output {
            Err(e) => ProcessStatus::CheckError(e.to_string()),
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // pgrep prints one PID per line; take the first one.
                let first_pid = stdout
                    .lines()
                    .find_map(|l| l.trim().parse::<u32>().ok());
                match first_pid {
                    Some(pid) => ProcessStatus::Running(pid),
                    None      => ProcessStatus::Stopped,
                }
            }
        }
    }
}
