//! AI Assistant page for OpenClaw+
//!
//! Provides a conversational panel with:
//! - System maintenance quick-action buttons (start/stop sandbox, emergency stop, clear log)
//! - Preset query chips for fast diagnostics
//! - Color-coded chat bubbles (user vs AI)
//! - Processing indicator
//! - Independent AI settings sub-panel

use cosmic::iced::widget::container::Style as ContainerStyle;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container, scrollable};
use cosmic::Element;
use openclaw_assistant::{OpenClawAssistant, SystemContext};
use openclaw_config::RagConfig;
use openclaw_security::SecurityConfig;

use crate::app::AppMessage;
use crate::theme::Language;
use crate::tooltip_helper::{with_tooltip_bubble, with_tooltip_bubble_icon_arrow_i18n, TooltipPosition, TooltipTexts};

// ── Config ────────────────────────────────────────────────────────────────────

/// Vectorization status for a single RAG item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum RagItemStatus {
    /// Item added, not yet indexed.
    Pending,
    /// Currently being vectorized (0–100 %).
    Indexing(u8),
    /// Successfully vectorized; stores chunk count.
    Ready(usize),
    /// Vectorization failed; stores error summary.
    Failed(String),
}

impl std::fmt::Display for RagItemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RagItemStatus::Pending       => write!(f, "⏳ Pending"),
            RagItemStatus::Indexing(pct) => write!(f, "⚙ Indexing {pct}%"),
            RagItemStatus::Ready(n)      => write!(f, "✅ Ready ({n} chunks)"),
            RagItemStatus::Failed(e)     => write!(f, "❌ {e}"),
        }
    }
}

/// A single RAG knowledge item (file or folder) tracked in the assistant panel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantRagItem {
    /// Display label (file name or folder name).
    pub label: String,
    /// Absolute path on disk.
    pub path: String,
    /// Whether this is a folder (true) or a single file (false).
    pub is_folder: bool,
    /// Current vectorization status.
    #[serde(default = "default_rag_status")]
    pub status: RagItemStatus,
}

fn default_rag_status() -> RagItemStatus { RagItemStatus::Pending }

impl AssistantRagItem {
    pub fn new_file(path: &str) -> Self {
        let label = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        Self { label, path: path.to_string(), is_folder: false, status: RagItemStatus::Pending }
    }

    pub fn new_folder(path: &str) -> Self {
        let label = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        Self { label, path: path.to_string(), is_folder: true, status: RagItemStatus::Pending }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantConfig {
    pub endpoint: String,
    pub model: String,
    pub rag_path: String,
    pub temperature_str: String,
    pub top_k_str: String,
    /// RAG knowledge items (files + folders) added in the assistant panel.
    #[serde(default)]
    pub rag_items: Vec<AssistantRagItem>,
}

impl Default for AssistantConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
            model: "qwen2:7b".to_string(),
            rag_path: String::new(),
            temperature_str: "0.7".to_string(),
            top_k_str: "40".to_string(),
            rag_items: Vec::new(),
        }
    }
}

impl AssistantConfig {
    fn config_path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-plus")
            .join("assistant_config.toml")
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if let Ok(s) = std::fs::read_to_string(&path) {
            toml::from_str(&s).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.endpoint.trim().is_empty() {
            return Err("endpoint must not be empty".to_string());
        }
        if self.model.trim().is_empty() {
            return Err("model must not be empty".to_string());
        }
        let temp: f64 = self.temperature_str.parse()
            .map_err(|_| format!("temperature must be a number, got '{}'", self.temperature_str))?;
        if !(0.0..=2.0).contains(&temp) {
            return Err(format!("temperature must be in [0.0, 2.0], got {}", temp));
        }
        let top_k: i64 = self.top_k_str.parse()
            .map_err(|_| format!("top_k must be an integer, got '{}'", self.top_k_str))?;
        if !(1..=1000).contains(&top_k) {
            return Err(format!("top_k must be in [1, 1000], got {}", top_k));
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.validate().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let path = Self::config_path();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let s = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &s)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Build a `RagConfig` from the assistant-panel rag_items.
    ///
    /// This is **separate** from the main system `RagConfig` (which holds
    /// project-level folders/files configured in Settings → RAG).  The
    /// assistant has its own independent knowledge base used only for
    /// AI-assistant queries.
    pub fn to_rag_config(&self) -> openclaw_config::RagConfig {
        let mut folders = Vec::new();
        let mut files   = Vec::new();

        for item in &self.rag_items {
            let path = std::path::PathBuf::from(&item.path);
            if item.is_folder {
                folders.push(openclaw_config::RagFolder {
                    host_path: path,
                    name: item.label.clone(),
                    description: String::new(),
                    include_extensions: Vec::new(),
                    allow_agent_write: false,
                    max_size_mb: None,
                    watch_enabled: false,
                    indexing_status: openclaw_config::IndexingStatus::default(),
                    last_indexed: None,
                    indexed_file_count: 0,
                    indexed_size_bytes: 0,
                });
            } else {
                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("txt")
                    .to_string();
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                files.push(openclaw_config::RagFile {
                    file_path: path,
                    name: item.label.clone(),
                    content_type: ext,
                    size_bytes: size,
                    content_hash: None,
                    indexing_status: openclaw_config::IndexingStatus::default(),
                    last_indexed: None,
                    indexing_error: None,
                    tags: Vec::new(),
                    priority: 5,
                    enabled: true,
                });
            }
        }

        openclaw_config::RagConfig {
            folders,
            files,
            settings: openclaw_config::RagSettings::default(),
        }
    }
}

// ── Conversation model ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConversationItem {
    pub is_user: bool,
    pub text: String,
    pub actions: Vec<String>,
}

// ── Page state ────────────────────────────────────────────────────────────────

