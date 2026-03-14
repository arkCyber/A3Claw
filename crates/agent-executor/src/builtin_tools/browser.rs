//! `web.screenshot`, `web.navigate`, `web.click`, `web.fill` — browser automation stubs.
//!
//! These skills model browser interaction. A full implementation requires a
//! headless browser backend (e.g. Playwright via Node IPC, or a Chromium DevTools
//! Protocol client). Without such a backend these functions degrade gracefully:
//!
//! - `web.screenshot` — fetches the URL with `web_fetch` and returns a textual
//!   representation (base64 screenshot requires a real browser backend).
//! - `web.navigate` — validates the URL and records it in a session-level
//!   navigation log; real page load requires a browser backend.
//! - `web.click` / `web.fill` — validate arguments and stub-respond; real DOM
//!   interaction requires a browser backend.
//!
//! To enable real browser automation, register a `BrowserSkillHandler` that
//! implements `SkillHandler` and claims `["web.screenshot", "web.navigate",
//! "web.click", "web.fill"]`.
//!
//! ## Security
//! All browser skills are classified `Safe` except `web.navigate` and `web.fill`
//! which are `Confirm` (they mutate page state / submit data).

use std::collections::HashMap;

// ── Argument types ─────────────────────────────────────────────────────────────

pub struct ScreenshotArgs<'a> {
    pub url: &'a str,
    pub width: u32,
    pub height: u32,
}

impl<'a> ScreenshotArgs<'a> {
    pub fn from_json(args: &'a serde_json::Value) -> Result<Self, String> {
        let url = args["url"].as_str().ok_or("missing 'url' argument")?;
        let width  = args["width"].as_u64().unwrap_or(1280) as u32;
        let height = args["height"].as_u64().unwrap_or(800) as u32;
        Ok(Self { url, width, height })
    }
}

pub struct NavigateArgs<'a> {
    pub url: &'a str,
}

impl<'a> NavigateArgs<'a> {
    pub fn from_json(args: &'a serde_json::Value) -> Result<Self, String> {
        let url = args["url"].as_str().ok_or("missing 'url' argument")?;
        Ok(Self { url })
    }
}

pub struct ClickArgs<'a> {
    pub selector: &'a str,
}

impl<'a> ClickArgs<'a> {
    pub fn from_json(args: &'a serde_json::Value) -> Result<Self, String> {
        let selector = args["selector"].as_str().ok_or("missing 'selector' argument")?;
        Ok(Self { selector })
    }
}

pub struct FillArgs<'a> {
    pub selector: &'a str,
    pub value: &'a str,
}

impl<'a> FillArgs<'a> {
    pub fn from_json(args: &'a serde_json::Value) -> Result<Self, String> {
        let selector = args["selector"].as_str().ok_or("missing 'selector' argument")?;
        let value    = args["value"].as_str().ok_or("missing 'value' argument")?;
        Ok(Self { selector, value })
    }
}

// ── Browser session state (in-process, per-executor) ─────────────────────────

/// Lightweight in-process browser session tracker.
///
/// Stores the current URL and a navigation history for the current agent
/// session. This is used by the stub implementations; a real browser backend
/// would maintain its own state.
#[derive(Debug, Default, Clone)]
pub struct BrowserSession {
    /// Current page URL (last navigated-to).
    pub current_url: Option<String>,
    /// Navigation history (most-recent-last).
    pub history: Vec<String>,
    /// Pending form fills recorded for audit: selector → value.
    pub pending_fills: HashMap<String, String>,
}

impl BrowserSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn navigate(&mut self, url: &str) {
        if let Some(prev) = self.current_url.take() {
            self.history.push(prev);
        }
        self.current_url = Some(url.to_string());
    }

    pub fn record_fill(&mut self, selector: &str, value: &str) {
        self.pending_fills.insert(selector.to_string(), value.to_string());
    }

    pub fn current(&self) -> &str {
        self.current_url.as_deref().unwrap_or("(no page loaded)")
    }
}

// ── Skill implementations ─────────────────────────────────────────────────────

