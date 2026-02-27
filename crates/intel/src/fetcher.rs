//! HTTP 页面抓取器 — 带重试、User-Agent 轮换、内容清洗

use anyhow::{Context, Result};
use std::time::Duration;

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
];

/// 抓取结果
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub url: String,
    pub status: u16,
    /// 提取的纯文本内容（去除 HTML 标签）
    pub text: String,
    /// 原始 HTML（用于进一步解析）
    pub html: String,
    /// 响应延迟 (ms)
    pub latency_ms: u64,
    /// 抓取时间戳（Unix 秒）
    pub fetched_at: u64,
}

/// 构建共享的 HTTP 客户端（复用连接池）
pub fn build_client() -> Result<reqwest::Client> {
    let ua_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize)
        % USER_AGENTS.len();
    reqwest::Client::builder()
        .user_agent(USER_AGENTS[ua_idx])
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .context("build HTTP client")
}

/// 抓取指定 URL，最多重试 `retries` 次
pub async fn fetch_url(url: &str, retries: u8) -> Result<FetchResult> {
    let client = build_client()?;
    let mut last_err = anyhow::anyhow!("no attempts made");

    for attempt in 0..=retries {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
        }
        let t0 = std::time::Instant::now();
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let html = resp.text().await.unwrap_or_default();
                let text = html_to_text(&html);
                return Ok(FetchResult {
                    url: url.to_string(),
                    status,
                    text,
                    html,
                    latency_ms: t0.elapsed().as_millis() as u64,
                    fetched_at: now_unix_secs(),
                });
            }
            Err(e) => {
                tracing::warn!(url, attempt, error = %e, "[Intel] fetch failed");
                last_err = e.into();
            }
        }
    }
    Err(last_err)
}

/// 从 HTML 提取可读纯文本（使用 scraper 去标签）
pub fn html_to_text(html: &str) -> String {
    use scraper::{Html, Selector};

    let doc = Html::parse_document(html);

    // 移除 script / style 节点后收集文本
    let body_sel = Selector::parse("body").unwrap_or_else(|_| Selector::parse("*").unwrap());
    let script_sel = Selector::parse("script,style,nav,header,footer,aside").ok();

    let mut parts: Vec<String> = Vec::new();

    for node in doc.select(&body_sel) {
        for text_node in node.text() {
            let t = text_node.trim();
            if !t.is_empty() && t.len() > 2 {
                parts.push(t.to_string());
            }
        }
    }

    // 如果 body 选择器没找到内容，回退到全文本
    if parts.is_empty() {
        let _ = script_sel; // suppress unused warning
        for node in doc.root_element().text() {
            let t = node.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        }
    }

    parts.join("\n")
}

/// 从文本中提取文章标题候选（取前 3 行非空行中最长的）
pub fn extract_title(text: &str) -> String {
    text.lines()
        .filter(|l| l.len() > 5)
        .take(10)
        .max_by_key(|l| l.len())
        .unwrap_or("")
        .trim()
        .chars()
        .take(120)
        .collect()
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
