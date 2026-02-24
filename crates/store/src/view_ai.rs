//! # `view_ai.rs` — AI Model Configuration Page
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Renders the "AI Models" page, which lets the user create and manage
//! local AI inference profiles.  Each profile stores:
//!
//! - The backend type (Ollama, LM Studio, llama.cpp, or Custom).
//! - The endpoint URL and optional API key.
//! - Generation parameters: model name, max tokens, temperature.
//! - A live connection status tested via the "Test Connection" button.
//!
//! ## Warm-tone notes
//! Connection status text is colour-coded:
//! - Online  → warm green (`STATUS_ONLINE`).
//! - Offline → warm red (`STATUS_OFFLINE`).
//! - Checking → amber (`STATUS_CHECKING`).
//! - Unknown → default theme colour.

use crate::app::StoreMessage;
use crate::i18n;
use crate::types::{AiBackend, AiModelConfig, AiModelStatus};
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget;
use cosmic::Element;

// ── Warm-tone palette ───────────────────────────────────────────────────────────────

/// Warm green for "Online" connection status.
const STATUS_ONLINE: Color   = Color { r: 0.20, g: 0.62, b: 0.28, a: 1.0 };
/// Warm red for "Offline" / error status.
const STATUS_OFFLINE: Color  = Color { r: 0.80, g: 0.22, b: 0.18, a: 1.0 };
/// Amber for "Checking" / in-progress status.
const STATUS_CHECKING: Color = Color { r: 0.85, g: 0.52, b: 0.08, a: 1.0 };

pub fn view_ai_models(profiles: &[AiModelConfig]) -> Element<'static, StoreMessage> {
    let s = i18n::strings();
    let mut col = widget::column()
        .push(
            widget::row()
                .push(
                    widget::column()
                        .push(widget::text(s.ai_title).size(16))
                        .push(widget::text(s.ai_subtitle).size(12))
                        .spacing(2),
                )
                .push(widget::horizontal_space())
                .push(
                    widget::button::suggested(s.ai_add_profile)
                        .on_press(StoreMessage::AiAddProfile),
                )
                .align_y(Alignment::Center),
        )
        .push(widget::divider::horizontal::default())
        .spacing(14)
        .padding([16, 16]);

    for (i, profile) in profiles.iter().enumerate() {
        col = col.push(ai_profile_card(i, profile));
    }

    widget::scrollable(col).height(Length::Fill).into()
}

pub fn ai_profile_card(i: usize, p: &AiModelConfig) -> Element<'static, StoreMessage> {
    let endpoint  = p.endpoint.clone();
    let model     = p.model.clone();
    let api_key   = p.api_key.clone();
    let max_tok   = p.max_tokens.to_string();
    let temp      = format!("{:.1}", p.temperature);
    let is_active = p.is_active;
    let backend   = p.backend.clone();
    let name      = p.name.clone();

    // Status string and colour — warm tones give immediate visual feedback.
    let s = i18n::strings();
    let (status_str, status_color): (String, Option<Color>) = match &p.status {
        AiModelStatus::Unknown =>
            (s.ai_status_unknown.to_string(), None),
        AiModelStatus::Checking =>
            (s.ai_status_checking.to_string(), Some(STATUS_CHECKING)),
        AiModelStatus::Online { model_count } =>
            (format!("\u{2705} Online  \u{b7}  {model_count} model(s)"), Some(STATUS_ONLINE)),
        AiModelStatus::Offline(e) =>
            (format!("\u{274c} {e}"), Some(STATUS_OFFLINE)),
    };
    let status_widget: Element<'static, StoreMessage> = if let Some(c) = status_color {
        widget::text(status_str).size(11).class(cosmic::theme::Text::Color(c)).into()
    } else {
        widget::text(status_str).size(11).into()
    };

    // Backend selector
    let backends: &[(&str, AiBackend)] = &[
        ("Ollama",    AiBackend::Ollama),
        ("LM Studio", AiBackend::LmStudio),
        ("llama.cpp", AiBackend::LlamaCpp),
        ("Custom",    AiBackend::Custom),
    ];
    let mut backend_row = widget::row().spacing(4);
    for (lbl, b) in backends {
        let active = backend == *b;
        let b2 = b.clone();
        let lbl = *lbl;
        backend_row = backend_row.push(if active {
            widget::button::suggested(lbl)
                .on_press(StoreMessage::AiBackendChanged(i, b2))
        } else {
            widget::button::text(lbl)
                .on_press(StoreMessage::AiBackendChanged(i, b2))
        });
    }

    let active_lbl = if is_active { s.ai_active } else { s.ai_set_active };

    widget::container(
        widget::column()
            .push(
                widget::row()
                    .push(widget::text(name).size(15))
                    .push(widget::horizontal_space())
                    .push(status_widget)
                    .align_y(Alignment::Center),
            )
            .push(backend_row)
            .push(widget::divider::horizontal::default())
            .push(widget::text(s.ai_endpoint).size(11))
            .push(
                widget::text_input("http://localhost:11434", endpoint)
                    .on_input(move |v| StoreMessage::AiEndpointChanged(i, v))
                    .width(Length::Fill),
            )
            .push(widget::text(s.ai_model_name).size(11))
            .push(
                widget::text_input("llama3", model)
                    .on_input(move |v| StoreMessage::AiModelNameChanged(i, v))
                    .width(Length::Fill),
            )
            .push(widget::text(s.ai_api_key).size(11))
            .push(
                widget::text_input("sk-…", api_key)
                    .on_input(move |v| StoreMessage::AiApiKeyChanged(i, v))
                    .width(Length::Fill),
            )
            .push(
                widget::row()
                    .push(
                        widget::column()
                            .push(widget::text(s.ai_max_tokens).size(11))
                            .push(
                                widget::text_input("2048", max_tok)
                                    .on_input(move |v| StoreMessage::AiMaxTokensChanged(i, v))
                                    .width(Length::Fixed(90.0)),
                            )
                            .spacing(3),
                    )
                    .push(
                        widget::column()
                            .push(widget::text(s.ai_temperature).size(11))
                            .push(
                                widget::text_input("0.7", temp)
                                    .on_input(move |v| StoreMessage::AiTemperatureChanged(i, v))
                                    .width(Length::Fixed(70.0)),
                            )
                            .spacing(3),
                    )
                    .push(widget::horizontal_space())
                    .push(
                        widget::button::text(s.ai_test_conn)
                            .on_press(StoreMessage::AiTestConnection(i)),
                    )
                    .push(if is_active {
                        widget::button::suggested(active_lbl)
                            .on_press(StoreMessage::AiSetActive(i))
                    } else {
                        widget::button::text(active_lbl)
                            .on_press(StoreMessage::AiSetActive(i))
                    })
                    .push(
                        widget::button::destructive(s.ai_remove)
                            .on_press(StoreMessage::AiRemoveProfile(i)),
                    )
                    .spacing(8)
                    .align_y(Alignment::End),
            )
            .spacing(6)
            .padding(14),
    )
    .width(Length::Fill)
    .into()
}
