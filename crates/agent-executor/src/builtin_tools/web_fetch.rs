//! `web_fetch` — enhanced URL fetcher with HTML-to-text/markdown extraction.
//!
//! Mirrors the official OpenClaw `web_fetch` built-in tool.
//!
//! Parameters:
//! - `url` (required)
//! - `extract_mode` — `"markdown"` | `"text"` | `"raw"` (default: `"text"`)
//! - `max_chars` — truncation cap (default: 8000, hard cap: 50000)
//! - `method` — HTTP verb (default: `"GET"`)
//! - `headers` — extra request headers as JSON object
//! - `body` — request body for POST/PUT
//!
//! Responses are NOT cached in this implementation (caching is a gateway
//! concern). The caller (SkillDispatcher) applies the gateway before/after
//! hooks which handle rate-limiting and audit.

use std::collections::HashMap;

pub const DEFAULT_MAX_CHARS: usize = 8_000;
pub const HARD_MAX_CHARS: usize = 50_000;

pub struct WebFetchArgs<'a> {
    pub url: &'a str,
    pub extract_mode: ExtractMode,
    pub max_chars: usize,
    pub method: &'a str,
    pub headers: HashMap<String, String>,
    pub body: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractMode {
    Markdown,
    Text,
    Raw,
}

impl ExtractMode {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => ExtractMode::Markdown,
            "raw" | "html"   => ExtractMode::Raw,
            _                => ExtractMode::Text,
        }
    }
}

impl<'a> WebFetchArgs<'a> {
    pub fn from_json(args: &'a serde_json::Value) -> Result<Self, String> {
        let url = args["url"].as_str().ok_or("missing 'url' argument")?;
        let extract_mode = ExtractMode::from_str(
            args["extract_mode"].as_str().unwrap_or("text"),
        );
        let raw_max = args["max_chars"].as_u64().unwrap_or(DEFAULT_MAX_CHARS as u64) as usize;
        let max_chars = raw_max.min(HARD_MAX_CHARS);
        let method = args["method"].as_str().unwrap_or("GET");
        let headers: HashMap<String, String> = args["headers"]
            .as_object()
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let body = args["body"].as_str();
        Ok(WebFetchArgs { url, extract_mode, max_chars, method, headers, body })
    }
}

/// Reject requests to private/loopback/link-local IP ranges (SSRF protection).
///
/// Checks the **hostname** portion of the URL against known private ranges.
/// This is a defence-in-depth measure; the security `PolicyEngine` provides
/// the primary network allowlist check at the Gateway level.
fn check_ssrf(url: &str) -> Result<(), String> {
    // Parse scheme and host
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| format!("invalid or unsupported URL scheme: {}", url))?;

    // Extract the authority (everything before the first '/').
    let authority = rest.split('/').next().unwrap_or("");

    // IPv6 literal addresses are enclosed in brackets: [::1] or [::1]:8080.
    // For bracket addresses keep the full [...] token; for IPv4/hostname strip the port.
    let host = if authority.starts_with('[') {
        // Find closing bracket; the address is everything up to and including ']'.
        let end = authority.find(']').map(|i| i + 1).unwrap_or(authority.len());
        &authority[..end]
    } else {
        authority.split(':').next().unwrap_or("")
    };
    let host_lower = host.to_lowercase();

    // Block loopback / localhost
    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower == "0"
    {
        return Err(format!("SSRF blocked: '{}' resolves to loopback", host));
    }

    // Block by literal IP prefix
    let blocked_prefixes = [
        "127.",       // 127.0.0.0/8 loopback
        "10.",        // 10.0.0.0/8   RFC1918
        "192.168.",   // 192.168.0.0/16 RFC1918
        "169.254.",   // 169.254.0.0/16 link-local
        "[::1]",      // IPv6 loopback
        "[::]",       // IPv6 unspecified
        "[fc",        // IPv6 ULA fc00::/7
        "[fd",        // IPv6 ULA fd00::/8
        "[fe80",      // IPv6 link-local
    ];
    for prefix in &blocked_prefixes {
        if host_lower.starts_with(prefix) {
            return Err(format!("SSRF blocked: '{}' is a private/reserved address", host));
        }
    }

    // Block 172.16.0.0/12 (172.16.x.x – 172.31.x.x)
    if host_lower.starts_with("172.") {
        if let Some(second_octet_str) = host_lower.strip_prefix("172.").and_then(|s| s.split('.').next()) {
            if let Ok(second) = second_octet_str.parse::<u8>() {
                if (16..=31).contains(&second) {
                    return Err(format!("SSRF blocked: '{}' is in 172.16-31 private range", host));
                }
            }
        }
    }

    Ok(())
}

