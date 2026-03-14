//! # `view_bot.rs` — Bot Integrations & Local API Endpoints Page
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Renders the "Bot & API" page, split into two sections:
//!
//! - **Bot Integrations** — configure Telegram, Discord, WeChat, Slack, or
//!   custom bots.  Each bot card shows the platform selector, token / API key
//!   field, optional webhook URL, and enable / disable controls.
//!
//! - **Local API Endpoints** — expose HTTP, WebSocket, gRPC, or MQTT servers
//!   on the local machine so mobile apps or IoT devices can reach OpenClaw+.
//!   Each endpoint card shows transport selector, bind host, port, auth token,
//!   and start / stop controls.
//!
//! ## Warm-tone notes
//! Bot and API status text is colour-coded using warm tones:
//! - Connected / Running → warm green.
//! - Connecting / Starting → amber.
//! - Error → warm red.
//! - Disconnected / Stopped → default theme colour.

use crate::app::StoreMessage;
use crate::i18n;
use crate::types::{
    ApiEndpointStatus, ApiTransport, BotConfig, BotPlatform, BotStatus, LocalApiConfig,
};
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget;
use cosmic::Element;

// ── Warm-tone palette ───────────────────────────────────────────────────────────────

/// Warm green for connected / running status.
const STATUS_OK: Color      = Color { r: 0.20, g: 0.62, b: 0.28, a: 1.0 };
/// Amber for in-progress / transitioning status.
const STATUS_BUSY: Color    = Color { r: 0.85, g: 0.52, b: 0.08, a: 1.0 };
/// Warm red for error status.
const STATUS_ERROR: Color   = Color { r: 0.80, g: 0.22, b: 0.18, a: 1.0 };

pub fn view_bot_api(
    bots: &[BotConfig],
    local_apis: &[LocalApiConfig],
) -> Element<'static, StoreMessage> {
    let s = i18n::strings();
    // ── Bot integrations section ──────────────────────────────────────────────
    let mut bots_col = widget::column()
        .push(
            widget::row()
                .push(
                    widget::column()
                        .push(widget::text(s.bot_title).size(15))
                        .push(widget::text(s.bot_subtitle).size(12))
                        .spacing(2),
                )
                .push(widget::horizontal_space())
                .push(
                    widget::button::suggested(s.bot_add)
                        .on_press(StoreMessage::BotAdd),
                )
                .align_y(Alignment::Center),
        )
        .push(widget::divider::horizontal::default())
        .spacing(10);

    for (i, bot) in bots.iter().enumerate() {
        bots_col = bots_col.push(bot_card(i, bot));
    }

    // ── Local API endpoints section ───────────────────────────────────────────
    let mut apis_col = widget::column()
        .push(
            widget::row()
                .push(
                    widget::column()
                        .push(widget::text(s.api_title).size(15))
                        .push(widget::text(s.api_subtitle).size(12))
                        .spacing(2),
                )
                .push(widget::horizontal_space())
                .push(
                    widget::button::suggested(s.api_add)
                        .on_press(StoreMessage::ApiAdd),
                )
                .align_y(Alignment::Center),
        )
        .push(widget::divider::horizontal::default())
        .spacing(10);

    for (i, api) in local_apis.iter().enumerate() {
        apis_col = apis_col.push(api_card(i, api));
    }

    widget::scrollable(
        widget::column()
            .push(bots_col)
            .push(widget::vertical_space())
            .push(apis_col)
            .spacing(20)
            .padding([16, 16]),
    )
    .height(Length::Fill)
    .into()
}

// ── Bot card ──────────────────────────────────────────────────────────────────

pub fn bot_card(i: usize, b: &BotConfig) -> Element<'static, StoreMessage> {
    let name     = b.name.clone();
    let token    = b.token.clone();
    let webhook  = b.webhook_url.clone();
    let enabled  = b.enabled;
    let platform = b.platform.clone();

    let s = i18n::strings();
    let (status_str, status_color): (String, Option<Color>) = match &b.status {
        BotStatus::Disconnected           =>
            (s.bot_disconnected.to_string(), None),
        BotStatus::Connecting             =>
            (s.bot_connecting.to_string(), Some(STATUS_BUSY)),
        BotStatus::Connected { username } =>
            (format!("\u{2705} @{username}"), Some(STATUS_OK)),
        BotStatus::Error(e)               =>
            (format!("\u{274c} {e}"), Some(STATUS_ERROR)),
    };
    let bot_status_widget: Element<'static, StoreMessage> = if let Some(c) = status_color {
        widget::text(status_str).size(11).class(cosmic::theme::Text::Color(c)).into()
    } else {
        widget::text(status_str).size(11).into()
    };

    let platforms: &[(&str, BotPlatform)] = &[
        ("Telegram", BotPlatform::Telegram),
        ("Discord",  BotPlatform::Discord),
        ("WeChat",   BotPlatform::WeChat),
        ("Slack",    BotPlatform::Slack),
        ("Custom",   BotPlatform::Custom),
    ];
    let mut plat_row = widget::row().spacing(4);
    for (lbl, pl) in platforms {
        let active = platform == *pl;
        let pl2 = pl.clone();
        let lbl = *lbl;
        plat_row = plat_row.push(if active {
            widget::button::suggested(lbl)
                .on_press(StoreMessage::BotPlatformChanged(i, pl2))
        } else {
            widget::button::text(lbl)
                .on_press(StoreMessage::BotPlatformChanged(i, pl2))
        });
    }

    let toggle_lbl = if enabled { s.bot_disable } else { s.bot_enable };

    widget::container(
        widget::column()
            .push(
                widget::row()
                    .push(widget::text(name).size(15))
                    .push(widget::horizontal_space())
                    .push(bot_status_widget)
                    .align_y(Alignment::Center),
            )
            .push(plat_row)
            .push(widget::divider::horizontal::default())
            .push(widget::text(s.bot_token).size(11))
            .push(
                widget::text_input("1234567890:ABC…", token)
                    .on_input(move |v| StoreMessage::BotTokenChanged(i, v))
                    .width(Length::Fill),
            )
            .push(widget::text(s.bot_webhook).size(11))
            .push(
                widget::text_input(
                    "https://your-server.example/webhook",
                    webhook,
                )
                .on_input(move |v| StoreMessage::BotWebhookChanged(i, v))
                .width(Length::Fill),
            )
            .push(
                widget::row()
                    .push(widget::horizontal_space())
                    .push(
                        if enabled {
                            widget::button::destructive(toggle_lbl)
                                .on_press(StoreMessage::BotToggle(i))
                        } else {
                            widget::button::suggested(toggle_lbl)
                                .on_press(StoreMessage::BotToggle(i))
                        },
                    )
                    .push(
                        widget::button::destructive(s.bot_remove)
                            .on_press(StoreMessage::BotRemove(i)),
                    )
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .spacing(6)
            .padding(14),
    )
    .width(Length::Fill)
    .into()
}

