use crate::app::{AppMessage, OllamaModel};
use crate::i18n::strings_for;
use crate::theme::Language;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use openclaw_security::{AgentKind, AiProvider, ChannelKind, SecurityConfig};

pub struct GeneralSettingsPage;

impl GeneralSettingsPage {
    pub fn view<'a>(
        lang: Language,
        warm_theme: bool,
        ai_endpoint: &'a str,
        ai_model: &'a str,
        available_models: &'a [OllamaModel],
        model_download_input: &'a str,
        model_search: &'a str,
        download_status: Option<&'a (String, String, u8)>,
        github_orgs_input: &'a str,
        github_repos_input: &'a str,
        agent_entry_input: &'a str,
        config: &'a SecurityConfig,
        ai_test_status: Option<&'a (bool, String)>,
        channel_test_status: &'a [Option<(bool, String)>],
        ai_max_tokens_input: &'a str,
        ai_temperature_input: &'a str,
    ) -> Element<'a, AppMessage> {
        let s = strings_for(lang);
        let color_muted = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);

        // ── Appearance & Language ─────────────────────────────────────────────
        let theme_btn = widget::button::text(if warm_theme {
            s.set_theme_default
        } else {
            s.set_theme_warm
        })
        .on_press(AppMessage::ToggleTheme)
        .class(cosmic::theme::Button::Standard);

        let section_appearance = gs_section_with_content(
            s.set_appearance,
            s.set_appearance_sub,
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(s.set_language).size(13).font(cosmic::font::bold()).into(),
                        widget::text(s.set_language_hint).size(11)
                            .class(cosmic::theme::Text::Color(color_muted)).into(),
                    ]).spacing(2).width(Length::Fill).into(),
                    language_grid(lang),
                ]).spacing(12).align_y(Alignment::Center).into(),
                widget::divider::horizontal::default().into(),
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(s.set_theme).size(13).font(cosmic::font::bold()).into(),
                    ]).spacing(2).width(Length::Fill).into(),
                    theme_btn.into(),
                ]).spacing(12).align_y(Alignment::Center).into(),
            ]).spacing(12).into(),
        );

        // ── AI Inference ──────────────────────────────────────────────────────
        let section_ai = gs_section_with_content(
            s.set_ai,
            s.set_ai_sub,
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(s.set_endpoint).size(13).font(cosmic::font::bold()).into(),
                        widget::text(s.set_endpoint_hint).size(11)
                            .class(cosmic::theme::Text::Color(color_muted)).into(),
                    ]).spacing(2).width(Length::Fill).into(),
                    widget::text_input("http://localhost:11434", ai_endpoint)
                        .on_input(AppMessage::AiEndpointChanged)
                        .width(Length::Fixed(280.0)).into(),
                ]).spacing(12).align_y(Alignment::Center).into(),
                widget::divider::horizontal::light().into(),
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(s.set_model).size(13).font(cosmic::font::bold()).into(),
                        widget::text(s.set_model_hint).size(11)
                            .class(cosmic::theme::Text::Color(color_muted)).into(),
                    ]).spacing(2).width(Length::Fill).into(),
                    widget::text_input("qwen2.5:0.5b", ai_model)
                        .on_input(AppMessage::AiModelChanged)
                        .width(Length::Fixed(200.0)).into(),
                ]).spacing(12).align_y(Alignment::Center).into(),
            ]).spacing(12).into(),
        );

        // ── AI Model Management ───────────────────────────────────────────────
        let section_model_mgmt = build_model_mgmt_section(
            ai_model, available_models, model_download_input, model_search, download_status,
        );

        // ── Agent Runtime ─────────────────────────────────────────────────────
        let section_agent = build_agent_section(config, agent_entry_input);

        // ── GitHub Policy ─────────────────────────────────────────────────────
        let section_github = build_github_section(lang, config, github_orgs_input, github_repos_input);

        // ── OpenClaw AI Model ─────────────────────────────────────────────
        let section_openclaw_ai = build_openclaw_ai_section(
            config, ai_test_status,
            ai_max_tokens_input, ai_temperature_input,
        );

        // ── Communication Channels ────────────────────────────────────────────
        let section_channels = build_channels_section(config, channel_test_status);

        // ── Config file path ──────────────────────────────────────────────────
        let config_path = dirs::config_dir()
            .unwrap_or_default()
            .join("openclaw-plus")
            .join("config.toml")
            .display()
            .to_string();

        let section_config = gs_section_with_content(
            "Configuration File",
            "Location of the active configuration on disk",
            widget::container(
                widget::row::with_children(vec![
                    widget::text("📄").size(16).into(),
                    widget::column::with_children(vec![
                        widget::text("config.toml").size(13).font(cosmic::font::bold()).into(),
                        widget::text(config_path.clone()).size(11)
                            .class(cosmic::theme::Text::Color(color_muted)).into(),
                    ]).spacing(2).width(Length::Fill).into(),
                ])
                .spacing(10).align_y(Alignment::Center).padding([8, 12]),
            )
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .into(),
        );

        // ── Page layout ───────────────────────────────────────────────────────
        widget::scrollable(
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    crate::icons::gear(20).into(),
                    widget::Space::new(10, 0).into(),
                    widget::text(crate::theme::t(lang, "General Settings", "通用设置"))
                        .size(22).font(cosmic::font::bold()).into(),
                ]).spacing(0).align_y(Alignment::Center).into(),
                widget::Space::new(0, 4).into(),
                widget::text(crate::theme::t(
                    lang,
                    "Appearance, AI models, agent runtime and integrations",
                    "外观、AI 模型、Agent 运行时及集成配置",
                ))
                .size(13).class(cosmic::theme::Text::Color(color_muted)).into(),
                widget::Space::new(0, 20).into(),
                section_appearance,
                widget::Space::new(0, 12).into(),
                section_ai,
                widget::Space::new(0, 12).into(),
                section_model_mgmt,
                widget::Space::new(0, 12).into(),
                section_agent,
                widget::Space::new(0, 12).into(),
                section_github,
                widget::Space::new(0, 12).into(),
                section_openclaw_ai,
                widget::Space::new(0, 12).into(),
                section_channels,
                widget::Space::new(0, 12).into(),
                section_config,
                widget::Space::new(0, 32).into(),
            ])
            .padding(24)
            .spacing(4),
        )
        .into()
    }
}

