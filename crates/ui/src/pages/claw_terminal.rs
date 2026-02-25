use crate::app::{AppMessage, ClawEntry, ClawEntrySource, ClawEntryStatus, ClawQuickAction};
use crate::pages::ai_chat::EngineStatus;
use crate::app::SandboxStatus;
use crate::theme::Language;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

/// Stable ID for the Claw Terminal input box.
pub static CLAW_INPUT_ID: std::sync::LazyLock<cosmic::widget::Id> =
    std::sync::LazyLock::new(cosmic::widget::Id::unique);

/// Stable ID for the Claw Terminal history scrollable.
pub static CLAW_SCROLL_ID: std::sync::LazyLock<cosmic::widget::Id> =
    std::sync::LazyLock::new(cosmic::widget::Id::unique);

pub struct ClawTerminalPage;

impl ClawTerminalPage {
    pub fn view<'a>(
        lang: Language,
        history: &'a [ClawEntry],
        input: &'a str,
        sandbox_status: &'a SandboxStatus,
        ai_status: &'a EngineStatus,
        nl_mode: bool,
        gateway_url: Option<&'a str>,
        gateway_reachable: bool,
        tg_polling: bool,
        tg_bot_username: Option<&'a str>,
        selected_agent_id: Option<&'a str>,
        agent_list: &'a [openclaw_security::AgentProfile],
        attachment_name: Option<&'a str>,
        recording: bool,
        voice_status: Option<&'a str>,
    ) -> Element<'a, AppMessage> {
        let color_muted   = cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48);
        let color_accent  = cosmic::iced::Color::from_rgb(0.28, 0.92, 0.78);
        let color_success = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
        let color_error   = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);
        let color_stderr  = cosmic::iced::Color::from_rgb(0.96, 0.62, 0.22);
        let color_running = cosmic::iced::Color::from_rgb(0.96, 0.82, 0.22);

        // ── Header ────────────────────────────────────────────────────────────
        let sandbox_ok = matches!(sandbox_status, SandboxStatus::Running);
        let ai_ok = matches!(ai_status, EngineStatus::Ready | EngineStatus::Idle);

        let status_dot = |ok: bool| -> Element<'a, AppMessage> {
            let c = if ok { color_success } else { color_error };
            widget::container(widget::Space::new(7, 7))
                .style(move |_: &cosmic::Theme| {
                    cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(c)),
                        border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                })
                .into()
        };

        let color_nl = cosmic::iced::Color::from_rgb(0.82, 0.52, 0.98);

        // ── Agent selector ────────────────────────────────────────────────────
        let color_agent = cosmic::iced::Color::from_rgb(0.58, 0.40, 0.98);
        let agent_selector = if !agent_list.is_empty() {
            let selected_name = selected_agent_id
                .and_then(|id| agent_list.iter().find(|a| a.id.as_str() == id))
                .map(|a| a.display_name.as_str())
                .unwrap_or("选择数字员工");
            
            let mut agent_buttons: Vec<Element<AppMessage>> = vec![
                widget::button::text(crate::theme::t(lang, "No Agent", "无 Agent"))
                    .on_press(AppMessage::ClawSelectAgent(None))
                    .class(if selected_agent_id.is_none() {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
                    .into(),
            ];
            
            for agent in agent_list.iter().take(5) {
                let is_selected = selected_agent_id == Some(agent.id.as_str());
                agent_buttons.push(
                    widget::button::text(&agent.display_name)
                        .on_press(AppMessage::ClawSelectAgent(Some(agent.id.as_str().to_string())))
                        .class(if is_selected {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::Standard
                        })
                        .into()
                );
            }
            
            widget::container(
                widget::column::with_children(vec![
                    widget::text(format!("🤖 {}", selected_name))
                        .size(11)
                        .class(cosmic::theme::Text::Color(color_agent))
                        .into(),
                    widget::Space::new(0, 4).into(),
                    widget::row::with_children(agent_buttons).spacing(4).into(),
                ])
                .spacing(0)
            )
            .padding([4, 8])
            .class(cosmic::theme::Container::Card)
            .into()
        } else {
            widget::Space::new(0, 0).into()
        };

        let header = widget::row::with_children(vec![
            crate::icons::claw_term(20).into(),
            widget::Space::new(10, 0).into(),
            widget::text(crate::theme::t(lang, "Claw Terminal", "Claw 终端"))
                .size(22).font(cosmic::font::bold()).into(),
            widget::Space::new(Length::Fill, 0).into(),
            agent_selector,
            widget::Space::new(12, 0).into(),
            // NL mode toggle
            widget::button::standard(
                if nl_mode {
                    crate::theme::t(lang, "NL Mode: ON", "NL 模式: 开")
                } else {
                    crate::theme::t(lang, "NL Mode: OFF", "NL 模式: 关")
                }
            )
            .on_press(AppMessage::ClawToggleNlMode)
            .class(if nl_mode { cosmic::theme::Button::Suggested } else { cosmic::theme::Button::Standard })
            .into(),
            widget::Space::new(12, 0).into(),
            // Status chips
            status_dot(sandbox_ok),
            widget::text(format!("Sandbox: {}", sandbox_status))
                .size(11).class(cosmic::theme::Text::Color(
                    if sandbox_ok { color_success } else { color_error }
                )).into(),
            widget::Space::new(16, 0).into(),
            status_dot(ai_ok),
            widget::text(format!("AI: {}", ai_status))
                .size(11).class(cosmic::theme::Text::Color(
                    if ai_ok { color_success } else { color_muted }
                )).into(),
            widget::Space::new(16, 0).into(),
            widget::button::destructive(
                crate::theme::t(lang, "Clear", "清空")
            )
                .on_press(AppMessage::ClawClearHistory)
                .into(),
        ])
        .spacing(6)
        .align_y(Alignment::Center);

        // ── Status bar (Gateway + Telegram) ────────────────────────────────────
        let color_gw_ok  = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
        let color_gw_off = cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48);
        let color_tg     = cosmic::iced::Color::from_rgb(0.28, 0.78, 0.96);
        let gw_bar = widget::container(
            widget::row::with_children(vec![
                // Gateway indicator
                widget::container(widget::Space::new(7, 7))
                    .style(move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            if gateway_reachable { color_gw_ok } else { color_gw_off }
                        )),
                        border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .into(),
                widget::Space::new(6, 0).into(),
                widget::text(
                    if gateway_reachable {
                        crate::theme::t(lang, "Gateway: Connected", "Gateway: 已连接")
                    } else {
                        crate::theme::t(lang, "Gateway: Offline", "Gateway: 离线")
                    }
                )
                .size(11)
                .class(cosmic::theme::Text::Color(
                    if gateway_reachable { color_gw_ok } else { color_gw_off }
                ))
                .into(),
                widget::Space::new(8, 0).into(),
                widget::text(gateway_url.unwrap_or("(set OPENCLAW_GATEWAY_URL)"))
                    .size(10)
                    .class(cosmic::theme::Text::Color(color_gw_off))
                    .into(),
                widget::Space::new(16, 0).into(),
                // Telegram indicator
                widget::container(widget::Space::new(7, 7))
                    .style(move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            if tg_polling { color_tg } else { color_gw_off }
                        )),
                        border: cosmic::iced::Border { radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .into(),
                widget::Space::new(6, 0).into(),
                widget::text(
                    if tg_polling {
                        if let Some(_u) = tg_bot_username {
                            crate::theme::t(lang, "Telegram: polling", "Telegram: 轮询中")
                        } else {
                            crate::theme::t(lang, "Telegram: polling", "Telegram: 轮询中")
                        }
                    } else {
                        crate::theme::t(lang, "Telegram: idle", "Telegram: 空闲")
                    }
                )
                .size(11)
                .class(cosmic::theme::Text::Color(if tg_polling { color_tg } else { color_gw_off }))
                .into(),
                widget::Space::new(Length::Fill, 0).into(),
                // Action buttons
                widget::button::standard(
                    crate::theme::t(lang, "Gateway", "Gateway")
                )
                .on_press(AppMessage::ClawProbeGateway)
                .into(),
                widget::Space::new(6, 0).into(),
                widget::button::standard(
                    if tg_polling {
                        crate::theme::t(lang, "Stop TG", "停止TG")
                    } else {
                        crate::theme::t(lang, "Start TG", "开始TG")
                    }
                )
                .on_press(if tg_polling { AppMessage::TgStopPolling } else { AppMessage::TgStartPolling })
                .class(if tg_polling { cosmic::theme::Button::Suggested } else { cosmic::theme::Button::Standard })
                .into(),
            ])
            .spacing(4)
            .align_y(Alignment::Center)
            .padding([4, 12]),
        )
        .class(cosmic::theme::Container::Card)
        .width(Length::Fill);

        let subtitle = if nl_mode {
            widget::text(crate::theme::t(
                lang,
                "NL Mode active — type natural language instructions, AI will plan and execute",
                "NL 模式已开启 — 用自然语言下指令，AI 自动规划并执行",
            ))
            .size(12)
            .class(cosmic::theme::Text::Color(color_nl))
        } else {
            widget::text(crate::theme::t(
                lang,
                "Send commands to OpenClaw, execute shell commands, view live output",
                "直接向 OpenClaw 发送命令、执行 Shell 命令、查看实时输出",
            ))
            .size(12)
            .class(cosmic::theme::Text::Color(color_muted))
        };

        // ── Quick-action buttons ──────────────────────────────────────────────
        let quick_btns: Vec<Element<AppMessage>> = ClawQuickAction::all().iter().map(|action| {
            widget::button::standard(action.label())
                .on_press(AppMessage::ClawQuickAction(action.clone()))
                .into()
        }).collect();

        let quick_row = widget::container(
            widget::column::with_children(vec![
                widget::text("Quick Actions").size(11)
                    .class(cosmic::theme::Text::Color(color_muted)).into(),
                widget::Space::new(0, 6).into(),
                {
                    let mut rows: Vec<Element<AppMessage>> = Vec::new();
                    let mut i = 0;
                    while i < quick_btns.len() {
                        let mut row_items: Vec<Element<AppMessage>> = Vec::new();
                        for j in 0..4 {
                            if i + j < quick_btns.len() {
                                // We can't move out of a vec by index in a loop easily,
                                // so we rebuild from ClawQuickAction::all()
                                let action = &ClawQuickAction::all()[i + j];
                                row_items.push(
                                    widget::button::standard(action.label())
                                        .on_press(AppMessage::ClawQuickAction(action.clone()))
                                        .into()
                                );
                            }
                        }
                        rows.push(widget::row::with_children(row_items).spacing(6).into());
                        i += 4;
                    }
                    widget::column::with_children(rows).spacing(6).into()
                },
            ])
            .spacing(0)
            .padding([8, 12]),
        )
        .class(cosmic::theme::Container::Card)
        .width(Length::Fill);

        // ── Command history ───────────────────────────────────────────────────
        let history_items: Vec<Element<AppMessage>> = if history.is_empty() {
            vec![
                widget::container(
                    widget::column::with_children(vec![
                        widget::text("No commands yet").size(14).font(cosmic::font::bold())
                            .class(cosmic::theme::Text::Color(color_muted)).into(),
                        widget::Space::new(0, 6).into(),
                        widget::text("Type a command below or click a Quick Action button.")
                            .size(12).class(cosmic::theme::Text::Color(color_muted)).into(),
                        widget::Space::new(0, 8).into(),
                        widget::text("Built-in: status, sandbox start/stop, emergency stop,")
                            .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                        widget::text("          models list, events clear, config show, ai restart, help")
                            .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                        widget::Space::new(0, 4).into(),
                        widget::text("Shell: any system command (ls, pwd, git status, …)")
                            .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                    ])
                    .spacing(0)
                    .padding([20, 24]),
                )
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .into(),
            ]
        } else {
            history.iter().map(|entry| {
                build_entry_card(entry, color_accent, color_success, color_error, color_stderr, color_running, color_muted)
            }).collect()
        };

        // ── Input bar ─────────────────────────────────────────────────────────
        let (prompt_sym, prompt_color, placeholder) = if nl_mode {
            (
                "🤖",
                color_nl,
                crate::theme::t(lang,
                    "Describe what you want OpenClaw to do… (natural language)",
                    "用自然语言描述你想让 OpenClaw 做什么…",
                ),
            )
        } else {
            (
                "❯",
                color_accent,
                crate::theme::t(lang,
                    "Type a command… (Enter to run, Cmd+Enter to send to Agent, 'help' for list)",
                    "输入命令… (Enter 执行，Cmd+Enter 发给 Agent，'help' 查看列表)",
                ),
            )
        };

        let color_voice = if recording {
            cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28)
        } else {
            cosmic::iced::Color::from_rgb(0.72, 0.72, 0.72)
        };
        let color_img = cosmic::iced::Color::from_rgb(0.28, 0.72, 0.98);

        // Attachment preview row (shown when image is attached or voice status active)
        let has_attachment = attachment_name.is_some();
        let has_voice_status = voice_status.is_some();

        let mut input_col_children: Vec<Element<AppMessage>> = Vec::new();

        // Attachment / voice status info bar
        if has_attachment || has_voice_status {
            let mut info_items: Vec<Element<AppMessage>> = Vec::new();
            if let Some(name) = attachment_name {
                info_items.push(
                    widget::text(format!("📎 {}", name)).size(11)
                        .class(cosmic::theme::Text::Color(color_img)).into()
                );
                info_items.push(widget::Space::new(8, 0).into());
                info_items.push(
                    widget::button::text("✕")
                        .on_press(AppMessage::ClawClearAttachment)
                        .class(cosmic::theme::Button::Destructive)
                        .into()
                );
                info_items.push(widget::Space::new(16, 0).into());
            }
            if let Some(vs) = voice_status {
                info_items.push(
                    widget::text(vs).size(11)
                        .class(cosmic::theme::Text::Color(color_voice)).into()
                );
            }
            input_col_children.push(
                widget::row::with_children(info_items)
                    .spacing(0).align_y(Alignment::Center)
                    .padding([4, 14, 0, 14])
                    .into()
            );
        }

        // Main input row
        input_col_children.push(
            widget::row::with_children(vec![
                widget::text(prompt_sym)
                    .size(16)
                    .font(cosmic::font::bold())
                    .class(cosmic::theme::Text::Color(prompt_color))
                    .into(),
                widget::Space::new(8, 0).into(),
                widget::container(
                    widget::text_input(placeholder, input)
                        .id(CLAW_INPUT_ID.clone())
                        .on_input(|s| AppMessage::ClawInputChanged(s))
                        .on_submit(|_| AppMessage::ClawSendCommand)
                        .font(cosmic::font::mono())
                        .size(13)
                        .width(Length::Fill)
                )
                .style(|_: &cosmic::Theme| {
                    cosmic::iced::widget::container::Style {
                        border: cosmic::iced::Border {
                            color: cosmic::iced::Color::from_rgba(0.6, 0.6, 0.6, 0.35),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .width(Length::Fill)
                .into(),
                widget::Space::new(8, 0).into(),
                // Voice button
                widget::button::text(if recording { "⏹" } else { "🎙" })
                    .on_press(if recording {
                        AppMessage::ClawStopRecording
                    } else {
                        AppMessage::ClawStartRecording
                    })
                    .class(if recording {
                        cosmic::theme::Button::Destructive
                    } else {
                        cosmic::theme::Button::Standard
                    })
                    .into(),
                // Image button
                widget::button::text(if has_attachment { "🖼✓" } else { "🖼" })
                    .on_press(AppMessage::ClawPickImage)
                    .class(if has_attachment {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
                    .into(),
                widget::Space::new(4, 0).into(),
                widget::button::suggested(
                    if nl_mode {
                        crate::theme::t(lang, "Ask", "执行")
                    } else {
                        crate::theme::t(lang, "Run", "执行")
                    }
                )
                .on_press(AppMessage::ClawSendCommand)
                .into(),
            ])
            .spacing(4)
            .align_y(Alignment::Center)
            .padding([10, 14])
            .into()
        );

        let input_bar = widget::container(
            widget::column::with_children(input_col_children).spacing(0),
        )
        .style(move |theme: &cosmic::Theme| {
            let bc: cosmic::iced::Color = theme.cosmic().accent_color().into();
            cosmic::iced::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    theme.cosmic().background.base.into()
                )),
                border: cosmic::iced::Border { color: bc, width: 1.0, radius: 6.0.into() },
                ..Default::default()
            }
        })
        .width(Length::Fill);

        // ── Page layout ───────────────────────────────────────────────────────
        let mut page_children: Vec<Element<AppMessage>> = vec![
            header.into(),
            widget::Space::new(0, 4).into(),
            subtitle.into(),
            widget::Space::new(0, 8).into(),
            gw_bar.into(),
            widget::Space::new(0, 8).into(),
            quick_row.into(),
            widget::Space::new(0, 16).into(),
        ];
        page_children.extend(history_items);
        page_children.push(widget::Space::new(0, 32).into());

        let scrollable_content = widget::scrollable(
            widget::column::with_children(page_children)
                .spacing(8)
                .padding([24, 24, 8, 24]),
        )
        .id(CLAW_SCROLL_ID.clone());

        widget::column::with_children(vec![
            scrollable_content.height(Length::Fill).into(),
            widget::container(input_bar)
                .padding([0, 16, 12, 16])
                .width(Length::Fill)
                .into(),
        ])
        .height(Length::Fill)
        .into()
    }
}

fn build_entry_card<'a>(
    entry: &'a ClawEntry,
    color_accent: cosmic::iced::Color,
    color_success: cosmic::iced::Color,
    color_error: cosmic::iced::Color,
    color_stderr: cosmic::iced::Color,
    color_running: cosmic::iced::Color,
    color_muted: cosmic::iced::Color,
) -> Element<'a, AppMessage> {
    match &entry.source {
        ClawEntrySource::Telegram { from } => {
            build_telegram_card(entry, from, color_muted)
        }
        ClawEntrySource::BotChannel { platform, from } => {
            build_bot_channel_card(entry, platform, from, color_muted)
        }
        ClawEntrySource::OpenClaw => {
            build_openclaw_card(entry, color_accent, color_success, color_error, color_muted)
        }
        ClawEntrySource::System => {
            build_system_card(entry, color_muted)
        }
        ClawEntrySource::User => {
            build_user_card(entry, color_accent, color_success, color_error,
                color_stderr, color_running, color_muted)
        }
    }
}

