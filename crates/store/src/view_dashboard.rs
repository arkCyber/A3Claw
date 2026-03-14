//! # `view_dashboard.rs` — Dashboard Page
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Renders the top-level Dashboard page, which gives the user an at-a-glance
//! view of the OpenClaw+ security shell status:
//!
//! - **Process status** — is the OpenClaw agent running on this machine?
//!   Detected by scanning the process list for a binary named `openclaw`.
//! - **Sandbox metrics** — syscalls intercepted, policy violations blocked,
//!   plugins loaded, and sandbox memory usage.
//! - **Security audit log** — a scrollable, colour-coded list of recent
//!   interception events emitted by the `crates/security` policy engine.
//!
//! ## Layout
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │  [● Running  PID 12345]   [↻ Refresh]                               │ ← status bar
//! ├──────────────┬───────────────────────────────────────────────────────┤
//! │  Metrics     │  Security Audit Log                                   │
//! │  ─────────── │  ─────────────────────────────────────────────────── │
//! │  Intercepted │  HH:MM:SS  [INFO]    description …                   │
//! │  Blocked     │  HH:MM:SS  [WARN]    description …                   │
//! │  Plugins     │  HH:MM:SS  [BLOCK]   description …                   │
//! │  Memory      │                                                       │
//! └──────────────┴───────────────────────────────────────────────────────┘
//! ```
//!
//! ## Warm-tone colour palette
//! | Constant        | Colour     | Used for                        |
//! |---|---|---|
//! | `AMBER`         | #D98514    | Warnings, metric values         |
//! | `GREEN`         | #33A047    | Running status, Info events     |
//! | `RED`           | #CC3830    | Stopped status, Blocked events  |
//! | `GOLD`          | #B89420    | Section headings                |

use crate::app::StoreMessage;
use crate::i18n;
use crate::types::{AuditSeverity, DashboardState, ProcessStatus};
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget;
use cosmic::Element;

// ── Warm-tone palette ─────────────────────────────────────────────────────────

/// Amber — warnings and metric values.
const AMBER: Color = Color { r: 0.85, g: 0.52, b: 0.08, a: 1.0 };
/// Warm green — running / healthy status.
const GREEN: Color = Color { r: 0.20, g: 0.63, b: 0.28, a: 1.0 };
/// Warm red — stopped / blocked status.
const RED: Color   = Color { r: 0.80, g: 0.22, b: 0.18, a: 1.0 };
/// Muted gold — section headings.
const GOLD: Color  = Color { r: 0.72, g: 0.58, b: 0.12, a: 1.0 };

// ── Public entry point ────────────────────────────────────────────────────────

/// Render the full Dashboard page.
pub fn view_dashboard(state: &DashboardState) -> Element<'static, StoreMessage> {
    let top_bar  = render_status_bar(state);
    let metrics  = render_metrics(state);
    let audit    = render_audit_log(state);

    let body = widget::row()
        .push(
            widget::container(metrics)
                .width(Length::Fixed(220.0))
                .height(Length::Fill)
                .padding([12, 12]),
        )
        .push(widget::divider::vertical::default())
        .push(
            widget::container(audit)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([12, 12]),
        );

    widget::column()
        .push(top_bar)
        .push(widget::divider::horizontal::default())
        .push(body)
        .into()
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn render_status_bar(state: &DashboardState) -> Element<'static, StoreMessage> {
    let s = i18n::strings();
    // Process status chip
    let (status_text, status_color): (&str, Color) = match &state.process_status {
        ProcessStatus::Unknown      => (s.dash_checking,     AMBER),
        ProcessStatus::Running(_)   => (s.dash_running,      GREEN),
        ProcessStatus::Stopped      => (s.dash_stopped,      RED),
        ProcessStatus::CheckError(_)=> (s.dash_check_failed, RED),
    };

    let pid_suffix: Element<'static, StoreMessage> =
        if let ProcessStatus::Running(pid) = &state.process_status {
            widget::text(format!("  ({} {})", s.dash_pid, pid)).size(11).into()
        } else {
            widget::text("").size(11).into()
        };

    let status_chip = widget::row()
        .push(
            widget::text(status_text)
                .size(13)
                .class(cosmic::theme::Text::Color(status_color)),
        )
        .push(pid_suffix)
        .align_y(Alignment::Center)
        .spacing(4);

    widget::container(
        widget::row()
            .push(status_chip)
            .push(widget::horizontal_space())
            .push(
                widget::button::text(s.dash_refresh)
                    .on_press(StoreMessage::DashboardRefresh),
            )
            .push(
                widget::button::destructive(s.dash_clear_log)
                    .on_press(StoreMessage::DashboardClearLog),
            )
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .padding([8, 16])
    .width(Length::Fill)
    .into()
}

