//! OpenClaw+ 全网情报员核心引擎
//!
//! 提供以下能力：
//! - `fetcher`  — HTTP 页面抓取（带重试、User-Agent 轮换）
//! - `rss`      — RSS/Atom feed 解析
//! - `monitor`  — 竞品站点变化监控（diff 检测）
//! - `summarize`— AI 摘要（通过本地 Ollama / 外部 API）
//! - `report`   — 情报汇总报告生成
//! - `task`     — 情报任务定义与执行引擎

pub mod fetcher;
pub mod rss;
pub mod monitor;
pub mod summarize;
pub mod report;
pub mod task;

pub use task::{IntelTask, IntelTaskKind, IntelTaskStatus, IntelTaskResult};
pub use report::{IntelReport, IntelItem};
pub use monitor::{MonitorTarget, MonitorResult};
