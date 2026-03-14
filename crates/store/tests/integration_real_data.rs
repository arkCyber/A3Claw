//! Integration tests for `openclaw-store` using real data.
//!
//! Covers:
//! - `RegistryIndex` / `PluginEntry` JSON round-trip with real registry fixture
//! - `RegistryClient::fetch_index` via `file://` path (no network required)
//! - `RegistryClient::uninstall_plugin` idempotent delete
//! - `RegistryClient::installed_ids` scanning a real temp directory
//! - `i18n`: all 16 locales produce non-empty nav/title strings and no key collisions
//! - `types`: serde round-trips for every serialisable enum and struct

use openclaw_store::i18n::{self, Locale};
use openclaw_store::types::*;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_plugin_entry(id: &str, version: &str) -> PluginEntry {
    PluginEntry {
        id: id.to_string(),
        name: format!("Plugin {}", id),
        description: "Test plugin".to_string(),
        long_description: "Detailed description.".to_string(),
        version: version.to_string(),
        author: "OpenClaw Test".to_string(),
        author_url: "https://github.com/openclaw".to_string(),
        license: "MIT".to_string(),
        wasm_url: format!("https://registry.example.com/{}-{}.wasm", id, version),
        sha256: "a".repeat(64),
        signature: String::new(),
        min_host_version: "0.1.0".to_string(),
        tags: vec!["test".to_string(), "integration".to_string()],
        category: PluginCategory::Developer,
        language: PluginLanguage::Rust,
        build_target: "wasm32-wasip1".to_string(),
        converter: String::new(),
        icon_url: String::new(),
        screenshot_urls: vec![],
        permissions: PluginPermissions::default(),
        source: LibrarySource::ClawPlus,
        downloads: 42,
        rating: 4.5,
        rating_count: 10,
        reviews: vec![],
        reviews_url: String::new(),
        changelog: vec![],
        skills: vec![],
        preinstalled: false,
        installed: false,
        download_progress: None,
    }
}

fn make_registry_index() -> RegistryIndex {
    RegistryIndex {
        version: 2,
        name: "OpenClaw Test Registry".to_string(),
        description: "Integration test fixture registry".to_string(),
        plugins: vec![
            make_plugin_entry("com.clawplus.downloader", "1.0.0"),
            make_plugin_entry("com.clawplus.converter",  "2.1.3"),
            make_plugin_entry("com.clawplus.summarizer", "0.9.5"),
        ],
        category_index_urls: std::collections::HashMap::new(),
        stats: RegistryStats {
            total_plugins: 3,
            total_downloads: 42,
            total_authors: 2,
            supported_languages: 4,
        },
        updated_at: 1_710_000_000,
    }
}

// ── RegistryIndex serde round-trip ────────────────────────────────────────────

#[test]
fn registry_index_json_roundtrip_real_data() {
    let index = make_registry_index();
    let json = serde_json::to_string_pretty(&index).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("com.clawplus.downloader"));
    assert!(json.contains("wasm32-wasip1"));

    let decoded: RegistryIndex = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.version, 2);
    assert_eq!(decoded.name, "OpenClaw Test Registry");
    assert_eq!(decoded.plugins.len(), 3);
    assert_eq!(decoded.stats.total_plugins, 3);
}

#[test]
fn plugin_entry_all_fields_roundtrip() {
    let entry = make_plugin_entry("com.test.plugin", "1.2.3");
    let json = serde_json::to_string(&entry).unwrap();
    let decoded: PluginEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.id,      "com.test.plugin");
    assert_eq!(decoded.version, "1.2.3");
    assert_eq!(decoded.license, "MIT");
    assert_eq!(decoded.downloads, 42);
    assert_eq!(decoded.tags, vec!["test".to_string(), "integration".to_string()]);
}

#[test]
fn plugin_permissions_serde_roundtrip() {
    let p = PluginPermissions {
        fs_read:       vec!["/tmp/**".to_string()],
        fs_write:      vec!["/workspace/**".to_string()],
        network:       vec!["api.openai.com".to_string()],
        spawn_process: true,
        clipboard:     false,
        env_vars:      true,
    };
    let json = serde_json::to_string(&p).unwrap();
    let d: PluginPermissions = serde_json::from_str(&json).unwrap();
    assert_eq!(d.fs_read,   vec!["/tmp/**"]);
    assert_eq!(d.fs_write,  vec!["/workspace/**"]);
    assert_eq!(d.network,   vec!["api.openai.com"]);
    assert!(d.spawn_process);
    assert!(!d.clipboard);
    assert!(d.env_vars);
}

