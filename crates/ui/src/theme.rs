#![allow(dead_code)]
use cosmic::iced::Color;

// ── Status colours ────────────────────────────────────────────────────────────
pub const COLOR_ALLOW: Color   = Color { r: 0.22, g: 0.82, b: 0.46, a: 1.0 };
pub const COLOR_DENY: Color    = Color { r: 0.92, g: 0.28, b: 0.28, a: 1.0 };
pub const COLOR_PENDING: Color = Color { r: 0.98, g: 0.72, b: 0.18, a: 1.0 };
pub const COLOR_INFO: Color    = Color { r: 0.42, g: 0.72, b: 0.98, a: 1.0 };
pub const COLOR_MUTED: Color   = Color { r: 0.52, g: 0.50, b: 0.48, a: 1.0 };

// ── Event-kind colours ────────────────────────────────────────────────────────
pub const COLOR_FILE_DELETE: Color = Color { r: 0.92, g: 0.32, b: 0.32, a: 1.0 };
pub const COLOR_SHELL_EXEC: Color  = Color { r: 0.96, g: 0.62, b: 0.12, a: 1.0 };
pub const COLOR_NETWORK: Color     = Color { r: 0.28, g: 0.65, b: 0.95, a: 1.0 };
pub const COLOR_SYSTEM: Color      = Color { r: 0.58, g: 0.52, b: 0.88, a: 1.0 };

// ── Warm dark sidebar accent (amber-orange) ───────────────────────────────────
pub const COLOR_ACCENT_WARM: Color = Color { r: 0.98, g: 0.62, b: 0.22, a: 1.0 };
pub const COLOR_BG_WARM: Color     = Color { r: 0.13, g: 0.11, b: 0.10, a: 1.0 };
pub const COLOR_SURFACE_WARM: Color = Color { r: 0.18, g: 0.15, b: 0.13, a: 1.0 };
pub const COLOR_SIDEBAR_BG: Color  = Color { r: 0.15, g: 0.12, b: 0.11, a: 1.0 };
pub const COLOR_SIDEBAR_ACTIVE: Color = Color { r: 0.98, g: 0.62, b: 0.22, a: 0.18 };
pub const COLOR_SIDEBAR_TEXT: Color   = Color { r: 0.92, g: 0.88, b: 0.82, a: 1.0 };
pub const COLOR_SIDEBAR_MUTED: Color  = Color { r: 0.58, g: 0.54, b: 0.50, a: 1.0 };

// ── Language / Locale support ─────────────────────────────────────────────────

/// All 16 UI locales supported by OpenClaw+.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Language {
    #[default]
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

/// Backwards-compat alias used throughout the UI crate.
pub type Locale = Language;

impl Language {
    /// Human-readable name shown in the language switcher.
    pub fn display_name(self) -> &'static str {
        match self {
            Language::En   => "English",
            Language::ZhCn => "简体中文",
            Language::ZhTw => "繁體中文",
            Language::Ja   => "日本語",
            Language::Ko   => "한국어",
            Language::Es   => "Español",
            Language::Fr   => "Français",
            Language::De   => "Deutsch",
            Language::Pt   => "Português",
            Language::Ru   => "Русский",
            Language::Ar   => "العربية",
            Language::Hi   => "हिन्दी",
            Language::It   => "Italiano",
            Language::Nl   => "Nederlands",
            Language::Tr   => "Türkçe",
            Language::Pl   => "Polski",
        }
    }

    /// Short label used in compact UI (sidebar button etc.).
    pub fn label(self) -> &'static str {
        match self {
            Language::En   => "EN",
            Language::ZhCn => "中文",
            Language::ZhTw => "繁中",
            Language::Ja   => "日本語",
            Language::Ko   => "한국어",
            Language::Es   => "ES",
            Language::Fr   => "FR",
            Language::De   => "DE",
            Language::Pt   => "PT",
            Language::Ru   => "RU",
            Language::Ar   => "AR",
            Language::Hi   => "HI",
            Language::It   => "IT",
            Language::Nl   => "NL",
            Language::Tr   => "TR",
            Language::Pl   => "PL",
        }
    }

    /// All locales in stable order.
    pub fn all() -> &'static [Language] {
        &[
            Language::En, Language::ZhCn, Language::ZhTw,
            Language::Ja, Language::Ko,
            Language::Es, Language::Fr, Language::De, Language::Pt,
            Language::Ru, Language::Ar, Language::Hi,
            Language::It, Language::Nl, Language::Tr, Language::Pl,
        ]
    }

    /// Whether this locale uses right-to-left text direction.
    pub fn is_rtl(self) -> bool {
        matches!(self, Language::Ar)
    }

    /// Whether this locale uses CJK characters (needs IME).
    pub fn needs_ime(self) -> bool {
        matches!(self, Language::ZhCn | Language::ZhTw | Language::Ja | Language::Ko)
    }
}

