#![allow(dead_code)]
use crate::app::AppMessage;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use openclaw_security::SandboxEvent;

/// Inline confirmation dialog component embedded in the Dashboard.
///
/// Displayed when a sandbox operation has been suspended and is waiting
/// for the operator to explicitly allow or deny it.
pub struct ConfirmPage;

impl ConfirmPage {
    /// Renders a confirmation card for the given pending [`SandboxEvent`].
    ///
    /// Pressing **Allow** dispatches [`AppMessage::ConfirmAllow`] with the event ID.
    /// Pressing **Deny** dispatches [`AppMessage::ConfirmDeny`] with the event ID.
    pub fn dialog<'a>(event: &'a SandboxEvent) -> Element<'a, AppMessage> {
        let id = event.id;

        widget::container(
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::text("⚠️").size(20).into(),
                    widget::text("Action requires your approval")
                        .size(16)
                        .font(cosmic::font::bold())
                        .into(),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
                widget::divider::horizontal::default().into(),
                widget::text(format!("Operation : {}", event.kind))
                    .size(14)
                    .into(),
                widget::text(format!("Detail    : {}", event.detail))
                    .size(13)
                    .into(),
                if let Some(path) = &event.path {
                    widget::text(format!("Target    : {}", path))
                        .size(13)
                        .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.3, 0.6, 0.9)))
                        .into()
                } else {
                    widget::Space::new(0, 0).into()
                },
                widget::divider::horizontal::default().into(),
                widget::row::with_children(vec![
                    widget::button::text("✅ Allow")
                        .on_press(AppMessage::ConfirmAllow(id))
                        .class(cosmic::theme::Button::Suggested)
                        .into(),
                    widget::button::text("🚫 Deny")
                        .on_press(AppMessage::ConfirmDeny(id))
                        .class(cosmic::theme::Button::Destructive)
                        .into(),
                ])
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
            ])
            .spacing(10)
            .padding(20),
        )
        .class(cosmic::theme::Container::Card)
        .width(Length::Fill)
        .into()
    }
}