#[test]
fn registry_index_empty_plugins_list_valid() {
    let index = RegistryIndex {
        version: 1,
        name: "Empty Registry".to_string(),
        description: "No plugins yet.".to_string(),
        plugins: vec![],
        category_index_urls: std::collections::HashMap::new(),
        stats: RegistryStats::default(),
        updated_at: 0,
    };
    let json = serde_json::to_string(&index).unwrap();
    let d: RegistryIndex = serde_json::from_str(&json).unwrap();
    assert!(d.plugins.is_empty());
}

// ── RegistryClient::fetch_index via file:// ───────────────────────────────────

#[tokio::test]
async fn registry_client_fetch_index_from_local_file() {
    use openclaw_store::registry::RegistryClient;
    use openclaw_store::types::StorePrefs;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let index = make_registry_index();
    let json = serde_json::to_string(&index).unwrap();

    let index_path = dir.path().join("index.json");
    std::fs::write(&index_path, &json).unwrap();

    let url = format!("file://{}", index_path.display());
    let prefs = Arc::new(RwLock::new(StorePrefs::default()));
    let client = RegistryClient::new(prefs);

    let loaded = client.fetch_index(&url).await.unwrap();
    assert_eq!(loaded.version, 2);
    assert_eq!(loaded.name, "OpenClaw Test Registry");
    assert_eq!(loaded.plugins.len(), 3);
    assert_eq!(loaded.plugins[0].id, "com.clawplus.downloader");
}

#[tokio::test]
async fn registry_client_fetch_index_missing_file_returns_error() {
    use openclaw_store::registry::RegistryClient;
    use openclaw_store::types::StorePrefs;
    use std::sync::Arc;
    use parking_lot::RwLock;

    let prefs = Arc::new(RwLock::new(StorePrefs::default()));
    let client = RegistryClient::new(prefs);
    let result = client.fetch_index("file:///nonexistent/path/index.json").await;
    assert!(result.is_err(), "missing file must return error");
}

#[tokio::test]
async fn registry_client_fetch_index_malformed_json_returns_error() {
    use openclaw_store::registry::RegistryClient;
    use openclaw_store::types::StorePrefs;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let bad_path = dir.path().join("bad.json");
    std::fs::write(&bad_path, b"{ this is: not valid JSON }").unwrap();

    let url = format!("file://{}", bad_path.display());
    let prefs = Arc::new(RwLock::new(StorePrefs::default()));
    let client = RegistryClient::new(prefs);
    let result = client.fetch_index(&url).await;
    assert!(result.is_err(), "malformed JSON must return error");
}

#[tokio::test]
async fn registry_client_installed_ids_real_directory() {
    use openclaw_store::registry::RegistryClient;
    use openclaw_store::types::StorePrefs;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    // Create fake .wasm files
    let wasm_files = [
        "com.clawplus.downloader-1.0.0.wasm",
        "com.clawplus.converter-2.1.3.wasm",
        "README.md",         // non-wasm — must be excluded
        "config.toml",       // non-wasm — must be excluded
    ];
    for f in &wasm_files {
        std::fs::write(dir.path().join(f), b"fake wasm").unwrap();
    }

    let prefs = Arc::new(RwLock::new(StorePrefs {
        plugin_dir: dir.path().to_string_lossy().to_string(),
        ..StorePrefs::default()
    }));
    let client = RegistryClient::new(prefs);
    let ids: Vec<String> = client.installed_ids().await;

    assert_eq!(ids.len(), 2, "only .wasm files must be listed: {:?}", ids);
    assert!(ids.iter().any(|id| id.contains("downloader")));
    assert!(ids.iter().any(|id| id.contains("converter")));
}

#[tokio::test]
async fn registry_client_uninstall_nonexistent_is_idempotent() {
    use openclaw_store::registry::RegistryClient;
    use openclaw_store::types::StorePrefs;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let prefs = Arc::new(RwLock::new(StorePrefs {
        plugin_dir: dir.path().to_string_lossy().to_string(),
        ..StorePrefs::default()
    }));
    let client = RegistryClient::new(prefs);
    // Uninstalling a plugin that doesn't exist must return Ok (idempotent)
    let entry = make_plugin_entry("com.nonexistent.plugin", "1.0.0");
    let result = client.uninstall_plugin(&entry).await;
    assert!(result.is_ok(), "uninstall of nonexistent plugin must be idempotent");
}

#[tokio::test]
async fn registry_client_uninstall_real_wasm_file() {
    use openclaw_store::registry::RegistryClient;
    use openclaw_store::types::StorePrefs;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    // Create a real .wasm file to delete
    let wasm_name = "com.clawplus.test-1.0.0.wasm";
    std::fs::write(dir.path().join(wasm_name), b"\x00asm\x01\x00\x00\x00").unwrap();

    let prefs = Arc::new(RwLock::new(StorePrefs {
        plugin_dir: dir.path().to_string_lossy().to_string(),
        ..StorePrefs::default()
    }));
    let client = RegistryClient::new(prefs);
    let entry = make_plugin_entry("com.clawplus.test", "1.0.0");
    client.uninstall_plugin(&entry).await.unwrap();

    // The file must no longer exist
    assert!(!dir.path().join(wasm_name).exists(),
        "wasm file must be deleted after uninstall");
}

