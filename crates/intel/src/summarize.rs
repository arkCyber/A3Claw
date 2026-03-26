//! AI 摘要模块 — 通过 Ollama (本地) 或 OpenAI (外部) 生成情报摘要

use anyhow::Result;
use serde_json::json;

/// 摘要配置
#[derive(Debug, Clone)]
pub struct SummarizeConfig {
    /// Ollama endpoint, e.g. "http://localhost:11434"
    pub ollama_endpoint: String,
    /// 模型名称, e.g. "qwen3.5:9b"
    pub model: String,
    /// 摘要语言提示词
    pub lang: SummarizeLang,
    /// 单次最大输入字符数（防止超 token）
    pub max_input_chars: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum SummarizeLang {
    Chinese,
    English,
}

impl Default for SummarizeConfig {
    fn default() -> Self {
        Self {
            ollama_endpoint: "http://localhost:11434".to_string(),
            model: "qwen3.5:9b".to_string(),
            lang: SummarizeLang::Chinese,
            max_input_chars: 3000,
        }
    }
}

/// 调用 Ollama 生成摘要
pub async fn summarize_with_ollama(
    text: &str,
    cfg: &SummarizeConfig,
) -> Result<String> {
    let truncated: String = text.chars().take(cfg.max_input_chars).collect();

    let system_prompt = match cfg.lang {
        SummarizeLang::Chinese =>
            "你是一名专业情报分析师。请用中文对以下内容进行简洁的情报摘要（3-5 句话），\
             突出关键事件、竞品动态或行业趋势。保持客观，避免废话。",
        SummarizeLang::English =>
            "You are a professional intelligence analyst. Summarize the following content \
             concisely in 3-5 sentences, highlighting key events, competitor moves, or \
             industry trends. Be objective and concise.",
    };

    let payload = json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user",   "content": truncated }
        ],
        "stream": false
    });

    let url = format!("{}/api/chat", cfg.ollama_endpoint);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let resp = client.post(&url).json(&payload).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Ollama API error: HTTP {}", resp.status());
    }

    let body: serde_json::Value = resp.json().await?;
    let content = body["message"]["content"]
        .as_str()
        .unwrap_or("[无摘要]")
        .trim()
        .to_string();

    Ok(content)
}

/// 批量为多段文本生成摘要，返回相同顺序的摘要列表
pub async fn batch_summarize(
    texts: &[String],
    cfg: &SummarizeConfig,
) -> Vec<String> {
    let mut out = Vec::with_capacity(texts.len());
    for text in texts {
        let summary = summarize_with_ollama(text, cfg)
            .await
            .unwrap_or_else(|e| format!("[摘要失败: {}]", e));
        out.push(summary);
    }
    out
}

/// 将多条情报条目合并成一份综合日报摘要
pub async fn daily_digest(
    items: &[crate::report::IntelItem],
    cfg: &SummarizeConfig,
) -> Result<String> {
    if items.is_empty() {
        return Ok("今日无新情报".to_string());
    }

    let combined: String = items
        .iter()
        .enumerate()
        .map(|(i, item)| format!("【{}】{}\n来源: {}\n{}", i + 1, item.title, item.source_name, item.snippet))
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = match cfg.lang {
        SummarizeLang::Chinese => format!(
            "以下是今日收集到的 {} 条情报，请生成一份简洁的情报日报（不超过 300 字），\
             按重要性排序，分竞品动态和行业新闻两个维度汇总：\n\n{}",
            items.len(), combined
        ),
        SummarizeLang::English => format!(
            "Below are {} intelligence items collected today. Write a concise daily \
             intelligence digest (max 300 words), sorted by importance, covering \
             competitor updates and industry news:\n\n{}",
            items.len(), combined
        ),
    };

    summarize_with_ollama(&prompt, cfg).await
}