/// `web.screenshot` — fetch the page and return a textual representation.
///
/// A real screenshot (PNG → base64) requires a headless browser backend.
/// This implementation returns the page's text content via the `web_fetch`
/// extractor as a best-effort substitute and instructs callers how to upgrade.
pub async fn screenshot(
    client: &reqwest::Client,
    args: &ScreenshotArgs<'_>,
) -> Result<String, String> {
    // Validate URL
    if !args.url.starts_with("http://") && !args.url.starts_with("https://") {
        return Err(format!("invalid URL for web.screenshot: '{}'", args.url));
    }

    // Try to fetch page text as a fallback
    let resp = client
        .get(args.url)
        .header("User-Agent", "Mozilla/5.0 (compatible; OpenClaw/1.0)")
        .send()
        .await
        .map_err(|e| format!("web.screenshot fetch error: {}", e))?;

    let status = resp.status().as_u16();
    let body   = resp.text().await.unwrap_or_default();

    // Strip HTML tags for a readable text fallback
    let text = strip_html_tags(&body);
    let snippet: String = text.chars().take(2000).collect();

    Ok(format!(
        "(web.screenshot: no headless browser backend configured — \
         returning page text for {}x{} viewport)\n\
         URL: {} (HTTP {})\n\
         Page text preview:\n{}",
        args.width, args.height, args.url, status, snippet
    ))
}

/// `web.navigate` — navigate the browser session to a URL.
pub fn navigate(session: &mut BrowserSession, args: &NavigateArgs<'_>) -> Result<String, String> {
    if !args.url.starts_with("http://") && !args.url.starts_with("https://") {
        return Err(format!("invalid URL for web.navigate: '{}'", args.url));
    }
    session.navigate(args.url);
    Ok(format!(
        "Navigated to: {}\n\
         (web.navigate: no headless browser backend — navigation recorded in session state. \
         Register a BrowserSkillHandler for real page interaction.)",
        args.url
    ))
}

/// `web.click` — click an element by CSS selector.
pub fn click(session: &BrowserSession, args: &ClickArgs<'_>) -> Result<String, String> {
    if args.selector.is_empty() {
        return Err("web.click: selector must not be empty".into());
    }
    let page = session.current();
    Ok(format!(
        "(web.click: no headless browser backend — would click '{}' on {}. \
         Register a BrowserSkillHandler for real DOM interaction.)",
        args.selector, page
    ))
}

/// `web.fill` — fill a form input by CSS selector.
pub fn fill(session: &mut BrowserSession, args: &FillArgs<'_>) -> Result<String, String> {
    if args.selector.is_empty() {
        return Err("web.fill: selector must not be empty".into());
    }
    session.record_fill(args.selector, args.value);
    let page = session.current();
    Ok(format!(
        "(web.fill: no headless browser backend — recorded fill '{}' = '{}' on {}. \
         Register a BrowserSkillHandler for real form interaction.)",
        args.selector, args.value, page
    ))
}

// ── HTML tag stripper ─────────────────────────────────────────────────────────

fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => { in_tag = true; }
            '>' => { in_tag = false; out.push(' '); }
            _ if !in_tag => {
                if ch == '\n' || ch == '\r' || ch == '\t' {
                    if !out.ends_with(' ') { out.push(' '); }
                } else {
                    out.push(ch);
                }
            }
            _ => {}
        }
    }
    // Collapse multiple spaces
    let mut result = String::with_capacity(out.len());
    let mut prev_space = false;
    for ch in out.chars() {
        if ch == ' ' {
            if !prev_space { result.push(ch); }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

// ── BrowserSkillHandler trait ─────────────────────────────────────────────────
//
// Implement this trait and register it via `SkillDispatcher::register_handler()`
// to replace the stub implementations above with real browser automation.
//
// Example backends:
//   - Chrome DevTools Protocol (CDP) over WebSocket
//   - Playwright Node.js IPC bridge
//   - Puppeteer HTTP control server

use async_trait::async_trait;
use crate::dispatch::SkillHandler;

/// Trait for a real browser automation backend.
///
/// Implement this to connect a headless browser (CDP, Playwright, etc.)
/// to the `web.*` skill namespace.
#[async_trait]
pub trait BrowserBackend: Send + Sync {
    /// Navigate to a URL and wait for page load.
    async fn navigate(&self, url: &str) -> Result<String, String>;
    /// Take a screenshot of the current page, return base64 PNG.
    async fn screenshot(&self, width: u32, height: u32) -> Result<String, String>;
    /// Click an element by CSS selector.
    async fn click(&self, selector: &str) -> Result<(), String>;
    /// Fill a form field by CSS selector.
    async fn fill(&self, selector: &str, value: &str) -> Result<(), String>;
    /// Evaluate JavaScript and return result as string.
    async fn eval(&self, script: &str) -> Result<String, String>;
    /// Get the current page URL.
    async fn current_url(&self) -> Result<String, String>;
    /// Get page title.
    async fn title(&self) -> Result<String, String>;
}

/// `SkillHandler` that routes `web.*` skills to a `BrowserBackend`.
///
/// Register with `SkillDispatcher::register_handler(Arc::new(BrowserSkillHandler::new(backend)))`.
pub struct BrowserSkillHandler<B: BrowserBackend> {
    backend: B,
}

impl<B: BrowserBackend> BrowserSkillHandler<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl<B: BrowserBackend + 'static> SkillHandler for BrowserSkillHandler<B> {
    fn skill_names(&self) -> &[&'static str] {
        &["web.screenshot", "web.navigate", "web.click", "web.fill", "web.eval"]
    }

    async fn execute(&self, skill_name: &str, args: &serde_json::Value) -> Result<String, String> {
        match skill_name {
            "web.navigate" => {
                let url = args["url"].as_str()
                    .ok_or("web.navigate: missing 'url'")?;
                self.backend.navigate(url).await
            }
            "web.screenshot" => {
                let width  = args["width"].as_u64().unwrap_or(1280) as u32;
                let height = args["height"].as_u64().unwrap_or(800) as u32;
                self.backend.screenshot(width, height).await
            }
            "web.click" => {
                let sel = args["selector"].as_str()
                    .ok_or("web.click: missing 'selector'")?;
                self.backend.click(sel).await.map(|_| format!("Clicked: {}", sel))
            }
            "web.fill" => {
                let sel = args["selector"].as_str()
                    .ok_or("web.fill: missing 'selector'")?;
                let val = args["value"].as_str()
                    .ok_or("web.fill: missing 'value'")?;
                self.backend.fill(sel, val).await
                    .map(|_| format!("Filled '{}' = '{}'", sel, val))
            }
            "web.eval" => {
                let script = args["script"].as_str()
                    .or_else(|| args["js"].as_str())
                    .ok_or("web.eval: missing 'script'")?;
                self.backend.eval(script).await
            }
            other => Err(format!("BrowserSkillHandler: unknown skill '{}'", other)),
        }
    }
}

// ── CdpBrowserClient — Chrome DevTools Protocol client stub ──────────────────
//
// This is the framework for a real CDP client. It connects to a running
// Chrome/Chromium instance via WebSocket on the standard CDP port (9222).
//
// To activate:
//   1. Start Chrome: `chromium --remote-debugging-port=9222 --headless`
//   2. Build `CdpBrowserClient::connect("ws://localhost:9222")`.
//   3. Register a `BrowserSkillHandler::new(client)`.
//
// The actual WebSocket message loop is omitted here (requires `tokio-tungstenite`
// or `chromiumoxide`). This stub shows the connection interface.

/// CDP client configuration.
#[derive(Debug, Clone)]
pub struct CdpConfig {
    /// WebSocket URL of the Chrome DevTools remote debugging endpoint.
    /// Typically `ws://localhost:9222/json/version`.
    pub ws_url: String,
    /// Page load timeout in milliseconds.
    pub timeout_ms: u64,
    /// User-Agent override (empty = use browser default).
    pub user_agent: Option<String>,
}

impl Default for CdpConfig {
    fn default() -> Self {
        Self {
            ws_url: "ws://localhost:9222".into(),
            timeout_ms: 30_000,
            user_agent: None,
        }
    }
}

/// Stub CDP browser client.
///
/// All methods return descriptive stub messages. Replace with a real
/// `chromiumoxide` or `tokio-tungstenite` based implementation.
pub struct CdpBrowserClient {
    pub config: CdpConfig,
    /// In-process session for stub fallback.
    session: tokio::sync::Mutex<BrowserSession>,
}

impl CdpBrowserClient {
    pub fn new(config: CdpConfig) -> Self {
        Self {
            config,
            session: tokio::sync::Mutex::new(BrowserSession::new()),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(CdpConfig::default())
    }
}

#[async_trait]
impl BrowserBackend for CdpBrowserClient {
    async fn navigate(&self, url: &str) -> Result<String, String> {
        let mut session = self.session.lock().await;
        session.navigate(url);
        Ok(format!(
            "(CDP stub: would navigate to {} via WebSocket {}. \
             Connect a real Chrome instance to enable.)",
            url, self.config.ws_url
        ))
    }

    async fn screenshot(&self, width: u32, height: u32) -> Result<String, String> {
        let session = self.session.lock().await;
        let url = session.current().to_string();
        Ok(format!(
            "(CDP stub: would capture {}x{} PNG from {}. \
             Connect a real Chrome instance to enable.)",
            width, height, url
        ))
    }

    async fn click(&self, selector: &str) -> Result<(), String> {
        let session = self.session.lock().await;
        let url = session.current().to_string();
        tracing::debug!("CDP stub click '{}' on {}", selector, url);
        Ok(())
    }

    async fn fill(&self, selector: &str, value: &str) -> Result<(), String> {
        let mut session = self.session.lock().await;
        session.record_fill(selector, value);
        Ok(())
    }

    async fn eval(&self, script: &str) -> Result<String, String> {
        let preview: String = script.chars().take(80).collect();
        Ok(format!(
            "(CDP stub: would evaluate JS `{}` in browser. \
             Connect a real Chrome instance to enable.)",
            preview
        ))
    }

    async fn current_url(&self) -> Result<String, String> {
        Ok(self.session.lock().await.current().to_string())
    }

    async fn title(&self) -> Result<String, String> {
        Ok("(CDP stub: page title unavailable without browser connection)".into())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ScreenshotArgs ────────────────────────────────────────────────────

    #[test]
    fn screenshot_args_defaults() {
        let v = serde_json::json!({"url": "https://example.com"});
        let a = ScreenshotArgs::from_json(&v).unwrap();
        assert_eq!(a.url, "https://example.com");
        assert_eq!(a.width, 1280);
        assert_eq!(a.height, 800);
    }

    #[test]
    fn screenshot_args_custom_viewport() {
        let v = serde_json::json!({"url": "https://x.com", "width": 1920, "height": 1080});
        let a = ScreenshotArgs::from_json(&v).unwrap();
        assert_eq!(a.width, 1920);
        assert_eq!(a.height, 1080);
    }

    #[test]
    fn screenshot_args_missing_url() {
        let v = serde_json::json!({});
        assert!(ScreenshotArgs::from_json(&v).is_err());
    }

    // ── NavigateArgs ──────────────────────────────────────────────────────

    #[test]
    fn navigate_args_ok() {
        let v = serde_json::json!({"url": "https://google.com"});
        let a = NavigateArgs::from_json(&v).unwrap();
        assert_eq!(a.url, "https://google.com");
    }

    #[test]
    fn navigate_args_missing_url() {
        let v = serde_json::json!({});
        assert!(NavigateArgs::from_json(&v).is_err());
    }

    // ── ClickArgs ─────────────────────────────────────────────────────────

    #[test]
    fn click_args_ok() {
        let v = serde_json::json!({"selector": "#submit-btn"});
        let a = ClickArgs::from_json(&v).unwrap();
        assert_eq!(a.selector, "#submit-btn");
    }

    #[test]
    fn click_args_missing_selector() {
        let v = serde_json::json!({});
        assert!(ClickArgs::from_json(&v).is_err());
    }

    // ── FillArgs ──────────────────────────────────────────────────────────

    #[test]
    fn fill_args_ok() {
        let v = serde_json::json!({"selector": "input[name=email]", "value": "user@test.com"});
        let a = FillArgs::from_json(&v).unwrap();
        assert_eq!(a.selector, "input[name=email]");
        assert_eq!(a.value, "user@test.com");
    }

    #[test]
    fn fill_args_missing_value() {
        let v = serde_json::json!({"selector": "#x"});
        assert!(FillArgs::from_json(&v).is_err());
    }

    // ── BrowserSession ────────────────────────────────────────────────────

    #[test]
    fn session_navigate_updates_current_url() {
        let mut s = BrowserSession::new();
        assert_eq!(s.current(), "(no page loaded)");
        s.navigate("https://a.com");
        assert_eq!(s.current(), "https://a.com");
    }

    #[test]
    fn session_navigate_pushes_history() {
        let mut s = BrowserSession::new();
        s.navigate("https://a.com");
        s.navigate("https://b.com");
        assert_eq!(s.history, vec!["https://a.com"]);
        assert_eq!(s.current(), "https://b.com");
    }

    #[test]
    fn session_record_fill_stores_entry() {
        let mut s = BrowserSession::new();
        s.record_fill("#email", "test@example.com");
        assert_eq!(s.pending_fills.get("#email").map(|s| s.as_str()), Some("test@example.com"));
    }

    // ── navigate() ────────────────────────────────────────────────────────

    #[test]
    fn navigate_ok_records_url() {
        let mut s = BrowserSession::new();
        let a = NavigateArgs { url: "https://example.com" };
        let r = navigate(&mut s, &a).unwrap();
        assert!(r.contains("https://example.com"));
        assert_eq!(s.current(), "https://example.com");
    }

    #[test]
    fn navigate_invalid_url_returns_err() {
        let mut s = BrowserSession::new();
        let a = NavigateArgs { url: "ftp://bad.example" };
        assert!(navigate(&mut s, &a).is_err());
    }

    // ── click() ──────────────────────────────────────────────────────────

    #[test]
    fn click_empty_selector_returns_err() {
        let s = BrowserSession::new();
        let a = ClickArgs { selector: "" };
        assert!(click(&s, &a).is_err());
    }

    #[test]
    fn click_valid_selector_returns_stub_message() {
        let mut s = BrowserSession::new();
        s.navigate("https://example.com");
        let a = ClickArgs { selector: "button.submit" };
        let r = click(&s, &a).unwrap();
        assert!(r.contains("button.submit"));
        assert!(r.contains("https://example.com"));
    }

    // ── fill() ───────────────────────────────────────────────────────────

    #[test]
    fn fill_empty_selector_returns_err() {
        let mut s = BrowserSession::new();
        let a = FillArgs { selector: "", value: "x" };
        assert!(fill(&mut s, &a).is_err());
    }

    #[test]
    fn fill_records_and_returns_stub_message() {
        let mut s = BrowserSession::new();
        s.navigate("https://form.example.com");
        let a = FillArgs { selector: "#name", value: "Alice" };
        let r = fill(&mut s, &a).unwrap();
        assert!(r.contains("#name"));
        assert!(r.contains("Alice"));
        assert_eq!(s.pending_fills.get("#name").map(|s| s.as_str()), Some("Alice"));
    }

    // ── strip_html_tags ────────────────────────────────────────────────────

    #[test]
    fn strip_html_removes_tags() {
        let html = "<html><body><p>Hello <b>World</b></p></body></html>";
        let text = strip_html_tags(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(!text.contains('<'));
        assert!(!text.contains('>'));
    }

    #[test]
    fn strip_html_collapses_whitespace() {
        let html = "<p>  Lots   of   spaces  </p>";
        let text = strip_html_tags(html);
        assert!(!text.contains("   "));
    }

    #[test]
    fn strip_html_empty_input() {
        assert_eq!(strip_html_tags(""), "");
    }

    // ── CdpBrowserClient stubs ─────────────────────────────────────────────

    #[tokio::test]
    async fn cdp_navigate_returns_stub() {
        let client = CdpBrowserClient::with_defaults();
        let result = client.navigate("https://example.com").await.unwrap();
        assert!(result.contains("https://example.com"));
        assert!(result.contains("CDP stub"));
    }

    #[tokio::test]
    async fn cdp_screenshot_returns_stub() {
        let client = CdpBrowserClient::with_defaults();
        let result = client.screenshot(1280, 800).await.unwrap();
        assert!(result.contains("1280"));
        assert!(result.contains("800"));
        assert!(result.contains("CDP stub"));
    }

    #[tokio::test]
    async fn cdp_click_succeeds() {
        let client = CdpBrowserClient::with_defaults();
        assert!(client.click("#submit").await.is_ok());
    }

    #[tokio::test]
    async fn cdp_fill_records_in_session() {
        let client = CdpBrowserClient::with_defaults();
        client.fill("#email", "test@example.com").await.unwrap();
        let session = client.session.lock().await;
        assert_eq!(
            session.pending_fills.get("#email").map(|s| s.as_str()),
            Some("test@example.com")
        );
    }

    #[tokio::test]
    async fn cdp_eval_returns_stub() {
        let client = CdpBrowserClient::with_defaults();
        let result = client.eval("document.title").await.unwrap();
        assert!(result.contains("CDP stub"));
        assert!(result.contains("document.title"));
    }

    #[tokio::test]
    async fn cdp_current_url_before_navigate() {
        let client = CdpBrowserClient::with_defaults();
        let url = client.current_url().await.unwrap();
        assert_eq!(url, "(no page loaded)");
    }

    #[tokio::test]
    async fn cdp_current_url_after_navigate() {
        let client = CdpBrowserClient::with_defaults();
        client.navigate("https://rust-lang.org").await.unwrap();
        let url = client.current_url().await.unwrap();
        assert_eq!(url, "https://rust-lang.org");
    }

    #[tokio::test]
    async fn cdp_title_returns_stub() {
        let client = CdpBrowserClient::with_defaults();
        let title = client.title().await.unwrap();
        assert!(title.contains("CDP stub"));
    }

    // ── BrowserSkillHandler dispatch ──────────────────────────────────────

    #[tokio::test]
    async fn browser_handler_navigate() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let result = handler.execute(
            "web.navigate",
            &serde_json::json!({"url": "https://example.com"}),
        ).await.unwrap();
        assert!(result.contains("https://example.com"));
    }

    #[tokio::test]
    async fn browser_handler_screenshot() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let result = handler.execute(
            "web.screenshot",
            &serde_json::json!({"url": "https://example.com", "width": 1920, "height": 1080}),
        ).await.unwrap();
        assert!(result.contains("1920"));
    }

    #[tokio::test]
    async fn browser_handler_click() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let result = handler.execute(
            "web.click",
            &serde_json::json!({"selector": "#btn"}),
        ).await.unwrap();
        assert!(result.contains("#btn"));
    }

    #[tokio::test]
    async fn browser_handler_fill() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let result = handler.execute(
            "web.fill",
            &serde_json::json!({"selector": "#input", "value": "hello"}),
        ).await.unwrap();
        assert!(result.contains("#input"));
        assert!(result.contains("hello"));
    }

    #[tokio::test]
    async fn browser_handler_eval() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let result = handler.execute(
            "web.eval",
            &serde_json::json!({"script": "1+1"}),
        ).await.unwrap();
        assert!(result.contains("1+1") || result.contains("CDP stub"));
    }

    #[tokio::test]
    async fn browser_handler_missing_url_errors() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let err = handler.execute("web.navigate", &serde_json::json!({})).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn browser_handler_unknown_skill_errors() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let err = handler.execute("web.unknown", &serde_json::json!({})).await;
        assert!(err.is_err());
    }

    #[test]
    fn browser_handler_skill_names() {
        let handler = BrowserSkillHandler::new(CdpBrowserClient::with_defaults());
        let names = handler.skill_names();
        assert!(names.contains(&"web.navigate"));
        assert!(names.contains(&"web.screenshot"));
        assert!(names.contains(&"web.click"));
        assert!(names.contains(&"web.fill"));
        assert!(names.contains(&"web.eval"));
    }

    #[test]
    fn cdp_config_default_ws_url() {
        let cfg = CdpConfig::default();
        assert_eq!(cfg.ws_url, "ws://localhost:9222");
        assert_eq!(cfg.timeout_ms, 30_000);
        assert!(cfg.user_agent.is_none());
    }
}
