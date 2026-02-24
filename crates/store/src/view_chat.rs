//! # `view_chat.rs` — Local AI Chat Conversation Page
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Renders the "AI Chat" page, which provides a direct conversation interface
//! between the user and a locally-running AI model (Ollama, LM Studio,
//! llama.cpp, or any OpenAI-compatible endpoint configured on the AI Models
//! page).
//!
//! ## Layout
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │ Model selector chips   [System Prompt]  [Clear]         │  ← top bar
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ [optional system prompt editor]                        │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ scrollable message history (bubbles)                   │  ← fills
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ [text input field]                         [↑ Send]     │  ← bottom
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Warm-tone notes
//! - User message bubbles are right-aligned; assistant bubbles are left-aligned.
//! - Role labels use amber (`ROLE_AMBER`) for the user and gold (`ROLE_GOLD`)
//!   for the assistant to create a warm, readable contrast.
//! - Error text is rendered in warm red (`ERROR_RED`).
//! - The "thinking" indicator uses amber to signal activity.

use crate::app::StoreMessage;
use crate::i18n;
use crate::types::{AiModelConfig, ChatMessage, ChatRole, ChatState};
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget;
use cosmic::Element;

// ── Warm-tone palette ───────────────────────────────────────────────────────────────
// libcosmic containers do not support arbitrary fill colours, so warm tones
// are applied exclusively via `widget::text(...).color(c)` on label nodes.

/// Amber used for the user role label and "thinking" indicator.
const ROLE_AMBER: Color = Color { r: 0.85, g: 0.52, b: 0.08, a: 1.0 };

/// Muted gold used for the assistant role label.
const ROLE_GOLD: Color  = Color { r: 0.72, g: 0.58, b: 0.12, a: 1.0 };

/// Warm red used for inline error messages.
const ERROR_RED: Color  = Color { r: 0.80, g: 0.22, b: 0.18, a: 1.0 };