/// Telegram message card.
fn build_telegram_card<'a>(
    entry: &'a ClawEntry,
    from: &'a str,
    color_muted: cosmic::iced::Color,
) -> Element<'a, AppMessage> {
    let color_tg = cosmic::iced::Color::from_rgb(0.28, 0.78, 0.96);
    let color_text = cosmic::iced::Color::from_rgb(0.92, 0.92, 0.92);

    let mut children: Vec<Element<AppMessage>> = vec![
        // Header: badge + sender + platform label
        widget::row::with_children(vec![
            widget::text("✈ ").size(12).class(cosmic::theme::Text::Color(color_tg)).into(),
            widget::text(format!("@{}", from)).size(12).font(cosmic::font::bold())
                .class(cosmic::theme::Text::Color(color_tg)).into(),
            widget::Space::new(Length::Fill, 0).into(),
            widget::text("Telegram").size(10).class(cosmic::theme::Text::Color(color_muted)).into(),
        ]).spacing(4).align_y(Alignment::Center).into(),
        // Message text
        widget::Space::new(0, 4).into(),
        widget::text(&entry.command).size(13)
            .class(cosmic::theme::Text::Color(color_text))
            .width(Length::Fill).into(),
    ];

    // Metadata / output lines
    for (line, _) in &entry.output_lines {
        children.push(
            widget::text(line.as_str()).size(10)
                .class(cosmic::theme::Text::Color(color_muted))
                .width(Length::Fill).into()
        );
    }

    widget::container(
        widget::column::with_children(children).spacing(0).padding([8, 12]),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

/// OpenClaw / Agent reply card.
fn build_openclaw_card<'a>(
    entry: &'a ClawEntry,
    _color_accent: cosmic::iced::Color,
    _color_success: cosmic::iced::Color,
    color_error: cosmic::iced::Color,
    _color_muted: cosmic::iced::Color,
) -> Element<'a, AppMessage> {
    let color_oc    = cosmic::iced::Color::from_rgb(0.28, 0.92, 0.78);
    let color_reply = cosmic::iced::Color::from_rgb(0.88, 0.98, 0.92);
    let color_time  = cosmic::iced::Color::from_rgb(0.45, 0.72, 0.58);

    // Header: badge + agent name
    let mut children: Vec<Element<AppMessage>> = vec![
        widget::row::with_children(vec![
            widget::text("🤖 ").size(13).into(),
            widget::text(&entry.command).size(13).font(cosmic::font::bold())
                .class(cosmic::theme::Text::Color(color_oc))
                .width(Length::Fill)
                .into(),
        ])
        .spacing(0)
        .align_y(Alignment::Center)
        .into(),
        widget::Space::new(0, 6).into(),
    ];

    // Reply content lines
    if !entry.output_lines.is_empty() {
        for (line, is_err) in &entry.output_lines {
            let c = if *is_err { color_error } else { color_reply };
            children.push(
                widget::text(line.as_str())
                    .size(13)
                    .class(cosmic::theme::Text::Color(c))
                    .width(Length::Fill)
                    .into()
            );
        }
    } else {
        children.push(
            widget::text("(no output)").size(12)
                .class(cosmic::theme::Text::Color(
                    cosmic::iced::Color::from_rgb(0.55, 0.55, 0.55)
                ))
                .into()
        );
    }

    // Latency badge
    if let Some(ms) = entry.elapsed_ms {
        let t = if ms < 1000 { format!("{ms}ms") } else { format!("{:.1}s", ms as f64 / 1000.0) };
        children.push(widget::Space::new(0, 4).into());
        children.push(
            widget::text(format!("⏱ {t}")).size(10)
                .class(cosmic::theme::Text::Color(color_time))
                .into()
        );
    }

    widget::container(
        widget::column::with_children(children).spacing(0).padding([10, 12]),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

/// Discord / Matrix / Slack message card.
fn build_bot_channel_card<'a>(
    entry: &'a ClawEntry,
    platform: &'a str,
    from: &'a str,
    color_muted: cosmic::iced::Color,
) -> Element<'a, AppMessage> {
    let (color_label, icon) = match platform {
        "Discord" => (cosmic::iced::Color::from_rgb(0.55, 0.62, 0.98), "🎮"),
        "Matrix"  => (cosmic::iced::Color::from_rgb(0.20, 0.82, 0.62), "🔷"),
        "Slack"   => (cosmic::iced::Color::from_rgb(0.98, 0.42, 0.58), "💬"),
        _         => (cosmic::iced::Color::from_rgb(0.70, 0.70, 0.70), "📡"),
    };
    let color_text = cosmic::iced::Color::from_rgb(0.92, 0.92, 0.92);

    let mut children: Vec<Element<AppMessage>> = vec![
        // Header: icon + sender + platform
        widget::row::with_children(vec![
            widget::text(format!("{} ", icon)).size(12)
                .class(cosmic::theme::Text::Color(color_label)).into(),
            widget::text(from).size(12).font(cosmic::font::bold())
                .class(cosmic::theme::Text::Color(color_label)).into(),
            widget::Space::new(Length::Fill, 0).into(),
            widget::text(platform).size(10)
                .class(cosmic::theme::Text::Color(color_muted)).into(),
        ]).spacing(4).align_y(Alignment::Center).into(),
        // Message text
        widget::Space::new(0, 4).into(),
        widget::text(&entry.command).size(13)
            .class(cosmic::theme::Text::Color(color_text))
            .width(Length::Fill).into(),
    ];

    // Metadata / output lines
    for (line, _) in &entry.output_lines {
        children.push(
            widget::text(line.as_str()).size(10)
                .class(cosmic::theme::Text::Color(color_muted))
                .width(Length::Fill).into()
        );
    }

    widget::container(
        widget::column::with_children(children).spacing(0).padding([8, 12]),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

/// System info card.
fn build_system_card<'a>(
    entry: &'a ClawEntry,
    color_muted: cosmic::iced::Color,
) -> Element<'a, AppMessage> {
    let color_sys = cosmic::iced::Color::from_rgb(0.75, 0.72, 0.65);
    let color_err = cosmic::iced::Color::from_rgb(0.92, 0.30, 0.30);

    let mut children: Vec<Element<AppMessage>> = vec![
        widget::row::with_children(vec![
            widget::text("ℹ ").size(11).class(cosmic::theme::Text::Color(color_sys)).into(),
            widget::text(&entry.command).size(12)
                .class(cosmic::theme::Text::Color(color_sys))
                .width(Length::Fill).into(),
        ]).spacing(0).align_y(Alignment::Center).into(),
    ];

    for (line, is_err) in &entry.output_lines {
        let c = if *is_err { color_err } else { color_muted };
        children.push(
            widget::text(line.as_str()).size(11)
                .class(cosmic::theme::Text::Color(c))
                .width(Length::Fill).into()
        );
    }

    widget::container(
        widget::column::with_children(children).spacing(3).padding([6, 12]),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

/// User command card — original terminal style with status bar.
fn build_user_card<'a>(
    entry: &'a ClawEntry,
    color_accent: cosmic::iced::Color,
    color_success: cosmic::iced::Color,
    color_error: cosmic::iced::Color,
    color_stderr: cosmic::iced::Color,
    color_running: cosmic::iced::Color,
    color_muted: cosmic::iced::Color,
) -> Element<'a, AppMessage> {
    let (status_color, status_icon) = match &entry.status {
        ClawEntryStatus::Running    => (color_running,  "⟳"),
        ClawEntryStatus::Success    => (color_success,  "✓"),
        ClawEntryStatus::Error(_)   => (color_error,    "✗"),
        ClawEntryStatus::Killed     => (color_muted,    "⊘"),
    };

    let elapsed_str = entry.elapsed_ms
        .map(|ms| if ms < 1000 { format!("{}ms", ms) } else { format!("{:.1}s", ms as f64 / 1000.0) })
        .unwrap_or_default();

    let cmd_header = widget::row::with_children(vec![
        widget::container(widget::Space::new(3, 20))
            .style(move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(status_color)),
                border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                ..Default::default()
            })
            .into(),
        widget::Space::new(8, 0).into(),
        widget::text("❯").size(13).font(cosmic::font::bold())
            .class(cosmic::theme::Text::Color(color_accent)).into(),
        widget::Space::new(6, 0).into(),
        widget::text(&entry.command).size(13).font(cosmic::font::bold())
            .class(cosmic::theme::Text::Color(
                cosmic::iced::Color::from_rgb(0.92, 0.92, 0.92)
            )).width(Length::Fill).into(),
        widget::text(format!("{} {}", status_icon, entry.status)).size(11)
            .class(cosmic::theme::Text::Color(status_color)).into(),
        if !elapsed_str.is_empty() {
            widget::text(format!("  {}", elapsed_str)).size(11)
                .class(cosmic::theme::Text::Color(color_muted)).into()
        } else {
            widget::Space::new(0, 0).into()
        },
    ])
    .spacing(0)
    .align_y(Alignment::Center);

    let output_widgets: Vec<Element<AppMessage>> = entry.output_lines.iter().map(|(line, is_stderr)| {
        let color = if *is_stderr { color_stderr } else { cosmic::iced::Color::from_rgb(0.82, 0.82, 0.82) };
        widget::row::with_children(vec![
            widget::Space::new(20, 0).into(),
            widget::text(line.as_str()).size(12)
                .font(cosmic::font::mono())
                .class(cosmic::theme::Text::Color(color))
                .width(Length::Fill)
                .into(),
        ])
        .into()
    }).collect();

    let mut card_children: Vec<Element<AppMessage>> = vec![cmd_header.into()];
    if !output_widgets.is_empty() {
        card_children.push(widget::Space::new(0, 4).into());
        card_children.extend(output_widgets);
    }

    widget::container(
        widget::column::with_children(card_children).spacing(0).padding([8, 12]),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}
