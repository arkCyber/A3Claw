//! # `view_store.rs` — Plugin Store & Installed Pages
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Renders two related pages:
//!
//! - **Plugin Store** (`view_store`) — browsable, searchable, filterable list
//!   of all plugins available in the active registry (ClawPlus or OpenClaw).
//! - **Installed** (`view_installed`) — subset of plugins that have been
//!   downloaded and verified locally.
//!
//! Both pages share the `plugin_card` component which shows plugin metadata,
//! a warm-toned tag row, a status badge, and an install / uninstall action
//! button.
//!
//! ## Warm-tone notes
//! Tag badges use a warm amber text colour (`BADGE_AMBER`) to stand out
//! against the default theme background.  Status text for installed plugins
//! is rendered in a muted gold tone.  All colours are applied via
//! `widget::text(...).color(...)` since libcosmic containers do not support
//! arbitrary fill colours.

use crate::app::{NavPage, StoreMessage};
use crate::i18n;
use crate::types::{InstallState, LibrarySource, PluginEntry};
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget;
use cosmic::Element;
use std::collections::HashMap;

// ── Warm-tone palette (local to this module) ─────────────────────────────────

/// Amber used for tag badge text.
const BADGE_AMBER: Color = Color { r: 0.85, g: 0.52, b: 0.08, a: 1.0 };

/// Muted green-gold for "Installed" status text.
const STATUS_INSTALLED: Color = Color { r: 0.25, g: 0.65, b: 0.30, a: 1.0 };

/// Warm red for failed / error status text.
const STATUS_ERROR: Color = Color { r: 0.80, g: 0.22, b: 0.18, a: 1.0 };