// ── Metrics panel ─────────────────────────────────────────────────────────────

fn render_metrics(state: &DashboardState) -> Element<'static, StoreMessage> {
    let s = i18n::strings();
    let m = &state.metrics;

    let heading = widget::text(s.dash_metrics)
        .size(13)
        .class(cosmic::theme::Text::Color(GOLD));

    let row = |label: &'static str, value: String| -> Element<'static, StoreMessage> {
        widget::row()
            .push(widget::text(label).size(11))
            .push(widget::horizontal_space())
            .push(
                widget::text(value)
                    .size(12)
                    .class(cosmic::theme::Text::Color(AMBER)),
            )
            .align_y(Alignment::Center)
            .into()
    };

    widget::column()
        .push(heading)
        .push(widget::divider::horizontal::default())
        .push(row(
            s.dash_intercepted,
            format!("{}", m.syscalls_intercepted),
        ))
        .push(row(
            s.dash_blocked,
            format!("{}", m.violations_blocked),
        ))
        .push(row(
            s.dash_plugins,
            format!("{}", m.plugins_loaded),
        ))
        .push(row(
            s.dash_memory,
            format!("{:.1} MiB", m.memory_mib),
        ))
        .push(widget::divider::horizontal::default())
        .push(
            widget::text(crate::tr!(dash_events_in_log, &state.audit_log.len().to_string()))
            .size(10),
        )
        .spacing(8)
        .into()
}

// ── Audit log panel ───────────────────────────────────────────────────────────

fn render_audit_log(state: &DashboardState) -> Element<'static, StoreMessage> {
    let s = i18n::strings();
    let heading = widget::text(s.dash_audit_log)
        .size(13)
        .class(cosmic::theme::Text::Color(GOLD));

    let mut col = widget::column()
        .push(heading)
        .push(widget::divider::horizontal::default())
        .spacing(4);

    if state.audit_log.is_empty() {
        col = col.push(
            widget::container(
                widget::text(s.dash_no_events)
                .size(12),
            )
            .padding([20, 8]),
        );
    } else {
        // Show newest events first (reverse iteration).
        for event in state.audit_log.iter().rev().take(100) {
            col = col.push(audit_event_row(event));
        }
    }

    widget::scrollable(col).height(Length::Fill).into()
}

/// Render a single audit event row.
fn audit_event_row(event: &crate::types::AuditEvent) -> Element<'static, StoreMessage> {
    let (badge_text, badge_color) = match event.severity {
        AuditSeverity::Info    => ("[INFO] ", GREEN),
        AuditSeverity::Warning => ("[WARN] ", AMBER),
        AuditSeverity::Blocked => ("[BLOCK]", RED),
    };

    let desc = event.description.clone();
    let ts   = event.timestamp.clone();
    let detail_text = event.detail.clone().unwrap_or_default();

    let mut row = widget::row()
        .push(widget::text(ts).size(10))
        .push(
            widget::text(badge_text)
                .size(10)
                .class(cosmic::theme::Text::Color(badge_color)),
        )
        .push(widget::text(desc).size(11))
        .spacing(6)
        .align_y(Alignment::Center);

    if !detail_text.is_empty() {
        row = row.push(
            widget::text(format!("  \u{2192} {}", detail_text))
                .size(10)
                .class(cosmic::theme::Text::Color(AMBER)),
        );
    }

    widget::container(row).padding([3, 6]).width(Length::Fill).into()
}