// ── i18n: all 16 locales full coverage ───────────────────────────────────────

#[test]
fn i18n_all_locales_non_empty_nav_strings() {
    for locale in Locale::all() {
        let s = i18n::strings_for(*locale);
        assert!(!s.nav_dashboard.is_empty(), "{:?} nav_dashboard empty", locale);
        assert!(!s.nav_store.is_empty(),     "{:?} nav_store empty",     locale);
        assert!(!s.nav_chat.is_empty(),      "{:?} nav_chat empty",      locale);
        assert!(!s.nav_ai_models.is_empty(), "{:?} nav_ai_models empty", locale);
        assert!(!s.nav_settings.is_empty(),  "{:?} nav_settings empty",  locale);
    }
}

#[test]
fn i18n_all_locales_non_empty_title_strings() {
    for locale in Locale::all() {
        let s = i18n::strings_for(*locale);
        assert!(!s.title_dashboard.is_empty(), "{:?} title_dashboard empty", locale);
        assert!(!s.title_store.is_empty(),     "{:?} title_store empty",     locale);
        assert!(!s.title_chat.is_empty(),      "{:?} title_chat empty",      locale);
    }
}

#[test]
fn i18n_all_locales_store_strings_present() {
    for locale in Locale::all() {
        let s = i18n::strings_for(*locale);
        assert!(!s.store_loading.is_empty(),     "{:?} store_loading empty", locale);
        assert!(!s.store_search_hint.is_empty(), "{:?} store_search_hint empty", locale);
        assert!(!s.store_count.is_empty(),       "{:?} store_count empty", locale);
        assert!(s.store_count.contains("{0}"),   "{:?} store_count must have {{0}} template", locale);
    }
}

#[test]
fn i18n_locales_are_distinct_not_all_english() {
    let en = i18n::strings_for(Locale::En);
    let zh = i18n::strings_for(Locale::ZhCn);
    let ja = i18n::strings_for(Locale::Ja);
    let ar = i18n::strings_for(Locale::Ar);

    assert_ne!(en.nav_dashboard, zh.nav_dashboard, "ZhCn must differ from En");
    assert_ne!(en.nav_dashboard, ja.nav_dashboard, "Ja must differ from En");
    assert_ne!(en.nav_dashboard, ar.nav_dashboard, "Ar must differ from En");
}

#[test]
fn i18n_set_locale_and_current_locale_roundtrip() {
    // Save original
    let original = i18n::current_locale();

    i18n::set_locale(Locale::ZhCn);
    assert_eq!(i18n::current_locale(), Locale::ZhCn);

    i18n::set_locale(Locale::Ja);
    assert_eq!(i18n::current_locale(), Locale::Ja);

    i18n::set_locale(Locale::En);
    assert_eq!(i18n::current_locale(), Locale::En);

    // Restore original
    i18n::set_locale(original);
}

#[test]
fn i18n_locale_display_names_non_empty() {
    for locale in Locale::all() {
        let name = locale.display_name();
        assert!(!name.is_empty(), "{:?} display_name empty", locale);
    }
}

#[test]
fn i18n_locale_bcp47_tags_non_empty() {
    for locale in Locale::all() {
        let tag = locale.bcp47();
        assert!(!tag.is_empty(), "{:?} bcp47 empty", locale);
    }
}

#[test]
fn i18n_all_count_returns_16_locales() {
    assert_eq!(Locale::all().len(), 16, "must have exactly 16 locales");
}

// ── types: serde round-trips ──────────────────────────────────────────────────

#[test]
fn ai_backend_serde_all_variants() {
    for (variant, expected_str) in &[
        (AiBackend::Ollama,   "\"ollama\""),
        (AiBackend::LmStudio, "\"lm_studio\""),
        (AiBackend::LlamaCpp, "\"llama_cpp\""),
        (AiBackend::Custom,   "\"custom\""),
    ] {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(&json, expected_str, "AiBackend serde: {:?}", variant);
        let decoded: AiBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(&decoded, variant);
    }
}

#[test]
fn ai_model_config_default_serde_roundtrip() {
    let cfg = AiModelConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    let decoded: AiModelConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.backend,   AiBackend::Ollama);
    assert_eq!(decoded.endpoint,  "http://localhost:11434");
    assert_eq!(decoded.model,     "llama3");
    assert_eq!(decoded.max_tokens, 2048);
    assert!(decoded.is_active);
    // status is #[serde(skip)] so resets to default
    assert_eq!(decoded.status, AiModelStatus::Unknown);
}

