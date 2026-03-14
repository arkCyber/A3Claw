// Server Management UI section for general_settings.rs
use crate::app::{AppMessage, ServerInfo};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

pub fn build_server_mgmt_section<'a>(server_list: &'a [ServerInfo]) -> Element<'a, AppMessage> {
    let color_muted = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);
    let color_ok = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
    let color_warn = cosmic::iced::Color::from_rgb(0.96, 0.72, 0.22);
    let color_err = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);

    let header = widget::row::with_children(vec![
        widget::text(format!("{} servers configured", server_list.len()))
            .size(12).class(cosmic::theme::Text::Color(color_muted))
            .width(Length::Fill).into(),
        widget::button::standard("⟳  Refresh").on_press(AppMessage::ServerList).into(),
    ]).spacing(8).align_y(Alignment::Center);

    let server_cards: Vec<Element<AppMessage>> = if server_list.is_empty() {
        vec![widget::container(
            widget::column::with_children(vec![
                widget::text("No servers found").size(14).font(cosmic::font::bold()).into(),
                widget::Space::new(0, 4).into(),
                widget::text("Click 'Refresh' to load server list from server-ctl")
                    .size(12).class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(2).padding([16, 20]),
        ).class(cosmic::theme::Container::Card).width(Length::Fill).into()]
    } else {
        server_list.iter().map(|server| {
            let status_color = match server.status.as_str() {
                "Running" => color_ok,
                "Stopped" => color_muted,
                "Error" => color_err,
                _ => color_warn,
            };

            let type_badge = widget::container(widget::text(&server.server_type).size(10))
                .padding([2, 7])
                .class(cosmic::theme::Container::custom(move |_| {
                    cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgba(0.28, 0.65, 0.95, 0.18))),
                        border: cosmic::iced::Border {
                            radius: 4.0.into(), width: 1.0,
                            color: cosmic::iced::Color::from_rgba(0.28, 0.65, 0.95, 0.45),
                        },
                        text_color: Some(cosmic::iced::Color::from_rgb(0.28, 0.65, 0.95)),
                        ..Default::default()
                    }
                }));

            let status_badge = widget::container(widget::text(&server.status).size(10))
                .padding([2, 7])
                .class(cosmic::theme::Container::custom(move |_| {
                    cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgba(status_color.r, status_color.g, status_color.b, 0.18))),
                        border: cosmic::iced::Border {
                            radius: 4.0.into(), width: 1.0,
                            color: cosmic::iced::Color::from_rgba(status_color.r, status_color.g, status_color.b, 0.45),
                        },
                        text_color: Some(status_color),
                        ..Default::default()
                    }
                }));

            let mut info_parts: Vec<Element<AppMessage>> = vec![
                widget::text(format!("🌐 {}", server.endpoint)).size(11)
                    .class(cosmic::theme::Text::Color(color_muted)).into(),
            ];

            if let Some(cpu) = server.cpu_usage {
                info_parts.push(widget::text("  ·  ").size(11)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.4, 0.4, 0.4))).into());
                info_parts.push(widget::text(format!("CPU: {:.1}%", cpu)).size(11)
                    .class(cosmic::theme::Text::Color(color_muted)).into());
            }

            if let Some(mem) = server.memory_mb {
                info_parts.push(widget::text("  ·  ").size(11)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.4, 0.4, 0.4))).into());
                info_parts.push(widget::text(format!("MEM: {} MB", mem)).size(11)
                    .class(cosmic::theme::Text::Color(color_muted)).into());
            }

            let is_running = server.status == "Running";
            let control_buttons = if is_running {
                widget::column::with_children(vec![
                    widget::button::destructive("Stop")
                        .on_press(AppMessage::ServerStop(server.server_id.clone())).into(),
                    widget::Space::new(0, 4).into(),
                    widget::button::standard("Restart")
                        .on_press(AppMessage::ServerRestart(server.server_id.clone())).into(),
                    widget::Space::new(0, 4).into(),
                    widget::button::text("Health Check")
                        .on_press(AppMessage::ServerHealthCheck(server.server_id.clone())).into(),
                ]).spacing(0).align_x(Alignment::End)
            } else {
                widget::column::with_children(vec![
                    widget::button::suggested("Start")
                        .on_press(AppMessage::ServerStart(server.server_id.clone())).into(),
                    widget::Space::new(0, 4).into(),
                    widget::button::text("Health Check")
                        .on_press(AppMessage::ServerHealthCheck(server.server_id.clone())).into(),
                ]).spacing(0).align_x(Alignment::End)
            };

            widget::container(
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(&server.name).size(14).font(cosmic::font::bold()).into(),
                        widget::Space::new(0, 4).into(),
                        widget::row::with_children(vec![
                            type_badge.into(),
                            widget::Space::new(5, 0).into(),
                            status_badge.into(),
                        ]).spacing(0).into(),
                        widget::Space::new(0, 6).into(),
                        widget::row::with_children(info_parts).spacing(0).into(),
                    ]).spacing(0).width(Length::Fill).into(),
                    control_buttons.into(),
                ]).spacing(12).align_y(Alignment::Center).padding([12, 16]),
            ).class(cosmic::theme::Container::Card).width(Length::Fill).into()
        }).collect()
    };

    let mut children: Vec<Element<AppMessage>> = vec![
        header.into(),
        widget::Space::new(0, 8).into(),
    ];
    children.extend(server_cards);

    super::general_settings::gs_section_with_content(
        "Inference Server Management",
        "Manage local inference servers (Ollama, llama.cpp) and remote API endpoints",
        widget::column::with_children(children).spacing(4).into(),
    )
}
