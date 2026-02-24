use crate::app::AppMessage;
use crate::theme::{Language, t};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use openclaw_security::{EventKind, SandboxEvent};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct EventsPage;

impl EventsPage {
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
        events: &'a VecDeque<SandboxEvent>,
        filter: Option<EventKind>,
        search: &'a str,
        lang: Language,
    ) -> Element<'a, AppMessage> {
        // ── Filter chips ──────────────────────────────────────────────────────
        let chip = |label: &'static str, kind: Option<EventKind>| -> Element<'a, AppMessage> {
            let active = filter == kind;
            widget::button::text(label)
                .on_press(AppMessage::SetEventFilter(kind))
                .class(if active {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                })
                .into()
        };

        let filter_row = widget::row::with_children(vec![
            chip(t(lang, "All",     "全部"),    None),
            chip(t(lang, "File",    "文件"),    Some(EventKind::FileDelete)),
            chip(t(lang, "Shell",   "Shell"),   Some(EventKind::ShellExec)),
            chip(t(lang, "Network", "网络"),    Some(EventKind::NetworkRequest)),
            chip(t(lang, "Denied",  "已拒绝"),  Some(EventKind::PolicyDenied)),
            chip(t(lang, "Pending", "待确认"),  Some(EventKind::UserConfirmRequired)),
        ])
        .spacing(6)
        .align_y(Alignment::Center);

        // ── Search box ────────────────────────────────────────────────────────
        let search_box = widget::text_input(
            t(lang, "Search events…", "搜索事件…"),
            search,
        )
        .on_input(AppMessage::SetEventSearch)
        .width(Length::Fixed(260.0));

        // ── Apply filter + search ─────────────────────────────────────────────
        let search_lower = search.to_lowercase();
        let visible: Vec<&SandboxEvent> = events
            .iter()
            .rev()
            .filter(|e| {
                let kind_ok = filter.as_ref().map_or(true, |k| &e.kind == k);
                let search_ok = search_lower.is_empty()
                    || e.detail.to_lowercase().contains(&search_lower)
                    || e.path.as_deref().unwrap_or("").to_lowercase().contains(&search_lower)
                    || e.kind.to_string().to_lowercase().contains(&search_lower);
                kind_ok && search_ok
            })
            .collect();

        // ── Table header ──────────────────────────────────────────────────────
        let header = widget::row::with_children(vec![
            widget::text(t(lang, "Status",   "状态"))  .size(12).font(cosmic::font::bold()).width(Length::Fixed(72.0)).into(),
            widget::text(t(lang, "Kind",     "类型"))  .size(12).font(cosmic::font::bold()).width(Length::Fixed(120.0)).into(),
            widget::text(t(lang, "Resource", "资源"))  .size(12).font(cosmic::font::bold()).width(Length::Fixed(80.0)).into(),
            widget::text(t(lang, "Detail",   "详情"))  .size(12).font(cosmic::font::bold()).width(Length::Fill).into(),
            widget::text(t(lang, "Path",     "路径"))  .size(12).font(cosmic::font::bold()).width(Length::Fixed(180.0)).into(),
        ])
        .spacing(8)
        .padding([6, 16]);

        // ── Rows ──────────────────────────────────────────────────────────────
        let rows: Vec<Element<AppMessage>> = if visible.is_empty() {
            vec![widget::container(
                widget::text(if events.is_empty() {
                    t(lang, "No events recorded yet.", "暂无事件记录。")
                } else {
                    t(lang, "No events match the current filter.", "没有符合当前过滤条件的事件。")
                })
                .size(14)
                .class(cosmic::theme::Text::Color(
                    cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48),
                )),
            )
            .padding(32)
            .center_x(Length::Fill)
            .into()]
        } else {
            visible.iter().map(|e| event_table_row(e)).collect()
        };

        let table = widget::column::with_children(rows).spacing(2);

        // ── Title bar ─────────────────────────────────────────────────────────
        let title_bar = widget::row::with_children(vec![
            widget::text(t(lang, "Event Log", "事件日志"))
                .size(20)
                .font(cosmic::font::bold())
                .width(Length::Fill)
                .into(),
            widget::text(format!(
                "{} / {} {}",
                visible.len(),
                events.len(),
                t(lang, "events", "条事件"),
            ))
            .size(12)
            .class(cosmic::theme::Text::Color(
                cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48),
            ))
            .into(),
            widget::button::text(t(lang, "Clear", "清除"))
                .on_press(AppMessage::ClearEvents)
                .class(cosmic::theme::Button::Standard)
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([14, 24, 8, 24]);

        // ── Toolbar (filter + search) ─────────────────────────────────────────
        let toolbar = widget::row::with_children(vec![
            filter_row.into(),
            widget::Space::new(Length::Fill, 0).into(),
            search_box.into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([4, 16, 8, 16]);

        widget::column::with_children(vec![
            title_bar.into(),
            widget::divider::horizontal::default().into(),
            toolbar.into(),
            widget::divider::horizontal::default().into(),
            header.into(),
            widget::divider::horizontal::default().into(),
            widget::scrollable(
                widget::container(table).padding([4, 16]),
            )
            .height(Length::Fill)
            .into(),
        ])
        .into()
    }
}

/// Renders a single row in the event log table.
fn event_table_row(event: &SandboxEvent) -> Element<AppMessage> {
    let (status_icon, status_color) = match event.allowed {
        Some(true)  => ("✅ Allowed",  cosmic::iced::Color::from_rgb(0.2, 0.75, 0.4)),
        Some(false) => ("🚫 Denied",   cosmic::iced::Color::from_rgb(0.85, 0.2, 0.2)),
        None        => ("⏳ Pending",  cosmic::iced::Color::from_rgb(0.9, 0.65, 0.1)),
    };

    let kind_color = match event.kind {
        EventKind::FileDelete => cosmic::iced::Color::from_rgb(0.9, 0.3, 0.3),
        EventKind::ShellExec | EventKind::ProcessSpawn => {
            cosmic::iced::Color::from_rgb(0.9, 0.6, 0.1)
        }
        EventKind::NetworkRequest => cosmic::iced::Color::from_rgb(0.2, 0.6, 0.9),
        EventKind::PolicyDenied => cosmic::iced::Color::from_rgb(0.8, 0.1, 0.1),
        EventKind::SandboxStart | EventKind::SandboxStop => {
            cosmic::iced::Color::from_rgb(0.5, 0.5, 0.8)
        }
        _ => cosmic::iced::Color::from_rgb(0.6, 0.6, 0.6),
    };

    let bg_color = if event.allowed == Some(false) {
        cosmic::iced::Color::from_rgba(0.9, 0.2, 0.2, 0.05)
    } else if event.kind == EventKind::UserConfirmRequired {
        cosmic::iced::Color::from_rgba(0.9, 0.65, 0.1, 0.08)
    } else {
        cosmic::iced::Color::TRANSPARENT
    };

    // Format timestamp for display
    let time_str = EventsPage::format_timestamp(event.timestamp);
    let relative_str = EventsPage::format_relative_time(event.timestamp);

    widget::container(
        widget::row::with_children(vec![
            widget::text(status_icon)
                .size(12)
                .class(cosmic::theme::Text::Color(status_color))
                .width(Length::Fixed(60.0))
                .into(),
            widget::text(event.kind.to_string())
                .size(12)
                .class(cosmic::theme::Text::Color(kind_color))
                .width(Length::Fixed(110.0))
                .into(),
            widget::text(event.resource.to_string())
                .size(12)
                .width(Length::Fixed(80.0))
                .into(),
            widget::text(&event.detail)
                .size(12)
                .width(Length::Fill)
                .into(),
            widget::text(format!("{} ({})", time_str, relative_str))
                .size(11)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)))
                .width(Length::Fixed(175.0))
                .into(),
            widget::text(event.path.as_deref().unwrap_or("-"))
                .size(11)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)))
                .width(Length::Fixed(150.0))
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([6, 8]),
    )
    .style(move |_theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
        background: if bg_color != cosmic::iced::Color::TRANSPARENT {
            Some(cosmic::iced::Background::Color(bg_color))
        } else {
            None
        },
        border: cosmic::iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
