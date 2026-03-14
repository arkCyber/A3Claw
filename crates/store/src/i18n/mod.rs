//! Internationalization (i18n) for OpenClaw+ Store UI.
//!
//! # Usage
//! ```no_run
//! use openclaw_store::i18n::{set_locale, Locale};
//!
//! set_locale(Locale::ZhCn);
//! // tr!(nav_dashboard)       → "📊  仪表盘"
//! // tr!(store_count, "42")   → "找到 42 个插件"
//! ```
//!
//! # Adding a new locale
//! 1. Create `crates/store/src/i18n/<code>.rs` with a `pub static XX: Strings = Strings { … };`
//! 2. Add a `mod <code>;` line below.
//! 3. Add a variant to `Locale` and a match arm in `strings_for`.

pub mod strings;

mod ar;
mod de;
mod en;
mod es;
mod fr;
mod hi;
mod it;
mod ja;
mod ko;
mod nl;
mod pl;
mod pt;
mod ru;
mod tr;
mod tw;
mod zh;

pub use strings::Strings;

use std::sync::atomic::{AtomicU8, Ordering};

/// All supported UI locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Locale {
    En   = 0,
    ZhCn = 1,
    ZhTw = 2,
    Ja   = 3,
    Ko   = 4,
    Es   = 5,
    Fr   = 6,
    De   = 7,
    Pt   = 8,
    Ru   = 9,
    Ar   = 10,
    Hi   = 11,
    It   = 12,
    Nl   = 13,
    Tr   = 14,
    Pl   = 15,
}

impl Locale {
    /// Human-readable display name shown in the language switcher.
    pub fn display_name(self) -> &'static str {
        match self {
            Locale::En   => "English",
            Locale::ZhCn => "简体中文",
            Locale::ZhTw => "繁體中文",
            Locale::Ja   => "日本語",
            Locale::Ko   => "한국어",
            Locale::Es   => "Español",
            Locale::Fr   => "Français",
            Locale::De   => "Deutsch",
            Locale::Pt   => "Português",
            Locale::Ru   => "Русский",
            Locale::Ar   => "العربية",
            Locale::Hi   => "हिन्दी",
            Locale::It   => "Italiano",
            Locale::Nl   => "Nederlands",
            Locale::Tr   => "Türkçe",
            Locale::Pl   => "Polski",
        }
    }

    /// BCP-47 language tag.
    pub fn bcp47(self) -> &'static str {
        match self {
            Locale::En   => "en",
            Locale::ZhCn => "zh-Hans",
            Locale::ZhTw => "zh-Hant",
            Locale::Ja   => "ja",
            Locale::Ko   => "ko",
            Locale::Es   => "es",
            Locale::Fr   => "fr",
            Locale::De   => "de",
            Locale::Pt   => "pt",
            Locale::Ru   => "ru",
            Locale::Ar   => "ar",
            Locale::Hi   => "hi",
            Locale::It   => "it",
            Locale::Nl   => "nl",
            Locale::Tr   => "tr",
            Locale::Pl   => "pl",
        }
    }

    /// All locales in a stable order (used by the language switcher).
    pub fn all() -> &'static [Locale] {
        &[
            Locale::En,
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::Ja,
            Locale::Ko,
            Locale::Es,
            Locale::Fr,
            Locale::De,
            Locale::Pt,
            Locale::Ru,
            Locale::Ar,
            Locale::Hi,
            Locale::It,
            Locale::Nl,
            Locale::Tr,
            Locale::Pl,
        ]
    }

    fn from_u8(v: u8) -> Locale {
        match v {
            1  => Locale::ZhCn,
            2  => Locale::ZhTw,
            3  => Locale::Ja,
            4  => Locale::Ko,
            5  => Locale::Es,
            6  => Locale::Fr,
            7  => Locale::De,
            8  => Locale::Pt,
            9  => Locale::Ru,
            10 => Locale::Ar,
            11 => Locale::Hi,
            12 => Locale::It,
            13 => Locale::Nl,
            14 => Locale::Tr,
            15 => Locale::Pl,
            _  => Locale::En,
        }
    }
}

// ── Global locale state ───────────────────────────────────────────────────────

static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(Locale::En as u8);

/// Change the active locale for the whole application.
/// This is cheap (single atomic store) and takes effect on the next `tr!()` call.
pub fn set_locale(locale: Locale) {
    CURRENT_LOCALE.store(locale as u8, Ordering::Relaxed);
}