// ── AI Model Management section ───────────────────────────────────────────────
fn build_model_mgmt_section<'a>(
    ai_model: &'a str,
    available_models: &'a [OllamaModel],
    model_download_input: &'a str,
    model_search: &'a str,
    download_status: Option<&'a (String, String, u8)>,
) -> Element<'a, AppMessage> {
    let color_muted = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);

    let model_header = widget::row::with_children(vec![
        widget::text(format!("{} models installed", available_models.len()))
            .size(12).class(cosmic::theme::Text::Color(color_muted)).width(Length::Fill).into(),
        widget::button::standard("⟳  Refresh").on_press(AppMessage::AiListModels).into(),
    ]).spacing(8).align_y(Alignment::Center);

    let search_bar = widget::text_input("🔍  Filter models...", model_search)
        .on_input(AppMessage::ModelSearchChanged)
        .width(Length::Fill);

    let filtered: Vec<&OllamaModel> = available_models.iter().filter(|m| {
        model_search.is_empty()
            || m.name.to_lowercase().contains(&model_search.to_lowercase())
            || m.family.to_lowercase().contains(&model_search.to_lowercase())
    }).collect();

    let model_cards: Vec<Element<AppMessage>> = if available_models.is_empty() {
        vec![widget::container(
            widget::column::with_children(vec![
                widget::text("No models installed").size(14).font(cosmic::font::bold()).into(),
                widget::Space::new(0, 4).into(),
                widget::text("Click 'Refresh' to check, or download a model below.")
                    .size(12).class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(2).padding([16, 20]),
        ).class(cosmic::theme::Container::Card).width(Length::Fill).into()]
    } else if filtered.is_empty() {
        vec![widget::text(format!("No models match '{}'", model_search))
            .size(13).class(cosmic::theme::Text::Color(color_muted)).into()]
    } else {
        filtered.iter().map(|m| {
            let is_active = m.name == ai_model;
            let name_color = if is_active {
                cosmic::iced::Color::from_rgb(0.28, 0.78, 0.96)
            } else {
                cosmic::iced::Color::from_rgb(0.92, 0.92, 0.92)
            };
            let badge = |text: String, r: f32, g: f32, b: f32| -> Element<'a, AppMessage> {
                widget::container(widget::text(text).size(10))
                    .padding([2, 7])
                    .class(cosmic::theme::Container::custom(move |_| {
                        cosmic::iced::widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(
                                cosmic::iced::Color::from_rgba(r, g, b, 0.18))),
                            border: cosmic::iced::Border {
                                radius: 4.0.into(), width: 1.0,
                                color: cosmic::iced::Color::from_rgba(r, g, b, 0.45),
                            },
                            text_color: Some(cosmic::iced::Color::from_rgb(r, g, b)),
                            ..Default::default()
                        }
                    }))
                    .into()
            };
            let mut badges: Vec<Element<AppMessage>> = Vec::new();
            if is_active { badges.push(badge("● ACTIVE".to_string(), 0.28, 0.78, 0.42)); }
            if !m.parameter_size.is_empty() { badges.push(badge(m.parameter_size.clone(), 0.28, 0.65, 0.95)); }
            if !m.quantization.is_empty() { badges.push(badge(m.quantization.clone(), 0.96, 0.62, 0.12)); }
            if !m.family.is_empty() { badges.push(badge(m.family.clone(), 0.72, 0.45, 0.95)); }

            let use_btn = if is_active {
                widget::button::text("✓ In Use").on_press(AppMessage::Noop)
                    .class(cosmic::theme::Button::Text)
            } else {
                widget::button::standard("Use").on_press(AppMessage::AiSetActiveModel(m.name.clone()))
            };

            widget::container(
                widget::column::with_children(vec![
                    widget::row::with_children(vec![
                        widget::column::with_children(vec![
                            widget::text(&m.name).size(14).font(cosmic::font::bold())
                                .class(cosmic::theme::Text::Color(name_color)).into(),
                            widget::Space::new(0, 4).into(),
                            widget::row::with_children(badges).spacing(5).into(),
                            widget::Space::new(0, 6).into(),
                            widget::row::with_children(vec![
                                widget::text(format!("💾 {}", m.size_display())).size(11)
                                    .class(cosmic::theme::Text::Color(color_muted)).into(),
                                widget::text("  ·  ").size(11)
                                    .class(cosmic::theme::Text::Color(
                                        cosmic::iced::Color::from_rgb(0.4, 0.4, 0.4))).into(),
                                widget::text(format!("📅 {}", m.modified_display())).size(11)
                                    .class(cosmic::theme::Text::Color(color_muted)).into(),
                            ]).spacing(0).into(),
                        ]).spacing(0).width(Length::Fill).into(),
                        widget::column::with_children(vec![
                            use_btn.into(),
                            widget::Space::new(0, 6).into(),
                            widget::button::destructive("Delete")
                                .on_press(AppMessage::AiDeleteModel(m.name.clone())).into(),
                        ]).spacing(0).align_x(Alignment::End).into(),
                    ]).spacing(12).align_y(Alignment::Center).into(),
                ]).padding([12, 16]),
            ).class(cosmic::theme::Container::Card).width(Length::Fill).into()
        }).collect()
    };

    let progress_widget: Option<Element<AppMessage>> = download_status.map(|(model, status, percent)| {
        widget::container(
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::text(format!("⬇  Downloading: {}", model))
                        .size(13).font(cosmic::font::bold()).width(Length::Fill).into(),
                    widget::text(format!("{}%", percent)).size(13)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.28, 0.78, 0.42))).into(),
                ]).spacing(8).align_y(Alignment::Center).into(),
                widget::Space::new(0, 6).into(),
                widget::container(
                    widget::container(widget::Space::new(0, 0))
                        .width(Length::Fixed((*percent as f32 / 100.0) * 340.0))
                        .height(Length::Fixed(4.0))
                        .class(cosmic::theme::Container::custom(|_| {
                            cosmic::iced::widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color::from_rgb(0.28, 0.78, 0.42))),
                                border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                                ..Default::default()
                            }
                        })),
                )
                .width(Length::Fill).height(Length::Fixed(4.0))
                .class(cosmic::theme::Container::custom(|_| {
                    cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                        border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                }))
                .into(),
                widget::Space::new(0, 4).into(),
                widget::text(status.as_str()).size(11)
                    .class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(0).padding([10, 14]),
        ).class(cosmic::theme::Container::Card).width(Length::Fill).into()
    });

    let recommended: &[(&str, &str, &str)] = &[
        ("qwen2.5:0.5b",   "0.5B · ~400MB",  "Fast, lightweight, great for testing"),
        ("qwen2.5:7b",     "7B  · ~4.7GB",   "Balanced performance and quality"),
        ("llama3.2:3b",    "3B  · ~2.0GB",   "Meta's latest compact model"),
        ("llama3.1:8b",    "8B  · ~4.9GB",   "Meta's flagship open model"),
        ("deepseek-r1:7b", "7B  · ~4.7GB",   "Strong reasoning capabilities"),
        ("mistral:7b",     "7B  · ~4.1GB",   "Fast and efficient European model"),
        ("phi4:14b",       "14B · ~8.9GB",   "Microsoft's high-quality model"),
        ("gemma3:4b",      "4B  · ~3.3GB",   "Google's efficient small model"),
    ];

    let rec_cards: Vec<Element<AppMessage>> = recommended.iter().map(|(name, size_hint, desc)| {
        let installed = available_models.iter().any(|m| &m.name.as_str() == name);
        widget::container(
            widget::row::with_children(vec![
                widget::column::with_children(vec![
                    widget::text(*name).size(13).font(cosmic::font::bold()).into(),
                    widget::row::with_children(vec![
                        widget::text(*size_hint).size(11)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.28, 0.65, 0.95))).into(),
                        widget::text("  ·  ").size(11)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.4, 0.4, 0.4))).into(),
                        widget::text(*desc).size(11)
                            .class(cosmic::theme::Text::Color(color_muted)).into(),
                    ]).spacing(0).into(),
                ]).spacing(3).width(Length::Fill).into(),
                if installed {
                    widget::button::text("✓ Installed").on_press(AppMessage::Noop)
                        .class(cosmic::theme::Button::Text).into()
                } else {
                    widget::button::standard("⬇ Download")
                        .on_press(AppMessage::AiPullModel(name.to_string())).into()
                },
            ]).spacing(12).align_y(Alignment::Center).padding([8, 12]),
        ).class(cosmic::theme::Container::Card).width(Length::Fill).into()
    }).collect();

    let custom_dl = widget::column::with_children(vec![
        widget::text("Custom Model Name").size(12).font(cosmic::font::bold()).into(),
        widget::Space::new(0, 6).into(),
        widget::row::with_children(vec![
            widget::text_input("e.g., qwen2.5:14b, codellama:13b", model_download_input)
                .on_input(AppMessage::ModelDownloadInputChanged)
                .width(Length::Fill).into(),
            widget::button::suggested("⬇ Download")
                .on_press(if model_download_input.is_empty() {
                    AppMessage::Noop
                } else {
                    AppMessage::AiPullModel(model_download_input.to_string())
                }).into(),
        ]).spacing(8).align_y(Alignment::Center).into(),
        widget::Space::new(0, 4).into(),
        widget::text("Browse all models at ollama.com/library").size(11)
            .class(cosmic::theme::Text::Color(color_muted)).into(),
    ]).spacing(0);

    let mut children: Vec<Element<AppMessage>> = vec![
        model_header.into(),
        widget::Space::new(0, 8).into(),
        search_bar.into(),
        widget::Space::new(0, 8).into(),
    ];
    if let Some(p) = progress_widget {
        children.push(p);
        children.push(widget::Space::new(0, 8).into());
    }
    children.extend(model_cards);
    children.push(widget::Space::new(0, 16).into());
    children.push(widget::divider::horizontal::light().into());
    children.push(widget::Space::new(0, 12).into());
    children.push(widget::text("Recommended Models").size(13).font(cosmic::font::bold()).into());
    children.push(widget::Space::new(0, 8).into());
    children.extend(rec_cards);
    children.push(widget::Space::new(0, 16).into());
    children.push(widget::divider::horizontal::light().into());
    children.push(widget::Space::new(0, 12).into());
    children.push(custom_dl.into());

    gs_section_with_content(
        "AI Model Management",
        "Install, switch and remove local Ollama models",
        widget::column::with_children(children).spacing(4).into(),
    )
}

