//! 竞品站点变化追踪 — 对比上次抓取内容，检测更新

use serde::{Deserialize, Serialize};

/// 一个需要追踪的目标站点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorTarget {
    /// 展示名称（如 "竞品A官网"）
    pub name: String,
    /// 抓取 URL
    pub url: String,
    /// 上次内容的指纹（SHA256 前16字符），空字符串表示首次
    #[serde(default)]
    pub last_fingerprint: String,
    /// 上次抓取时间戳（Unix 秒）
    #[serde(default)]
    pub last_checked_at: u64,
    /// 是否为 RSS feed（用 RSS 解析器处理）
    #[serde(default)]
    pub is_rss: bool,
    /// 标签（如 "competitor", "industry", "tech"）
    #[serde(default)]
    pub tags: Vec<String>,
}

impl MonitorTarget {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            last_fingerprint: String::new(),
            last_checked_at: 0,
            is_rss: false,
            tags: Vec::new(),
        }
    }

    pub fn rss(name: impl Into<String>, url: impl Into<String>) -> Self {
        let mut t = Self::new(name, url);
        t.is_rss = true;
        t
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }
}

/// 追踪结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorResult {
    pub target_name: String,
    pub url: String,
    /// true = 内容有变化（或首次抓取）
    pub changed: bool,
    /// 内容摘要（前 500 字符）
    pub snippet: String,
    /// 新内容指纹
    pub fingerprint: String,
    /// 抓取时间戳
    pub checked_at: u64,
    /// 错误信息（如抓取失败）
    pub error: Option<String>,
}

/// 对单个目标执行一次追踪检查
pub async fn check_target(target: &mut MonitorTarget) -> MonitorResult {
    let now = now_unix_secs();

    let text_result = if target.is_rss {
        match crate::rss::fetch_feed(&target.url).await {
            Ok(items) => {
                let combined = items
                    .iter()
                    .take(10)
                    .map(|i| format!("{} — {}", i.title, i.link))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(combined)
            }
            Err(e) => Err(e),
        }
    } else {
        crate::fetcher::fetch_url(&target.url, 2)
            .await
            .map(|r| r.text)
    };

    match text_result {
        Err(e) => {
            target.last_checked_at = now;
            MonitorResult {
                target_name: target.name.clone(),
                url: target.url.clone(),
                changed: false,
                snippet: String::new(),
                fingerprint: target.last_fingerprint.clone(),
                checked_at: now,
                error: Some(e.to_string()),
            }
        }
        Ok(text) => {
            let fp = fingerprint(&text);
            let changed = target.last_fingerprint.is_empty() || target.last_fingerprint != fp;
            let snippet = text.chars().take(500).collect::<String>();
            target.last_fingerprint = fp.clone();
            target.last_checked_at = now;
            MonitorResult {
                target_name: target.name.clone(),
                url: target.url.clone(),
                changed,
                snippet,
                fingerprint: fp,
                checked_at: now,
                error: None,
            }
        }
    }
}

/// 批量检查一组目标，并发执行
pub async fn check_all(targets: &mut [MonitorTarget]) -> Vec<MonitorResult> {
    let mut results = Vec::new();
    for target in targets.iter_mut() {
        let r = check_target(target).await;
        results.push(r);
    }
    results
}

fn fingerprint(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