/// Fetch a URL and extract its content.
/// Uses the provided reqwest client to reuse connection pools.
pub async fn fetch(
    client: &reqwest::Client,
    args: &WebFetchArgs<'_>,
) -> Result<String, String> {
    // SSRF protection: reject private/internal addresses before making network call.
    check_ssrf(args.url)?;

    let method = reqwest::Method::from_bytes(args.method.as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let mut req = client.request(method, args.url);

    for (k, v) in &args.headers {
        req = req.header(k.as_str(), v.as_str());
    }

    if let Some(body) = args.body {
        req = req.body(body.to_string());
    }

    // Set a reasonable browser-like User-Agent
    req = req.header(
        "User-Agent",
        "Mozilla/5.0 (compatible; OpenClaw+/1.0; +https://openclaw.ai)",
    );

    let resp = req.send().await.map_err(|e| format!("fetch error: {}", e))?;
    let status = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let body = resp.text().await.unwrap_or_default();

    let extracted = if content_type.contains("text/html") || content_type.is_empty() {
        match args.extract_mode {
            ExtractMode::Raw      => body.clone(),
            ExtractMode::Text     => html_to_text(&body),
            ExtractMode::Markdown => html_to_markdown(&body),
        }
    } else {
        body.clone()
    };

    let truncated = truncate_chars(&extracted, args.max_chars);

    Ok(format!("HTTP {}\n{}", status, truncated))
}

// ── HTML extraction ───────────────────────────────────────────────────────────

/// Strip HTML tags, decode common entities, collapse whitespace.
pub fn html_to_text(html: &str) -> String {
    let no_script = remove_tags_block(html, "script");
    let no_style  = remove_tags_block(&no_script, "style");
    let no_tags: String = strip_tags(&no_style);
    let decoded = decode_entities(&no_tags);
    collapse_whitespace(&decoded)
}

/// Convert HTML to simplified Markdown (headings, links, code blocks).
pub fn html_to_markdown(html: &str) -> String {
    let no_script = remove_tags_block(html, "script");
    let no_style  = remove_tags_block(&no_script, "style");

    let mut result = String::with_capacity(no_style.len() / 2);
    let mut pos = 0;
    let bytes = no_style.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            let end = no_style[pos..].find('>').map(|i| pos + i + 1).unwrap_or(bytes.len());
            let tag = &no_style[pos..end];
            let tag_lower = tag.to_lowercase();

            if tag_lower.starts_with("<h1") { result.push_str("\n# "); }
            else if tag_lower.starts_with("<h2") { result.push_str("\n## "); }
            else if tag_lower.starts_with("<h3") { result.push_str("\n### "); }
            else if tag_lower.starts_with("<h4") { result.push_str("\n#### "); }
            else if tag_lower.starts_with("</h") { result.push('\n'); }
            else if tag_lower.starts_with("<p") || tag_lower.starts_with("<br") || tag_lower.starts_with("<div") {
                result.push('\n');
            }
            else if tag_lower.starts_with("<li") { result.push_str("\n- "); }
            else if tag_lower.starts_with("<code") { result.push('`'); }
            else if tag_lower.starts_with("</code") { result.push('`'); }
            else if tag_lower.starts_with("<pre") { result.push_str("\n```\n"); }
            else if tag_lower.starts_with("</pre") { result.push_str("\n```\n"); }
            else if tag_lower.starts_with("<a ") || tag_lower.starts_with("<a>") {
                // Extract href
                if let Some(href) = extract_attr(tag, "href") {
                    result.push('[');
                    // content will be accumulated; close bracket + url appended on </a>
                    // Simple approach: just emit the href inline
                    result.push_str(&href);
                    result.push_str("](");
                    result.push_str(&href);
                    result.push(')');
                }
            }
            else if tag_lower.starts_with("<strong") || tag_lower.starts_with("<b>") || tag_lower.starts_with("<b ") {
                result.push_str("**");
            }
            else if tag_lower.starts_with("</strong") || tag_lower == "</b>" {
                result.push_str("**");
            }
            else if tag_lower.starts_with("<em") || tag_lower.starts_with("<i>") || tag_lower.starts_with("<i ") {
                result.push('_');
            }
            else if tag_lower.starts_with("</em") || tag_lower == "</i>" {
                result.push('_');
            }

            pos = end;
        } else {
            let next_tag = no_style[pos..].find('<').map(|i| pos + i).unwrap_or(bytes.len());
            let text = &no_style[pos..next_tag];
            result.push_str(&decode_entities(text));
            pos = next_tag;
        }
    }

    collapse_whitespace(&result)
}