#[test]
fn bot_platform_serde_all_variants() {
    for (variant, expected_str) in &[
        (BotPlatform::Telegram, "\"telegram\""),
        (BotPlatform::Discord,  "\"discord\""),
        (BotPlatform::WeChat,   "\"we_chat\""),
        (BotPlatform::Slack,    "\"slack\""),
        (BotPlatform::Custom,   "\"custom\""),
    ] {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(&json, expected_str, "BotPlatform serde: {:?}", variant);
        let decoded: BotPlatform = serde_json::from_str(&json).unwrap();
        assert_eq!(&decoded, variant);
    }
}

#[test]
fn bot_config_default_serde_roundtrip() {
    let cfg = BotConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    let decoded: BotConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.platform, BotPlatform::Telegram);
    assert!(!decoded.enabled);
    // status is #[serde(skip)] → resets to Disconnected
    assert_eq!(decoded.status, BotStatus::Disconnected);
}

#[test]
fn plugin_category_serde_roundtrip_all_variants() {
    let categories = [
        PluginCategory::Productivity,
        PluginCategory::Developer,
        PluginCategory::FileSystem,
        PluginCategory::Web,
        PluginCategory::Communication,
        PluginCategory::DataAnalysis,
        PluginCategory::Media,
        PluginCategory::Security,
        PluginCategory::Automation,
        PluginCategory::AI,
        PluginCategory::Finance,
        PluginCategory::Education,
        PluginCategory::Gaming,
        PluginCategory::Other,
    ];
    for cat in &categories {
        let json = serde_json::to_string(cat).unwrap();
        let decoded: PluginCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(&decoded, cat, "PluginCategory round-trip: {:?}", cat);
    }
}

#[test]
fn plugin_language_serde_roundtrip_all_variants() {
    let langs = [
        PluginLanguage::Rust,
        PluginLanguage::TypeScript,
        PluginLanguage::Python,
        PluginLanguage::Go,
        PluginLanguage::C,
        PluginLanguage::DotNet,
        PluginLanguage::Precompiled,
    ];
    for lang in &langs {
        let json = serde_json::to_string(lang).unwrap();
        let decoded: PluginLanguage = serde_json::from_str(&json).unwrap();
        assert_eq!(&decoded, lang, "PluginLanguage round-trip: {:?}", lang);
    }
}

#[test]
fn local_api_config_default_serde_roundtrip() {
    let cfg = LocalApiConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    let decoded: LocalApiConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.bind_host, "127.0.0.1");
    assert_eq!(decoded.port, 8765);
    assert_eq!(decoded.transport, ApiTransport::Http);
    assert!(!decoded.enabled);
    assert!(!decoded.tls_enabled);
}

#[test]
fn api_transport_display_strings() {
    assert_eq!(ApiTransport::Http.to_string(),      "HTTP REST");
    assert_eq!(ApiTransport::WebSocket.to_string(), "WebSocket");
    assert_eq!(ApiTransport::Grpc.to_string(),      "gRPC");
    assert_eq!(ApiTransport::Mqtt.to_string(),      "MQTT");
}

#[test]
fn chat_message_serde_roundtrip() {
    let msg = ChatMessage {
        role: ChatRole::User,
        content: "Hello, OpenClaw!".to_string(),
        id: 42,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ChatMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.role, ChatRole::User);
    assert_eq!(decoded.content, "Hello, OpenClaw!");
    assert_eq!(decoded.id, 42);
}

#[test]
fn plugin_review_serde_roundtrip() {
    let review = PluginReview {
        author: "Alice".to_string(),
        rating: 5,
        body: "Excellent plugin, works flawlessly!".to_string(),
        date: "2024-03-01".to_string(),
    };
    let json = serde_json::to_string(&review).unwrap();
    let d: PluginReview = serde_json::from_str(&json).unwrap();
    assert_eq!(d.author, "Alice");
    assert_eq!(d.rating, 5);
    assert_eq!(d.date, "2024-03-01");
}

#[test]
fn registry_stats_serde_roundtrip() {
    let stats = RegistryStats {
        total_plugins:       1234,
        total_downloads: 9_876_543,
        total_authors:         89,
        supported_languages:    7,
    };
    let json = serde_json::to_string(&stats).unwrap();
    let d: RegistryStats = serde_json::from_str(&json).unwrap();
    assert_eq!(d.total_plugins, 1234);
    assert_eq!(d.total_downloads, 9_876_543);
    assert_eq!(d.total_authors, 89);
    assert_eq!(d.supported_languages, 7);
}