// ── Agent Runtime section ─────────────────────────────────────────────────────
fn build_agent_section<'a>(
    config: &'a SecurityConfig,
    agent_entry_input: &'a str,
) -> Element<'a, AppMessage> {
    let color_muted = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);
    let accent = cosmic::iced::Color::from_rgb(0.28, 0.78, 0.96);

    let agent_cards: Vec<Element<AppMessage>> = AgentKind::all().iter().map(|kind| {
        let is_active = *kind == config.agent.kind;
        widget::container(
            widget::row::with_children(vec![
                widget::column::with_children(vec![
                    widget::text(kind.to_string()).size(13).font(cosmic::font::bold())
                        .class(if is_active {
                            cosmic::theme::Text::Color(accent)
                        } else {
                            cosmic::theme::Text::Default
                        })
                        .into(),
                    widget::text(kind.description()).size(11)
                        .class(cosmic::theme::Text::Color(color_muted)).into(),
                ]).spacing(3).width(Length::Fill).into(),
                if is_active {
                    widget::button::text("✓ Active").on_press(AppMessage::Noop)
                        .class(cosmic::theme::Button::Text).into()
                } else {
                    widget::button::standard("Select")
                        .on_press(AppMessage::SetAgentKind(kind.clone())).into()
                },
            ]).spacing(12).align_y(Alignment::Center).padding([8, 12]),
        ).class(cosmic::theme::Container::Card).width(Length::Fill).into()
    }).collect();

    gs_section_with_content(
        "Agent Runtime",
        "Select which AI agent framework to run in the sandbox",
        widget::column::with_children({
            let mut ch: Vec<Element<AppMessage>> = agent_cards;
            ch.push(widget::Space::new(0, 12).into());
            ch.push(widget::divider::horizontal::light().into());
            ch.push(widget::Space::new(0, 8).into());
            ch.push(
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text("Entry Point Path").size(13).font(cosmic::font::bold()).into(),
                        widget::text("Path to the agent's main file (JS bundle, Python script, or .wasm)")
                            .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                    ]).spacing(2).width(Length::Fill).into(),
                    widget::text_input("e.g., /path/to/agent/index.js", agent_entry_input)
                        .on_input(AppMessage::AgentEntryPathChanged)
                        .width(Length::Fixed(280.0)).into(),
                ]).spacing(12).align_y(Alignment::Center).into(),
            );
            ch
        }).spacing(6).into(),
    )
}

