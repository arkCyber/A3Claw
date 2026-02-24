//! Shared `Strings` struct — one instance per locale.
//!
//! Every field is `&'static str`.  Fields whose value contains `{0}` are
//! format templates; callers substitute values with `.replace("{0}", …)`.

#[derive(Debug)]
pub struct Strings {
    // ── Navigation sidebar ────────────────────────────────────────────────────
    pub nav_dashboard: &'static str,
    pub nav_store:     &'static str,
    pub nav_installed: &'static str,
    pub nav_chat:      &'static str,
    pub nav_ai_models: &'static str,
    pub nav_bot_api:   &'static str,
    pub nav_settings:  &'static str,
    pub nav_language:  &'static str,

    // ── Header page titles ────────────────────────────────────────────────────
    pub title_dashboard: &'static str,
    pub title_store:     &'static str,
    pub title_installed: &'static str,
    pub title_chat:      &'static str,
    pub title_ai_models: &'static str,
    pub title_bot_api:   &'static str,
    pub title_settings:  &'static str,

    // ── Plugin Store page ─────────────────────────────────────────────────────
    pub store_loading:        &'static str,
    pub store_load_failed:    &'static str,
    pub store_retry:          &'static str,
    pub store_search_hint:    &'static str,
    pub store_all_filter:     &'static str,
    /// Template: replace `{0}` with count.
    pub store_count:          &'static str,
    pub store_no_results:     &'static str,
    pub store_badge_clawplus: &'static str,
    pub store_badge_openclaw: &'static str,

    // ── Plugin card ───────────────────────────────────────────────────────────
    pub card_install:    &'static str,
    pub card_uninstall:  &'static str,
    pub card_retry:      &'static str,
    pub card_working:    &'static str,
    pub card_installed:  &'static str,
    pub card_verifying:  &'static str,
    pub card_installing: &'static str,
    pub card_failed:     &'static str,

    // ── Installed page ────────────────────────────────────────────────────────
    /// Template: replace `{0}` with count.
    pub installed_count:      &'static str,
    pub installed_empty:      &'static str,
    pub installed_empty_hint: &'static str,
    pub installed_browse:     &'static str,

    // ── AI Models page ────────────────────────────────────────────────────────
    pub ai_title:           &'static str,
    pub ai_subtitle:        &'static str,
    pub ai_add_profile:     &'static str,
    pub ai_endpoint:        &'static str,
    pub ai_model_name:      &'static str,
    pub ai_api_key:         &'static str,
    pub ai_max_tokens:      &'static str,
    pub ai_temperature:     &'static str,
    pub ai_test_conn:       &'static str,
    pub ai_set_active:      &'static str,
    pub ai_active:          &'static str,
    pub ai_remove:          &'static str,
    pub ai_status_unknown:  &'static str,
    pub ai_status_checking: &'static str,
    pub ai_no_model:        &'static str,

    // ── Bot & API page ────────────────────────────────────────────────────────
    pub bot_title:        &'static str,
    pub bot_subtitle:     &'static str,
    pub bot_add:          &'static str,
    pub bot_token:        &'static str,
    pub bot_webhook:      &'static str,
    pub bot_enable:       &'static str,
    pub bot_disable:      &'static str,
    pub bot_remove:       &'static str,
    pub bot_disconnected: &'static str,
    pub bot_connecting:   &'static str,
    pub api_title:        &'static str,
    pub api_subtitle:     &'static str,
    pub api_add:          &'static str,
    pub api_bind_host:    &'static str,
    pub api_port:         &'static str,
    pub api_auth_token:   &'static str,
    pub api_start:        &'static str,
    pub api_stop:         &'static str,
    pub api_remove:       &'static str,
    pub api_stopped:      &'static str,
    pub api_starting:     &'static str,

    // ── Chat page ─────────────────────────────────────────────────────────────
    pub chat_welcome:       &'static str,
    pub chat_welcome_hint:  &'static str,
    pub chat_welcome_tip:   &'static str,
    pub chat_input_hint:    &'static str,
    pub chat_send:          &'static str,
    pub chat_thinking:      &'static str,
    pub chat_clear:         &'static str,
    pub chat_system_prompt: &'static str,
    pub chat_hide_prompt:   &'static str,
    pub chat_show_prompt:   &'static str,
    pub chat_role_user:     &'static str,
    pub chat_role_asst:     &'static str,
    pub chat_role_system:   &'static str,
    pub chat_error_prefix:  &'static str,

    // ── Dashboard page ────────────────────────────────────────────────────────
    pub dash_refresh:       &'static str,
    pub dash_clear_log:     &'static str,
    pub dash_checking:      &'static str,
    pub dash_running:       &'static str,
    pub dash_stopped:       &'static str,
    pub dash_check_failed:  &'static str,
    pub dash_pid:           &'static str,
    pub dash_metrics:       &'static str,
    pub dash_intercepted:   &'static str,
    pub dash_blocked:       &'static str,
    pub dash_plugins:       &'static str,
    pub dash_memory:        &'static str,
    /// Template: replace `{0}` with count.
    pub dash_events_in_log: &'static str,
    pub dash_audit_log:     &'static str,
    pub dash_no_events:     &'static str,

    // ── Settings page ─────────────────────────────────────────────────────────
    pub settings_plugin_dir:   &'static str,
    pub settings_apply:        &'static str,
    pub settings_registry_url: &'static str,

    // ── Common ────────────────────────────────────────────────────────────────
    pub common_remove:  &'static str,
    pub common_refresh: &'static str,
}