/// Translate a key into the current language.
/// Falls back to English when no translation is available.
pub fn t(lang: Language, en: &'static str, zh: &'static str) -> &'static str {
    tr(lang, en, zh, "", "", "", "", "", "", "", "", "", "", "", "", "", "")
}

/// Full 16-language translation helper.
/// Pass "" to fall back to English for that locale.
#[allow(clippy::too_many_arguments)]
pub fn tr(
    lang: Language,
    en: &'static str,
    zh: &'static str,   // ZhCn + ZhTw
    tw: &'static str,   // ZhTw override (empty = use zh)
    ja: &'static str,
    ko: &'static str,
    es: &'static str,
    fr: &'static str,
    de: &'static str,
    pt: &'static str,
    ru: &'static str,
    ar: &'static str,
    hi: &'static str,
    it: &'static str,
    nl: &'static str,
    tr_: &'static str,  // Turkish (tr conflicts with fn name)
    pl: &'static str,
) -> &'static str {
    let s = match lang {
        Language::En   => en,
        Language::ZhCn => zh,
        Language::ZhTw => if tw.is_empty() { zh } else { tw },
        Language::Ja   => ja,
        Language::Ko   => ko,
        Language::Es   => es,
        Language::Fr   => fr,
        Language::De   => de,
        Language::Pt   => pt,
        Language::Ru   => ru,
        Language::Ar   => ar,
        Language::Hi   => hi,
        Language::It   => it,
        Language::Nl   => nl,
        Language::Tr   => tr_,
        Language::Pl   => pl,
    };
    if s.is_empty() { en } else { s }
}