// ── GitHub Policy section ─────────────────────────────────────────────────────
fn build_github_section<'a>(
    lang: Language,
    config: &'a SecurityConfig,
    github_orgs_input: &'a str,
    github_repos_input: &'a str,
) -> Element<'a, AppMessage> {
    let color_muted = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);
    let gh = &config.github;

    gs_section_with_content(
        "GitHub / Git Security Policy",
        "Control how the AI agent interacts with Git and GitHub",
        widget::column::with_children(vec![
            gs_toggle_row(lang, "Deny Force Push", gh.deny_force_push,
                "Block git push --force and --force-with-lease",
                AppMessage::ToggleGithubDenyForcePush),
            gs_toggle_row(lang, "Confirm Push", gh.confirm_push,
                "Require confirmation before any git push",
                AppMessage::ToggleGithubConfirmPush),
            gs_toggle_row(lang, "Protect Default Branch", gh.protect_default_branch,
                "Extra confirmation when pushing to main/master/develop",
                AppMessage::ToggleGithubProtectDefaultBranch),
            gs_toggle_row(lang, "Confirm Branch Delete", gh.confirm_branch_delete,
                "Require confirmation before deleting a remote branch",
                AppMessage::ToggleGithubConfirmBranchDelete),
            gs_toggle_row(lang, "Confirm History Rewrite", gh.confirm_history_rewrite,
                "Require confirmation for git reset --hard / rebase",
                AppMessage::ToggleGithubConfirmHistoryRewrite),
            gs_toggle_row(lang, "Intercept GitHub API", gh.intercept_github_api,
                "Monitor and control calls to api.github.com",
                AppMessage::ToggleGithubInterceptApi),
            widget::divider::horizontal::light().into(),
            widget::row::with_children(vec![
                widget::column::with_children(vec![
                    widget::text("Allowed Orgs").size(13).font(cosmic::font::bold()).into(),
                    widget::text("Comma-separated GitHub org names (empty = all)")
                        .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                ]).spacing(2).width(Length::Fill).into(),
                widget::text_input("e.g., my-org, another-org", github_orgs_input)
                    .on_input(AppMessage::GithubAllowedOrgsChanged)
                    .width(Length::Fixed(240.0)).into(),
            ]).spacing(12).align_y(Alignment::Center).into(),
            widget::row::with_children(vec![
                widget::column::with_children(vec![
                    widget::text("Allowed Repos").size(13).font(cosmic::font::bold()).into(),
                    widget::text("Comma-separated repo paths (empty = all in allowed orgs)")
                        .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                ]).spacing(2).width(Length::Fill).into(),
                widget::text_input("e.g., my-org/repo-a, my-org/repo-b", github_repos_input)
                    .on_input(AppMessage::GithubAllowedReposChanged)
                    .width(Length::Fixed(240.0)).into(),
            ]).spacing(12).align_y(Alignment::Center).into(),
        ]).spacing(12).into(),
    )
}