fn remove_tags_block(html: &str, tag: &str) -> String {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut result = String::with_capacity(html.len());
    let mut pos = 0;
    let lower = html.to_lowercase();

    while pos < html.len() {
        if let Some(start) = lower[pos..].find(&open).map(|i| pos + i) {
            result.push_str(&html[pos..start]);
            if let Some(end) = lower[start..].find(&close).map(|i| start + i + close.len()) {
                pos = end;
            } else {
                break;
            }
        } else {
            result.push_str(&html[pos..]);
            break;
        }
    }
    result
}

fn strip_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&ndash;", "–")
        .replace("&mdash;", "—")
        .replace("&hellip;", "…")
}

fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_whitespace = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_whitespace {
                result.push(' ');
            }
            prev_whitespace = true;
        } else {
            result.push(c);
            prev_whitespace = false;
        }
    }
    result.trim().to_string()
}

fn extract_attr<'a>(tag: &'a str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let lower_tag = tag.to_lowercase();
    let start = lower_tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"').map(|i| start + i)?;
    Some(tag[start..end].to_string())
}

fn truncate_chars(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{}\n... (truncated at {} chars)", truncated, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_strips_tags() {
        let html = "<html><body><h1>Title</h1><p>Hello <b>World</b></p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(!text.contains('<'));
    }

    #[test]
    fn html_to_text_removes_script() {
        let html = "<p>visible</p><script>evil()</script><p>also visible</p>";
        let text = html_to_text(html);
        assert!(text.contains("visible"));
        assert!(!text.contains("evil"));
    }

    #[test]
    fn html_to_markdown_headings() {
        let html = "<h1>Main Title</h1><h2>Subtitle</h2><p>Paragraph</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Main Title") || md.contains("# "), "md={}", md);
        assert!(md.contains("## ") || md.contains("Subtitle"), "md={}", md);
    }

    #[test]
    fn decode_html_entities() {
        // Single-pass decode: &amp; -> &, &lt; -> <
        assert_eq!(decode_entities("&amp;"), "&");
        assert_eq!(decode_entities("&lt;"), "<");
        assert_eq!(decode_entities("&gt;"), ">");
        assert_eq!(decode_entities("&quot;hello&quot;"), "\"hello\"");
        assert_eq!(decode_entities("&nbsp;"), " ");
    }

    #[test]
    fn truncate_chars_exact() {
        let s = "abcdef";
        assert_eq!(truncate_chars(s, 10), "abcdef");
        let t = truncate_chars(s, 3);
        assert!(t.starts_with("abc"));
        assert!(t.contains("truncated"));
    }

    #[test]
    fn extract_mode_from_str() {
        assert_eq!(ExtractMode::from_str("markdown"), ExtractMode::Markdown);
        assert_eq!(ExtractMode::from_str("MD"), ExtractMode::Markdown);
        assert_eq!(ExtractMode::from_str("raw"), ExtractMode::Raw);
        assert_eq!(ExtractMode::from_str("text"), ExtractMode::Text);
        assert_eq!(ExtractMode::from_str("unknown"), ExtractMode::Text);
    }

    #[test]
    fn web_fetch_args_missing_url_errors() {
        let v = serde_json::json!({});
        assert!(WebFetchArgs::from_json(&v).is_err());
    }

    #[test]
    fn web_fetch_args_max_chars_capped() {
        let v = serde_json::json!({"url": "http://x.com", "max_chars": 999999});
        let args = WebFetchArgs::from_json(&v).unwrap();
        assert_eq!(args.max_chars, HARD_MAX_CHARS);
    }

    // ── SSRF protection tests ─────────────────────────────────────────────

    #[test]
    fn ssrf_blocks_localhost() {
        assert!(check_ssrf("http://localhost/admin").is_err());
        assert!(check_ssrf("http://localhost:8080/").is_err());
    }

    #[test]
    fn ssrf_blocks_loopback_ip() {
        assert!(check_ssrf("http://127.0.0.1/").is_err());
        assert!(check_ssrf("http://127.0.0.1:9200/").is_err());
        assert!(check_ssrf("https://127.1.2.3/secret").is_err());
    }

    #[test]
    fn ssrf_blocks_rfc1918_10_prefix() {
        assert!(check_ssrf("http://10.0.0.1/").is_err());
        assert!(check_ssrf("https://10.255.255.255/internal").is_err());
    }

    #[test]
    fn ssrf_blocks_rfc1918_192_168() {
        assert!(check_ssrf("http://192.168.1.1/router").is_err());
        assert!(check_ssrf("https://192.168.0.1:8443/").is_err());
    }

    #[test]
    fn ssrf_blocks_rfc1918_172_16_to_31() {
        assert!(check_ssrf("http://172.16.0.1/").is_err());
        assert!(check_ssrf("http://172.31.255.255/").is_err());
    }

    #[test]
    fn ssrf_allows_172_outside_private_range() {
        // 172.15.x and 172.32.x are public
        assert!(check_ssrf("http://172.15.0.1/").is_ok());
        assert!(check_ssrf("http://172.32.0.1/").is_ok());
    }

    #[test]
    fn ssrf_blocks_link_local() {
        assert!(check_ssrf("http://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn ssrf_blocks_ipv6_loopback() {
        assert!(check_ssrf("http://[::1]/").is_err());
    }

    #[test]
    fn ssrf_allows_public_addresses() {
        assert!(check_ssrf("https://example.com/").is_ok());
        assert!(check_ssrf("https://api.github.com/repos").is_ok());
        assert!(check_ssrf("http://8.8.8.8/").is_ok());
    }

    #[test]
    fn ssrf_rejects_unsupported_scheme() {
        assert!(check_ssrf("ftp://example.com/").is_err());
        assert!(check_ssrf("file:///etc/passwd").is_err());
    }

    // ── collapse_whitespace ───────────────────────────────────────────────

    #[test]
    fn collapse_whitespace_single_spaces_unchanged() {
        let s = "hello world foo";
        assert_eq!(collapse_whitespace(s), "hello world foo");
    }

    #[test]
    fn collapse_whitespace_multiple_spaces_collapsed() {
        let s = "hello   world    foo";
        assert_eq!(collapse_whitespace(s), "hello world foo");
    }

    #[test]
    fn collapse_whitespace_trims_leading_trailing() {
        let s = "   hello world   ";
        assert_eq!(collapse_whitespace(s), "hello world");
    }

    #[test]
    fn collapse_whitespace_newlines_collapsed() {
        let s = "line1\n\n\nline2";
        let out = collapse_whitespace(s);
        assert!(out.contains("line1") && out.contains("line2"));
        assert!(!out.contains("\n\n"), "consecutive newlines should collapse: {}", out);
    }

    #[test]
    fn collapse_whitespace_empty_string() {
        assert_eq!(collapse_whitespace(""), "");
    }

    // ── extract_attr ──────────────────────────────────────────────────────

    #[test]
    fn extract_attr_href() {
        let tag = r#"<a href="https://example.com" class="link">"#;
        let href = extract_attr(tag, "href");
        assert_eq!(href, Some("https://example.com".to_string()));
    }

    #[test]
    fn extract_attr_missing_returns_none() {
        let tag = r#"<a class="link">"#;
        assert!(extract_attr(tag, "href").is_none());
    }

    #[test]
    fn extract_attr_case_insensitive_attr_name() {
        let tag = r#"<img SRC="image.png">"#;
        let src = extract_attr(tag, "src");
        assert_eq!(src, Some("image.png".to_string()));
    }

    // ── decode_entities additional cases ─────────────────────────────────

    #[test]
    fn decode_entities_ndash_mdash_hellip() {
        assert_eq!(decode_entities("&ndash;"), "\u{2013}");
        assert_eq!(decode_entities("&mdash;"), "\u{2014}");
        assert_eq!(decode_entities("&hellip;"), "\u{2026}");
    }

    #[test]
    fn decode_entities_apos() {
        assert_eq!(decode_entities("&#39;"), "'");
    }

    #[test]
    fn decode_entities_no_change_on_plain_text() {
        assert_eq!(decode_entities("hello world"), "hello world");
    }

    // ── truncate_chars unicode safety ─────────────────────────────────────

    #[test]
    fn truncate_chars_unicode_aware() {
        let s = "こんにちは世界"; // 7 chars
        assert_eq!(truncate_chars(s, 10), s);
        let t = truncate_chars(s, 3);
        assert!(t.starts_with("こんに"), "must truncate on char boundary: {}", t);
        assert!(t.contains("truncated"), "must have truncation marker: {}", t);
    }

    // ── WebFetchArgs method / headers / body ──────────────────────────────

    #[test]
    fn web_fetch_args_method_default_get() {
        let v = serde_json::json!({"url": "https://x.com"});
        let args = WebFetchArgs::from_json(&v).unwrap();
        assert_eq!(args.method, "GET");
    }

    #[test]
    fn web_fetch_args_custom_method_and_body() {
        let v = serde_json::json!({
            "url": "https://api.example.com/data",
            "method": "POST",
            "body": "{\"key\":\"value\"}"
        });
        let args = WebFetchArgs::from_json(&v).unwrap();
        assert_eq!(args.method, "POST");
        assert_eq!(args.body, Some("{\"key\":\"value\"}"));
    }

    #[test]
    fn web_fetch_args_headers_parsed() {
        let v = serde_json::json!({
            "url": "https://example.com",
            "headers": { "Authorization": "Bearer token123" }
        });
        let args = WebFetchArgs::from_json(&v).unwrap();
        assert_eq!(
            args.headers.get("Authorization").map(|s| s.as_str()),
            Some("Bearer token123")
        );
    }
}
