//! AI Chat page — conversational interface to the local inference engine.
//!
//! # Architecture
//! - `AiChatPage` is a pure view struct; all mutable state lives in `OpenClawApp`.
//! - `AiChatState` holds the conversation history, input buffer, and engine status.
//! - Inference runs in a detached `tokio::spawn` task; the result is sent back
//!   via `AppMessage::AiResponse` / `AppMessage::AiError`.
//! - The engine is initialised lazily on first use and cached in `AiChatState`.

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::AppMessage;
use crate::theme::{Language, t};

/// Stable ID for the AI chat input box — used to re-focus after sending.
pub static AI_INPUT_ID: std::sync::LazyLock<cosmic::widget::Id> =
    std::sync::LazyLock::new(cosmic::widget::Id::unique);

// ── Chat message model ────────────────────────────────────────────────────────

/// The role of a single chat turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for ChatRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatRole::User      => write!(f, "You"),
            ChatRole::Assistant => write!(f, "AI"),
            ChatRole::System    => write!(f, "System"),
        }
    }
}

/// A single message in the chat history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Wall-clock timestamp (seconds since UNIX epoch).
    pub timestamp: u64,
    /// Inference latency in ms (only set for assistant messages).
    pub latency_ms: Option<u64>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            timestamp: unix_now(),
            latency_ms: None,
        }
    }

    pub fn assistant(content: impl Into<String>, latency_ms: u64) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            timestamp: unix_now(),
            latency_ms: Some(latency_ms),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            timestamp: unix_now(),
            latency_ms: None,
        }
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── AI Chat state ─────────────────────────────────────────────────────────────

/// Status of the inference engine connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    /// Not yet initialised.
    Idle,
    /// Actively generating a response.
    Thinking,
    /// Last request completed successfully.
    Ready,
    /// Last request failed; inner string is the error message.
    Error(String),
}

impl std::fmt::Display for EngineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineStatus::Idle         => write!(f, "Idle"),
            EngineStatus::Thinking     => write!(f, "Thinking…"),
            EngineStatus::Ready        => write!(f, "Ready"),
            EngineStatus::Error(e)     => write!(f, "Error: {e}"),
        }
    }
}

/// All mutable state for the AI Chat page, owned by `OpenClawApp`.
#[derive(Debug)]
pub struct AiChatState {
    /// Full conversation history (displayed in order).
    pub messages: Vec<ChatMessage>,
    /// Current text in the input box.
    pub input: String,
    /// Current status of the inference engine.
    pub status: EngineStatus,
    /// Model name currently in use.
    pub model_name: String,
    /// Ollama endpoint.
    pub endpoint: String,
    /// Total tokens generated this session (approximate).
    pub total_tokens: u64,
    /// Total inference calls this session.
    pub total_calls: u64,
}

impl Default for AiChatState {
    fn default() -> Self {
        Self {
            messages: vec![
                ChatMessage::system(
                    "AI assistant connected. Using Qwen2.5:0.5b via Ollama. \
                     Type a message and press Enter or click Send."
                ),
            ],
            input: String::new(),
            status: EngineStatus::Idle,
            model_name: "qwen2.5:0.5b".into(),
            endpoint: "http://localhost:11434".into(),
            total_tokens: 0,
            total_calls: 0,
        }
    }
}

impl AiChatState {
    /// Push a user message and return a clone of the full history for inference.
    pub fn push_user_message(&mut self, content: String) -> Vec<ChatMessage> {
        self.messages.push(ChatMessage::user(content));
        self.status = EngineStatus::Thinking;
        self.messages.clone()
    }

    /// Record a successful assistant response.
    pub fn push_assistant_response(&mut self, content: String, latency_ms: u64) {
        self.total_calls += 1;
        // Rough token estimate: 1 token ≈ 4 chars; empty content contributes 0 tokens.
        self.total_tokens += (content.len() / 4) as u64;
        self.messages.push(ChatMessage::assistant(content, latency_ms));
        self.status = EngineStatus::Ready;
    }

