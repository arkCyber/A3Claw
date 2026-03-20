//! `news_fetch` — 获取实时新闻的工具
//!
//! 支持多个新闻源：
//! - RSS feeds (CNN, BBC, Reuters等)
//! - 免费新闻 API
//!
//! Parameters:
//! - `source` — 新闻源 (default: "rss_cnn")
//! - `category` — 新闻类别 (default: "general")
//! - `count` — 返回新闻数量 (default: 5, max: 20)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_NEWS_COUNT: usize = 5;
pub const MAX_NEWS_COUNT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub summary: String,
    pub published_at: String,
    pub url: String,
}

pub struct NewsFetchArgs {
    pub source: String,
    pub category: String,
    pub count: usize,
}

impl NewsFetchArgs {
    pub fn from_json(args: &serde_json::Value) -> Result<Self, String> {
        let source = args["source"]
            .as_str()
            .unwrap_or("rss_cnn")
            .to_string();
        let category = args["category"]
            .as_str()
            .unwrap_or("general")
            .to_string();
        let count = args["count"]
            .as_u64()
            .unwrap_or(DEFAULT_NEWS_COUNT as u64)
            .min(MAX_NEWS_COUNT as u64) as usize;
        
        Ok(NewsFetchArgs { source, category, count })
    }
}

/// 获取新闻
pub async fn fetch_news(
    client: &reqwest::Client,
    args: &NewsFetchArgs,
) -> Result<Vec<NewsItem>, String> {
    match args.source.as_str() {
        "rss_cnn" => fetch_rss_feed(client, "http://rss.cnn.com/rss/cnn_topstories.rss", args.count).await,
        "rss_bbc" => fetch_rss_feed(client, "http://feeds.bbci.co.uk/news/rss.xml", args.count).await,
        "rss_reuters" => fetch_rss_feed(client, "https://www.reutersagency.com/feed/?taxonomy=best-topics&post_type=best", args.count).await,
        _ => Err(format!("不支持的新闻源: {}", args.source)),
    }
}

/// 从 RSS feed 获取新闻
async fn fetch_rss_feed(
    client: &reqwest::Client,
    feed_url: &str,
    count: usize,
) -> Result<Vec<NewsItem>, String> {
    let resp = client
        .get(feed_url)
        .header("User-Agent", "Mozilla/5.0 (compatible; OpenClaw+/1.0)")
        .send()
        .await
        .map_err(|e| format!("获取 RSS feed 失败: {}", e))?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    parse_rss(&body, count)
}

/// 解析 RSS XML
fn parse_rss(xml: &str, count: usize) -> Result<Vec<NewsItem>, String> {
    let mut items = Vec::new();
    
    // 简单的 XML 解析（生产环境应使用 xml-rs 或 quick-xml）
    let mut pos = 0;
    while items.len() < count && pos < xml.len() {
        // 查找 <item> 标签
        if let Some(item_start) = xml[pos..].find("<item>").or_else(|| xml[pos..].find("<item ")) {
            let item_start = pos + item_start;
            if let Some(item_end) = xml[item_start..].find("</item>") {
                let item_end = item_start + item_end + 7;
                let item_xml = &xml[item_start..item_end];
                
                if let Some(news_item) = parse_rss_item(item_xml) {
                    items.push(news_item);
                }
                
                pos = item_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    
    if items.is_empty() {
        Err("未能解析到任何新闻".to_string())
    } else {
        Ok(items)
    }
}

/// 解析单个 RSS item
fn parse_rss_item(item_xml: &str) -> Option<NewsItem> {
    let title = extract_xml_tag(item_xml, "title")?;
    let summary = extract_xml_tag(item_xml, "description")
        .or_else(|| extract_xml_tag(item_xml, "summary"))
        .unwrap_or_else(|| "无摘要".to_string());
    let url = extract_xml_tag(item_xml, "link")
        .or_else(|| extract_xml_tag(item_xml, "guid"))
        .unwrap_or_else(|| "".to_string());
    
    // 尝试解析发布时间
    let pub_date = extract_xml_tag(item_xml, "pubDate")
        .or_else(|| extract_xml_tag(item_xml, "published"))
        .or_else(|| extract_xml_tag(item_xml, "updated"));
    
    let published_at = if let Some(date_str) = pub_date {
        parse_rfc2822_or_iso8601(&date_str)
            .unwrap_or_else(|| format_current_time())
    } else {
        format_current_time()
    };
    
    Some(NewsItem {
        title: clean_html(&title),
        summary: clean_html(&truncate_text(&summary, 200)),
        published_at,
        url,
    })
}

/// 提取 XML 标签内容
fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    let start = xml.find(&open_tag)? + open_tag.len();
    let end = xml[start..].find(&close_tag)?;
    
    Some(xml[start..start + end].trim().to_string())
}

/// 清理 HTML 标签和实体
fn clean_html(text: &str) -> String {
    let no_tags = text
        .replace("<![CDATA[", "")
        .replace("]]>", "");
    
    // 移除 HTML 标签
    let mut result = String::new();
    let mut in_tag = false;
    for c in no_tags.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    
    // 解码 HTML 实体
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

/// 截断文本
fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

/// 解析 RFC2822 或 ISO8601 时间格式
fn parse_rfc2822_or_iso8601(date_str: &str) -> Option<String> {
    // 尝试 RFC2822 格式 (RSS 常用)
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return Some(dt.format("%Y-%m-%d %H:%M").to_string());
    }
    
    // 尝试 ISO8601 格式
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.format("%Y-%m-%d %H:%M").to_string());
    }
    
    None
}