pub fn view_store(
    source: &LibrarySource,
    plugins: &[PluginEntry],
    install_states: &HashMap<String, InstallState>,
    loading: bool,
    fetch_error: &Option<String>,
    search: &str,
    category_filter: &Option<String>,
) -> Element<'static, StoreMessage> {
    // Source switcher
    let src_row = widget::row()
        .push(if *source == LibrarySource::ClawPlus {
            widget::button::suggested("ClawPlus  (Rust/WASM)")
                .on_press(StoreMessage::SwitchSource(LibrarySource::ClawPlus))
        } else {
            widget::button::text("ClawPlus  (Rust/WASM)")
                .on_press(StoreMessage::SwitchSource(LibrarySource::ClawPlus))
        })
        .push(if *source == LibrarySource::OpenClaw {
            widget::button::suggested("OpenClaw  (JS)")
                .on_press(StoreMessage::SwitchSource(LibrarySource::OpenClaw))
        } else {
            widget::button::text("OpenClaw  (JS)")
                .on_press(StoreMessage::SwitchSource(LibrarySource::OpenClaw))
        })
        .spacing(8)
        .padding([10, 16, 6, 16]);

    let s = i18n::strings();
    let badge_text = match source {
        LibrarySource::ClawPlus => s.store_badge_clawplus,
        LibrarySource::OpenClaw => s.store_badge_openclaw,
    };
    let badge = widget::container(widget::text(badge_text).size(12))
        .padding([3, 16])
        .width(Length::Fill);

    if loading {
        return widget::column()
            .push(src_row)
            .push(badge)
            .push(widget::vertical_space())
            .push(
                widget::container(widget::text(s.store_loading).size(15))
                    .center_x(Length::Fill),
            )
            .push(widget::vertical_space())
            .into();
    }

    if let Some(err) = fetch_error {
        let err_col = widget::column()
            .push(widget::text(s.store_load_failed).size(15))
            .push(widget::text(err.clone()).size(12))
            .push(widget::button::suggested(s.store_retry).on_press(StoreMessage::Refresh))
            .spacing(10)
            .align_x(Alignment::Center);
        return widget::column()
            .push(src_row)
            .push(badge)
            .push(widget::vertical_space())
            .push(widget::container(err_col).center_x(Length::Fill))
            .push(widget::vertical_space())
            .into();
    }

    // Search bar
    let search_bar = widget::text_input(s.store_search_hint, search.to_string())
        .on_input(StoreMessage::SearchChanged)
        .width(Length::Fill);

    // Category chips
    let mut all_tags: Vec<String> = plugins.iter()
        .flat_map(|p| p.tags.iter().cloned())
        .collect();
    all_tags.sort();
    all_tags.dedup();

    let cat_filter = category_filter.clone();
    let all_active: &'static str = if cat_filter.is_none() {
        Box::leak(format!("{} \u{25cf}", s.store_all_filter).into_boxed_str())
    } else {
        s.store_all_filter
    };
    let all_lbl = all_active;
    let mut cat_row = widget::row().spacing(5);
    cat_row = cat_row.push(
        widget::button::text(all_lbl).on_press(StoreMessage::CategoryFilter(None))
    );
    for tag in &all_tags {
        let is_active = cat_filter.as_deref() == Some(tag.as_str());
        let lbl = if is_active { format!("{tag} \u{25cf}") } else { tag.clone() };
        let t = tag.clone();
        cat_row = cat_row.push(
            widget::button::text(lbl).on_press(StoreMessage::CategoryFilter(Some(t)))
        );
    }

    // Filter plugins
    let q = search.to_lowercase();
    let filtered: Vec<&PluginEntry> = plugins.iter().filter(|p| {
        let text_ok = q.is_empty()
            || p.name.to_lowercase().contains(&q)
            || p.description.to_lowercase().contains(&q)
            || p.tags.iter().any(|t| t.to_lowercase().contains(&q));
        let cat_ok = category_filter.as_ref()
            .map(|f| p.tags.iter().any(|t| t == f))
            .unwrap_or(true);
        text_ok && cat_ok
    }).collect();

    let count_lbl = crate::tr!(store_count, &filtered.len().to_string());

    let mut grid = widget::column().spacing(10);
    if filtered.is_empty() {
        grid = grid.push(
            widget::container(widget::text(s.store_no_results).size(14))
                .center_x(Length::Fill),
        );
    } else {
        for entry in filtered {
            grid = grid.push(plugin_card(entry, install_states));
        }
    }

    widget::column()
        .push(src_row)
        .push(badge)
        .push(widget::container(search_bar).padding([6, 16, 4, 16]))
        .push(
            widget::container(
                widget::scrollable(cat_row)
            )
            .padding([0, 16, 4, 16]),
        )
        .push(widget::container(widget::text(count_lbl).size(11)).padding([0, 16]))
        .push(widget::divider::horizontal::default())
        .push(
            widget::scrollable(widget::container(grid).padding([10, 16]))
                .height(Length::Fill),
        )
        .into()
}

pub fn view_installed(
    plugins: &[PluginEntry],
    install_states: &HashMap<String, InstallState>,
) -> Element<'static, StoreMessage> {
    let installed: Vec<&PluginEntry> = plugins.iter()
        .filter(|p| matches!(install_states.get(&p.id), Some(InstallState::Installed)))
        .collect();

    let s = i18n::strings();
    let count = installed.len();
    let mut col = widget::column()
        .push(widget::text(crate::tr!(installed_count, &count.to_string())).size(13))
        .push(widget::divider::horizontal::default())
        .spacing(10)
        .padding([16, 16]);

    if installed.is_empty() {
        col = col.push(
            widget::container(
                widget::column()
                    .push(widget::text(s.installed_empty).size(15))
                    .push(widget::text(s.installed_empty_hint).size(12))
                    .push(
                        widget::button::suggested(s.installed_browse)
                            .on_press(StoreMessage::NavTo(NavPage::Store)),
                    )
                    .spacing(10)
                    .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .padding([40, 0]),
        );
    } else {
        for entry in installed {
            col = col.push(plugin_card(entry, install_states));
        }
    }

    widget::scrollable(col).height(Length::Fill).into()
}

