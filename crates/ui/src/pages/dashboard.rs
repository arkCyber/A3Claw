use crate::app::{AppMessage, SandboxStats};
use crate::theme::{Language, t, tx};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use openclaw_security::{BreakerStats, EventKind, SandboxEvent};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use cosmic::iced::widget::container::Style as ContainerStyle;

pub struct DashboardPage;

impl DashboardPage {
    /// Format Unix timestamp to human-readable date/time (aerospace-grade precision).
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
        
        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hours, mins, secs)
    }
    
    /// Format Unix timestamp to relative time (e.g., "2m ago").
    fn format_relative_time(unix_secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if unix_secs > now {
            return "future".to_string();
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
    pub fn view<'a>(
        stats: &'a SandboxStats,
        breaker: &'a BreakerStats,
        events: &'a VecDeque<SandboxEvent>,
        pending: &'a [SandboxEvent],
        lang: Language,
    ) -> Element<'a, AppMessage> {
        let breaker_banner: Option<Element<AppMessage>> = if breaker.is_tripped {
            Some(
                widget::container(
                    widget::row::with_children(vec![
                        widget::icon::from_name("dialog-error-symbolic").size(20).into(),
                        widget::column::with_children(vec![
                            widget::text(t(lang,
                                "Circuit breaker tripped — sandbox forcefully terminated",
                                "熔断器触发 — 沙箱已被强制终止",
                            ))
                            .size(15)
                            .font(cosmic::font::bold())
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(1.0, 1.0, 1.0),
                            ))
                            .into(),
                            widget::text(format!(
                                "{}: {} | {}: {}",
                                t(lang, "Total denials", "总拒绝次数"),
                                breaker.total_denials,
                                t(lang, "Dangerous commands", "危险命令"),
                                breaker.dangerous_commands,
                            ))
                            .size(12)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.8),
                            ))
                            .into(),
                        ])
                        .spacing(2)
                        .width(Length::Fill)
                        .into(),
                    ])
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .padding([12, 16]),
                )
                .style(|_theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgb(0.75, 0.1, 0.1),
                    )),
                    border: cosmic::iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .width(Length::Fill)
                .into(),
            )
        } else {
            None
        };

        let top_row = widget::row::with_children(vec![
            compact_stat_card(
                tx(lang, "Total").to_owned(), stats.total_events.to_string(),
                crate::icons::total(22),
                cosmic::iced::Color::from_rgb(0.72, 0.70, 0.68),
            ),
            compact_stat_card(
                tx(lang, "Allowed").to_owned(), stats.allowed_count.to_string(),
                crate::icons::check(22),
                cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46),
            ),
            compact_stat_card(
                tx(lang, "Denied").to_owned(), stats.denied_count.to_string(),
                crate::icons::block(22),
                cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
            ),
            compact_stat_card(
                tx(lang, "Pending").to_owned(), stats.pending_count.to_string(),
                crate::icons::clock(22),
                cosmic::iced::Color::from_rgb(0.98, 0.72, 0.18),
            ),
        ])
        .spacing(8)
        .padding([0, 0, 12, 0]);

        let ops_row = widget::row::with_children(vec![
            compact_stat_card(
                tx(lang, "File Ops").to_owned(), stats.file_ops.to_string(),
                crate::icons::folder(22),
                cosmic::iced::Color::from_rgb(0.98, 0.62, 0.22),
            ),
            compact_stat_card(
                tx(lang, "Network").to_owned(), stats.network_ops.to_string(),
                crate::icons::network(22),
                cosmic::iced::Color::from_rgb(0.38, 0.72, 0.98),
            ),
            compact_stat_card(
                tx(lang, "Shell").to_owned(), stats.shell_ops.to_string(),
                crate::icons::terminal(22),
                cosmic::iced::Color::from_rgb(0.82, 0.52, 0.98),
            ),
            breaker_stat_card(breaker, lang),
        ])
        .spacing(8)
        .padding([0, 0, 12, 0]);

        let confirm_section: Element<AppMessage> = if pending.is_empty() {
            widget::container(
                widget::text(t(lang, "No operations awaiting confirmation.", "暂无待确认操作。"))
                    .size(14)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5),
                    )),
            )
            .padding(16)
            .into()
        } else {
            let items: Vec<Element<AppMessage>> = pending
                .iter()
                .map(|event| confirm_card(event))
                .collect();
            widget::column::with_children(items).spacing(8).into()
        };

        let recent_events: Vec<Element<AppMessage>> = events
            .iter()
            .rev()
            .take(10)
            .map(|e| event_row(e))
            .collect();

        let recent_section: Element<AppMessage> = if recent_events.is_empty() {
            widget::text(tx(lang, "No sandbox events yet.")).size(14).into()
        } else {
            widget::column::with_children(recent_events)
                .spacing(4)
                .into()
        };

        let emergency_btn = if breaker.is_tripped {
            widget::button::text(tx(lang, "Tripped"))
                .class(cosmic::theme::Button::Standard)
                .into()
        } else {
            widget::button::text(tx(lang, "Emergency Stop"))
                .on_press(AppMessage::EmergencyStop)
                .class(cosmic::theme::Button::Destructive)
                .into()
        };

        let control_row = widget::row::with_children(vec![
            emergency_btn,
            widget::Space::new(Length::Fill, 0).into(),
            widget::button::text(tx(lang, "Clear Log"))
                .on_press(AppMessage::ClearEvents)
                .class(cosmic::theme::Button::Text)
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center);

        let mut children: Vec<Element<AppMessage>> = vec![
            widget::text(tx(lang, "Overview"))
                .size(20)
                .font(cosmic::font::bold())
                .into(),
            widget::Space::new(0, 8).into(),
        ];

        if let Some(banner) = breaker_banner {
            children.push(banner);
            children.push(widget::Space::new(0, 8).into());
        }

        children.extend(vec![
            top_row.into(),
            ops_row.into(),
            // ── Control buttons directly below stat cards ───────────────────────────────
            control_row.into(),
            widget::Space::new(0, 12).into(),
            widget::divider::horizontal::default().into(),
            widget::Space::new(0, 8).into(),
            widget::text(tx(lang, "Pending Confirmations"))
                .size(16)
                .font(cosmic::font::bold())
                .into(),
            widget::Space::new(0, 4).into(),
            confirm_section,
            widget::Space::new(0, 8).into(),
            widget::divider::horizontal::default().into(),
            widget::Space::new(0, 8).into(),
            widget::text(tx(lang, "Recent Events"))
                .size(16)
                .font(cosmic::font::bold())
                .into(),
            widget::Space::new(0, 4).into(),
            recent_section,
            widget::Space::new(0, 16).into(),
        ]);

        widget::scrollable(
            widget::column::with_children(children)
                .padding(24)
                .spacing(4),
        )
        .into()
    }
}

