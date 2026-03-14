//! RSS / Atom feed 解析器

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 单条 feed 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub title: String,
    pub link: String,
    pub summary: String,
    pub published_at: Option<String>,
    pub author: Option<String>,
}

/// 解析 RSS / Atom XML，返回条目列表
pub fn parse_feed(xml: &str) -> Vec<FeedItem> {
    let mut items = Vec::new();

    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("[Intel/RSS] XML parse error: {}", e);
            return items;
        }
    };

    let root = doc.root_element();
    let is_atom = root.tag_name().name() == "feed";

    if is_atom {
        // Atom feed
        for entry in root.children().filter(|n| n.tag_name().name() == "entry") {
            let title = child_text(&entry, "title").unwrap_or_default();
            let link = entry
                .children()
                .find(|n| n.tag_name().name() == "link")
                .and_then(|n| n.attribute("href"))
                .unwrap_or("")
                .to_string();
            let summary = child_text(&entry, "summary")
                .or_else(|| child_text(&entry, "content"))
                .unwrap_or_default();
            let published_at = child_text(&entry, "published")
                .or_else(|| child_text(&entry, "updated"));
            let author = entry
                .children()
                .find(|n| n.tag_name().name() == "author")
                .and_then(|n| n.children().find(|c| c.tag_name().name() == "name"))
                .and_then(|n| n.text())
                .map(|s| s.to_string());
            items.push(FeedItem { title, link, summary, published_at, author });
        }
    } else {
        // RSS 2.0 — find <channel> → <item>
        let channel = root
            .descendants()
            .find(|n| n.tag_name().name() == "channel")
            .unwrap_or(root);

        for item in channel.children().filter(|n| n.tag_name().name() == "item") {
            let title = child_text(&item, "title").unwrap_or_default();
            let link = child_text(&item, "link").unwrap_or_default();
            let summary = child_text(&item, "description")
                .or_else(|| child_text(&item, "content:encoded"))
                .unwrap_or_default();
            let published_at = child_text(&item, "pubDate");
            let author = child_text(&item, "author")
                .or_else(|| child_text(&item, "dc:creator"));
            items.push(FeedItem { title, link, summary, published_at, author });
        }
    }

    items
}

fn child_text(node: &roxmltree::Node, tag: &str) -> Option<String> {
    node.children()
        .find(|n| n.tag_name().name() == tag)
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 抓取并解析一个 RSS feed URL
pub async fn fetch_feed(url: &str) -> Result<Vec<FeedItem>> {
    let result = crate::fetcher::fetch_url(url, 2).await?;
    Ok(parse_feed(&result.html))
}