/// Return the currently active locale.
pub fn current_locale() -> Locale {
    Locale::from_u8(CURRENT_LOCALE.load(Ordering::Relaxed))
}

/// Return the `Strings` table for the given locale.
pub fn strings_for(locale: Locale) -> &'static Strings {
    match locale {
        Locale::En   => &en::EN,
        Locale::ZhCn => &zh::ZH,
        Locale::ZhTw => &tw::TW,
        Locale::Ja   => &ja::JA,
        Locale::Ko   => &ko::KO,
        Locale::Es   => &es::ES,
        Locale::Fr   => &fr::FR,
        Locale::De   => &de::DE,
        Locale::Pt   => &pt::PT,
        Locale::Ru   => &ru::RU,
        Locale::Ar   => &ar::AR,
        Locale::Hi   => &hi::HI,
        Locale::It   => &it::IT,
        Locale::Nl   => &nl::NL,
        Locale::Tr   => &tr::TR,
        Locale::Pl   => &pl::PL,
    }
}

/// Return the `Strings` table for the current locale.
#[inline]
pub fn strings() -> &'static Strings {
    strings_for(current_locale())
}

// ── tr!() macro ───────────────────────────────────────────────────────────────

/// Translate a string key using the current locale.
///
/// # Forms
/// ```no_run
/// // tr!(nav_dashboard)           → &'static str
/// // tr!(store_count, "42")       → String  (replaces first {0})
/// // tr!(store_count, "42", "x")  → String  (replaces {0}, {1}, …)
/// ```
#[macro_export]
macro_rules! tr {
    // Key only — returns &'static str directly (zero allocation).
    ($key:ident) => {
        $crate::i18n::strings().$key
    };
    // Key + one or more substitution values — returns an owned String.
    ($key:ident, $($val:expr),+ $(,)?) => {{
        let mut s = $crate::i18n::strings().$key.to_owned();
        let vals: &[&str] = &[$($val),+];
        for (i, v) in vals.iter().enumerate() {
            s = s.replace(&format!("{{{}}}", i), v);
        }
        s
    }};
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn all_strings(locale: Locale) -> &'static Strings {
        strings_for(locale)
    }

    /// Every field in every locale must be non-empty.
    #[test]
    fn all_locales_all_keys_non_empty() {
        for &locale in Locale::all() {
            let s = all_strings(locale);
            let name = locale.display_name();

            assert!(!s.nav_dashboard.is_empty(),      "{name}: nav_dashboard");
            assert!(!s.nav_store.is_empty(),           "{name}: nav_store");
            assert!(!s.nav_installed.is_empty(),       "{name}: nav_installed");
            assert!(!s.nav_chat.is_empty(),            "{name}: nav_chat");
            assert!(!s.nav_ai_models.is_empty(),       "{name}: nav_ai_models");
            assert!(!s.nav_bot_api.is_empty(),         "{name}: nav_bot_api");
            assert!(!s.nav_settings.is_empty(),        "{name}: nav_settings");
            assert!(!s.nav_language.is_empty(),        "{name}: nav_language");

            assert!(!s.title_dashboard.is_empty(),     "{name}: title_dashboard");
            assert!(!s.title_store.is_empty(),         "{name}: title_store");
            assert!(!s.title_installed.is_empty(),     "{name}: title_installed");
            assert!(!s.title_chat.is_empty(),          "{name}: title_chat");
            assert!(!s.title_ai_models.is_empty(),     "{name}: title_ai_models");
            assert!(!s.title_bot_api.is_empty(),       "{name}: title_bot_api");
            assert!(!s.title_settings.is_empty(),      "{name}: title_settings");

            assert!(!s.store_loading.is_empty(),       "{name}: store_loading");
            assert!(!s.store_load_failed.is_empty(),   "{name}: store_load_failed");
            assert!(!s.store_retry.is_empty(),         "{name}: store_retry");
            assert!(!s.store_search_hint.is_empty(),   "{name}: store_search_hint");
            assert!(!s.store_all_filter.is_empty(),    "{name}: store_all_filter");
            assert!(!s.store_count.is_empty(),         "{name}: store_count");
            assert!(!s.store_no_results.is_empty(),    "{name}: store_no_results");
            assert!(!s.store_badge_clawplus.is_empty(),"{name}: store_badge_clawplus");
            assert!(!s.store_badge_openclaw.is_empty(),"{name}: store_badge_openclaw");

            assert!(!s.card_install.is_empty(),        "{name}: card_install");
            assert!(!s.card_uninstall.is_empty(),      "{name}: card_uninstall");
            assert!(!s.card_retry.is_empty(),          "{name}: card_retry");
            assert!(!s.card_working.is_empty(),        "{name}: card_working");
            assert!(!s.card_installed.is_empty(),      "{name}: card_installed");
            assert!(!s.card_verifying.is_empty(),      "{name}: card_verifying");
            assert!(!s.card_installing.is_empty(),     "{name}: card_installing");
            assert!(!s.card_failed.is_empty(),         "{name}: card_failed");

            assert!(!s.installed_count.is_empty(),     "{name}: installed_count");
            assert!(!s.installed_empty.is_empty(),     "{name}: installed_empty");
            assert!(!s.installed_empty_hint.is_empty(),"{name}: installed_empty_hint");
            assert!(!s.installed_browse.is_empty(),    "{name}: installed_browse");

            assert!(!s.ai_title.is_empty(),            "{name}: ai_title");
            assert!(!s.ai_subtitle.is_empty(),         "{name}: ai_subtitle");
            assert!(!s.ai_add_profile.is_empty(),      "{name}: ai_add_profile");
            assert!(!s.ai_endpoint.is_empty(),         "{name}: ai_endpoint");
            assert!(!s.ai_model_name.is_empty(),       "{name}: ai_model_name");
            assert!(!s.ai_api_key.is_empty(),          "{name}: ai_api_key");
            assert!(!s.ai_max_tokens.is_empty(),       "{name}: ai_max_tokens");
            assert!(!s.ai_temperature.is_empty(),      "{name}: ai_temperature");
            assert!(!s.ai_test_conn.is_empty(),        "{name}: ai_test_conn");
            assert!(!s.ai_set_active.is_empty(),       "{name}: ai_set_active");
            assert!(!s.ai_active.is_empty(),           "{name}: ai_active");
            assert!(!s.ai_remove.is_empty(),           "{name}: ai_remove");
            assert!(!s.ai_status_unknown.is_empty(),   "{name}: ai_status_unknown");
            assert!(!s.ai_status_checking.is_empty(),  "{name}: ai_status_checking");
            assert!(!s.ai_no_model.is_empty(),         "{name}: ai_no_model");

            assert!(!s.bot_title.is_empty(),           "{name}: bot_title");
            assert!(!s.bot_subtitle.is_empty(),        "{name}: bot_subtitle");
            assert!(!s.bot_add.is_empty(),             "{name}: bot_add");
            assert!(!s.bot_token.is_empty(),           "{name}: bot_token");
            assert!(!s.bot_webhook.is_empty(),         "{name}: bot_webhook");
            assert!(!s.bot_enable.is_empty(),          "{name}: bot_enable");
            assert!(!s.bot_disable.is_empty(),         "{name}: bot_disable");
            assert!(!s.bot_remove.is_empty(),          "{name}: bot_remove");
            assert!(!s.bot_disconnected.is_empty(),    "{name}: bot_disconnected");
            assert!(!s.bot_connecting.is_empty(),      "{name}: bot_connecting");

            assert!(!s.api_title.is_empty(),           "{name}: api_title");
            assert!(!s.api_subtitle.is_empty(),        "{name}: api_subtitle");
            assert!(!s.api_add.is_empty(),             "{name}: api_add");
            assert!(!s.api_bind_host.is_empty(),       "{name}: api_bind_host");
            assert!(!s.api_port.is_empty(),            "{name}: api_port");
            assert!(!s.api_auth_token.is_empty(),      "{name}: api_auth_token");
            assert!(!s.api_start.is_empty(),           "{name}: api_start");
            assert!(!s.api_stop.is_empty(),            "{name}: api_stop");
            assert!(!s.api_remove.is_empty(),          "{name}: api_remove");
            assert!(!s.api_stopped.is_empty(),         "{name}: api_stopped");
            assert!(!s.api_starting.is_empty(),        "{name}: api_starting");

            assert!(!s.chat_welcome.is_empty(),        "{name}: chat_welcome");
            assert!(!s.chat_welcome_hint.is_empty(),   "{name}: chat_welcome_hint");
            assert!(!s.chat_welcome_tip.is_empty(),    "{name}: chat_welcome_tip");
            assert!(!s.chat_input_hint.is_empty(),     "{name}: chat_input_hint");
            assert!(!s.chat_send.is_empty(),           "{name}: chat_send");
            assert!(!s.chat_thinking.is_empty(),       "{name}: chat_thinking");
            assert!(!s.chat_clear.is_empty(),          "{name}: chat_clear");
            assert!(!s.chat_system_prompt.is_empty(),  "{name}: chat_system_prompt");
            assert!(!s.chat_hide_prompt.is_empty(),    "{name}: chat_hide_prompt");
            assert!(!s.chat_show_prompt.is_empty(),    "{name}: chat_show_prompt");
            assert!(!s.chat_role_user.is_empty(),      "{name}: chat_role_user");
            assert!(!s.chat_role_asst.is_empty(),      "{name}: chat_role_asst");
            assert!(!s.chat_role_system.is_empty(),    "{name}: chat_role_system");
            assert!(!s.chat_error_prefix.is_empty(),   "{name}: chat_error_prefix");

            assert!(!s.dash_refresh.is_empty(),        "{name}: dash_refresh");
            assert!(!s.dash_clear_log.is_empty(),      "{name}: dash_clear_log");
            assert!(!s.dash_checking.is_empty(),       "{name}: dash_checking");
            assert!(!s.dash_running.is_empty(),        "{name}: dash_running");
            assert!(!s.dash_stopped.is_empty(),        "{name}: dash_stopped");
            assert!(!s.dash_check_failed.is_empty(),   "{name}: dash_check_failed");
            assert!(!s.dash_pid.is_empty(),            "{name}: dash_pid");
            assert!(!s.dash_metrics.is_empty(),        "{name}: dash_metrics");
            assert!(!s.dash_intercepted.is_empty(),    "{name}: dash_intercepted");
            assert!(!s.dash_blocked.is_empty(),        "{name}: dash_blocked");
            assert!(!s.dash_plugins.is_empty(),        "{name}: dash_plugins");
            assert!(!s.dash_memory.is_empty(),         "{name}: dash_memory");
            assert!(!s.dash_events_in_log.is_empty(),  "{name}: dash_events_in_log");
            assert!(!s.dash_audit_log.is_empty(),      "{name}: dash_audit_log");
            assert!(!s.dash_no_events.is_empty(),      "{name}: dash_no_events");

            assert!(!s.settings_plugin_dir.is_empty(),   "{name}: settings_plugin_dir");
            assert!(!s.settings_apply.is_empty(),         "{name}: settings_apply");
            assert!(!s.settings_registry_url.is_empty(),  "{name}: settings_registry_url");

            assert!(!s.common_remove.is_empty(),       "{name}: common_remove");
            assert!(!s.common_refresh.is_empty(),      "{name}: common_refresh");
        }
    }

    /// Template keys must contain `{0}`.
    #[test]
    fn template_keys_contain_placeholder() {
        for &locale in Locale::all() {
            let s = all_strings(locale);
            let name = locale.display_name();
            assert!(s.store_count.contains("{0}"),      "{name}: store_count missing {{0}}");
            assert!(s.installed_count.contains("{0}"),  "{name}: installed_count missing {{0}}");
            assert!(s.dash_events_in_log.contains("{0}"),"{name}: dash_events_in_log missing {{0}}");
        }
    }

    /// `set_locale` / `current_locale` round-trip for every variant.
    #[test]
    fn locale_round_trip() {
        for &locale in Locale::all() {
            set_locale(locale);
            assert_eq!(current_locale(), locale, "round-trip failed for {:?}", locale);
        }
        set_locale(Locale::En);
    }

    /// `tr!` macro substitution works correctly.
    #[test]
    fn tr_macro_substitution() {
        set_locale(Locale::En);
        let result = crate::tr!(store_count, "7");
        assert_eq!(result, "7 plugin(s) found");
    }

    /// `tr!` macro with no args returns `&'static str`.
    #[test]
    fn tr_macro_no_args() {
        set_locale(Locale::En);
        let label: &'static str = crate::tr!(nav_dashboard);
        assert!(label.contains("Dashboard"));
    }

    /// All locales have distinct `display_name` values.
    #[test]
    fn display_names_are_unique() {
        let names: Vec<_> = Locale::all().iter().map(|l| l.display_name()).collect();
        let mut dedup = names.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(names.len(), dedup.len(), "duplicate display names found");
    }

    /// All locales have distinct BCP-47 tags.
    #[test]
    fn bcp47_tags_are_unique() {
        let tags: Vec<_> = Locale::all().iter().map(|l| l.bcp47()).collect();
        let mut dedup = tags.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(tags.len(), dedup.len(), "duplicate BCP-47 tags found");
    }

    /// `Locale::all()` covers all 16 variants.
    #[test]
    fn all_returns_sixteen_locales() {
        assert_eq!(Locale::all().len(), 16);
    }
}
