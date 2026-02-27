//! 情报任务定义与执行引擎
//!
//! 支持两类任务：
//! - `ScanAll`   — 全量扫描所有配置目标，生成情报报告
//! - `ScanUrl`   — 临时抓取单个 URL 并摘要
//! - `RssScan`   — 抓取单个 RSS feed

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::monitor::MonitorTarget;
use crate::report::{IntelCategory, IntelItem, IntelReport};
use crate::summarize::SummarizeConfig;

// ── 任务定义 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntelTaskKind {
    /// 对所有配置目标做全量扫描并生成报告
    ScanAll,
    /// 抓取并摘要单个 URL
    ScanUrl { url: String, name: String, category: IntelCategory },
    /// 抓取单个 RSS feed
    RssScan { url: String, name: String, category: IntelCategory },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntelTaskStatus {
    Pending,
    Running,
    Done,
    Failed(String),
}

impl IntelTaskStatus {
    pub fn label_zh(&self) -> &str {
        match self {
            IntelTaskStatus::Pending    => "等待",
            IntelTaskStatus::Running    => "执行中",
            IntelTaskStatus::Done       => "完成",
            IntelTaskStatus::Failed(_)  => "失败",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelTask {
    pub id: u64,
    pub kind: IntelTaskKind,
    pub status: IntelTaskStatus,
    pub created_at: u64,
    pub finished_at: Option<u64>,
}

impl IntelTask {
    pub fn new(id: u64, kind: IntelTaskKind) -> Self {
        Self {
            id,
            kind,
            status: IntelTaskStatus::Pending,
            created_at: now_unix_secs(),
            finished_at: None,
        }
    }
}

/// 任务执行结果
#[derive(Debug, Clone)]
pub struct IntelTaskResult {
    pub task_id: u64,
    pub report: Option<IntelReport>,
    /// 单条结果（ScanUrl / RssScan 返回的条目列表）
    pub items: Vec<IntelItem>,
    pub error: Option<String>,
    pub elapsed_ms: u64,
}

// ── 默认监控目标（全网情报员预置） ────────────────────────────────────────────

pub fn default_targets() -> Vec<MonitorTarget> {
    vec![
        // ── 行业 RSS / 新闻 ──────────────────────────────────────────────
        MonitorTarget::rss("Hacker News Top", "https://hnrss.org/frontpage")
            .with_tags(vec!["tech", "industry"]),
        MonitorTarget::rss("TechCrunch", "https://techcrunch.com/feed/")
            .with_tags(vec!["tech", "industry"]),
        MonitorTarget::rss("36氪", "https://36kr.com/feed")
            .with_tags(vec!["industry", "finance"]),
        MonitorTarget::rss("Product Hunt Daily", "https://www.producthunt.com/feed")
            .with_tags(vec!["tech", "product"]),
        MonitorTarget::rss("V2EX 最新", "https://www.v2ex.com/?tab=tech.rss")
            .with_tags(vec!["tech", "community"]),
        // ── 竞品官网变化追踪（示例，用户可在设置中自定义） ──────────────
        MonitorTarget::new("示例竞品 A", "https://example.com")
            .with_tags(vec!["competitor"]),
    ]
}

/// tag → IntelCategory 推断
fn category_from_tags(tags: &[String]) -> IntelCategory {
    if tags.iter().any(|t| t == "competitor") {
        IntelCategory::Competitor
    } else if tags.iter().any(|t| t == "finance") {
        IntelCategory::Finance
    } else if tags.iter().any(|t| t == "tech" || t == "product") {
        IntelCategory::Tech
    } else if tags.iter().any(|t| t == "industry") {
        IntelCategory::Industry
    } else {
        IntelCategory::Other
    }
}

// ── 执行引擎 ──────────────────────────────────────────────────────────────────

/// 执行 ScanAll 任务：检查所有目标，返回情报报告（不含 AI 摘要，摘要异步填充）
pub async fn run_scan_all(
    targets: &mut Vec<MonitorTarget>,
    sum_cfg: &SummarizeConfig,
    agent_id: &str,
    with_ai_summary: bool,
) -> IntelTaskResult {
    let t0 = std::time::Instant::now();
    let targets_count = targets.len();

    // 1. 并行检查所有目标
    let results = crate::monitor::check_all(targets).await;

    // 2. 将结果转换为 IntelItem
    let mut items: Vec<IntelItem> = Vec::new();

    for (i, result) in results.iter().enumerate() {
        let target = targets.get(i).cloned().unwrap_or_else(|| MonitorTarget::new("unknown", &result.url));
        let category = category_from_tags(&target.tags);

        if result.error.is_some() {
            tracing::warn!(url = %result.url, err = ?result.error, "[Intel] target check error");
            continue;
        }

        // 如果是 RSS，解析并拆成多条
        if target.is_rss && result.changed {
            if let Ok(feed_items) = crate::rss::fetch_feed(&target.url).await {
                for fi in feed_items.into_iter().take(5) {
                    items.push(IntelItem {
                        title: fi.title,
                        source_name: target.name.clone(),
                        url: fi.link,
                        snippet: fi.summary.chars().take(300).collect(),
                        ai_summary: String::new(),
                        category: category.clone(),
                        fetched_at: result.checked_at,
                        is_new: true,
                    });
                }
            }
        } else if result.changed {
            let title = crate::fetcher::extract_title(&result.snippet);
            items.push(IntelItem {
                title: if title.is_empty() { target.name.clone() } else { title },
                source_name: target.name.clone(),
                url: result.url.clone(),
                snippet: result.snippet.chars().take(300).collect(),
                ai_summary: String::new(),
                category,
                fetched_at: result.checked_at,
                is_new: true,
            });
        }
    }

    // 3. AI 摘要（可选，为每条 item 生成摘要）
    if with_ai_summary && !items.is_empty() {
        let texts: Vec<String> = items.iter()
            .map(|i| format!("{}\n{}", i.title, i.snippet))
            .collect();
        let summaries = crate::summarize::batch_summarize(&texts, sum_cfg).await;
        for (item, summary) in items.iter_mut().zip(summaries.into_iter()) {
            item.ai_summary = summary;
        }
    }

    // 4. 生成报告
    let mut report = IntelReport::new(items.clone(), targets_count);
    report.has_ai_summary = with_ai_summary;

    if with_ai_summary && !items.is_empty() {
        match crate::summarize::daily_digest(&items, sum_cfg).await {
            Ok(digest) => report.daily_digest = digest,
            Err(e) => report.daily_digest = format!("[日报生成失败: {}]", e),
        }
    }

    // 5. 保存报告
    let _ = report.save(agent_id);

    IntelTaskResult {
        task_id: 0,
        report: Some(report),
        items,
        error: None,
        elapsed_ms: t0.elapsed().as_millis() as u64,
    }
}

/// 执行 ScanUrl 任务：抓取单个 URL 并生成 AI 摘要
pub async fn run_scan_url(
    url: &str,
    name: &str,
    category: IntelCategory,
    sum_cfg: &SummarizeConfig,
) -> IntelTaskResult {
    let t0 = std::time::Instant::now();

    match crate::fetcher::fetch_url(url, 2).await {
        Err(e) => IntelTaskResult {
            task_id: 0,
            report: None,
            items: Vec::new(),
            error: Some(e.to_string()),
            elapsed_ms: t0.elapsed().as_millis() as u64,
        },
        Ok(fetch) => {
            let title = crate::fetcher::extract_title(&fetch.text);
            let ai_summary = crate::summarize::summarize_with_ollama(&fetch.text, sum_cfg)
                .await
                .unwrap_or_else(|e| format!("[摘要失败: {}]", e));
            let item = IntelItem {
                title: if title.is_empty() { name.to_string() } else { title },
                source_name: name.to_string(),
                url: url.to_string(),
                snippet: fetch.text.chars().take(300).collect(),
                ai_summary,
                category,
                fetched_at: fetch.fetched_at,
                is_new: true,
            };
            IntelTaskResult {
                task_id: 0,
                report: None,
                items: vec![item],
                error: None,
                elapsed_ms: t0.elapsed().as_millis() as u64,
            }
        }
    }
}

/// 执行 RssScan 任务：抓取 RSS 并列出前 10 条
pub async fn run_rss_scan(
    url: &str,
    name: &str,
    category: IntelCategory,
) -> IntelTaskResult {
    let t0 = std::time::Instant::now();

    match crate::rss::fetch_feed(url).await {
        Err(e) => IntelTaskResult {
            task_id: 0,
            report: None,
            items: Vec::new(),
            error: Some(e.to_string()),
            elapsed_ms: t0.elapsed().as_millis() as u64,
        },
        Ok(feed_items) => {
            let now = now_unix_secs();
            let items: Vec<IntelItem> = feed_items
                .into_iter()
                .take(10)
                .map(|fi| IntelItem {
                    title: fi.title,
                    source_name: name.to_string(),
                    url: fi.link,
                    snippet: fi.summary.chars().take(300).collect(),
                    ai_summary: String::new(),
                    category: category.clone(),
                    fetched_at: now,
                    is_new: true,
                })
                .collect();
            IntelTaskResult {
                task_id: 0,
                report: None,
                items,
                error: None,
                elapsed_ms: t0.elapsed().as_millis() as u64,
            }
        }
    }
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
