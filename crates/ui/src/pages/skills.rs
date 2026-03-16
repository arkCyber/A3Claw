//! Skills Browser Page — browse and search all available skills
//!
//! This page provides a comprehensive view of all 310+ built-in skills,
//! organized by category with search and filtering capabilities.

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::AppMessage;
use crate::theme::{Language, tx};

/// Skill information for display
#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub risk_level: SkillRisk,
    pub parameters: Vec<SkillParam>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRisk {
    Safe,
    Confirm,
    Deny,
}

impl SkillRisk {
    pub fn color(&self) -> cosmic::iced::Color {
        match self {
            SkillRisk::Safe => cosmic::iced::Color::from_rgb(0.22, 0.82, 0.46),
            SkillRisk::Confirm => cosmic::iced::Color::from_rgb(0.98, 0.72, 0.22),
            SkillRisk::Deny => cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
        }
    }

    pub fn label(&self, lang: Language) -> &'static str {
        match self {
            SkillRisk::Safe => tx(lang, "Safe"),
            SkillRisk::Confirm => tx(lang, "Confirm"),
            SkillRisk::Deny => tx(lang, "Deny"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

pub struct SkillsPage;

impl SkillsPage {
    /// Main view for the Skills Browser page
    pub fn view<'a>(
        skills: &'a [SkillInfo],
        search_query: &'a str,
        selected_category: Option<&'a str>,
        selected_skill: Option<&'a str>,
        lang: Language,
    ) -> Element<'a, AppMessage> {
        // Get unique categories
        let categories: Vec<String> = skills
            .iter()
            .map(|s| s.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Build the page components
        let header = Self::build_header(lang, skills.len(), skills.len());
        let search_bar = Self::build_search_bar(search_query, lang);
        let category_filter = Self::build_category_filter(&categories, selected_category, lang);
        
        // Build skills list directly from skills slice
        let skills_list = Self::build_skills_list_from_slice(
            skills,
            search_query,
            selected_category,
            selected_skill,
            lang,
        );
        
        let details_panel = if let Some(skill_name) = selected_skill {
            if let Some(skill) = skills.iter().find(|s| s.name == skill_name) {
                Self::build_details_panel(Some(&skill), lang)
            } else {
                Self::build_empty_details(lang)
            }
        } else {
            Self::build_empty_details(lang)
        };

        // Layout: left sidebar (categories + list), right panel (details)
        let left_panel = widget::column::with_children(vec![
            header,
            widget::divider::horizontal::default().into(),
            search_bar,
            category_filter,
            widget::divider::horizontal::default().into(),
            skills_list,
        ])
        .spacing(8)
        .padding(12)
        .width(Length::FillPortion(2));

        let right_panel = widget::container(details_panel)
            .padding(12)
            .width(Length::FillPortion(3))
            .height(Length::Fill);

        widget::row::with_children(vec![
            left_panel.into(),
            widget::container(widget::Space::new(1, 0))
                .style(|theme: &cosmic::Theme| {
                    let c = theme.cosmic().bg_divider();
                    cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgb(c.red, c.green, c.blue),
                        )),
                        ..Default::default()
                    }
                })
                .height(Length::Fill)
                .into(),
            right_panel.into(),
        ])
        .height(Length::Fill)
        .into()
    }

    fn build_header<'a>(lang: Language, total: usize, filtered: usize) -> Element<'a, AppMessage> {
        let title = widget::text(tx(lang, "Skills Browser"))
            .size(20)
            .font(cosmic::font::bold());

        let count = if filtered == total {
            widget::text(format!("{} skills", total)).size(13)
        } else {
            widget::text(format!("{} / {} skills", filtered, total)).size(13)
        };

        widget::column::with_children(vec![
            title.into(),
            count.into(),
        ])
        .spacing(4)
        .into()
    }

    fn build_search_bar<'a>(query: &'a str, lang: Language) -> Element<'a, AppMessage> {
        widget::text_input(tx(lang, "Search skills..."), query)
            .on_input(AppMessage::SkillSearchChanged)
            .width(Length::Fill)
            .into()
    }

    fn build_category_filter<'a>(
        categories: &[String],
        selected: Option<&str>,
        lang: Language,
    ) -> Element<'a, AppMessage> {
        let mut buttons = vec![
            widget::button::text(tx(lang, "All"))
                .on_press(AppMessage::SkillCategorySelected(None))
                .class(if selected.is_none() {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                })
                .into(),
        ];

        for category in categories {
            let is_selected = selected == Some(category.as_str());
            let category_owned = category.clone();
            buttons.push(
                widget::button::text(category_owned.clone())
                    .on_press(AppMessage::SkillCategorySelected(Some(category_owned)))
                    .class(if is_selected {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
                    .into(),
            );
        }

        widget::row::with_children(buttons)
            .spacing(6)
            .wrap()
            .into()
    }

    fn build_skills_list_from_slice<'a>(
        skills: &'a [SkillInfo],
        search_query: &str,
        selected_category: Option<&str>,
        selected_skill: Option<&str>,
        lang: Language,
    ) -> Element<'a, AppMessage> {
        let mut items: Vec<Element<'a, AppMessage>> = Vec::new();
        let mut count = 0;

        for skill in skills {
            // Filter by search
            let matches_search = if search_query.is_empty() {
                true
            } else {
                skill.name.to_lowercase().contains(&search_query.to_lowercase())
                    || skill.display_name.to_lowercase().contains(&search_query.to_lowercase())
                    || skill.description.to_lowercase().contains(&search_query.to_lowercase())
            };

            // Filter by category
            let matches_category = if let Some(cat) = selected_category {
                &skill.category == cat
            } else {
                true
            };

            if matches_search && matches_category {
                let is_selected = selected_skill == Some(&skill.name);
                items.push(Self::build_skill_item(skill, is_selected, lang));
                count += 1;
            }
        }

        if count == 0 {
            return widget::container(
                widget::text(tx(lang, "No skills found"))
                    .size(14)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                    )),
            )
            .padding(20)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into();
        }

        widget::scrollable(
            widget::column::with_children(items)
                .spacing(4)
                .padding(4),
        )
        .height(Length::Fill)
        .into()
    }

    fn build_skill_item<'a>(
        skill: &'a SkillInfo,
        is_selected: bool,
        _lang: Language,
    ) -> Element<'a, AppMessage> {
        let risk_color = skill.risk_level.color();
        let risk_dot = widget::container(widget::Space::new(8, 8))
            .style(move |_: &cosmic::Theme| cosmic::iced::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(risk_color)),
                border: cosmic::iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let content = widget::row::with_children(vec![
            risk_dot.into(),
            widget::column::with_children(vec![
                widget::text(&skill.display_name)
                    .size(13)
                    .font(cosmic::font::bold())
                    .into(),
                widget::text(&skill.name)
                    .size(11)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                    ))
                    .into(),
            ])
            .spacing(2)
            .width(Length::Fill)
            .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([6, 8]);

        widget::button::custom(content)
            .on_press(AppMessage::SkillSelected(skill.name.clone()))
            .class(if is_selected {
                cosmic::theme::Button::MenuItem
            } else {
                cosmic::theme::Button::MenuRoot
            })
            .width(Length::Fill)
            .into()
    }

    fn build_details_panel<'a>(
        skill: Option<&&'a SkillInfo>,
        lang: Language,
    ) -> Element<'a, AppMessage> {
        if let Some(skill) = skill {
            let title = widget::text(&skill.display_name)
                .size(18)
                .font(cosmic::font::bold());

            let name_label = widget::text(tx(lang, "Skill Name:"))
                .size(12)
                .font(cosmic::font::bold());
            let name_value = widget::text(&skill.name).size(12);

            let category_label = widget::text(tx(lang, "Category:"))
                .size(12)
                .font(cosmic::font::bold());
            let category_value = widget::text(&skill.category).size(12);

            let risk_label = widget::text(tx(lang, "Risk Level:"))
                .size(12)
                .font(cosmic::font::bold());
            let risk_color = skill.risk_level.color();
            let risk_value = widget::text(skill.risk_level.label(lang))
                .size(12)
                .class(cosmic::theme::Text::Color(risk_color));

            let desc_label = widget::text(tx(lang, "Description:"))
                .size(12)
                .font(cosmic::font::bold());
            let desc_value = widget::text(&skill.description).size(12);

            let mut content = vec![
                title.into(),
                widget::Space::new(0, 12).into(),
                widget::row::with_children(vec![
                    name_label.into(),
                    widget::Space::new(8, 0).into(),
                    name_value.into(),
                ])
                .into(),
                widget::Space::new(0, 6).into(),
                widget::row::with_children(vec![
                    category_label.into(),
                    widget::Space::new(8, 0).into(),
                    category_value.into(),
                ])
                .into(),
                widget::Space::new(0, 6).into(),
                widget::row::with_children(vec![
                    risk_label.into(),
                    widget::Space::new(8, 0).into(),
                    risk_value.into(),
                ])
                .into(),
                widget::Space::new(0, 12).into(),
                widget::divider::horizontal::default().into(),
                widget::Space::new(0, 12).into(),
                desc_label.into(),
                widget::Space::new(0, 6).into(),
                desc_value.into(),
            ];

            // Parameters section
            if !skill.parameters.is_empty() {
                content.push(widget::Space::new(0, 12).into());
                content.push(widget::divider::horizontal::default().into());
                content.push(widget::Space::new(0, 12).into());
                content.push(
                    widget::text(tx(lang, "Parameters:"))
                        .size(12)
                        .font(cosmic::font::bold())
                        .into(),
                );
                content.push(widget::Space::new(0, 8).into());

                for param in &skill.parameters {
                    let required_badge = if param.required {
                        widget::text("required")
                            .size(10)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.92, 0.28, 0.28),
                            ))
                    } else {
                        widget::text("optional")
                            .size(10)
                            .class(cosmic::theme::Text::Color(
                                cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                            ))
                    };

                    content.push(
                        widget::column::with_children(vec![
                            widget::row::with_children(vec![
                                widget::text(&param.name)
                                    .size(12)
                                    .font(cosmic::font::bold())
                                    .into(),
                                widget::Space::new(8, 0).into(),
                                widget::text(format!("({})", param.param_type))
                                    .size(11)
                                    .class(cosmic::theme::Text::Color(
                                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                                    ))
                                    .into(),
                                widget::Space::new(8, 0).into(),
                                required_badge.into(),
                            ])
                            .into(),
                            widget::text(&param.description)
                                .size(11)
                                .class(cosmic::theme::Text::Color(
                                    cosmic::iced::Color::from_rgb(0.65, 0.62, 0.58),
                                ))
                                .into(),
                        ])
                        .spacing(2)
                        .into(),
                    );
                    content.push(widget::Space::new(0, 8).into());
                }
            }

            // Quick execute button
            content.push(widget::Space::new(0, 12).into());
            content.push(widget::divider::horizontal::default().into());
            content.push(widget::Space::new(0, 12).into());
            content.push(
                widget::button::suggested(tx(lang, "Execute in Terminal"))
                    .on_press(AppMessage::SkillExecuteInTerminal(skill.name.clone()))
                    .into(),
            );

            widget::scrollable(widget::column::with_children(content).spacing(0))
                .height(Length::Fill)
                .into()
        } else {
            Self::build_empty_details(lang)
        }
    }

    fn build_empty_details<'a>(lang: Language) -> Element<'a, AppMessage> {
        widget::container(
            widget::column::with_children(vec![
                widget::text(tx(lang, "Select a skill"))
                    .size(16)
                    .font(cosmic::font::bold())
                    .into(),
                widget::Space::new(0, 8).into(),
                widget::text(tx(lang, "Choose a skill from the list to view details"))
                    .size(13)
                    .class(cosmic::theme::Text::Color(
                        cosmic::iced::Color::from_rgb(0.55, 0.52, 0.48),
                    ))
                    .into(),
            ])
            .spacing(0)
            .align_x(Alignment::Center),
        )
        .padding(40)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