pub struct AssistantPage {
    assistant: OpenClawAssistant,
    pub query_input: String,
    conversation_history: Vec<ConversationItem>,
    pub is_processing: bool,
}

impl Default for AssistantPage {
    fn default() -> Self {
        Self::new()
    }
}

impl AssistantPage {
    pub fn new() -> Self {
        let assistant = OpenClawAssistant::new()
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create assistant, using default: {}", e);
                OpenClawAssistant::default()
            });
        Self {
            assistant,
            query_input: String::new(),
            conversation_history: Vec::new(),
            is_processing: false,
        }
    }

    pub fn process_query(
        &mut self,
        rag_config: &RagConfig,
        security_config: &SecurityConfig,
        error_logs: Vec<String>,
    ) {
        if self.query_input.trim().is_empty() {
            return;
        }
        let query = self.query_input.clone();
        self.query_input.clear();

        self.conversation_history.push(ConversationItem {
            is_user: true,
            text: query.clone(),
            actions: Vec::new(),
        });
        self.is_processing = true;

        let context = SystemContext::new()
            .with_rag_config(rag_config.clone())
            .with_security_config(security_config.clone())
            .with_error_logs(error_logs);

        match self.assistant.process_query(&query, &context) {
            Ok(response) => {
                let actions: Vec<String> = response.actions
                    .iter()
                    .map(|a| format!("{:?}", a))
                    .collect();
                self.conversation_history.push(ConversationItem {
                    is_user: false,
                    text: response.text,
                    actions,
                });
            }
            Err(e) => {
                self.conversation_history.push(ConversationItem {
                    is_user: false,
                    text: format!("⚠ Error: {}", e),
                    actions: Vec::new(),
                });
            }
        }
        self.is_processing = false;
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Push a user message and mark as processing (for Ollama inference).
    pub fn push_user_message(&mut self, text: String) {
        self.conversation_history.push(ConversationItem {
            is_user: true,
            text,
            actions: Vec::new(),
        });
        self.is_processing = true;
    }

    /// Push an assistant response (from Ollama inference).
    pub fn push_assistant_response(&mut self, text: String, _latency_ms: u64) {
        self.conversation_history.push(ConversationItem {
            is_user: false,
            text,
            actions: Vec::new(),
        });
        self.is_processing = false;
    }

    /// Push an error message.
    pub fn push_error(&mut self, error: String) {
        self.conversation_history.push(ConversationItem {
            is_user: false,
            text: format!("⚠ Error: {}", error),
            actions: Vec::new(),
        });
        self.is_processing = false;
    }

    /// Get conversation history for inference engine.
    pub fn get_conversation_history(&self) -> &[ConversationItem] {
        &self.conversation_history
    }

    // ── Main view ─────────────────────────────────────────────────────────────

    pub fn view<'a>(
        &'a self,
        lang: Language,
        cfg: &'a AssistantConfig,
        show_settings: bool,
        sandbox_running: bool,
        pending_count: usize,
        available_models: &'a [crate::app::OllamaModel],
    ) -> Element<'a, AppMessage> {
        if show_settings {
            return self.view_settings(lang, cfg, available_models);
        }
        self.view_chat(lang, sandbox_running, pending_count)
    }

    // ── Quick-action panel (system maintenance controls) ──────────────────────

    fn view_quick_actions<'a>(
        lang: Language,
        _sandbox_running: bool,
        pending_count: usize,
    ) -> Element<'a, AppMessage> {
        let (lbl_emergency, lbl_clear, _lbl_dashboard, lbl_section,
             lbl_pending, lbl_go_dash) =
            match lang {
                Language::ZhCn | Language::ZhTw => (
                    "⛔ 紧急停止", "🗑 清空日志",
                    "仪表板", "系统维护操作",
                    "待确认", "查看",
                ),
                _ => (
                    "⛔ Emergency", "🗑 Clear Log",
                    "Dashboard", "System Maintenance",
                    "Pending", "View",
                ),
            };

        let emergency_btn = widget::button::custom(
            widget::text(lbl_emergency).size(12)
                .class(cosmic::theme::Text::Color(
                    cosmic::iced::Color::from_rgb(0.92, 0.35, 0.35)
                ))
        )
        .padding([6, 10])
        .class(cosmic::theme::Button::Text)
        .on_press(AppMessage::EmergencyStop);

        let clear_btn = widget::button::custom(
            widget::text(lbl_clear).size(12)
                .class(cosmic::theme::Text::Color(
                    cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                ))
        )
        .padding([6, 10])
        .class(cosmic::theme::Button::Text)
        .on_press(AppMessage::ClearEvents);

        let control_row = widget::row::with_children(vec![
            widget::Space::new(Length::Fill, 0).into(),
            with_tooltip_bubble_icon_arrow_i18n(
                emergency_btn,
                lang,
                TooltipTexts::ASSISTANT_EMERGENCY_STOP.0,
                TooltipTexts::ASSISTANT_EMERGENCY_STOP.1,
                "⛔",
                TooltipPosition::Bottom,
            ),
            with_tooltip_bubble_icon_arrow_i18n(
                clear_btn,
                lang,
                TooltipTexts::ASSISTANT_CLEAR_LOG.0,
                TooltipTexts::ASSISTANT_CLEAR_LOG.1,
                "🗑",
                TooltipPosition::Bottom,
            ),
        ])
        .spacing(4)
        .align_y(Alignment::Center);

        // Pending confirmations badge row
        let pending_row: Element<'a, AppMessage> = if pending_count > 0 {
            let pending_text = match lang {
                Language::ZhCn | Language::ZhTw =>
                    format!("⚠ {} 项操作 {}", pending_count, lbl_pending),
                _ =>
                    format!("⚠ {} operation(s) {}", pending_count, lbl_pending),
            };
            widget::container(
                widget::row::with_children(vec![
                    widget::text(pending_text)
                        .size(12)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12)
                        ))
                        .width(Length::Fill)
                        .into(),
                    widget::button::custom(
                        widget::text(lbl_go_dash).size(11)
                    )
                    .class(cosmic::theme::Button::Text)
                    .on_press(AppMessage::NavSelect(crate::app::NavPage::Dashboard))
                    .into(),
                ])
                .align_y(Alignment::Center)
                .spacing(6)
                .padding([6, 10])
            )
            .style(|_: &cosmic::Theme| ContainerStyle {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(0.96, 0.72, 0.12, 0.10),
                )),
                border: cosmic::iced::Border {
                    radius: 6.0.into(),
                    color: cosmic::iced::Color::from_rgba(0.96, 0.72, 0.12, 0.35),
                    width: 1.0,
                },
                ..Default::default()
            })
            .width(Length::Fill)
            .into()
        } else {
            widget::Space::new(0, 0).into()
        };

        widget::container(
            widget::column::with_children(vec![
                widget::text(lbl_section)
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                    ))
                    .into(),
                widget::Space::new(0, 6).into(),
                control_row.into(),
                widget::Space::new(0, 4).into(),
                pending_row,
            ])
            .spacing(0)
            .padding([10, 12])
        )
        .style(|theme: &cosmic::Theme| {
            let bg = theme.cosmic().bg_component_color();
            ContainerStyle {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(bg.red, bg.green, bg.blue, 0.6),
                )),
                border: cosmic::iced::Border {
                    radius: 0.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .width(Length::Fill)
        .into()
    }

    // ── Preset query chips ────────────────────────────────────────────────────

    fn view_preset_chips(lang: Language) -> Element<'static, AppMessage> {
        // (display label, query text to send, icon)
        let presets: &[(&str, &str, &str)] = match lang {
            Language::ZhCn | Language::ZhTw => &[
                ("🔍 诊断", "分析最近的安全事件并提供诊断建议", "🔍"),
                ("⚡ 优化", "分析当前配置并提供性能优化建议", "⚡"),
                ("🛡 审计", "检查当前安全策略是否存在漏洞或过宽配置", "🛡"),
                ("📚 RAG", "检查 RAG 知识库配置和索引状态", "📚"),
            ],
            _ => &[
                ("🔍 Diagnose", "Analyze recent security events and suggest fixes", "🔍"),
                ("⚡ Optimize", "Review config and suggest performance improvements", "⚡"),
                ("🛡 Audit", "Check for security policy gaps or over-permissive rules", "🛡"),
                ("📚 RAG", "Check RAG knowledge base config and index state", "📚"),
            ],
        };

        // Build all 4 chips — each chip sends the query at once on click
        let mut chips: Vec<Element<'static, AppMessage>> = presets
            .iter()
            .map(|(label, query, icon)| {
                let lbl: String = label.to_string();
                let q: String = query.to_string();
                let chip = widget::container(
                    widget::button::custom(widget::text(lbl).size(13))
                        .padding([7, 10])
                        .class(cosmic::theme::Button::Standard)
                        .width(Length::Fill)
                        .on_press(AppMessage::AssistantPresetQuery(q))
                )
                .style(|_: &cosmic::Theme| ContainerStyle {
                    border: cosmic::iced::Border {
                        radius: 20.0.into(),
                        color: cosmic::iced::Color::from_rgba(0.82, 0.52, 0.98, 0.45),
                        width: 1.0,
                    },
                    ..Default::default()
                })
                .width(Length::Fill);
                
                // Add simple tooltip for preset chips (avoid lifetime issues)
                with_tooltip_bubble(
                    chip,
                    match lang {
                        Language::ZhCn | Language::ZhTw => "点击发送预设查询",
                        _ => "Click to send preset query",
                    },
                    TooltipPosition::Top,
                )
            })
            .collect();

        // 2 × 2 grid: drain in reverse to maintain index stability
        let chip3 = chips.remove(3);
        let chip2 = chips.remove(2);
        let chip1 = chips.remove(1);
        let chip0 = chips.remove(0);

        let row0 = widget::row::with_children(vec![chip0, chip1])
            .spacing(6)
            .width(Length::Fill);

        let row1 = widget::row::with_children(vec![chip2, chip3])
            .spacing(6)
            .width(Length::Fill);

        widget::container(
            widget::column::with_children(vec![
                row0.into(),
                widget::Space::new(0, 5).into(),
                row1.into(),
            ])
            .padding([8, 12])
        )
        .width(Length::Fill)
        .into()
    }

    // ── Chat view ─────────────────────────────────────────────────────────────

    fn view_chat<'a>(
        &'a self,
        lang: Language,
        sandbox_running: bool,
        pending_count: usize,
    ) -> Element<'a, AppMessage> {
        let placeholder = match lang {
            Language::ZhCn | Language::ZhTw => "输入问题，按 Enter 发送...",
            _ => "Type a question, press Enter to send...",
        };
        let send_label = match lang {
            Language::ZhCn | Language::ZhTw => "发送",
            _ => "Send",
        };
        let clear_label = match lang {
            Language::ZhCn | Language::ZhTw => "清空对话",
            _ => "Clear Chat",
        };

        // ── Chat messages area
        let mut chat_col = widget::column::with_capacity(16).spacing(8).padding([10, 12]);

        if self.conversation_history.is_empty() {
            chat_col = chat_col.push(Self::view_welcome(lang));
        } else {
            for item in &self.conversation_history {
                chat_col = chat_col.push(Self::view_bubble(item, lang));
            }
        }

        // Processing indicator
        if self.is_processing {
            let thinking = match lang {
                Language::ZhCn | Language::ZhTw => "思考中…",
                _ => "Thinking…",
            };
            let indicator: Element<'_, AppMessage> = widget::container(
                widget::text(thinking)
                    .size(12)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.82, 0.52, 0.98)
                    ))
            )
            .padding([6, 12])
            .into();
            chat_col = chat_col.push(indicator);
        }

        let chat_area = scrollable(chat_col)
            .height(Length::Fill)
            .anchor_bottom();

        // ── Input row
        let send_disabled = self.query_input.trim().is_empty() || self.is_processing;
        let input_row = widget::row::with_children(vec![
            with_tooltip_bubble_icon_arrow_i18n(
                widget::text_input(placeholder, &self.query_input)
                    .on_input(AppMessage::AssistantQueryChanged)
                    .on_submit(|_| AppMessage::AssistantSendQuery)
                    .width(Length::Fill),
                lang,
                TooltipTexts::ASSISTANT_INPUT.0,
                TooltipTexts::ASSISTANT_INPUT.1,
                "💬",
                TooltipPosition::Top,
            ),
            with_tooltip_bubble_icon_arrow_i18n(
                widget::button::custom(
                    widget::text(send_label).size(13)
                )
                .class(cosmic::theme::Button::Suggested)
                .on_press_maybe(if send_disabled { None } else { Some(AppMessage::AssistantSendQuery) }),
                lang,
                TooltipTexts::ASSISTANT_SEND_QUERY.0,
                TooltipTexts::ASSISTANT_SEND_QUERY.1,
                "📤",
                TooltipPosition::Top,
            ),
        ])
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([8, 10]);

        // ── Bottom bar: preset chips + clear button
        let bottom_bar = widget::row::with_children(vec![
            widget::Space::new(Length::Fill, 0).into(),
            widget::button::custom(
                widget::text(clear_label).size(11)
            )
            .class(cosmic::theme::Button::Text)
            .on_press(AppMessage::AssistantClearHistory)
            .into(),
        ])
        .padding([2, 10, 4, 10]);

        widget::column::with_children(vec![
            // System maintenance quick-action strip
            Self::view_quick_actions(lang, sandbox_running, pending_count),
            widget::divider::horizontal::light().into(),
            // Preset chips
            Self::view_preset_chips(lang),
            widget::divider::horizontal::light().into(),
            // Chat area
            chat_area.into(),
            widget::divider::horizontal::light().into(),
            input_row.into(),
            bottom_bar.into(),
        ])
        .height(Length::Fill)
        .into()
    }

    // ── Welcome card ──────────────────────────────────────────────────────────

    fn view_welcome(lang: Language) -> Element<'static, AppMessage> {
        let (title, body) = match lang {
            Language::ZhCn | Language::ZhTw => (
                "👋 你好，我是 OpenClaw AI 助手",
                "我可以帮助你：\n\
                 • 诊断沙箱安全事件和拒绝原因\n\
                 • 分析 RAG 知识库配置问题\n\
                 • 优化性能和策略配置\n\
                 • 审计安全策略完整性\n\
                 • 解释系统日志和错误信息\n\n\
                 点击上方快捷按钮可直接控制沙箱，\n\
                 或使用预设问题快速开始对话。",
            ),
            _ => (
                "👋 Hello, I'm your OpenClaw AI Assistant",
                "I can help you with:\n\
                 • Diagnosing sandbox security events & denials\n\
                 • Analyzing RAG knowledge base configuration\n\
                 • Optimizing performance and policy settings\n\
                 • Auditing security policy completeness\n\
                 • Explaining system logs and errors\n\n\
                 Use the quick-action buttons above to control\n\
                 the sandbox, or pick a preset question to start.",
            ),
        };

        widget::container(
            widget::column::with_children(vec![
                widget::text(title)
                    .size(14)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.82, 0.52, 0.98)
                    ))
                    .into(),
                widget::Space::new(0, 8).into(),
                widget::text(body).size(13).into(),
            ])
            .spacing(0)
            .padding(14)
        )
        .style(|_: &cosmic::Theme| ContainerStyle {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgba(0.82, 0.52, 0.98, 0.07),
            )),
            border: cosmic::iced::Border {
                radius: 8.0.into(),
                color: cosmic::iced::Color::from_rgba(0.82, 0.52, 0.98, 0.20),
                width: 1.0,
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    // ── Chat bubble ───────────────────────────────────────────────────────────

    fn view_bubble<'a>(item: &'a ConversationItem, lang: Language) -> Element<'a, AppMessage> {
        if item.is_user {
            // User bubble: right-aligned, accent blue background
            widget::container(
                widget::row::with_children(vec![
                    widget::Space::new(Length::Fill, 0).into(),
                    widget::container(
                        widget::text(&item.text).size(13)
                    )
                    .style(|_: &cosmic::Theme| ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgba(0.25, 0.55, 0.98, 0.20),
                        )),
                        border: cosmic::iced::Border {
                            radius: 10.0.into(),
                            color: cosmic::iced::Color::from_rgba(0.25, 0.55, 0.98, 0.40),
                            width: 1.0,
                        },
                        ..Default::default()
                    })
                    .padding([8, 12])
                    .max_width(280)
                    .into(),
                ])
                .align_y(Alignment::Start)
            )
            .width(Length::Fill)
            .into()
        } else {
            // AI bubble: left-aligned, card background with purple accent
            let has_error = item.text.starts_with("⚠");
            let accent = if has_error {
                cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28)
            } else {
                cosmic::iced::Color::from_rgb(0.82, 0.52, 0.98)
            };

            let mut content_col = widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::text(if has_error { "⚠ AI" } else { "✦ AI" })
                        .size(10)
                        .class(cosmic::theme::Text::Color(accent))
                        .into(),
                ])
                .into(),
                widget::Space::new(0, 4).into(),
                widget::text(&item.text).size(13).into(),
            ])
            .spacing(0);

            if !item.actions.is_empty() {
                let actions_lbl = match lang {
                    Language::ZhCn | Language::ZhTw => "建议操作：",
                    _ => "Suggested Actions:",
                };
                content_col = content_col.push(widget::Space::new(0, 6));
                content_col = content_col.push(
                    widget::text(actions_lbl)
                        .size(11)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.60, 0.58, 0.56)
                        ))
                );
                for action in &item.actions {
                    content_col = content_col.push(
                        widget::text(format!("• {}", action))
                            .size(11)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                            ))
                    );
                }
            }

            widget::container(
                widget::container(content_col)
                    .style(move |_: &cosmic::Theme| ContainerStyle {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgba(
                                if has_error { 0.92 } else { 0.82 },
                                if has_error { 0.28 } else { 0.52 },
                                if has_error { 0.28 } else { 0.98 },
                                0.07,
                            ),
                        )),
                        border: cosmic::iced::Border {
                            radius: 10.0.into(),
                            color: cosmic::iced::Color::from_rgba(
                                if has_error { 0.92 } else { 0.82 },
                                if has_error { 0.28 } else { 0.52 },
                                if has_error { 0.28 } else { 0.98 },
                                0.25,
                            ),
                            width: 1.0,
                        },
                        ..Default::default()
                    })
                    .padding([8, 12])
                    .max_width(300)
            )
            .width(Length::Fill)
            .into()
        }
    }

    // ── RAG items list ────────────────────────────────────────────────────────

    fn view_rag_items<'a>(lang: Language, cfg: &'a AssistantConfig) -> Element<'a, AppMessage> {
        if cfg.rag_items.is_empty() {
            let empty_text = match lang {
                Language::ZhCn | Language::ZhTw => "暂无文件/文件夹，点击上方按钮添加",
                _ => "No files or folders added yet. Use the buttons above to add.",
            };
            return widget::container(
                widget::text(empty_text)
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                    ))
            )
            .padding([8, 10])
            .width(Length::Fill)
            .style(|_: &cosmic::Theme| ContainerStyle {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(0.15, 0.15, 0.18, 0.5),
                )),
                border: cosmic::iced::Border {
                    radius: 6.0.into(),
                    color: cosmic::iced::Color::from_rgba(0.4, 0.4, 0.45, 0.3),
                    width: 1.0,
                },
                ..Default::default()
            })
            .into();
        }

        let lbl_remove = match lang {
            Language::ZhCn | Language::ZhTw => "移除",
            _ => "✕",
        };
        let lbl_ingest = match lang {
            Language::ZhCn | Language::ZhTw => "向量化",
            _ => "Index",
        };

        let mut rows: Vec<Element<'a, AppMessage>> = Vec::new();

        // Column header
        rows.push(
            widget::container(
                widget::row::with_children(vec![
                    widget::text(match lang {
                        Language::ZhCn | Language::ZhTw => "类型",
                        _ => "Type",
                    })
                    .size(10)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                    ))
                    .width(Length::Fixed(32.0))
                    .into(),
                    widget::text(match lang {
                        Language::ZhCn | Language::ZhTw => "名称",
                        _ => "Name",
                    })
                    .size(10)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                    ))
                    .width(Length::Fill)
                    .into(),
                    widget::text(match lang {
                        Language::ZhCn | Language::ZhTw => "状态",
                        _ => "Status",
                    })
                    .size(10)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                    ))
                    .width(Length::Fixed(130.0))
                    .into(),
                    widget::Space::new(Length::Fixed(52.0), 0).into(),
                ])
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([3, 8])
            )
            .width(Length::Fill)
            .into()
        );

        for (idx, item) in cfg.rag_items.iter().enumerate() {
            let icon = if item.is_folder { "📁" } else { "📄" };
            let status_color = match &item.status {
                RagItemStatus::Pending        => cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50),
                RagItemStatus::Indexing(_)    => cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12),
                RagItemStatus::Ready(_)       => cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46),
                RagItemStatus::Failed(_)      => cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
            };
            let status_str = item.status.to_string();
            let label = item.label.clone();

            let can_ingest = matches!(item.status, RagItemStatus::Pending | RagItemStatus::Failed(_));

            let row = widget::container(
                widget::row::with_children(vec![
                    widget::text(icon).size(13).width(Length::Fixed(32.0)).into(),
                    widget::text(label)
                        .size(12)
                        .width(Length::Fill)
                        .into(),
                    widget::text(status_str)
                        .size(11)
                        .class(cosmic::theme::Text::Color(status_color))
                        .width(Length::Fixed(130.0))
                        .into(),
                    widget::button::custom(
                        widget::text(lbl_ingest).size(11)
                    )
                    .padding([3, 7])
                    .class(cosmic::theme::Button::Standard)
                    .on_press_maybe(if can_ingest {
                        Some(AppMessage::AssistantRagIngest(idx))
                    } else {
                        None
                    })
                    .into(),
                    widget::button::custom(
                        widget::text(lbl_remove).size(11)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.75, 0.30, 0.30)
                            ))
                    )
                    .padding([3, 7])
                    .class(cosmic::theme::Button::Text)
                    .on_press(AppMessage::AssistantRagRemove(idx))
                    .into(),
                ])
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([5, 8])
            )
            .style(move |_: &cosmic::Theme| ContainerStyle {
                background: Some(cosmic::iced::Background::Color(
                    if idx % 2 == 0 {
                        cosmic::iced::Color::from_rgba(0.15, 0.15, 0.18, 0.4)
                    } else {
                        cosmic::iced::Color::from_rgba(0.12, 0.12, 0.15, 0.2)
                    }
                )),
                ..Default::default()
            })
            .width(Length::Fill);

            rows.push(row.into());
        }

        widget::container(
            widget::column::with_children(rows).spacing(1)
        )
        .style(|_: &cosmic::Theme| ContainerStyle {
            border: cosmic::iced::Border {
                radius: 6.0.into(),
                color: cosmic::iced::Color::from_rgba(0.4, 0.4, 0.45, 0.3),
                width: 1.0,
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    // ── Settings sub-panel ────────────────────────────────────────────────────

    fn view_settings<'a>(
        &'a self,
        lang: Language,
        cfg: &'a AssistantConfig,
        available_models: &'a [crate::app::OllamaModel],
    ) -> Element<'a, AppMessage> {
        let (lbl_endpoint, lbl_model, lbl_rag, lbl_temp, lbl_topk, lbl_save, lbl_hint) =
            match lang {
                Language::ZhCn | Language::ZhTw => (
                    "API 端点",
                    "选择模型",
                    "RAG 知识库路径",
                    "温度 (Temperature)",
                    "Top-K",
                    "保存配置",
                    "以下配置独立于系统 AI 设置，仅用于此 AI 助手面板。",
                ),
                _ => (
                    "API Endpoint",
                    "Select Model",
                    "RAG Knowledge Path",
                    "Temperature",
                    "Top-K",
                    "Save Config",
                    "These settings are independent from the main system AI config.",
                ),
            };

        let (lbl_download_docs, lbl_rag_section, lbl_add_file, lbl_add_folder) = match lang {
            Language::ZhCn | Language::ZhTw => (
                "⬇ 下载官方参考文档",
                "RAG 知识库",
                "添加文件",
                "添加文件夹",
            ),
            _ => (
                "⬇ Download Official Docs",
                "RAG Knowledge Base",
                "Add File",
                "Add Folder",
            ),
        };

        let col = widget::column::with_children(vec![
            container(
                widget::text(lbl_hint).size(12)
            )
            .style(|_: &cosmic::Theme| ContainerStyle {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(0.82, 0.52, 0.98, 0.08),
                )),
                border: cosmic::iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding([8, 10])
            .width(Length::Fill)
            .into(),
            widget::Space::new(0, 8).into(),
            // ── RAG section ───────────────────────────────────────────────────
            // Header with item count badge
            widget::row::with_children(vec![
                widget::text(lbl_rag_section)
                    .size(13)
                    .font(cosmic::font::bold())
                    .width(Length::Fill)
                    .into(),
                {
                    let total = cfg.rag_items.len();
                    let ready = cfg.rag_items.iter()
                        .filter(|i| matches!(i.status, RagItemStatus::Ready(_)))
                        .count();
                    let badge_text = if total == 0 {
                        match lang {
                            Language::ZhCn | Language::ZhTw => "空".to_string(),
                            _ => "empty".to_string(),
                        }
                    } else {
                        format!("{}/{}", ready, total)
                    };
                    let badge_color = if total == 0 {
                        cosmic::iced::Color::from_rgb(0.55, 0.53, 0.50)
                    } else if ready == total {
                        cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46)
                    } else {
                        cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12)
                    };
                    widget::container(
                        widget::text(badge_text).size(11)
                            .class(cosmic::theme::Text::Color(badge_color))
                    )
                    .padding([2, 7])
                    .class(cosmic::theme::Container::Card)
                    .into()
                },
            ])
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
            widget::Space::new(0, 4).into(),
            widget::text(lbl_rag).size(12).into(),
            widget::text_input("/path/to/knowledge", &cfg.rag_path)
                .on_input(AppMessage::AssistantCfgRagPathChanged)
                .width(Length::Fill)
                .into(),
            widget::Space::new(0, 4).into(),
            // Action buttons row
            widget::row::with_children(vec![
                widget::button::custom(
                    widget::text(lbl_add_file).size(13)
                )
                .padding([6, 12])
                .class(cosmic::theme::Button::Standard)
                .on_press(AppMessage::AssistantRagPickFile)
                .into(),
                widget::button::custom(
                    widget::text(lbl_add_folder).size(13)
                )
                .padding([6, 12])
                .class(cosmic::theme::Button::Standard)
                .on_press(AppMessage::AssistantRagPickFolder)
                .into(),
                widget::button::custom(
                    widget::text(lbl_download_docs).size(13)
                )
                .padding([6, 12])
                .class(cosmic::theme::Button::Suggested)
                .on_press(AppMessage::RagDownloadOfficialDocs)
                .into(),
            ])
            .spacing(6)
            .into(),
            widget::Space::new(0, 6).into(),
            // ── RAG items list ─────────────────────────────────────────────
            Self::view_rag_items(lang, cfg),
            widget::Space::new(0, 8).into(),
            widget::divider::horizontal::light().into(),
            widget::Space::new(0, 8).into(),
            widget::text(lbl_endpoint).size(12).into(),
            widget::text_input("http://localhost:11434", &cfg.endpoint)
                .on_input(AppMessage::AssistantCfgEndpointChanged)
                .width(Length::Fill)
                .into(),
            widget::Space::new(0, 4).into(),
            // ── Model selector ────────────────────────────────────────────
            {
                let lbl_refresh = match lang {
                    Language::ZhCn | Language::ZhTw => "刷新",
                    _ => "Refresh",
                };
                widget::row::with_children(vec![
                    widget::text(lbl_model).size(12).width(Length::Fill).into(),
                    widget::button::custom(
                        widget::text(lbl_refresh).size(11)
                    )
                    .padding([3, 8])
                    .class(cosmic::theme::Button::Text)
                    .on_press(AppMessage::AssistantFetchModels)
                    .into(),
                ])
                .align_y(Alignment::Center)
                .into()
            },
            {
                // Build merged model list: loaded models first, then static presets
                // not already in the list
                const PRESET_MODELS: &[&str] = &[
                    "qwen2:7b", "qwen2.5:7b", "qwen2.5:14b", "qwen2.5:32b",
                    "llama3.1:8b", "llama3.1:70b", "llama3.2:3b",
                    "mistral:7b", "deepseek-r1:7b", "deepseek-r1:14b",
                    "phi3:mini", "phi3:medium", "gemma2:9b", "gemma2:27b",
                    "codellama:7b", "codellama:13b",
                ];

                let mut model_names: Vec<String> = available_models
                    .iter()
                    .map(|m| m.name.clone())
                    .collect();
                for preset in PRESET_MODELS {
                    if !model_names.iter().any(|n| n == *preset) {
                        model_names.push(preset.to_string());
                    }
                }

                // Find currently selected index
                let selected_idx = model_names
                    .iter()
                    .position(|n| n == &cfg.model);

                // Build button list — consume model_names by value to avoid borrow issue
                let loaded_set: std::collections::HashSet<String> = available_models
                    .iter().map(|m| m.name.clone()).collect();
                let model_btns: Vec<Element<'a, AppMessage>> = model_names
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let is_selected = selected_idx == Some(i);
                        let is_loaded = loaded_set.contains(&name);
                        let indicator = if is_loaded { "● " } else { "○ " };
                        let color = if is_loaded {
                            cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46)
                        } else {
                            cosmic::iced::Color::from_rgb(0.45, 0.44, 0.43)
                        };
                        let display = name.clone();
                        widget::button::custom(
                            widget::row::with_children(vec![
                                widget::text(indicator).size(11)
                                    .class(cosmic::theme::Text::Color(color))
                                    .into(),
                                widget::text(display).size(12)
                                    .width(Length::Fill)
                                    .into(),
                            ])
                            .align_y(Alignment::Center)
                            .spacing(2)
                            .padding([4, 8])
                        )
                        .class(if is_selected {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::Text
                        })
                        .width(Length::Fill)
                        .on_press(AppMessage::AssistantCfgModelChanged(name))
                        .into()
                    })
                    .collect();

                widget::container(
                    scrollable(
                        widget::column::with_children(model_btns).spacing(1)
                    )
                    .height(Length::Fixed(160.0))
                )
                .style(|_: &cosmic::Theme| ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgba(0.10, 0.10, 0.13, 0.9),
                    )),
                    border: cosmic::iced::Border {
                        radius: 6.0.into(),
                        color: cosmic::iced::Color::from_rgba(0.4, 0.4, 0.48, 0.5),
                        width: 1.0,
                    },
                    ..Default::default()
                })
                .width(Length::Fill)
                .into()
            },
            widget::Space::new(0, 4).into(),
            widget::row::with_children(vec![
                widget::column::with_children(vec![
                    widget::text(lbl_temp).size(12).into(),
                    widget::text_input("0.7", &cfg.temperature_str)
                        .on_input(AppMessage::AssistantCfgTemperatureChanged)
                        .width(Length::Fill)
                        .into(),
                ])
                .spacing(4)
                .width(Length::Fill)
                .into(),
                widget::Space::new(8, 0).into(),
                widget::column::with_children(vec![
                    widget::text(lbl_topk).size(12).into(),
                    widget::text_input("40", &cfg.top_k_str)
                        .on_input(AppMessage::AssistantCfgTopKChanged)
                        .width(Length::Fill)
                        .into(),
                ])
                .spacing(4)
                .width(Length::Fill)
                .into(),
            ])
            .into(),
            widget::Space::new(0, 12).into(),
            widget::button::standard(lbl_save)
                .on_press(AppMessage::AssistantCfgSave)
                .class(cosmic::theme::Button::Suggested)
                .width(Length::Fill)
                .into(),
        ])
        .spacing(4)
        .padding(16);

        scrollable(col)
            .height(Length::Fill)
            .into()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cfg(paths: &[(&str, bool)]) -> AssistantConfig {
        let mut cfg = AssistantConfig::default();
        for (path, is_folder) in paths {
            cfg.rag_items.push(if *is_folder {
                AssistantRagItem::new_folder(path)
            } else {
                AssistantRagItem::new_file(path)
            });
        }
        cfg
    }

    // ── to_rag_config / dual-RAG isolation ───────────────────────────────────

    #[test]
    fn empty_assistant_rag_produces_empty_rag_config() {
        let rag = AssistantConfig::default().to_rag_config();
        assert!(rag.folders.is_empty());
        assert!(rag.files.is_empty());
    }

    #[test]
    fn file_item_maps_to_rag_file() {
        let cfg = make_cfg(&[("/tmp/notes.md", false)]);
        let rag = cfg.to_rag_config();
        assert_eq!(rag.files.len(), 1);
        assert!(rag.folders.is_empty());
        assert_eq!(rag.files[0].file_path.to_str().unwrap(), "/tmp/notes.md");
        assert_eq!(rag.files[0].name, "notes.md");
        assert_eq!(rag.files[0].content_type, "md");
        assert!(rag.files[0].enabled);
        assert!(!rag.files[0].file_path.to_str().unwrap().is_empty());
    }

    #[test]
    fn folder_item_maps_to_rag_folder() {
        let cfg = make_cfg(&[("/home/user/docs", true)]);
        let rag = cfg.to_rag_config();
        assert_eq!(rag.folders.len(), 1);
        assert!(rag.files.is_empty());
        assert_eq!(rag.folders[0].name, "docs");
        assert!(!rag.folders[0].allow_agent_write);
    }

    #[test]
    fn mixed_items_split_correctly() {
        let cfg = make_cfg(&[
            ("/tmp/f1.txt", false),
            ("/tmp/dir_a",  true),
            ("/tmp/f2.rs",  false),
            ("/tmp/dir_b",  true),
        ]);
        let rag = cfg.to_rag_config();
        assert_eq!(rag.files.len(), 2);
        assert_eq!(rag.folders.len(), 2);
    }

    #[test]
    fn assistant_rag_independent_from_main_system_rag() {
        let main_rag = openclaw_config::RagConfig::default();

        let mut asst = AssistantConfig::default();
        asst.rag_items.push(AssistantRagItem::new_file("/private/notes.txt"));
        asst.rag_items.push(AssistantRagItem::new_folder("/private/docs"));
        let asst_rag = asst.to_rag_config();

        // Main system RAG untouched
        assert!(main_rag.folders.is_empty());
        assert!(main_rag.files.is_empty());
        // Assistant RAG has its own items
        assert_eq!(asst_rag.files.len(), 1);
        assert_eq!(asst_rag.folders.len(), 1);
    }

    #[test]
    fn to_rag_config_is_pure() {
        let cfg = make_cfg(&[("/tmp/a.txt", false)]);
        let r1 = cfg.to_rag_config();
        let r2 = cfg.to_rag_config();
        assert_eq!(r1.files[0].file_path, r2.files[0].file_path);
    }

    // ── AssistantRagItem ──────────────────────────────────────────────────────

    #[test]
    fn new_file_label_is_filename() {
        let item = AssistantRagItem::new_file("/a/b/doc.pdf");
        assert_eq!(item.label, "doc.pdf");
        assert!(!item.is_folder);
        assert!(matches!(item.status, RagItemStatus::Pending));
    }

    #[test]
    fn new_folder_label_is_dirname() {
        let item = AssistantRagItem::new_folder("/a/b/knowledge_base");
        assert_eq!(item.label, "knowledge_base");
        assert!(item.is_folder);
        assert!(matches!(item.status, RagItemStatus::Pending));
    }

    #[test]
    fn duplicate_path_detection() {
        let cfg = make_cfg(&[("/tmp/a.txt", false), ("/tmp/b.txt", false)]);
        assert!(cfg.rag_items.iter().any(|i| i.path == "/tmp/a.txt"));
        assert!(!cfg.rag_items.iter().any(|i| i.path == "/tmp/c.txt"));
    }

    // ── Model selector merge logic ────────────────────────────────────────────

    #[test]
    fn preset_models_follow_name_tag_format() {
        const PRESETS: &[&str] = &[
            "qwen2:7b", "qwen2.5:7b", "llama3.1:8b", "mistral:7b",
            "deepseek-r1:7b", "phi3:mini", "gemma2:9b", "codellama:7b",
        ];
        for m in PRESETS {
            assert!(m.contains(':'), "'{m}' should be name:tag format");
        }
    }

    #[test]
    fn model_merge_no_duplicates() {
        let loaded = vec!["qwen2:7b".to_string(), "llama3.1:8b".to_string()];
        let presets = ["qwen2:7b", "mistral:7b", "llama3.1:8b", "phi3:mini"];
        let mut merged: Vec<String> = loaded.clone();
        for p in &presets {
            if !merged.iter().any(|n| n == *p) {
                merged.push(p.to_string());
            }
        }
        // No duplicates
        let before = merged.len();
        merged.sort();
        merged.dedup();
        assert_eq!(before, merged.len());
    }

    #[test]
    fn model_merge_loaded_first() {
        let loaded = vec!["qwen2:7b".to_string(), "llama3.1:8b".to_string()];
        let presets = ["qwen2:7b", "mistral:7b"];
        let mut merged: Vec<String> = loaded.clone();
        for p in &presets {
            if !merged.iter().any(|n| n == *p) {
                merged.push(p.to_string());
            }
        }
        assert_eq!(merged[0], "qwen2:7b");
        assert_eq!(merged[1], "llama3.1:8b");
        assert!(merged.contains(&"mistral:7b".to_string()));
    }

    #[test]
    fn selected_model_index() {
        let names = vec!["qwen2:7b".to_string(), "llama3.1:8b".to_string(), "mistral:7b".to_string()];
        assert_eq!(names.iter().position(|n| n == "llama3.1:8b"), Some(1));
        assert_eq!(names.iter().position(|n| n == "nonexistent:1b"), None);
    }

    // ── AssistantConfig validation ────────────────────────────────────────────

    #[test]
    fn default_config_valid() {
        assert!(AssistantConfig::default().validate().is_ok());
    }

    #[test]
    fn empty_endpoint_invalid() {
        let mut cfg = AssistantConfig::default();
        cfg.endpoint = String::new();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn empty_model_invalid() {
        let mut cfg = AssistantConfig::default();
        cfg.model = String::new();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn temperature_out_of_range_invalid() {
        let mut cfg = AssistantConfig::default();
        cfg.temperature_str = "3.5".to_string();
        assert!(cfg.validate().is_err());
        cfg.temperature_str = "-0.1".to_string();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn temperature_boundary_values_valid() {
        for v in &["0.0", "0.7", "1.0", "2.0"] {
            let mut cfg = AssistantConfig::default();
            cfg.temperature_str = v.to_string();
            assert!(cfg.validate().is_ok(), "temperature {v} should be valid");
        }
    }

    #[test]
    fn top_k_boundary_values() {
        let mut cfg = AssistantConfig::default();
        cfg.top_k_str = "0".to_string();
        assert!(cfg.validate().is_err());
        cfg.top_k_str = "1".to_string();
        assert!(cfg.validate().is_ok());
        cfg.top_k_str = "1000".to_string();
        assert!(cfg.validate().is_ok());
        cfg.top_k_str = "1001".to_string();
        assert!(cfg.validate().is_err());
    }
}
