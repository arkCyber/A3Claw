//! Startup environment check overlay page.
//!
//! Shown as a full-screen modal during startup. Displays each check item
//! with its status icon, name, and latency. A "进入应用" button appears
//! once all checks are complete (even if some failed).

use crate::app::AppMessage;
use crate::env_check::{CheckStatus, EnvCheckReport};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

pub struct EnvCheckPage;

impl EnvCheckPage {
    pub fn view(report: &EnvCheckReport) -> Element<'_, AppMessage> {
        let color_ok      = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
        let color_warn    = cosmic::iced::Color::from_rgb(0.96, 0.72, 0.12);
        let color_error   = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);
        let color_muted   = cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48);
        let color_running = cosmic::iced::Color::from_rgb(0.38, 0.72, 0.98);
        let color_title   = cosmic::iced::Color::from_rgb(0.92, 0.90, 0.88);

        // ── Title ──────────────────────────────────────────────────────────
        let title = widget::column::with_children(vec![
            widget::text("🛡  OpenClaw+")
                .size(32)
                .font(cosmic::font::bold())
                .class(cosmic::theme::Text::Color(color_title))
                .into(),
            widget::text("启动环境自检")
                .size(16)
                .class(cosmic::theme::Text::Color(color_muted))
                .into(),
        ])
        .spacing(4)
        .align_x(Alignment::Center);

        // ── Progress bar ───────────────────────────────────────────────────
        let pct = (report.progress() * 100.0) as u8;
        let bar_fill_color = if report.is_complete() { color_ok } else { color_running };
        let bar_width = (500.0 * report.progress()).max(4.0);

        let progress_bar = widget::column::with_children(vec![
            widget::container(
                widget::container(widget::Space::new(bar_width, 6.0))
                    .style(move |_: &cosmic::Theme| {
                        cosmic::iced::widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(bar_fill_color)),
                            border: cosmic::iced::Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    }),
            )
            .width(Length::Fixed(500.0))
            .style(|_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgb(0.18, 0.17, 0.16),
                )),
                border: cosmic::iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            })
            .into(),
            widget::text(format!("检测进度 {}%", pct))
                .size(11)
                .class(cosmic::theme::Text::Color(color_muted))
                .into(),
        ])
        .spacing(4)
        .align_x(Alignment::Center);

        // ── Check item rows ────────────────────────────────────────────────
        let mut rows: Vec<Element<AppMessage>> = Vec::new();

        for item in &report.items {
            let (icon, name_color) = match &item.status {
                CheckStatus::Pending  => ("  ○", color_muted),
                CheckStatus::Running  => ("  ◌", color_running),
                CheckStatus::Ok       => ("  ✓", color_ok),
                CheckStatus::Warning(_) => ("  ⚠", color_warn),
                CheckStatus::Error(_)   => ("  ✗", color_error),
                CheckStatus::Skipped    => ("  –", color_muted),
            };

            let status_text = match &item.status {
                CheckStatus::Pending      => "等待中".to_string(),
                CheckStatus::Running      => "检测中…".to_string(),
                CheckStatus::Ok           => {
                    if let Some(ms) = item.latency_ms {
                        format!("正常  {}ms", ms)
                    } else {
                        "正常".to_string()
                    }
                }
                CheckStatus::Warning(msg) => msg.clone(),
                CheckStatus::Error(msg)   => msg.clone(),
                CheckStatus::Skipped      => "未配置，跳过".to_string(),
            };

            let status_color = match &item.status {
                CheckStatus::Ok      => color_ok,
                CheckStatus::Warning(_) => color_warn,
                CheckStatus::Error(_)   => color_error,
                CheckStatus::Skipped    => color_muted,
                _                       => color_muted,
            };

            let row = widget::row::with_children(vec![
                // Icon + name
                widget::text(icon)
                    .size(14)
                    .font(cosmic::font::mono())
                    .class(cosmic::theme::Text::Color(name_color))
                    .into(),
                widget::text(&item.name)
                    .size(13)
                    .font(cosmic::font::bold())
                    .class(cosmic::theme::Text::Color(name_color))
                    .width(Length::Fixed(150.0))
                    .into(),
                // Description (muted)
                widget::text(&item.description)
                    .size(11)
                    .class(cosmic::theme::Text::Color(color_muted))
                    .width(Length::Fixed(200.0))
                    .into(),
                // Status
                widget::text(status_text)
                    .size(12)
                    .class(cosmic::theme::Text::Color(status_color))
                    .width(Length::Fill)
                    .into(),
            ])
            .spacing(12)
            .align_y(Alignment::Center)
            .padding([4, 8]);

            rows.push(row.into());

            // Show ollama auto-start hint
            if item.id == "ollama_running" && report.ollama_was_started {
                rows.push(
                    widget::text("    → Ollama 已自动启动")
                        .size(11)
                        .class(cosmic::theme::Text::Color(color_ok))
                        .into(),
                );
            }

            // Show detail hint
            if let Some(detail) = &item.detail {
                if !detail.is_empty() {
                    rows.push(
                        widget::text(format!("    → {}", detail))
                            .size(11)
                            .class(cosmic::theme::Text::Color(color_muted))
                            .into(),
                    );
                }
            }
        }

        let items_col = widget::column::with_children(rows).spacing(2);

        let items_container = widget::container(items_col)
            .width(Length::Fixed(560.0))
            .padding([12, 16])
            .style(|_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(0.12, 0.11, 0.10, 0.95),
                )),
                border: cosmic::iced::Border {
                    color: cosmic::iced::Color::from_rgb(0.28, 0.26, 0.24),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            });

        // ── Ollama auto-start notice ───────────────────────────────────────
        let ollama_notice: Option<Element<AppMessage>> = if report.ollama_was_started {
            Some(
                widget::container(
                    widget::row::with_children(vec![
                        widget::text("🚀").size(16).into(),
                        widget::text("Ollama 服务已自动启动")
                            .size(13)
                            .class(cosmic::theme::Text::Color(color_ok))
                            .into(),
                    ])
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding([6, 14])
                .style(move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgba(0.10, 0.35, 0.18, 0.9),
                    )),
                    border: cosmic::iced::Border {
                        color: color_ok,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                })
                .into(),
            )
        } else {
            None
        };

        // ── Action buttons ─────────────────────────────────────────────────
        let enter_btn = widget::button::suggested("进入应用  →")
            .on_press(AppMessage::EnvCheckDismiss);

        let skip_btn = widget::button::standard("跳过检测")
            .on_press(AppMessage::EnvCheckDismiss);

        let buttons: Element<AppMessage> = if report.is_complete() {
            widget::row::with_children(vec![
                skip_btn.into(),
                enter_btn.into(),
            ])
            .spacing(12)
            .into()
        } else {
            widget::row::with_children(vec![
                skip_btn.into(),
            ])
            .into()
        };

        // ── Summary line ───────────────────────────────────────────────────
        let ok_count = report.items.iter().filter(|i| i.status.is_ok()).count();
        let warn_count = report.items.iter().filter(|i| matches!(i.status, CheckStatus::Warning(_))).count();
        let err_count  = report.items.iter().filter(|i| matches!(i.status, CheckStatus::Error(_))).count();
        let skip_count = report.items.iter().filter(|i| matches!(i.status, CheckStatus::Skipped)).count();

        let summary = if report.is_complete() {
            format!(
                "检测完成：{} 正常  {} 警告  {} 失败  {} 跳过",
                ok_count, warn_count, err_count, skip_count
            )
        } else {
            format!(
                "正在检测… {}/{}",
                report.finished_count(),
                report.total_count()
            )
        };

        let summary_color = if err_count > 0 {
            color_error
        } else if warn_count > 0 {
            color_warn
        } else {
            color_ok
        };

        // ── Assemble ───────────────────────────────────────────────────────
        let mut card_children: Vec<Element<AppMessage>> = vec![
            title.into(),
            widget::Space::new(0, 16).into(),
            progress_bar.into(),
            widget::Space::new(0, 12).into(),
            items_container.into(),
            widget::Space::new(0, 8).into(),
            widget::text(summary)
                .size(12)
                .class(cosmic::theme::Text::Color(summary_color))
                .into(),
        ];

        if let Some(notice) = ollama_notice {
            card_children.push(widget::Space::new(0, 4).into());
            card_children.push(notice);
        }

        card_children.push(widget::Space::new(0, 16).into());
        card_children.push(buttons);

        let card = widget::container(
            widget::column::with_children(card_children)
                .spacing(4)
                .align_x(Alignment::Center),
        )
        .padding([28, 36])
        .style(|_: &cosmic::Theme| cosmic::iced::widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgb(0.10, 0.09, 0.08),
            )),
            border: cosmic::iced::Border {
                color: cosmic::iced::Color::from_rgb(0.30, 0.28, 0.25),
                width: 1.0,
                radius: 14.0.into(),
            },
            ..Default::default()
        });

        // Full-screen backdrop
        widget::container(
            widget::container(card)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_: &cosmic::Theme| cosmic::iced::widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgba(0.04, 0.04, 0.04, 0.97),
            )),
            ..Default::default()
        })
        .into()
    }
}