    /// Record an inference error.
    pub fn push_error(&mut self, error: String) {
        self.messages.push(ChatMessage::system(format!("⚠ Error: {error}")));
        self.status = EngineStatus::Error(error);
    }

    /// Clear conversation history (keeps system greeting).
    pub fn clear(&mut self) {
        self.messages.clear();
        self.messages.push(ChatMessage::system(
            "Conversation cleared. Ready for a new session."
        ));
        self.status = EngineStatus::Idle;
        self.total_tokens = 0;
        self.total_calls = 0;
    }
}

// ── View ──────────────────────────────────────────────────────────────────────

/// Stateless view renderer for the AI Chat page.
pub struct AiChatPage;

/// Preset model options shown in the model selector.
const MODEL_PRESETS: &[&str] = &[
    "qwen2.5:0.5b",
    "qwen2.5:1.5b",
    "qwen2.5:3b",
    "llama3.2:1b",
    "llama3.2:3b",
    "phi3.5:mini",
    "gemma2:2b",
];

impl AiChatPage {
    pub fn view<'a>(state: &'a AiChatState, lang: Language) -> Element<'a, AppMessage> {
        // ── Status dot ────────────────────────────────────────────────
        let status_color = match &state.status {
            EngineStatus::Ready | EngineStatus::Idle => cosmic::iced::Color::from_rgb(0.2, 0.8, 0.4),
            EngineStatus::Thinking => cosmic::iced::Color::from_rgb(0.9, 0.7, 0.1),
            EngineStatus::Error(_) => cosmic::iced::Color::from_rgb(0.9, 0.2, 0.2),
        };
        let status_dot = widget::container(widget::Space::new(8, 8))
            .style(move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(status_color)),
                border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            });

        // ── Model selector chips ──────────────────────────────────────
        let model_chips: Vec<Element<AppMessage>> = MODEL_PRESETS
            .iter()
            .map(|&m| {
                widget::button::text(m)
                    .on_press(AppMessage::AiModelChanged(m.to_string()))
                    .class(if state.model_name == m {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
                    .into()
            })
            .collect();

        let model_row = widget::row::with_children(vec![
            widget::icon::from_name("applications-science-symbolic").size(14).into(),
            widget::text(t(lang, "Model:", "模型:")).size(12).into(),
        ])
        .spacing(4)
        .align_y(Alignment::Center);

        let model_selector = widget::row::with_children(
            std::iter::once(model_row.into())
                .chain(model_chips)
                .collect::<Vec<_>>(),
        )
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([4, 16]);

        // ── Header bar ────────────────────────────────────────────────
        let header = widget::row::with_children(vec![
            widget::icon::from_name("applications-science-symbolic").size(18).into(),
            widget::text(t(lang, "AI Assistant", "AI 助手"))
                .size(18)
                .font(cosmic::font::bold())
                .width(Length::Fill)
                .into(),
            status_dot.into(),
            widget::text(state.status.to_string())
                .size(13)
                .class(cosmic::theme::Text::Color(status_color))
                .into(),
            widget::text(format!("│ {} {}", state.total_calls, t(lang, "calls", "次")))
                .size(12)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48)))
                .into(),
            widget::text(format!("│ ~{} tokens", state.total_tokens))
                .size(12)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48)))
                .into(),
            widget::button::text(t(lang, "Clear", "清除"))
                .on_press(AppMessage::AiClearChat)
                .class(cosmic::theme::Button::Text)
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([10, 16]);

        // ── Message list ──────────────────────────────────────────────
        let message_items: Vec<Element<AppMessage>> = state
            .messages
            .iter()
            .map(|msg| message_bubble(msg, lang))
            .collect();

        let message_list = widget::scrollable(
            widget::column::with_children(message_items)
                .spacing(8)
                .padding([8, 16]),
        )
        .height(Length::Fill);

        // ── Thinking indicator ────────────────────────────────────────
        let thinking_row: Option<Element<AppMessage>> =
            if state.status == EngineStatus::Thinking {
                Some(
                    widget::container(
                        widget::row::with_children(vec![
                            widget::icon::from_name("applications-science-symbolic").size(16).into(),
                            widget::text(t(lang, "Thinking…", "思考中…"))
                                .size(13)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.9, 0.7, 0.1),
                                ))
                                .into(),
                        ])
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .padding([6, 16]),
                    )
                    .into(),
                )
            } else {
                None
            };

        // ── Input row ─────────────────────────────────────────────────
        let send_enabled = !state.input.trim().is_empty()
            && state.status != EngineStatus::Thinking;

        let send_btn = if send_enabled {
            widget::button::suggested(t(lang, "Send", "发送"))
                .on_press(AppMessage::AiSendMessage)
        } else {
            widget::button::suggested(t(lang, "Send", "发送"))
                .class(cosmic::theme::Button::Standard)
        };

        let input_row = widget::row::with_children(vec![
            widget::text_input(
                t(lang, "Ask the AI assistant…", "向 AI 助手提问…"),
                &state.input,
            )
            .id(AI_INPUT_ID.clone())
            .on_input(AppMessage::AiInputChanged)
            .on_submit(|_| AppMessage::AiSendMessage)
            .on_focus(AppMessage::AiFocused)
            .width(Length::Fill)
            .into(),
            send_btn.into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([8, 16]);

        // ── Assemble ──────────────────────────────────────────────────
        let mut children: Vec<Element<AppMessage>> = vec![
            header.into(),
            widget::divider::horizontal::default().into(),
            model_selector.into(),
            widget::divider::horizontal::default().into(),
            message_list.into(),
        ];

        if let Some(thinking) = thinking_row {
            children.push(widget::divider::horizontal::default().into());
            children.push(thinking);
        }

        children.push(widget::divider::horizontal::default().into());
        children.push(input_row.into());

        widget::column::with_children(children).into()
    }
}

