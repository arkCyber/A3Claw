#![allow(dead_code)]
use crate::app::AppMessage;
use crate::theme;
use cosmic::widget;
use cosmic::Element;

/// Status badge widget showing Allow / Deny / Pending state.
pub fn status_badge(allowed: Option<bool>) -> Element<'static, AppMessage> {
    let (text, color) = match allowed {
        Some(true)  => ("✅ Allow",   theme::COLOR_ALLOW),
        Some(false) => ("🚫 Deny",    theme::COLOR_DENY),
        None        => ("⏳ Pending", theme::COLOR_PENDING),
    };

    widget::text(text)
        .size(12)
        .class(cosmic::theme::Text::Color(color))
        .into()
}