fn breaker_stat_card(breaker: &BreakerStats, lang: Language) -> Element<AppMessage> {
    let (value, color) = if breaker.is_tripped {
        (t(lang, "Tripped", "已触发"), cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28))
    } else {
        (t(lang, "Normal", "正常"),   cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46))
    };

    widget::container(
        widget::row::with_children(vec![
            widget::container(crate::icons::breaker(20))
                .style(move |_: &cosmic::Theme| ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color { a: 0.12, ..color },
                    )),
                    border: cosmic::iced::Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding(8)
                .into(),
            widget::column::with_children(vec![
                widget::text(value)
                    .size(18)
                    .font(cosmic::font::bold())
                    .class(cosmic::theme::Text::Color(color))
                    .into(),
                widget::text(t(lang, "Breaker", "熔断器")).size(11).into(),
                widget::text(format!("D:{} C:{}", breaker.total_denials, breaker.dangerous_commands))
                    .size(10)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48)))
                    .into(),
            ])
            .spacing(2)
            .into(),
        ])
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([10, 12]),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

/// Compact horizontal stat card: icon on left, value+label on right.
fn compact_stat_card(
    label: String,
    value: String,
    icon: cosmic::widget::icon::Icon,
    color: cosmic::iced::Color,
) -> Element<'static, AppMessage> {
    widget::container(
        widget::row::with_children(vec![
            widget::container(icon)
                .style(move |_: &cosmic::Theme| ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color { a: 0.15, ..color },
                    )),
                    border: cosmic::iced::Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding(8)
                .into(),
            widget::column::with_children(vec![
                widget::text(value)
                    .size(20)
                    .font(cosmic::font::bold())
                    .class(cosmic::theme::Text::Color(color))
                    .into(),
                widget::text(label)
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.62, 0.60, 0.58),
                    ))
                    .into(),
            ])
            .spacing(2)
            .into(),
        ])
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([10, 12]),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

