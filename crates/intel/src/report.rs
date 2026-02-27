//! 情报报告生成 — 汇总所有抓取结果，生成结构化报告

use serde::{Deserialize, Serialize};

/// 单条情报条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelItem {
    /// 标题
    pub title: String,
    /// 来源名称（站点名/Feed 名）
    pub source_name: String,
    /// 原始 URL
    pub url: String,
    /// 内容片段（前 300 字符）
    pub snippet: String,
    /// AI 生成的摘要（可能为空，等待异步摘要完成后填充）
    pub ai_summary: String,
    /// 分类标签
    pub category: IntelCategory,
    /// 抓取时间戳
    pub fetched_at: u64,
    /// 是否为新内容（本次检测有变化）
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntelCategory {
    /// 竞品动态
    Competitor,
    /// 行业新闻
    Industry,
    /// 技术资讯
    Tech,
    /// 投融资信息
    Finance,
    /// 其他
    Other,
}

impl IntelCategory {
    pub fn label_zh(&self) -> &'static str {
        match self {
            IntelCategory::Competitor => "竞品动态",
            IntelCategory::Industry   => "行业新闻",
            IntelCategory::Tech       => "技术资讯",
            IntelCategory::Finance    => "投融资",
            IntelCategory::Other      => "其他",
        }
    }
    pub fn color_rgb(&self) -> (f32, f32, f32) {
        match self {
            IntelCategory::Competitor => (0.92, 0.30, 0.30),
            IntelCategory::Industry   => (0.30, 0.70, 0.92),
            IntelCategory::Tech       => (0.30, 0.92, 0.60),
            IntelCategory::Finance    => (0.98, 0.78, 0.20),
            IntelCategory::Other      => (0.60, 0.60, 0.60),
        }
    }
}

/// 完整情报报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelReport {
    /// 报告 ID（Unix 时间戳字符串）
    pub id: String,
    /// 报告生成时间戳
    pub generated_at: u64,
    /// 报告标题（默认含日期）
    pub title: String,
    /// 综合日报摘要（AI 生成）
    pub daily_digest: String,
    /// 情报条目列表
    pub items: Vec<IntelItem>,
    /// 本次扫描的目标数量
    pub targets_checked: usize,
    /// 有变化的目标数量
    pub changed_count: usize,
    /// 是否已生成 AI 摘要
    pub has_ai_summary: bool,
}

impl IntelReport {
    pub fn new(items: Vec<IntelItem>, targets_checked: usize) -> Self {
        let now = now_unix_secs();
        let changed_count = items.iter().filter(|i| i.is_new).count();
        let title = {
            let dt = format_ts(now);
            format!("情报日报 — {}", dt)
        };
        Self {
            id: now.to_string(),
            generated_at: now,
            title,
            daily_digest: String::new(),
            items,
            targets_checked,
            changed_count,
            has_ai_summary: false,
        }
    }

    pub fn new_items(&self) -> Vec<&IntelItem> {
        self.items.iter().filter(|i| i.is_new).collect()
    }

    pub fn by_category(&self, cat: &IntelCategory) -> Vec<&IntelItem> {
        self.items.iter().filter(|i| &i.category == cat).collect()
    }

    pub fn summary_stats(&self) -> ReportStats {
        ReportStats {
            total: self.items.len(),
            new_count: self.changed_count,
            competitor: self.by_category(&IntelCategory::Competitor).len(),
            industry: self.by_category(&IntelCategory::Industry).len(),
            tech: self.by_category(&IntelCategory::Tech).len(),
            finance: self.by_category(&IntelCategory::Finance).len(),
        }
    }

    /// 保存到本地 JSON 文件（agent workspace 目录下）
    pub fn save(&self, agent_id: &str) -> anyhow::Result<()> {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".openclaw-plus")
            .join("agents")
            .join(agent_id)
            .join("intel_reports");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        tracing::info!(agent_id, report_id = %self.id, items = self.items.len(), "Intel report saved");
        Ok(())
    }

    /// 加载历史报告列表（按时间降序，最多返回 `limit` 条）
    pub fn list_history(agent_id: &str, limit: usize) -> Vec<Self> {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".openclaw-plus")
            .join("agents")
            .join(agent_id)
            .join("intel_reports");
        let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new(); };
        let mut reports: Vec<Self> = entries
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some("json") {
                    std::fs::read_to_string(&p).ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                } else {
                    None
                }
            })
            .collect();
        reports.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));
        reports.truncate(limit);
        reports
    }
}

#[derive(Debug, Clone)]
pub struct ReportStats {
    pub total: usize,
    pub new_count: usize,
    pub competitor: usize,
    pub industry: usize,
    pub tech: usize,
    pub finance: usize,
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn format_ts(ts: u64) -> String {
    // Simple YYYY-MM-DD formatter without chrono dependency
    let secs = ts;
    let days_since_epoch = secs / 86400;
    let year_approx = 1970 + days_since_epoch / 365;
    let day_of_year = days_since_epoch % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{}-{:02}-{:02}", year_approx, month.min(12), day.min(31))
}
