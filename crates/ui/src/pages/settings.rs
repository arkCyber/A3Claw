use crate::app::{AppMessage, OllamaModel};
use crate::i18n::strings_for;
use crate::theme::Language;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use openclaw_security::{AgentKind, SecurityConfig};

pub struct SettingsPage;

impl SettingsPage {
    pub fn view<'a>(
        config: &'a SecurityConfig,
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
        wasm_policy_path: &'a str,
        wasm_policy_status: &'a str,
        folder_access_path: &'a str,
        folder_access_label: &'a str,
        rag_folder_path: &'a str,
        rag_folder_name: &'a str,
    ) -> Element<'a, AppMessage> {
        let s = strings_for(lang);

        // Appearance & Language moved to General Settings page

        // ── Sandbox resource limits ───────────────────────────────────────────
        let memory_input = widget::row::with_children(vec![
            widget::column::with_children(vec![
                widget::text(s.set_memory)
                    .size(13).font(cosmic::font::bold()).into(),
                widget::text(s.set_memory_hint)
                    .size(11)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48)))
                    .into(),
            ])
            .spacing(2)
            .width(Length::Fill)
            .into(),
            widget::text_input("512", "")
                .on_input(AppMessage::MemoryLimitChanged)
                .width(Length::Fixed(100.0))
                .into(),
            widget::text(format!("{} {}", config.memory_limit_mb, s.common_mb)).size(13).into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center);

        let section_sandbox = settings_section_with_content(
            s.set_sandbox,
            "",
            widget::column::with_children(vec![
                memory_input.into(),
                widget::divider::horizontal::light().into(),
                setting_row(
                    s.set_entry,
                    config.openclaw_entry.display().to_string(),
                    s.set_entry_hint,
                ),
                widget::divider::horizontal::light().into(),
                setting_row(
                    s.set_workspace,
                    config.workspace_dir.display().to_string(),
                    s.set_workspace_hint,
                ),
            ])
            .spacing(12)
            .into(),
        );

        // ── Interception policy (interactive toggles) ────────────────────────
        let section_intercept = settings_section(
            s.set_intercept,
            vec![
                interactive_toggle_row(
                    lang,
                    s.set_shell_intercept,
                    config.intercept_shell,
                    s.set_shell_int_hint,
                    AppMessage::ToggleInterceptShell,
                ),
                interactive_toggle_row(
                    lang,
                    s.set_file_del,
                    config.confirm_file_delete,
                    s.set_file_del_hint,
                    AppMessage::ToggleConfirmFileDelete,
                ),
                interactive_toggle_row(
                    lang,
                    s.set_shell_exec,
                    config.confirm_shell_exec,
                    s.set_shell_exec_hint,
                    AppMessage::ToggleConfirmShellExec,
                ),
                interactive_toggle_row(
                    lang,
                    s.set_net_confirm,
                    config.confirm_network,
                    s.set_net_confirm_hint,
                    AppMessage::ToggleConfirmNetwork,
                ),
            ],
        );

        // ── Network allowlist ─────────────────────────────────────────────────
        let network_list: Vec<Element<AppMessage>> = if config.network_allowlist.is_empty() {
            vec![widget::text(s.set_no_allowlist)
                .size(13)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48)))
                .into()]
        } else {
            config
                .network_allowlist
                .iter()
                .map(|host| {
                    widget::container(
                        widget::row::with_children(vec![
                            widget::text("🌐").size(14).into(),
                            widget::text(host).size(13).width(Length::Fill).into(),
                            widget::text(s.common_allowed)
                                .size(12)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.22, 0.78, 0.42),
                                ))
                                .into(),
                        ])
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .padding([6, 12]),
                    )
                    .class(cosmic::theme::Container::Card)
                    .into()
                })
                .collect()
        };

        let section_network = settings_section_with_content(
            s.set_allowlist,
            s.set_allowlist_sub,
            widget::column::with_children(network_list).spacing(4).into(),
        );

        // ── Filesystem mounts ─────────────────────────────────────────────────
        let fs_list: Vec<Element<AppMessage>> = if config.fs_mounts.is_empty() {
            vec![widget::text(s.set_no_mounts)
                .size(13)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48)))
                .into()]
        } else {
            config
                .fs_mounts
                .iter()
                .map(|mount| {
                    widget::container(
                        widget::row::with_children(vec![
                            widget::text("📁").size(14).into(),
                            widget::column::with_children(vec![
                                widget::text(mount.host_path.display().to_string())
                                    .size(13)
                                    .font(cosmic::font::bold())
                                    .into(),
                                widget::text(format!(
                                    "→ {}",
                                    mount.guest_path
                                ))
                                .size(12)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                                ))
                                .into(),
                            ])
                            .spacing(2)
                            .width(Length::Fill)
                            .into(),
                            widget::text(if mount.readonly {
                                s.set_readonly
                            } else {
                                s.set_readwrite
                            })
                            .size(12)
                            .class(cosmic::theme::Text::Color(if mount.readonly {
                                cosmic::iced::Color::from_rgb(0.96, 0.62, 0.12)
                            } else {
                                cosmic::iced::Color::from_rgb(0.28, 0.65, 0.95)
                            }))
                            .into(),
                        ])
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .padding([8, 12]),
                    )
                    .class(cosmic::theme::Container::Card)
                    .into()
                })
                .collect()
        };

        let section_fs = settings_section_with_content(
            s.set_fs_mounts,
            s.set_fs_mounts_sub,
            widget::column::with_children(fs_list).spacing(4).into(),
        );

        // ── Audit log ─────────────────────────────────────────────────────────
        let audit_section = settings_section(
            s.set_audit,
            vec![setting_row(
                s.set_log_path,
                config.audit_log_path.display().to_string(),
                s.set_log_path_hint,
            )],
        );

        // ── AI Inference settings (editable) ──────────────────────────────────
        let section_ai = settings_section_with_content(
            s.set_ai,
            s.set_ai_sub,
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(s.set_endpoint)
                            .size(13).font(cosmic::font::bold()).into(),
                        widget::text(s.set_endpoint_hint)
                            .size(11)
                            .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48)))
                            .into(),
                    ])
                    .spacing(2)
                    .width(Length::Fill)
                    .into(),
                    widget::text_input("http://localhost:11434", ai_endpoint)
                        .on_input(AppMessage::AiEndpointChanged)
                        .width(Length::Fixed(280.0))
                        .into(),
                ])
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
                widget::divider::horizontal::light().into(),
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(s.set_model)
                            .size(13).font(cosmic::font::bold()).into(),
                        widget::text(s.set_model_hint)
                            .size(11)
                            .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48)))
                            .into(),
                    ])
                    .spacing(2)
                    .width(Length::Fill)
                    .into(),
                    widget::text_input("qwen2.5:0.5b", ai_model)
                        .on_input(AppMessage::AiModelChanged)
                        .width(Length::Fixed(200.0))
                        .into(),
                ])
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
            ])
            .spacing(12)
            .into(),
        );

        // ── AI Model Management ───────────────────────────────────────────────
        // Header row: title + refresh button
        let model_header = widget::row::with_children(vec![
            widget::text(format!("{} models installed", available_models.len()))
                .size(12)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48)))
                .width(Length::Fill)
                .into(),
            widget::button::standard("⟳  Refresh")
                .on_press(AppMessage::AiListModels)
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center);

        // Search/filter bar
        let search_bar = widget::text_input("🔍  Filter models...", model_search)
            .on_input(AppMessage::ModelSearchChanged)
            .width(Length::Fill);

        // Filter models by search text
        let filtered: Vec<&OllamaModel> = available_models
            .iter()
            .filter(|m| {
                model_search.is_empty()
                    || m.name.to_lowercase().contains(&model_search.to_lowercase())
                    || m.family.to_lowercase().contains(&model_search.to_lowercase())
            })
            .collect();

        // Model cards
        let model_cards: Vec<Element<AppMessage>> = if available_models.is_empty() {
            vec![
                widget::container(
                    widget::column::with_children(vec![
                        widget::text("No models installed")
                            .size(14)
                            .font(cosmic::font::bold())
                            .into(),
                        widget::Space::new(0, 4).into(),
                        widget::text("Click 'Refresh' to check, or download a model below.")
                            .size(12)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                            ))
                            .into(),
                    ])
                    .spacing(2)
                    .padding([16, 20]),
                )
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .into(),
            ]
        } else if filtered.is_empty() {
            vec![
                widget::text(format!("No models match '{}'", model_search))
                    .size(13)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                    ))
                    .into(),
            ]
        } else {
            filtered.iter().map(|m| {
                let is_active = m.name == ai_model;
                let name_color = if is_active {
                    cosmic::iced::Color::from_rgb(0.28, 0.78, 0.96)
                } else {
                    cosmic::iced::Color::from_rgb(0.92, 0.92, 0.92)
                };

                // Badge helpers
                let badge = |text: String, r: f32, g: f32, b: f32| -> Element<'a, AppMessage> {
                    widget::container(
                        widget::text(text).size(10),
                    )
                    .padding([2, 7])
                    .class(cosmic::theme::Container::custom(move |_theme| {
                        cosmic::iced::widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(
                                cosmic::iced::Color::from_rgba(r, g, b, 0.18),
                            )),
                            border: cosmic::iced::Border {
                                radius: 4.0.into(),
                                width: 1.0,
                                color: cosmic::iced::Color::from_rgba(r, g, b, 0.45),
                            },
                            text_color: Some(cosmic::iced::Color::from_rgb(r, g, b)),
                            ..Default::default()
                        }
                    }))
                    .into()
                };

                // Build badge row
                let mut badges: Vec<Element<AppMessage>> = Vec::new();
                if is_active {
                    badges.push(badge("● ACTIVE".to_string(), 0.28, 0.78, 0.42));
                }
                if !m.parameter_size.is_empty() {
                    badges.push(badge(m.parameter_size.clone(), 0.28, 0.65, 0.95));
                }
                if !m.quantization.is_empty() {
                    badges.push(badge(m.quantization.clone(), 0.96, 0.62, 0.12));
                }
                if !m.family.is_empty() {
                    badges.push(badge(m.family.clone(), 0.72, 0.45, 0.95));
                }

                let badge_row = widget::row::with_children(badges).spacing(5);

                // Info row: size + date
                let info_row = widget::row::with_children(vec![
                    widget::text(format!("💾 {}", m.size_display()))
                        .size(11)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                        ))
                        .into(),
                    widget::text("  ·  ").size(11)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.4, 0.4, 0.4),
                        ))
                        .into(),
                    widget::text(format!("📅 {}", m.modified_display()))
                        .size(11)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                        ))
                        .into(),
                ])
                .spacing(0);

                // Action buttons
                let use_btn = if is_active {
                    widget::button::text("✓ In Use")
                        .on_press(AppMessage::Noop)
                        .class(cosmic::theme::Button::Text)
                } else {
                    widget::button::standard("Use")
                        .on_press(AppMessage::AiSetActiveModel(m.name.clone()))
                };

                let del_btn = widget::button::destructive("Delete")
                    .on_press(AppMessage::AiDeleteModel(m.name.clone()));

                let card = widget::container(
                    widget::column::with_children(vec![
                        widget::row::with_children(vec![
                            widget::column::with_children(vec![
                                widget::text(&m.name)
                                    .size(14)
                                    .font(cosmic::font::bold())
                                    .class(cosmic::theme::Text::Color(name_color))
                                    .into(),
                                widget::Space::new(0, 4).into(),
                                badge_row.into(),
                                widget::Space::new(0, 6).into(),
                                info_row.into(),
                            ])
                            .spacing(0)
                            .width(Length::Fill)
                            .into(),
                            widget::column::with_children(vec![
                                use_btn.into(),
                                widget::Space::new(0, 6).into(),
                                del_btn.into(),
                            ])
                            .spacing(0)
                            .align_x(Alignment::End)
                            .into(),
                        ])
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .into(),
                    ])
                    .padding([12, 16]),
                )
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill);

                card.into()
            }).collect()
        };

        // Download progress bar
        let progress_widget: Option<Element<AppMessage>> = download_status.map(|(model, status, percent)| {
            widget::container(
                widget::column::with_children(vec![
                    widget::row::with_children(vec![
                        widget::text(format!("⬇  Downloading: {}", model))
                            .size(13)
                            .font(cosmic::font::bold())
                            .width(Length::Fill)
                            .into(),
                        widget::text(format!("{}%", percent))
                            .size(13)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.28, 0.78, 0.42),
                            ))
                            .into(),
                    ])
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
                    widget::Space::new(0, 6).into(),
                    // Progress bar (manual)
                    widget::container(
                        widget::container(widget::Space::new(0, 0))
                            .width(Length::Fixed((*percent as f32 / 100.0) * 340.0))
                            .height(Length::Fixed(4.0))
                            .class(cosmic::theme::Container::custom(|_| {
                                cosmic::iced::widget::container::Style {
                                    background: Some(cosmic::iced::Background::Color(
                                        cosmic::iced::Color::from_rgb(0.28, 0.78, 0.42),
                                    )),
                                    border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                                    ..Default::default()
                                }
                            })),
                    )
                    .width(Length::Fill)
                    .height(Length::Fixed(4.0))
                    .class(cosmic::theme::Container::custom(|_| {
                        cosmic::iced::widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(
                                cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                            )),
                            border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    }))
                    .into(),
                    widget::Space::new(0, 4).into(),
                    widget::text(status.as_str())
                        .size(11)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                        ))
                        .into(),
                ])
                .spacing(0)
                .padding([10, 14]),
            )
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .into()
        });

        // Recommended models list
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
            let already_installed = available_models.iter().any(|m| &m.name.as_str() == name);
            widget::container(
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(*name)
                            .size(13)
                            .font(cosmic::font::bold())
                            .into(),
                        widget::row::with_children(vec![
                            widget::text(*size_hint)
                                .size(11)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.28, 0.65, 0.95),
                                ))
                                .into(),
                            widget::text("  ·  ").size(11)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.4, 0.4, 0.4),
                                ))
                                .into(),
                            widget::text(*desc)
                                .size(11)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                                ))
                                .into(),
                        ])
                        .spacing(0)
                        .into(),
                    ])
                    .spacing(3)
                    .width(Length::Fill)
                    .into(),
                    if already_installed {
                        widget::button::text("✓ Installed")
                            .on_press(AppMessage::Noop)
                            .class(cosmic::theme::Button::Text)
                            .into()
                    } else {
                        widget::button::standard("⬇ Download")
                            .on_press(AppMessage::AiPullModel(name.to_string()))
                            .into()
                    },
                ])
                .spacing(12)
                .align_y(Alignment::Center)
                .padding([8, 12]),
            )
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .into()
        }).collect();

        // Custom download row
        let custom_download = widget::column::with_children(vec![
            widget::text("Custom Model Name")
                .size(12)
                .font(cosmic::font::bold())
                .into(),
            widget::Space::new(0, 6).into(),
            widget::row::with_children(vec![
                widget::text_input("e.g., qwen2.5:14b, codellama:13b", model_download_input)
                    .on_input(AppMessage::ModelDownloadInputChanged)
                    .width(Length::Fill)
                    .into(),
                widget::button::suggested("⬇ Download")
                    .on_press(if model_download_input.is_empty() {
                        AppMessage::Noop
                    } else {
                        AppMessage::AiPullModel(model_download_input.to_string())
                    })
                    .into(),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
            widget::Space::new(0, 4).into(),
            widget::text("Browse all models at ollama.com/library")
                .size(11)
                .class(cosmic::theme::Text::Color(
                    cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                ))
                .into(),
        ])
        .spacing(0);

        // Assemble full model management section
        let mut mgmt_children: Vec<Element<AppMessage>> = vec![
            // Installed models header
            model_header.into(),
            widget::Space::new(0, 8).into(),
            search_bar.into(),
            widget::Space::new(0, 8).into(),
        ];
        // Progress bar (if downloading)
        if let Some(prog) = progress_widget {
            mgmt_children.push(prog);
            mgmt_children.push(widget::Space::new(0, 8).into());
        }
        // Model cards
        mgmt_children.extend(model_cards);
        mgmt_children.push(widget::Space::new(0, 16).into());
        mgmt_children.push(widget::divider::horizontal::light().into());
        mgmt_children.push(widget::Space::new(0, 12).into());
        // Recommended section
        mgmt_children.push(
            widget::text("Recommended Models")
                .size(13)
                .font(cosmic::font::bold())
                .into(),
        );
        mgmt_children.push(widget::Space::new(0, 8).into());
        mgmt_children.extend(rec_cards);
        mgmt_children.push(widget::Space::new(0, 16).into());
        mgmt_children.push(widget::divider::horizontal::light().into());
        mgmt_children.push(widget::Space::new(0, 12).into());
        // Custom download
        mgmt_children.push(custom_download.into());

        let section_model_mgmt = settings_section_with_content(
            "AI Model Management",
            "Install, switch and remove local Ollama models",
            widget::column::with_children(mgmt_children)
                .spacing(4)
                .into(),
        );

        // ── GitHub Policy section ─────────────────────────────────────────────
        let gh = &config.github;
        let section_github = settings_section_with_content(
            "GitHub / Git Security Policy",
            "Control how the AI agent interacts with Git and GitHub",
            widget::column::with_children(vec![
                interactive_toggle_row(
                    lang,
                    "Deny Force Push",
                    gh.deny_force_push,
                    "Block git push --force and --force-with-lease",
                    AppMessage::ToggleGithubDenyForcePush,
                ),
                interactive_toggle_row(
                    lang,
                    "Confirm Push",
                    gh.confirm_push,
                    "Require confirmation before any git push",
                    AppMessage::ToggleGithubConfirmPush,
                ),
                interactive_toggle_row(
                    lang,
                    "Protect Default Branch",
                    gh.protect_default_branch,
                    "Extra confirmation when pushing to main/master/develop",
                    AppMessage::ToggleGithubProtectDefaultBranch,
                ),
                interactive_toggle_row(
                    lang,
                    "Confirm Branch Delete",
                    gh.confirm_branch_delete,
                    "Require confirmation before deleting a remote branch",
                    AppMessage::ToggleGithubConfirmBranchDelete,
                ),
                interactive_toggle_row(
                    lang,
                    "Confirm History Rewrite",
                    gh.confirm_history_rewrite,
                    "Require confirmation for git reset --hard / rebase",
                    AppMessage::ToggleGithubConfirmHistoryRewrite,
                ),
                interactive_toggle_row(
                    lang,
                    "Intercept GitHub API",
                    gh.intercept_github_api,
                    "Monitor and control calls to api.github.com",
                    AppMessage::ToggleGithubInterceptApi,
                ),
                widget::divider::horizontal::light().into(),
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text("Allowed Orgs")
                            .size(13).font(cosmic::font::bold()).into(),
                        widget::text("Comma-separated GitHub org names (empty = all)")
                            .size(11)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                            ))
                            .into(),
                    ])
                    .spacing(2).width(Length::Fill).into(),
                    widget::text_input("e.g., my-org, another-org", github_orgs_input)
                        .on_input(AppMessage::GithubAllowedOrgsChanged)
                        .width(Length::Fixed(240.0))
                        .into(),
                ])
                .spacing(12).align_y(Alignment::Center).into(),
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text("Allowed Repos")
                            .size(13).font(cosmic::font::bold()).into(),
                        widget::text("Comma-separated repo paths (empty = all in allowed orgs)")
                            .size(11)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                            ))
                            .into(),
                    ])
                    .spacing(2).width(Length::Fill).into(),
                    widget::text_input("e.g., my-org/repo-a, my-org/repo-b", github_repos_input)
                        .on_input(AppMessage::GithubAllowedReposChanged)
                        .width(Length::Fixed(240.0))
                        .into(),
                ])
                .spacing(12).align_y(Alignment::Center).into(),
            ])
            .spacing(12)
            .into(),
        );

        // ── Agent Selector section ────────────────────────────────────────────
        let agent_cards: Vec<Element<AppMessage>> = AgentKind::all().iter().map(|kind| {
            let is_active = *kind == config.agent.kind;
            let accent = cosmic::iced::Color::from_rgb(0.28, 0.78, 0.96);
            let card = widget::container(
                widget::row::with_children(vec![
                    widget::column::with_children(vec![
                        widget::text(kind.to_string())
                            .size(13)
                            .font(cosmic::font::bold())
                            .class(if is_active {
                                cosmic::theme::Text::Color(accent)
                            } else {
                                cosmic::theme::Text::Default
                            })
                            .into(),
                        widget::text(kind.description())
                            .size(11)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                            ))
                            .into(),
                    ])
                    .spacing(3)
                    .width(Length::Fill)
                    .into(),
                    if is_active {
                        widget::button::text("✓ Active")
                            .on_press(AppMessage::Noop)
                            .class(cosmic::theme::Button::Text)
                            .into()
                    } else {
                        widget::button::standard("Select")
                            .on_press(AppMessage::SetAgentKind(kind.clone()))
                            .into()
                    },
                ])
                .spacing(12)
                .align_y(Alignment::Center)
                .padding([8, 12]),
            )
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill);
            card.into()
        }).collect();

        let section_agent = settings_section_with_content(
            "Agent Runtime",
            "Select which AI agent framework to run in the sandbox",
            widget::column::with_children({
                let mut children: Vec<Element<AppMessage>> = agent_cards;
                children.push(widget::Space::new(0, 12).into());
                children.push(widget::divider::horizontal::light().into());
                children.push(widget::Space::new(0, 8).into());
                children.push(
                    widget::row::with_children(vec![
                        widget::column::with_children(vec![
                            widget::text("Entry Point Path")
                                .size(13).font(cosmic::font::bold()).into(),
                            widget::text("Path to the agent's main file (JS bundle, Python script, or .wasm)")
                                .size(11)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                                ))
                                .into(),
                        ])
                        .spacing(2).width(Length::Fill).into(),
                        widget::text_input("e.g., /path/to/agent/index.js", agent_entry_input)
                            .on_input(AppMessage::AgentEntryPathChanged)
                            .width(Length::Fixed(280.0))
                            .into(),
                    ])
                    .spacing(12).align_y(Alignment::Center).into(),
                );
                children
            })
            .spacing(6)
            .into(),
        );

        // ── WASM Policy Plugin section ────────────────────────────────────────
        let wasm_status_color = if wasm_policy_status.starts_with('✓') {
            cosmic::iced::Color::from_rgb(0.28, 0.78, 0.42)
        } else if wasm_policy_status.starts_with('✗') {
            cosmic::iced::Color::from_rgb(0.96, 0.35, 0.35)
        } else {
            cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48)
        };

        let section_wasm_policy = settings_section_with_content(
            "WASM Policy Plugin (Hot-Reload)",
            "Load custom security rules from a compiled .wasm file — reloads automatically on change",
            widget::column::with_children(vec![
                widget::container(
                    widget::column::with_children(vec![
                        widget::text("How it works:")
                            .size(12).font(cosmic::font::bold()).into(),
                        widget::text(
                            "1. Write policy rules as JSON in a WASM custom section\n\
                             2. Compile to .wasm with: cargo build --target wasm32-unknown-unknown\n\
                             3. Set the path below — rules reload automatically when the file changes\n\
                             4. WASM rules are evaluated first; unmatched events fall through to Rust policy"
                        )
                        .size(11)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                        ))
                        .into(),
                    ])
                    .spacing(4)
                    .padding([10, 12]),
                )
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .into(),
                widget::Space::new(0, 8).into(),
                widget::row::with_children(vec![
                    widget::text_input(
                        "e.g., /path/to/policy.wasm",
                        wasm_policy_path,
                    )
                    .on_input(AppMessage::WasmPolicyPathChanged)
                    .width(Length::Fill)
                    .into(),
                    widget::button::standard("⟳ Reload")
                        .on_press(AppMessage::WasmPolicyReload)
                        .into(),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
                if !wasm_policy_status.is_empty() {
                    widget::text(wasm_policy_status)
                        .size(12)
                        .class(cosmic::theme::Text::Color(wasm_status_color))
                        .into()
                } else {
                    widget::Space::new(0, 0).into()
                },
                widget::Space::new(0, 8).into(),
                widget::container(
                    widget::column::with_children(vec![
                        widget::text("Example rule (JSON in WASM custom section):")
                            .size(11).font(cosmic::font::bold()).into(),
                        widget::text(
                            r#"[{"event_kind":"Git Push","detail_contains":"--force","decision":"deny","reason":"Force push blocked by WASM policy"}]"#
                        )
                        .size(10)
                        .class(cosmic::theme::Text::Color(
                            cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                        ))
                        .into(),
                    ])
                    .spacing(4)
                    .padding([8, 12]),
                )
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .into(),
            ])
            .spacing(6)
            .into(),
        );

        // ── Shared colour palette ─────────────────────────────────────────────
        let accent_green      = cosmic::iced::Color::from_rgb(0.22, 0.78, 0.42);
        let accent_orange     = cosmic::iced::Color::from_rgb(0.96, 0.62, 0.12);
        let accent_red        = cosmic::iced::Color::from_rgb(0.96, 0.35, 0.35);
        let color_muted       = cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48);
        let color_accent_blue = cosmic::iced::Color::from_rgb(0.30, 0.62, 0.96);

        // ── Folder Access Whitelist section ───────────────────────────────────
        let fa_count = config.folder_access.len();

        // Header: title + count badge
        let fa_header = widget::row::with_children(vec![
            widget::text("📁  Authorised Folders")
                .size(13).font(cosmic::font::bold()).width(Length::Fill).into(),
            widget::container(
                widget::text(format!("{} folder{}", fa_count,
                    if fa_count == 1 { "" } else { "s" }))
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        if fa_count == 0 { accent_red } else { accent_green }
                    )),
            )
            .padding([3, 8]).class(cosmic::theme::Container::Card).into(),
        ])
        .spacing(8).align_y(Alignment::Center);

        // List body
        let fa_list_body: Element<AppMessage> = if config.folder_access.is_empty() {
            widget::container(
                widget::column::with_children(vec![
                    widget::Space::new(0, 18).into(),
                    widget::row::with_children(vec![
                        widget::Space::new(Length::Fill, 0).into(),
                        widget::column::with_children(vec![
                            widget::text("�").size(26).into(),
                            widget::Space::new(0, 6).into(),
                            widget::text("No folders authorised")
                                .size(13).font(cosmic::font::bold()).into(),
                            widget::Space::new(0, 3).into(),
                            widget::text("The AI agent cannot access any files.")
                                .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                            widget::text("Click \"Browse…\" below to add a folder.")
                                .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                        ])
                        .spacing(2)
                        .align_x(cosmic::iced::Alignment::Center).into(),
                        widget::Space::new(Length::Fill, 0).into(),
                    ]).into(),
                    widget::Space::new(0, 18).into(),
                ])
                .spacing(0),
            )
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .into()
        } else {
            // Column header
            let col_hdr = widget::container(
                widget::row::with_children(vec![
                    widget::text("#").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fixed(28.0)).into(),
                    widget::text("Label / Path").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fill).into(),
                    widget::text("Permissions").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fixed(150.0)).into(),
                    widget::text("Actions").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fixed(210.0)).into(),
                ])
                .spacing(8).align_y(Alignment::Center).padding([5, 10]),
            )
            .width(Length::Fill);

            let mut rows: Vec<Element<AppMessage>> = vec![
                col_hdr.into(),
                widget::divider::horizontal::light().into(),
            ];

            for (idx, fa) in config.folder_access.iter().enumerate() {
                // Permission badges
                let badge_read = widget::container(
                    widget::text("READ").size(10).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(accent_green)),
                ).padding([2, 5]).class(cosmic::theme::Container::Card);

                let badge_write = widget::container(
                    widget::text(if fa.allow_write { "WRITE" } else { "NO WRITE" })
                        .size(10).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(
                            if fa.allow_write { accent_orange } else { color_muted }
                        )),
                ).padding([2, 5]).class(cosmic::theme::Container::Card);

                let badge_del = widget::container(
                    widget::text(if fa.allow_delete { "DELETE" } else { "NO DEL" })
                        .size(10).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(
                            if fa.allow_delete { accent_red } else { color_muted }
                        )),
                ).padding([2, 5]).class(cosmic::theme::Container::Card);

                let ext_badge: Element<AppMessage> = if !fa.allowed_extensions.is_empty() {
                    widget::container(
                        widget::text(format!(".{}", fa.allowed_extensions.join(" .")))
                            .size(10).class(cosmic::theme::Text::Color(color_accent_blue)),
                    ).padding([2, 5]).class(cosmic::theme::Container::Card).into()
                } else {
                    widget::Space::new(0, 0).into()
                };

                let perms = widget::row::with_children(vec![
                    badge_read.into(), badge_write.into(), badge_del.into(), ext_badge,
                ])
                .spacing(4).align_y(Alignment::Center).width(Length::Fixed(150.0));

                // Action buttons
                let btn_write = widget::button::text(
                    if fa.allow_write { "✓ Write" } else { "Write" }
                )
                .on_press(AppMessage::FolderAccessToggleWrite(idx))
                .class(if fa.allow_write {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                });

                let btn_del = widget::button::text(
                    if fa.allow_delete { "✓ Delete" } else { "Delete" }
                )
                .on_press(AppMessage::FolderAccessToggleDelete(idx))
                .class(if fa.allow_delete {
                    cosmic::theme::Button::Destructive
                } else {
                    cosmic::theme::Button::Standard
                });

                let btn_remove = widget::button::destructive("✕ Remove")
                    .on_press(AppMessage::FolderAccessRemove(idx));

                let actions = widget::row::with_children(vec![
                    btn_write.into(), btn_del.into(), btn_remove.into(),
                ])
                .spacing(4).align_y(Alignment::Center).width(Length::Fixed(210.0));

                let label_col = widget::column::with_children(vec![
                    widget::text(fa.label.clone())
                        .size(12).font(cosmic::font::bold()).into(),
                    widget::text(fa.host_path.display().to_string())
                        .size(10).class(cosmic::theme::Text::Color(color_muted)).into(),
                ])
                .spacing(1).width(Length::Fill);

                let row_bg = if idx % 2 == 0 {
                    cosmic::theme::Container::Card
                } else {
                    cosmic::theme::Container::Primary
                };

                let entry = widget::container(
                    widget::row::with_children(vec![
                        widget::text(format!("{}", idx + 1)).size(11)
                            .class(cosmic::theme::Text::Color(color_muted))
                            .width(Length::Fixed(28.0)).into(),
                        label_col.into(),
                        perms.into(),
                        actions.into(),
                    ])
                    .spacing(8).align_y(Alignment::Center).padding([8, 10]),
                )
                .class(row_bg).width(Length::Fill);

                rows.push(entry.into());
            }

            widget::container(widget::column::with_children(rows).spacing(0))
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .into()
        };

        // Add-folder toolbar
        let fa_add_bar = widget::container(
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::button::suggested("🗂  Browse…")
                        .on_press(AppMessage::FolderAccessPickFolder).into(),
                    widget::text_input("/path/to/folder", folder_access_path)
                        .on_input(AppMessage::FolderAccessPathChanged)
                        .width(Length::Fill).into(),
                    widget::text_input("Label (optional)", folder_access_label)
                        .on_input(AppMessage::FolderAccessLabelChanged)
                        .width(Length::Fixed(150.0)).into(),
                    widget::button::standard("+ Read-only")
                        .on_press(AppMessage::FolderAccessAdd {
                            path: folder_access_path.to_string(),
                            label: folder_access_label.to_string(),
                            allow_write: false,
                        }).into(),
                    widget::button::suggested("+ Read-Write")
                        .on_press(AppMessage::FolderAccessAdd {
                            path: folder_access_path.to_string(),
                            label: folder_access_label.to_string(),
                            allow_write: true,
                        }).into(),
                ])
                .spacing(8).align_y(Alignment::Center).into(),
                widget::Space::new(0, 4).into(),
                widget::text("⚠  Only folders listed above can be accessed by the AI agent. All other paths are denied.")
                    .size(11).class(cosmic::theme::Text::Color(accent_orange)).into(),
            ])
            .spacing(0).padding([10, 12]),
        )
        .class(cosmic::theme::Container::Card).width(Length::Fill);

        let section_folder_access = settings_section_with_content(
            "Folder Access Whitelist",
            "Only whitelisted folders can be read or written by the AI agent",
            widget::column::with_children(vec![
                fa_header.into(),
                widget::Space::new(0, 8).into(),
                fa_list_body,
                widget::Space::new(0, 10).into(),
                widget::divider::horizontal::default().into(),
                widget::Space::new(0, 10).into(),
                fa_add_bar.into(),
            ])
            .spacing(0).into(),
        );

        // ── RAG Knowledge-Base Folders section ───────────────────────────────
        let rag_count = config.rag_folders.len();

        // Header: title + count badge
        let rag_header = widget::row::with_children(vec![
            widget::text("🗂  RAG Knowledge-Base Folders")
                .size(13).font(cosmic::font::bold()).width(Length::Fill).into(),
            widget::container(
                widget::text(format!("{} folder{}", rag_count,
                    if rag_count == 1 { "" } else { "s" }))
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        if rag_count == 0 { color_muted } else { accent_green }
                    )),
            )
            .padding([3, 8]).class(cosmic::theme::Container::Card).into(),
        ])
        .spacing(8).align_y(Alignment::Center);

        // List body
        let rag_list_body: Element<AppMessage> = if config.rag_folders.is_empty() {
            widget::container(
                widget::column::with_children(vec![
                    widget::Space::new(0, 18).into(),
                    widget::row::with_children(vec![
                        widget::Space::new(Length::Fill, 0).into(),
                        widget::column::with_children(vec![
                            widget::text("�").size(26).into(),
                            widget::Space::new(0, 6).into(),
                            widget::text("No RAG folders configured")
                                .size(13).font(cosmic::font::bold()).into(),
                            widget::Space::new(0, 3).into(),
                            widget::text("Add a folder to enable AI knowledge-base search.")
                                .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                            widget::text("Supported tools: LlamaIndex · Chroma · Qdrant · Weaviate")
                                .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                        ])
                        .spacing(2)
                        .align_x(cosmic::iced::Alignment::Center).into(),
                        widget::Space::new(Length::Fill, 0).into(),
                    ]).into(),
                    widget::Space::new(0, 18).into(),
                ])
                .spacing(0),
            )
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .into()
        } else {
            // Column header
            let rag_col_hdr = widget::container(
                widget::row::with_children(vec![
                    widget::text("#").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fixed(28.0)).into(),
                    widget::text("Name / Path").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fill).into(),
                    widget::text("Extensions").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fixed(120.0)).into(),
                    widget::text("Status").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fixed(100.0)).into(),
                    widget::text("Actions").size(11)
                        .class(cosmic::theme::Text::Color(color_muted))
                        .width(Length::Fixed(210.0)).into(),
                ])
                .spacing(8).align_y(Alignment::Center).padding([5, 10]),
            )
            .width(Length::Fill);

            let mut rag_rows: Vec<Element<AppMessage>> = vec![
                rag_col_hdr.into(),
                widget::divider::horizontal::light().into(),
            ];

            for (idx, rf) in config.rag_folders.iter().enumerate() {
                let ext_str = if rf.include_extensions.is_empty() {
                    "all".to_string()
                } else {
                    rf.include_extensions.iter()
                        .map(|e| format!(".{}", e))
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                // Extension badge
                let ext_badge = widget::container(
                    widget::text(ext_str)
                        .size(10)
                        .class(cosmic::theme::Text::Color(color_accent_blue)),
                ).padding([2, 5]).class(cosmic::theme::Container::Card);

                // Status badges
                let badge_watch = widget::container(
                    widget::text(if rf.watch_enabled { "● WATCH" } else { "○ PAUSED" })
                        .size(10).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(
                            if rf.watch_enabled { accent_green } else { color_muted }
                        )),
                ).padding([2, 5]).class(cosmic::theme::Container::Card);

                let badge_write = widget::container(
                    widget::text(if rf.allow_agent_write { "✎ WRITE" } else { "READ" })
                        .size(10).font(cosmic::font::bold())
                        .class(cosmic::theme::Text::Color(
                            if rf.allow_agent_write { accent_orange } else { color_muted }
                        )),
                ).padding([2, 5]).class(cosmic::theme::Container::Card);

                let status_col = widget::row::with_children(vec![
                    badge_watch.into(), badge_write.into(),
                ])
                .spacing(4).align_y(Alignment::Center).width(Length::Fixed(100.0));

                // Action buttons
                let btn_watch = widget::button::text(
                    if rf.watch_enabled { "✓ Watch" } else { "Watch" }
                )
                .on_press(AppMessage::RagFolderToggleWatch(idx))
                .class(if rf.watch_enabled {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                });

                let btn_write_rag = widget::button::text(
                    if rf.allow_agent_write { "✓ Write" } else { "Write" }
                )
                .on_press(AppMessage::RagFolderToggleWrite(idx))
                .class(if rf.allow_agent_write {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                });

                let btn_remove_rag = widget::button::destructive("✕ Remove")
                    .on_press(AppMessage::RagFolderRemove(idx));

                let rag_actions = widget::row::with_children(vec![
                    btn_watch.into(), btn_write_rag.into(), btn_remove_rag.into(),
                ])
                .spacing(4).align_y(Alignment::Center).width(Length::Fixed(210.0));

                let name_col = widget::column::with_children(vec![
                    widget::text(rf.name.clone())
                        .size(12).font(cosmic::font::bold()).into(),
                    widget::text(rf.host_path.display().to_string())
                        .size(10).class(cosmic::theme::Text::Color(color_muted)).into(),
                    if !rf.description.is_empty() {
                        widget::text(rf.description.clone())
                            .size(10).class(cosmic::theme::Text::Color(color_muted)).into()
                    } else {
                        widget::Space::new(0, 0).into()
                    },
                ])
                .spacing(1).width(Length::Fill);

                let row_bg = if idx % 2 == 0 {
                    cosmic::theme::Container::Card
                } else {
                    cosmic::theme::Container::Primary
                };

                let rag_entry = widget::container(
                    widget::row::with_children(vec![
                        widget::text(format!("{}", idx + 1)).size(11)
                            .class(cosmic::theme::Text::Color(color_muted))
                            .width(Length::Fixed(28.0)).into(),
                        name_col.into(),
                        ext_badge.width(Length::Fixed(120.0)).into(),
                        status_col.into(),
                        rag_actions.into(),
                    ])
                    .spacing(8).align_y(Alignment::Center).padding([8, 10]),
                )
                .class(row_bg).width(Length::Fill);

                rag_rows.push(rag_entry.into());
            }

            widget::container(widget::column::with_children(rag_rows).spacing(0))
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .into()
        };

        // Add-RAG-folder toolbar
        let rag_add_bar = widget::container(
            widget::column::with_children(vec![
                widget::row::with_children(vec![
                    widget::button::suggested("🗂  Browse…")
                        .on_press(AppMessage::RagFolderPickFolder).into(),
                    widget::text_input("/path/to/knowledge-base", rag_folder_path)
                        .on_input(AppMessage::RagFolderPathChanged)
                        .width(Length::Fill).into(),
                    widget::text_input("Name (optional)", rag_folder_name)
                        .on_input(AppMessage::RagFolderNameChanged)
                        .width(Length::Fixed(160.0)).into(),
                    widget::button::suggested("+ Add Folder")
                        .on_press(AppMessage::RagFolderAdd {
                            path: rag_folder_path.to_string(),
                            name: rag_folder_name.to_string(),
                        }).into(),
                ])
                .spacing(8).align_y(Alignment::Center).into(),
                widget::Space::new(0, 6).into(),
                widget::container(
                    widget::column::with_children(vec![
                        widget::text("Default indexed types: .md  .txt  .pdf  .rst  .html  .json")
                            .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                        widget::text("Compatible vectorizers: LlamaIndex · Chroma · Qdrant · Weaviate · pgvector")
                            .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                        widget::text("Files added to the folder are automatically detected when Watch is enabled.")
                            .size(11).class(cosmic::theme::Text::Color(color_muted)).into(),
                    ])
                    .spacing(3).padding([8, 10]),
                )
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .into(),
            ])
            .spacing(0).padding([10, 12]),
        )
        .class(cosmic::theme::Container::Card).width(Length::Fill);

        let section_rag = settings_section_with_content(
            "RAG Knowledge-Base Folders",
            "Add folders for vectorization — files here become searchable by the AI agent",
            widget::column::with_children(vec![
                rag_header.into(),
                widget::Space::new(0, 8).into(),
                rag_list_body,
                widget::Space::new(0, 10).into(),
                widget::divider::horizontal::default().into(),
                widget::Space::new(0, 10).into(),
                rag_add_bar.into(),
            ])
            .spacing(0).into(),
        );

        let config_path = dirs::config_dir()
            .unwrap_or_default()
            .join("openclaw-plus")
            .join("config.toml")
            .display()
            .to_string();

        widget::scrollable(
            widget::column::with_children(vec![
                widget::text(s.set_title)
                    .size(22)
                    .font(cosmic::font::bold())
                    .into(),
                widget::Space::new(0, 16).into(),
                section_appearance,
                widget::Space::new(0, 12).into(),
                section_agent,
                widget::Space::new(0, 12).into(),
                section_ai,
                widget::Space::new(0, 12).into(),
                section_model_mgmt,
                widget::Space::new(0, 12).into(),
                section_folder_access,
                widget::Space::new(0, 12).into(),
                section_rag,
                widget::Space::new(0, 12).into(),
                section_github,
                widget::Space::new(0, 12).into(),
                section_wasm_policy,
                widget::Space::new(0, 12).into(),
                section_sandbox,
                widget::Space::new(0, 12).into(),
                section_intercept,
                widget::Space::new(0, 12).into(),
                section_network,
                widget::Space::new(0, 12).into(),
                section_fs,
                widget::Space::new(0, 12).into(),
                audit_section,
                widget::Space::new(0, 24).into(),
                widget::text(s.set_config_file)
                    .size(12)
                    .font(cosmic::font::bold())
                    .into(),
                widget::text(config_path)
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                    ))
                    .into(),
                widget::Space::new(0, 32).into(),
            ])
            .padding(24)
            .spacing(4),
        )
        .into()
    }
}