pub fn view_chat(
    messages: &[ChatMessage],
    input: &str,
    chat_state: &ChatState,
    ai_profiles: &[AiModelConfig],
    active_profile_idx: usize,
    system_prompt: &str,
    show_system_prompt: bool,
) -> Element<'static, StoreMessage> {
    // ── Top bar: model selector + controls ────────────────────────────────────
    let s = i18n::strings();
    let active_name = ai_profiles
        .get(active_profile_idx)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| s.ai_no_model.to_string());

    let model_label = widget::text(format!("\u{1f916}  {active_name}")).size(14);

    let mut model_row = widget::row()
        .push(model_label)
        .push(widget::horizontal_space())
        .spacing(8)
        .align_y(Alignment::Center);

    // Profile switcher chips
    for (i, profile) in ai_profiles.iter().enumerate() {
        let name = profile.name.clone();
        let is_active = i == active_profile_idx;
        model_row = model_row.push(if is_active {
            widget::button::suggested(name)
                .on_press(StoreMessage::ChatSelectProfile(i))
        } else {
            widget::button::text(name)
                .on_press(StoreMessage::ChatSelectProfile(i))
        });
    }

    model_row = model_row.push(
        widget::button::text(if show_system_prompt {
            s.chat_hide_prompt
        } else {
            s.chat_show_prompt
        })
        .on_press(StoreMessage::ChatToggleSystemPrompt),
    );
    model_row = model_row.push(
        widget::button::destructive(s.chat_clear)
            .on_press(StoreMessage::ChatClear),
    );

    let top_bar = widget::container(model_row)
        .padding([8, 16])
        .width(Length::Fill);

    // ── Optional system prompt editor ─────────────────────────────────────────
    let system_section: Option<Element<'static, StoreMessage>> = if show_system_prompt {
        let sp = system_prompt.to_string();
        Some(
            widget::container(
                widget::column()
                    .push(widget::text(s.chat_system_prompt).size(11))
                    .push(
                        widget::text_input(
                            "You are a helpful assistant…",
                            sp,
                        )
                        .on_input(StoreMessage::ChatSystemPromptChanged)
                        .width(Length::Fill),
                    )
                    .spacing(4),
            )
            .padding([6, 16])
            .width(Length::Fill)
            .into(),
        )
    } else {
        None
    };

    // ── Message history ───────────────────────────────────────────────────────
    let mut msg_col = widget::column().spacing(10).padding([10, 16]);

    if messages.is_empty() {
        msg_col = msg_col.push(
            widget::container(
                widget::column()
                    .push(widget::text(s.chat_welcome).size(18))
                    .push(widget::text(s.chat_welcome_hint).size(13))
                    .push(widget::text(s.chat_welcome_tip).size(12))
                    .spacing(8)
                    .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .padding([60, 20]),
        );
    } else {
        for msg in messages {
            msg_col = msg_col.push(message_bubble(msg));
        }
    }

    // Streaming indicator
    if let ChatState::Thinking = chat_state {
        msg_col = msg_col.push(
            widget::container(
                widget::row()
                    .push(widget::text("🤖").size(16))
                    .push(widget::text(s.chat_thinking).size(13).class(cosmic::theme::Text::Color(ROLE_AMBER)))
                    .spacing(4)
                    .align_y(Alignment::Center),
            )
            .padding([8, 14])
            .width(Length::Fixed(180.0)),
        );
    }
    if let ChatState::Streaming(partial) = chat_state {
        msg_col = msg_col.push(
            widget::container(
                widget::column()
                    .push(
                        widget::row()
                            .push(widget::text(s.chat_role_asst).size(11))
                            .push(widget::text("  ●●●").size(10))
                            .spacing(4),
                    )
                    .push(widget::text(partial.clone()).size(14))
                    .spacing(4),
            )
            .padding([10, 14])
            .width(Length::Fill),
        );
    }
    if let ChatState::Error(e) = chat_state {
        msg_col = msg_col.push(
            widget::container(
                widget::row()
                    .push(widget::text(s.chat_error_prefix).size(13).class(cosmic::theme::Text::Color(ERROR_RED)))
                    .push(widget::text(e.clone()).size(13).class(cosmic::theme::Text::Color(ERROR_RED)))
                    .spacing(4),
            )
            .padding([8, 14])
            .width(Length::Fill),
        );
    }

    let history = widget::scrollable(msg_col).height(Length::Fill);

    // ── Input area ────────────────────────────────────────────────────────────
    let is_busy = matches!(chat_state, ChatState::Thinking | ChatState::Streaming(_));
    let input_val = input.to_string();

    let text_input = widget::text_input(
        s.chat_input_hint,
        input_val,
    )
    .on_input(StoreMessage::ChatInputChanged)
    .on_submit(|_| StoreMessage::ChatSend)
    .width(Length::Fill);

    let send_btn: Element<'static, StoreMessage> = if is_busy {
        widget::button::text(s.chat_thinking).into()
    } else {
        widget::button::suggested(s.chat_send)
            .on_press(StoreMessage::ChatSend)
            .into()
    };

    let input_row: Element<'static, StoreMessage> = widget::container(
        widget::row()
            .push(text_input)
            .push(send_btn)
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .padding([10, 16])
    .width(Length::Fill)
    .into();

    // ── Assemble ──────────────────────────────────────────────────────────────
    let mut page = widget::column()
        .push(top_bar)
        .push(widget::divider::horizontal::default());

    if let Some(sys) = system_section {
        page = page
            .push(sys)
            .push(widget::divider::horizontal::default());
    }

    page = page
        .push(history)
        .push(widget::divider::horizontal::default())
        .push(input_row);

    page.into()
}

// ── Message bubble ────────────────────────────────────────────────────────────

fn message_bubble(msg: &ChatMessage) -> Element<'static, StoreMessage> {
    let is_user = msg.role == ChatRole::User;
    let content = msg.content.clone();

    // Role label colour: amber for user, gold for assistant, default for system.
    let s = i18n::strings();
    let role_label_widget: Element<'static, StoreMessage> = match msg.role {
        ChatRole::User =>
            widget::text(s.chat_role_user).size(11).class(cosmic::theme::Text::Color(ROLE_AMBER)).into(),
        ChatRole::Assistant =>
            widget::text(s.chat_role_asst).size(11).class(cosmic::theme::Text::Color(ROLE_GOLD)).into(),
        ChatRole::System =>
            widget::text(s.chat_role_system).size(11).into(),
    };

    let bubble_content = widget::column()
        .push(role_label_widget)
        .push(widget::text(content).size(14))
        .spacing(4)
        .width(Length::Fill);

    let bubble = widget::container(bubble_content).padding([10, 14]).width(
        if is_user {
            Length::FillPortion(4)
        } else {
            Length::Fill
        },
    );

    if is_user {
        widget::row()
            .push(widget::horizontal_space())
            .push(bubble)
            .into()
    } else {
        widget::row().push(bubble).into()
    }
}