pub fn plugin_card(
    entry: &PluginEntry,
    install_states: &HashMap<String, InstallState>,
) -> Element<'static, StoreMessage> {
    let state = install_states.get(&entry.id).cloned().unwrap_or_default();

    let entry_id        = entry.id.clone();
    let entry_name      = entry.name.clone();
    let entry_version   = entry.version.clone();
    let entry_author    = entry.author.clone();
    let entry_downloads = entry.downloads;
    let entry_desc      = entry.description.clone();
    let entry_license   = entry.license.clone();

    // Tag badges — amber text on a lightly padded container gives a warm
    // "chip" appearance without requiring a custom background colour.
    let mut tags_row = widget::row().spacing(4);
    for tag in entry.tags.iter().cloned() {
        tags_row = tags_row.push(
            widget::container(
                widget::text(tag).size(10).class(cosmic::theme::Text::Color(BADGE_AMBER))
            ).padding([2, 6])
        );
    }

    // Status badge — warm-toned colours signal state at a glance.
    let s = i18n::strings();
    let state_badge: Element<'static, StoreMessage> = match &state {
        InstallState::Installed =>
            widget::text(s.card_installed).size(11).class(cosmic::theme::Text::Color(STATUS_INSTALLED)).into(),
        InstallState::Downloading { progress } =>
            widget::text(format!("↧ {:.0}%", progress * 100.0)).size(11)
                .class(cosmic::theme::Text::Color(BADGE_AMBER)).into(),
        InstallState::Verifying =>
            widget::text(s.card_verifying).size(11).class(cosmic::theme::Text::Color(BADGE_AMBER)).into(),
        InstallState::Installing =>
            widget::text(s.card_installing).size(11).class(cosmic::theme::Text::Color(BADGE_AMBER)).into(),
        InstallState::Failed(_) =>
            widget::text(s.card_failed).size(11).class(cosmic::theme::Text::Color(STATUS_ERROR)).into(),
        InstallState::Idle =>
            widget::text("").size(11).into(),
    };

    let action_btn: Element<'static, StoreMessage> = match &state {
        InstallState::Idle =>
            widget::button::suggested(s.card_install)
                .on_press(StoreMessage::InstallPlugin(entry_id))
                .into(),
        InstallState::Downloading { .. } | InstallState::Verifying | InstallState::Installing =>
            widget::button::text(s.card_working).into(),
        InstallState::Installed =>
            widget::button::destructive(s.card_uninstall)
                .on_press(StoreMessage::UninstallPlugin(entry_id))
                .into(),
        InstallState::Failed(_) =>
            widget::button::suggested(s.card_retry)
                .on_press(StoreMessage::InstallPlugin(entry_id))
                .into(),
    };

    let icon_ch = entry_name.chars().next().unwrap_or('?')
        .to_uppercase().next().unwrap_or('?').to_string();
    let icon = widget::container(widget::text(icon_ch).size(20))
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .padding(10);

    let meta = widget::column()
        .push(
            widget::row()
                .push(widget::text(entry_name).size(15))
                .push(widget::horizontal_space())
                .push(state_badge)
                .align_y(Alignment::Center),
        )
        .push(widget::text(format!(
            "v{entry_version}  \u{b7}  {entry_author}  \u{b7}  {entry_license}  \u{b7}  {entry_downloads}\u{2193}"
        )).size(11))
        .push(widget::text(entry_desc).size(13))
        .push(tags_row)
        .spacing(4)
        .width(Length::Fill);

    widget::container(
        widget::column()
            .push(
                widget::row()
                    .push(icon)
                    .push(meta)
                    .push(action_btn)
                    .spacing(10)
                    .align_y(Alignment::Start),
            )
            .spacing(4),
    )
    .padding(14)
    .width(Length::Fill)
    .into()
}