// ── Shared UI helpers ─────────────────────────────────────────────────────────

fn language_grid<'a>(lang: Language) -> Element<'a, AppMessage> {
    let langs = Language::all();
    let mut rows: Vec<Element<'a, AppMessage>> = Vec::new();
    let mut i = 0;
    while i < langs.len() {
        let mut row_children: Vec<Element<'a, AppMessage>> = Vec::new();
        for j in 0..4 {
            if let Some(&l) = langs.get(i + j) {
                row_children.push(
                    widget::button::text(l.display_name())
                        .on_press(AppMessage::SetLanguage(l))
                        .class(if lang == l {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::Standard
                        })
                        .into(),
                );
            }
        }
        rows.push(widget::row::with_children(row_children).spacing(8).into());
        i += 4;
    }
    widget::column::with_children(rows).spacing(8).into()
}

fn gs_section_with_content<'a>(
    title: &'a str,
    subtitle: &'a str,
    content: Element<'a, AppMessage>,
) -> Element<'a, AppMessage> {
    widget::container(
        widget::column::with_children(vec![
            widget::text(title).size(15).font(cosmic::font::bold()).into(),
            widget::text(subtitle).size(12)
                .class(cosmic::theme::Text::Color(
                    cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5))).into(),
            widget::divider::horizontal::default().into(),
            content,
        ])
        .spacing(8)
        .padding(16),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

fn gs_toggle_row<'a>(
    lang: Language,
    label: &'a str,
    enabled: bool,
    hint: &'a str,
    on_toggle: AppMessage,
) -> Element<'a, AppMessage> {
    let s = strings_for(lang);
    let btn_label = if enabled { s.common_on } else { s.common_off };
    widget::row::with_children(vec![
        widget::column::with_children(vec![
            widget::text(label).size(13).font(cosmic::font::bold()).into(),
            widget::text(hint).size(11)
                .class(cosmic::theme::Text::Color(
                    cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48))).into(),
        ]).spacing(2).width(Length::Fill).into(),
        widget::button::text(btn_label)
            .on_press(on_toggle)
            .class(if enabled {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            })
            .into(),
    ])
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