/// Translate common UI strings across all 16 languages.
/// This is the main entry point — maps well-known English keys to all locales.
/// Now uses the structured i18n module for keys defined in Strings.
pub fn tx(lang: Language, key: &'static str) -> &'static str {
    let s = crate::i18n::strings_for(lang);
    
    match key {
        // ── Navigation (migrated to i18n::Strings) ────────────────────────
        "Dashboard" => s.nav_dashboard,
        "Event Log" => s.nav_events,
        "Security Settings" => s.nav_settings,
        "AI Assistant" => s.nav_ai,
        "Plugin Store" => s.nav_plugins,
        // ── Menu (migrated to i18n::Strings) ──────────────────────────────
        "File" => s.menu_file,
        "Sandbox" => s.menu_sandbox,
        "View" => s.menu_view,
        "Plugins" => s.menu_plugins,
        "Help" => s.menu_help,
        "Clear Events" => s.menu_clear_events,
        "Emergency Stop" => s.menu_emergency,
        "Start Sandbox" => s.menu_start,
        "Stop Sandbox" => s.menu_stop,
        "Toggle Sidebar" => s.menu_toggle_sidebar,
        "Open Plugin Store" => s.menu_open_store,
        "About OpenClaw+" => s.menu_about,
        // ── Dashboard (migrated to i18n::Strings) ─────────────────────────
        "Overview" => s.dash_overview,
        "Total" => s.dash_total,
        "Allowed" => s.dash_allowed,
        "Denied" => s.dash_denied,
        "Pending" => s.dash_pending,
        "File Ops" => s.dash_file_ops,
        "Network" => s.dash_network,
        "Shell" => s.dash_shell,
        "Breaker" => s.dash_breaker,
        "Pending Confirmations" => s.dash_pending_conf,
        "Recent Events" => s.dash_recent,
        "No sandbox events yet." => s.dash_no_events,
        "Awaiting Confirmation" => s.dash_awaiting,
        "Allow" => s.dash_allow,
        "Deny" => s.dash_deny,
        "Clear Log" => s.dash_clear_log,
        "Tripped" => s.dash_tripped,
        // ── Settings ───────────────────────────────────────────────────────
        "Appearance & Language" => tr(lang,
            "Appearance & Language","外观与语言","外觀與語言","外観と言語","외관 및 언어",
            "Apariencia e idioma","Apparence et langue","Erscheinungsbild & Sprache","Aparência e idioma",
            "Внешний вид и язык","المظهر واللغة","रूप और भाषा","Aspetto e lingua","Uiterlijk en taal","Görünüm ve Dil","Wygląd i język"),
        "Language" => tr(lang,
            "Language","语言","語言","言語","언어",
            "Idioma","Langue","Sprache","Idioma",
            "Язык","اللغة","भाषा","Lingua","Taal","Dil","Język"),
        "Switch the UI language." => tr(lang,
            "Switch the UI language.","切换界面语言。","切換介面語言。","UI言語を切り替えます。","UI 언어를 전환합니다.",
            "Cambiar el idioma de la interfaz.","Changer la langue de l'interface.","UI-Sprache wechseln.","Mudar o idioma da interface.",
            "Сменить язык интерфейса.","تغيير لغة الواجهة.","UI भाषा बदलें।","Cambia la lingua dell'interfaccia.","UI-taal wijzigen.","Arayüz dilini değiştir.","Zmień język interfejsu."),
        "Choose display language and colour theme." => tr(lang,
            "Choose display language and colour theme.","选择显示语言与颜色主题。","選擇顯示語言與顏色主題。","表示言語とカラーテーマを選択します。","표시 언어 및 색상 테마를 선택합니다.",
            "Elige el idioma y el tema de color.","Choisissez la langue et le thème de couleur.","Sprache und Farbthema wählen.","Escolha o idioma e o tema de cores.",
            "Выберите язык и цветовую тему.","اختر لغة العرض وموضوع الألوان.","प्रदर्शन भाषा और रंग थीम चुनें।","Scegli lingua e tema colori.","Kies taal en kleurthema.","Dil ve renk teması seçin.","Wybierz język i motyw kolorystyczny."),
        "Switch to Default Dark" => tr(lang,
            "Switch to Default Dark","切换为默认暗色","切換為預設暗色","デフォルトダークに切替","기본 다크로 전환",
            "Cambiar a oscuro predeterminado","Passer au sombre par défaut","Zu Standard-Dunkel wechseln","Mudar para escuro padrão",
            "Переключить на стандартную тёмную","التبديل إلى الداكن الافتراضي","डिफ़ॉल्ट डार्क पर स्विच करें","Passa al tema scuro predefinito","Overschakelen naar standaard donker","Varsayılan Karanlığa Geç","Przełącz na domyślny ciemny"),
        "Switch to Warm Dark" => tr(lang,
            "Switch to Warm Dark","切换为暖色暗色","切換為暖色暗色","ウォームダークに切替","따뜻한 다크로 전환",
            "Cambiar a oscuro cálido","Passer au sombre chaud","Zu warmem Dunkel wechseln","Mudar para escuro quente",
            "Переключить на тёплую тёмную","التبديل إلى الداكن الدافئ","वार्म डार्क पर स्विच करें","Passa al tema scuro caldo","Overschakelen naar warm donker","Sıcak Karanlığa Geç","Przełącz na ciepły ciemny"),
        // ── AI Chat ────────────────────────────────────────────────────────
        "Type a message…" => tr(lang,
            "Type a message…","输入消息…","輸入訊息…","メッセージを入力…","메시지 입력…",
            "Escribe un mensaje…","Tapez un message…","Nachricht eingeben…","Digite uma mensagem…",
            "Введите сообщение…","اكتب رسالة…","संदेश लिखें…","Scrivi un messaggio…","Typ een bericht…","Mesaj yazın…","Wpisz wiadomość…"),
        "Send" => tr(lang,
            "Send","发送","傳送","送信","전송",
            "Enviar","Envoyer","Senden","Enviar",
            "Отправить","إرسال","भेजें","Invia","Verzenden","Gönder","Wyślij"),
        // ── Status ─────────────────────────────────────────────────────────
        "Stopped" => tr(lang,
            "Stopped","已停止","已停止","停止中","중지됨",
            "Detenido","Arrêté","Gestoppt","Parado",
            "Остановлен","متوقف","रुका हुआ","Fermato","Gestopt","Durduruldu","Zatrzymany"),
        "Running" => tr(lang,
            "Running","运行中","運行中","実行中","실행 중",
            "En ejecución","En cours","Läuft","Em execução",
            "Работает","يعمل","चल रहा है","In esecuzione","Actief","Çalışıyor","Działa"),
        _ => key,
    }
}

/// Build the warm dark COSMIC theme used by OpenClaw+.
///
/// Uses `ThemeBuilder::dark()` with an amber-orange accent and a slightly
/// warm neutral tint so the background reads as dark-brown rather than
/// pure grey.
pub fn warm_dark_theme() -> cosmic::Theme {
    use cosmic::cosmic_theme::ThemeBuilder;
    use cosmic::cosmic_theme::palette::Srgb;

    let accent = Srgb::new(0.98_f32, 0.62, 0.22);
    let neutral_tint = Srgb::new(0.18_f32, 0.14, 0.12);

    let cosmic_theme = ThemeBuilder::dark()
        .accent(accent)
        .neutral_tint(neutral_tint)
        .build();

    cosmic::Theme::custom(std::sync::Arc::new(cosmic_theme))
}