// ── Local API endpoint card ───────────────────────────────────────────────────

pub fn api_card(i: usize, a: &LocalApiConfig) -> Element<'static, StoreMessage> {
    let name      = a.name.clone();
    let host      = a.bind_host.clone();
    let port      = a.port.to_string();
    let token     = a.auth_token.clone();
    let enabled   = a.enabled;
    let transport = a.transport.clone();

    let s = i18n::strings();
    let (status_str, status_color): (String, Option<Color>) = match &a.status {
        ApiEndpointStatus::Stopped =>
            (s.api_stopped.to_string(), None),
        ApiEndpointStatus::Starting =>
            (s.api_starting.to_string(), Some(STATUS_BUSY)),
        ApiEndpointStatus::Running { connections } =>
            (format!("\u{25b6} Running  \u{b7}  {connections} connection(s)"), Some(STATUS_OK)),
        ApiEndpointStatus::Error(e) =>
            (format!("\u{274c} {e}"), Some(STATUS_ERROR)),
    };
    let api_status_widget: Element<'static, StoreMessage> = if let Some(c) = status_color {
        widget::text(status_str).size(11).class(cosmic::theme::Text::Color(c)).into()
    } else {
        widget::text(status_str).size(11).into()
    };

    let transports: &[(&str, ApiTransport)] = &[
        ("HTTP",      ApiTransport::Http),
        ("WebSocket", ApiTransport::WebSocket),
        ("gRPC",      ApiTransport::Grpc),
        ("MQTT",      ApiTransport::Mqtt),
    ];
    let mut trans_row = widget::row().spacing(4);
    for (lbl, t) in transports {
        let active = transport == *t;
        let t2 = t.clone();
        let lbl = *lbl;
        trans_row = trans_row.push(if active {
            widget::button::suggested(lbl)
                .on_press(StoreMessage::ApiTransportChanged(i, t2))
        } else {
            widget::button::text(lbl)
                .on_press(StoreMessage::ApiTransportChanged(i, t2))
        });
    }

    let toggle_lbl = if enabled { s.api_stop } else { s.api_start };

    widget::container(
        widget::column()
            .push(
                widget::row()
                    .push(widget::text(name).size(15))
                    .push(widget::horizontal_space())
                    .push(api_status_widget)
                    .align_y(Alignment::Center),
            )
            .push(trans_row)
            .push(widget::divider::horizontal::default())
            .push(
                widget::row()
                    .push(
                        widget::column()
                            .push(widget::text(s.api_bind_host).size(11))
                            .push(
                                widget::text_input("127.0.0.1", host)
                                    .on_input(move |v| StoreMessage::ApiHostChanged(i, v))
                                    .width(Length::Fixed(160.0)),
                            )
                            .spacing(3),
                    )
                    .push(
                        widget::column()
                            .push(widget::text(s.api_port).size(11))
                            .push(
                                widget::text_input("8765", port)
                                    .on_input(move |v| StoreMessage::ApiPortChanged(i, v))
                                    .width(Length::Fixed(80.0)),
                            )
                            .spacing(3),
                    )
                    .spacing(12)
                    .align_y(Alignment::End),
            )
            .push(widget::text(s.api_auth_token).size(11))
            .push(
                widget::text_input("Bearer token…", token)
                    .on_input(move |v| StoreMessage::ApiTokenChanged(i, v))
                    .width(Length::Fill),
            )
            .push(
                widget::row()
                    .push(widget::horizontal_space())
                    .push(
                        if enabled {
                            widget::button::destructive(toggle_lbl)
                                .on_press(StoreMessage::ApiToggle(i))
                        } else {
                            widget::button::suggested(toggle_lbl)
                                .on_press(StoreMessage::ApiToggle(i))
                        },
                    )
                    .push(
                        widget::button::destructive(s.api_remove)
                            .on_press(StoreMessage::ApiRemove(i)),
                    )
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .spacing(6)
            .padding(14),
    )
    .width(Length::Fill)
    .into()
}