/// 获取当前时间
fn format_current_time() -> String {
    Utc::now().format("%Y-%m-%d %H:%M").to_string()
}

/// 格式化新闻列表为文本
pub fn format_news_list(news: &[NewsItem]) -> String {
    let mut result = String::new();
    
    for (i, item) in news.iter().enumerate() {
        result.push_str(&format!(
            "\n{}. [{}] {}\n摘要：{}\n",
            i + 1,
            item.published_at,
            item.title,
            item.summary
        ));
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_xml_tag() {
        let xml = "<item><title>Test Title</title><link>http://example.com</link></item>";
        assert_eq!(extract_xml_tag(xml, "title"), Some("Test Title".to_string()));
        assert_eq!(extract_xml_tag(xml, "link"), Some("http://example.com".to_string()));
        assert_eq!(extract_xml_tag(xml, "missing"), None);
    }

    #[test]
    fn test_clean_html() {
        assert_eq!(clean_html("<p>Hello</p>"), "Hello");
        assert_eq!(clean_html("&amp;&lt;&gt;"), "&<>");
        assert_eq!(clean_html("<![CDATA[Test]]>"), "Test");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("Hello", 10), "Hello");
        assert_eq!(truncate_text("Hello World", 5), "Hello...");
    }

    #[test]
    fn test_parse_rfc2822() {
        let date = "Wed, 19 Mar 2026 16:00:00 +0800";
        let result = parse_rfc2822_or_iso8601(date);
        assert!(result.is_some());
        assert!(result.unwrap().contains("2026-03-19"));
    }

    #[test]
    fn test_news_fetch_args_defaults() {
        let json = serde_json::json!({});
        let args = NewsFetchArgs::from_json(&json).unwrap();
        assert_eq!(args.source, "rss_cnn");
        assert_eq!(args.category, "general");
        assert_eq!(args.count, DEFAULT_NEWS_COUNT);
    }

    #[test]
    fn test_news_fetch_args_custom() {
        let json = serde_json::json!({
            "source": "rss_bbc",
            "category": "tech",
            "count": 10
        });
        let args = NewsFetchArgs::from_json(&json).unwrap();
        assert_eq!(args.source, "rss_bbc");
        assert_eq!(args.category, "tech");
        assert_eq!(args.count, 10);
    }

    #[test]
    fn test_news_fetch_args_max_count() {
        let json = serde_json::json!({"count": 999});
        let args = NewsFetchArgs::from_json(&json).unwrap();
        assert_eq!(args.count, MAX_NEWS_COUNT);
    }
}