fn confirm_card(event: &SandboxEvent) -> Element<AppMessage> {
    let id = event.id;

    widget::container(
        widget::row::with_children(vec![
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::icon::from_name("dialog-warning-symbolic").size(16).into(),
                    widget::text(event.kind.to_string())
                        .size(14)
                        .font(cosmic::font::bold())
                        .into(),
                ])
                .spacing(6)
                .align_y(Alignment::Center)
                .into(),
                widget::text(event.detail.as_str()).size(13).into(),
                if event.path.as_deref().unwrap_or("").is_empty() {
                    widget::Space::new(0, 0).into()
                } else {
                    widget::text(format!("Path: {}", event.path.as_deref().unwrap_or("")))
                        .size(12)
                        .into()
                },
            ])
            .spacing(4)
            .width(Length::Fill)
            .into(),
            widget::row::with_children(vec![
                widget::button::text("Allow")
                    .on_press(AppMessage::ConfirmAllow(id))
                    .class(cosmic::theme::Button::Suggested)
                    .into(),
                widget::button::text("Deny")
                    .on_press(AppMessage::ConfirmDeny(id))
                    .class(cosmic::theme::Button::Destructive)
                    .into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        ])
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .class(cosmic::theme::Container::Card)
    .padding(12)
    .into()
}

fn event_row(event: &SandboxEvent) -> Element<AppMessage> {
    let (status_icon, status_color) = match event.allowed {
        Some(true)  => (crate::icons::ok(14),      cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46)),
        Some(false) => (crate::icons::denied(14),  cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28)),
        None        => (crate::icons::pending(14), cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12)),
    };

    let kind_color = match event.kind {
        EventKind::FileDelete => cosmic::iced::Color::from_rgb(0.9, 0.3, 0.3),
        EventKind::ShellExec | EventKind::ProcessSpawn => cosmic::iced::Color::from_rgb(0.9, 0.6, 0.1),
        EventKind::NetworkRequest => cosmic::iced::Color::from_rgb(0.2, 0.6, 0.9),
        EventKind::PolicyDenied => cosmic::iced::Color::from_rgb(0.8, 0.1, 0.1),
        _ => cosmic::iced::Color::from_rgb(0.4, 0.8, 0.4),
    };

    // Format timestamp for display
    let time_str = DashboardPage::format_timestamp(event.timestamp);
    let relative_str = DashboardPage::format_relative_time(event.timestamp);

    widget::container(
        widget::row::with_children(vec![
            widget::container(status_icon)
                .style(move |_: &cosmic::Theme| ContainerStyle {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color { a: 0.12, ..status_color },
                    )),
                    border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding(3)
                .into(),
            widget::text(event.kind.to_string())
                .size(13)
                .class(cosmic::theme::Text::Color(kind_color))
                .width(Length::Fixed(120.0))
                .into(),
            widget::text(event.detail.as_str())
                .size(12)
                .width(Length::Fill)
                .into(),
            widget::text(format!("{} ({})", time_str, relative_str))
                .size(11)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6)))
                .width(Length::Fixed(155.0))
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([4, 8]),
    )
    .into()
}