fn language_grid<'a>(lang: Language) -> Element<'a, AppMessage> {
    let langs = Language::all();

    let mut rows: Vec<Element<'a, AppMessage>> = Vec::new();
    let mut i = 0;
    while i < langs.len() {
        let mut row_children: Vec<Element<'a, AppMessage>> = Vec::new();
        for j in 0..4 {
            if let Some(&l) = langs.get(i + j) {
                let btn = widget::button::text(l.display_name())
                    .on_press(AppMessage::SetLanguage(l))
                    .class(if lang == l {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    });
                row_children.push(btn.into());
            }
        }
        rows.push(widget::row::with_children(row_children).spacing(8).into());
        i += 4;
    }

    widget::column::with_children(rows)
        .spacing(8)
        .into()
}

fn settings_section<'a>(
    title: &'a str,
    rows: Vec<Element<'a, AppMessage>>,
) -> Element<'a, AppMessage> {
    widget::container(
        widget::column::with_children(
            std::iter::once(
                widget::text(title)
                    .size(15)
                    .font(cosmic::font::bold())
                    .into(),
            )
            .chain(std::iter::once(widget::divider::horizontal::default().into()))
            .chain(rows.into_iter())
            .collect::<Vec<_>>(),
        )
        .spacing(8)
        .padding(16),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

fn settings_section_with_content<'a>(
    title: &'a str,
    subtitle: &'a str,
    content: Element<'a, AppMessage>,
) -> Element<'a, AppMessage> {
    widget::container(
        widget::column::with_children(vec![
            widget::text(title)
                .size(15)
                .font(cosmic::font::bold())
                .into(),
            widget::text(subtitle)
                .size(12)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)))
                .into(),
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

fn setting_row(label: &'static str, value: String, hint: &'static str) -> Element<'static, AppMessage> {
    widget::row::with_children(vec![
        widget::column::with_children(vec![
            widget::text(label).size(13).font(cosmic::font::bold()).into(),
            widget::text(hint)
                .size(11)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5)))
                .into(),
        ])
        .spacing(2)
        .width(Length::Fill)
        .into(),
        widget::text(value)
            .size(13)
            .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.3, 0.6, 0.9)))
            .into(),
    ])
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

/// A read-only toggle display row.
#[allow(dead_code)]
fn toggle_row<'a>(label: &'a str, enabled: bool, hint: &'a str) -> Element<'a, AppMessage> {
    interactive_toggle_row(Language::En, label, enabled, hint, AppMessage::Noop)
}

/// An interactive toggle row — clicking the button fires `on_toggle`.
fn interactive_toggle_row<'a>(
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
            widget::text(hint)
                .size(11)
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(0.52, 0.50, 0.48)))
                .into(),
        ])
        .spacing(2)
        .width(Length::Fill)
        .into(),
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