// ── Message bubble ────────────────────────────────────────────────────────────

fn message_bubble(msg: &ChatMessage, lang: Language) -> Element<AppMessage> {
    let (icon_name, role_label, name_color, bg_alpha) = match msg.role {
        ChatRole::User => (
            "avatar-default-symbolic",
            t(lang, "You", "你"),
            cosmic::iced::Color::from_rgb(0.4, 0.7, 1.0),
            0.06f32,
        ),
        ChatRole::Assistant => (
            "applications-science-symbolic",
            t(lang, "AI", "AI"),
            cosmic::iced::Color::from_rgb(0.4, 0.9, 0.6),
            0.08f32,
        ),
        ChatRole::System => (
            "dialog-information-symbolic",
            t(lang, "System", "系统"),
            cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6),
            0.04f32,
        ),
    };

    let mut header_items = vec![
        widget::icon::from_name(icon_name).size(14).into(),
        widget::text(role_label)
            .size(13)
            .font(cosmic::font::bold())
            .class(cosmic::theme::Text::Color(name_color))
            .into(),
    ];

    if let Some(latency) = msg.latency_ms {
        header_items.push(
            widget::text(format!("({latency}ms)"))
                .size(11)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)))
                .into(),
        );
    }

    let bubble_bg = cosmic::iced::Color::from_rgba(bg_alpha, bg_alpha, bg_alpha, 1.0);

    widget::container(
        widget::column::with_children(vec![
            widget::row::with_children(header_items)
                .spacing(6)
                .align_y(Alignment::Center)
                .into(),
            widget::text(&msg.content).size(14).into(),
        ])
        .spacing(4),
    )
    .style(move |_theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(bubble_bg)),
        border: cosmic::iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .padding(12)
    .width(Length::Fill)
    .into()
}