// ── OpenClaw AI Model section ─────────────────────────────────────────────────
fn build_openclaw_ai_section<'a>(
    config: &'a SecurityConfig,
    test_status: Option<&'a (bool, String)>,
    max_tokens_str: &'a str,
    temperature_str: &'a str,
) -> Element<'a, AppMessage> {
    let color_muted   = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);
    let color_ok      = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
    let color_err     = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);
    let ai = &config.openclaw_ai;

    // Provider selector cards
    let provider_cards: Vec<Element<AppMessage>> = AiProvider::all().iter().map(|p| {
        let is_active = *p == ai.provider;
        let accent = cosmic::iced::Color::from_rgb(0.28, 0.78, 0.96);
        widget::container(
            widget::row::with_children(vec![
                widget::column::with_children(vec![
                    widget::text(p.to_string()).size(13).font(cosmic::font::bold())
                        .class(if is_active {
                            cosmic::theme::Text::Color(accent)
                        } else {
                            cosmic::theme::Text::Default
                        })
                        .into(),
                    widget::text(p.default_endpoint()).size(11)
                        .class(cosmic::theme::Text::Color(color_muted)).into(),
                    widget::text(if p.requires_api_key() { "Requires API Key" } else { "No API key needed" })
                        .size(10)
                        .class(cosmic::theme::Text::Color(
                            if p.requires_api_key() {
                                cosmic::iced::Color::from_rgb(0.96, 0.72, 0.22)
                            } else {
                                color_ok
                            }
                        ))
                        .into(),
                ]).spacing(2).width(Length::Fill).into(),
                if is_active {
                    widget::button::text("✓ Active").on_press(AppMessage::Noop)
                        .class(cosmic::theme::Button::Text).into()
                } else {
                    widget::button::standard("Select")
                        .on_press(AppMessage::OpenClawAiProviderChanged(p.clone())).into()
                },
            ]).spacing(12).align_y(Alignment::Center).padding([8, 12]),
        ).class(cosmic::theme::Container::Card).width(Length::Fill).into()
    }).collect();

    // Endpoint + model + API key fields
    let mut fields: Vec<Element<AppMessage>> = vec![
        widget::Space::new(0, 4).into(),
        widget::divider::horizontal::light().into(),
        widget::Space::new(0, 8).into(),
        widget::row::with_children(vec![
            widget::column::with_children(vec![
                widget::text("API Endpoint").size(13).font(cosmic::font::bold()).into(),
                widget::text("URL of the AI provider endpoint")
                    .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(2).width(Length::Fill).into(),
            widget::text_input("http://localhost:11434", ai.endpoint.as_str())
                .on_input(AppMessage::OpenClawAiEndpointChanged)
                .width(Length::Fixed(300.0)).into(),
        ]).spacing(12).align_y(Alignment::Center).into(),
        widget::row::with_children(vec![
            widget::column::with_children(vec![
                widget::text("Model Name").size(13).font(cosmic::font::bold()).into(),
                widget::text("Model identifier (e.g. gpt-4o-mini, qwen2.5:7b)")
                    .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(2).width(Length::Fill).into(),
            widget::text_input("model name", ai.model.as_str())
                .on_input(AppMessage::OpenClawAiModelChanged)
                .width(Length::Fixed(220.0)).into(),
        ]).spacing(12).align_y(Alignment::Center).into(),
    ];

    if ai.provider.requires_api_key() {
        fields.push(
            widget::row::with_children(vec![
                widget::column::with_children(vec![
                    widget::text("API Key").size(13).font(cosmic::font::bold()).into(),
                    widget::text("Stored in config.toml — use env var for production")
                        .size(11).class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.96, 0.72, 0.22)
                        )).into(),
                ]).spacing(2).width(Length::Fill).into(),
                widget::text_input("sk-...", ai.api_key.as_str())
                    .on_input(AppMessage::OpenClawAiApiKeyChanged)
                    .width(Length::Fixed(280.0)).into(),
            ]).spacing(12).align_y(Alignment::Center).into(),
        );
    }

    // Advanced: max_tokens, temperature, stream
    fields.push(widget::divider::horizontal::light().into());
    fields.push(
        widget::row::with_children(vec![
            widget::column::with_children(vec![
                widget::text("Max Tokens").size(13).font(cosmic::font::bold()).into(),
                widget::text("Maximum tokens per response")
                    .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(2).width(Length::Fill).into(),
            widget::text_input("4096", max_tokens_str)
                .on_input(AppMessage::OpenClawAiMaxTokensChanged)
                .width(Length::Fixed(100.0)).into(),
            widget::Space::new(16, 0).into(),
            widget::column::with_children(vec![
                widget::text("Temperature").size(13).font(cosmic::font::bold()).into(),
                widget::text("0.0 = deterministic, 1.0 = creative")
                    .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(2).width(Length::Fill).into(),
            widget::text_input("0.7", temperature_str)
                .on_input(AppMessage::OpenClawAiTemperatureChanged)
                .width(Length::Fixed(80.0)).into(),
        ]).spacing(12).align_y(Alignment::Center).into(),
    );
    fields.push(
        widget::row::with_children(vec![
            widget::column::with_children(vec![
                widget::text("Streaming").size(13).font(cosmic::font::bold()).into(),
                widget::text("Stream tokens as they are generated")
                    .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
            ]).spacing(2).width(Length::Fill).into(),
            widget::button::text(if ai.stream { "ON" } else { "OFF" })
                .on_press(AppMessage::OpenClawAiToggleStream)
                .class(if ai.stream { cosmic::theme::Button::Suggested } else { cosmic::theme::Button::Standard })
                .into(),
        ]).spacing(12).align_y(Alignment::Center).into(),
    );

    // Test connection button + result
    let mut test_row: Vec<Element<AppMessage>> = vec![
        widget::button::suggested("Test Connection")
            .on_press(AppMessage::OpenClawAiTestConnection)
            .into(),
    ];
    if let Some((ok, msg)) = test_status {
        test_row.push(widget::Space::new(12, 0).into());
        test_row.push(
            widget::text(if *ok { format!("✓ {}", msg) } else { format!("✗ {}", msg) })
                .size(12)
                .class(cosmic::theme::Text::Color(if *ok { color_ok } else { color_err }))
                .into()
        );
    }
    fields.push(widget::row::with_children(test_row).spacing(0).align_y(Alignment::Center).into());

    let mut all: Vec<Element<AppMessage>> = provider_cards;
    all.extend(fields);

    gs_section_with_content(
        "OpenClaw AI Model",
        "Configure which AI model OpenClaw uses to process your instructions",
        widget::column::with_children(all).spacing(8).into(),
    )
}

// ── Communication Channels section ───────────────────────────────────────────
fn build_channels_section<'a>(
    config: &'a SecurityConfig,
    test_status: &'a [Option<(bool, String)>],
) -> Element<'a, AppMessage> {
    let color_muted = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);
    let color_ok    = cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46);
    let color_err   = cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28);
    let color_warn  = cosmic::iced::Color::from_rgb(0.96, 0.72, 0.22);

    // "Add channel" buttons — one per kind not yet added
    let added_kinds: Vec<&ChannelKind> = config.channels.iter().map(|c| &c.kind).collect();
    let add_btns: Vec<Element<AppMessage>> = ChannelKind::all().iter().map(|kind| {
        let already = added_kinds.contains(&kind);
        widget::button::standard(
            format!("{} {}", kind.icon(), kind.to_string())
        )
        .on_press(if already { AppMessage::Noop } else { AppMessage::ChannelAdd(kind.clone()) })
        .class(if already { cosmic::theme::Button::Text } else { cosmic::theme::Button::Standard })
        .into()
    }).collect();

    // Row of add buttons (wrap into 4 per row)
    let mut add_rows: Vec<Element<AppMessage>> = Vec::new();
    let mut i = 0;
    while i < add_btns.len() {
        let mut row_items: Vec<Element<AppMessage>> = Vec::new();
        for j in 0..4 {
            if i + j < ChannelKind::all().len() {
                let kind = &ChannelKind::all()[i + j];
                let already = added_kinds.contains(&kind);
                row_items.push(
                    widget::button::standard(format!("{} {}", kind.icon(), kind.to_string()))
                        .on_press(if already { AppMessage::Noop } else { AppMessage::ChannelAdd(kind.clone()) })
                        .class(if already { cosmic::theme::Button::Text } else { cosmic::theme::Button::Standard })
                        .into()
                );
            }
        }
        add_rows.push(widget::row::with_children(row_items).spacing(6).into());
        i += 4;
    }

    // Configured channel cards
    let channel_cards: Vec<Element<AppMessage>> = config.channels.iter().enumerate().map(|(idx, ch)| {
        let status_test = test_status.get(idx).and_then(|s| s.as_ref());
        let enabled_color = if ch.enabled { color_ok } else { color_muted };

        let mut card_rows: Vec<Element<AppMessage>> = vec![
            // Header row: icon + name + enable toggle + remove
            widget::row::with_children(vec![
                widget::text(format!("{} {}", ch.kind.icon(), ch.label))
                    .size(14).font(cosmic::font::bold())
                    .class(cosmic::theme::Text::Color(enabled_color))
                    .width(Length::Fill).into(),
                widget::button::text(if ch.enabled { "● Enabled" } else { "○ Disabled" })
                    .on_press(AppMessage::ChannelToggleEnabled(idx))
                    .class(if ch.enabled { cosmic::theme::Button::Suggested } else { cosmic::theme::Button::Standard })
                    .into(),
                widget::button::destructive("Remove")
                    .on_press(AppMessage::ChannelRemove(idx)).into(),
            ]).spacing(8).align_y(Alignment::Center).into(),
            // Setup hint
            widget::text(ch.kind.setup_hint()).size(11)
                .class(cosmic::theme::Text::Color(color_warn)).into(),
            widget::divider::horizontal::light().into(),
        ];

        // Token field
        card_rows.push(
            widget::row::with_children(vec![
                widget::text("Bot Token / API Key").size(12).font(cosmic::font::bold())
                    .width(Length::Fixed(160.0)).into(),
                widget::text_input("paste token here…", ch.token.as_str())
                    .on_input(move |v| AppMessage::ChannelTokenChanged { idx, value: v })
                    .width(Length::Fill).into(),
            ]).spacing(8).align_y(Alignment::Center).into(),
        );

        // Channel ID (Telegram chat_id, Discord channel_id, Slack channel)
        match ch.kind {
            ChannelKind::WhatsApp | ChannelKind::Signal => {
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Phone Number").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("+1234567890", ch.phone_number.as_str())
                            .on_input(move |v| AppMessage::ChannelPhoneChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
            }
            ChannelKind::Discord => {
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Channel ID").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("Discord channel snowflake ID", ch.channel_id.as_str())
                            .on_input(move |v| AppMessage::ChannelIdChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Guild ID (optional)").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("server/guild snowflake ID", ch.guild_id.as_str())
                            .on_input(move |v| AppMessage::ChannelGuildIdChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Webhook URL (optional)").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("https://discord.com/api/webhooks/…", ch.webhook_url.as_str())
                            .on_input(move |v| AppMessage::ChannelWebhookChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
            }
            ChannelKind::Matrix => {
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Homeserver URL").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("https://matrix.org", ch.homeserver_url.as_str())
                            .on_input(move |v| AppMessage::ChannelHomserverChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Access Token").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("syt_…", ch.token.as_str())
                            .on_input(move |v| AppMessage::ChannelTokenChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Room ID").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("!roomid:matrix.org", ch.channel_id.as_str())
                            .on_input(move |v| AppMessage::ChannelIdChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Bot User ID (optional)").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("@openclaw-bot:matrix.org", ch.matrix_user_id.as_str())
                            .on_input(move |v| AppMessage::ChannelMatrixUserIdChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
            }
            ChannelKind::Slack | ChannelKind::MicrosoftTeams => {
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Channel ID").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("channel or server ID", ch.channel_id.as_str())
                            .on_input(move |v| AppMessage::ChannelIdChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Webhook URL (optional)").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("https://hooks.slack.com/…", ch.webhook_url.as_str())
                            .on_input(move |v| AppMessage::ChannelWebhookChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
            }
            ChannelKind::Telegram => {
                card_rows.push(
                    widget::row::with_children(vec![
                        widget::text("Chat ID").size(12).font(cosmic::font::bold())
                            .width(Length::Fixed(160.0)).into(),
                        widget::text_input("your Telegram chat_id", ch.channel_id.as_str())
                            .on_input(move |v| AppMessage::ChannelIdChanged { idx, value: v })
                            .width(Length::Fill).into(),
                    ]).spacing(8).align_y(Alignment::Center).into(),
                );
            }
            _ => {}
        }

        // Test button + result
        let mut test_row_items: Vec<Element<AppMessage>> = vec![
            widget::button::standard("Test Connection")
                .on_press(AppMessage::ChannelTest(idx)).into(),
        ];
        if let Some((ok, msg)) = status_test {
            test_row_items.push(widget::Space::new(10, 0).into());
            test_row_items.push(
                widget::text(if *ok { format!("✓ {}", msg) } else { format!("✗ {}", msg) })
                    .size(12)
                    .class(cosmic::theme::Text::Color(if *ok { color_ok } else { color_err }))
                    .into()
            );
        }
        card_rows.push(
            widget::row::with_children(test_row_items).spacing(0).align_y(Alignment::Center).into()
        );

        widget::container(
            widget::column::with_children(card_rows).spacing(8).padding([12, 16]),
        )
        .class(cosmic::theme::Container::Card)
        .width(Length::Fill)
        .into()
    }).collect();

    let mut content: Vec<Element<AppMessage>> = vec![
        widget::text("Add Channel").size(13).font(cosmic::font::bold()).into(),
        widget::Space::new(0, 6).into(),
        widget::column::with_children(add_rows).spacing(6).into(),
    ];

    if !config.channels.is_empty() {
        content.push(widget::Space::new(0, 12).into());
        content.push(widget::divider::horizontal::default().into());
        content.push(widget::Space::new(0, 8).into());
        content.push(
            widget::text(format!("Configured Channels ({})", config.channels.len()))
                .size(13).font(cosmic::font::bold()).into()
        );
        content.push(widget::Space::new(0, 8).into());
        content.extend(channel_cards);
    } else {
        content.push(widget::Space::new(0, 8).into());
        content.push(
            widget::text("No channels configured yet. Add one above.")
                .size(12).class(cosmic::theme::Text::Color(color_muted)).into()
        );
    }

    gs_section_with_content(
        "Communication Channels",
        "Connect OpenClaw to Telegram, Discord, Slack and other messaging platforms",
        widget::column::with_children(content).spacing(6).into(),
    )
}
