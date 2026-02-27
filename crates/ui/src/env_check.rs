//! Startup environment health checker for OpenClaw+.
//!
//! Runs a series of checks on startup:
//!   1. Ollama service — reachable? auto-start if not.
//!   2. Local AI model installed (qwen2.5:0.5b or configured model).
//!   3. External AI APIs (OpenAI, DeepSeek, Anthropic) — optional, based on config.
//!   4. WasmEdge runtime — needed for WASM policy plugins.
//!   5. OpenClaw Gateway — optional integration.
//!   6. Disk space on workspace dir.
//!   7. Internet connectivity (ping check).

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// ── Check status ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    /// Check hasn't run yet.
    Pending,
    /// Check is currently running.
    Running,
    /// Check passed.
    Ok,
    /// Check failed but non-fatal (degraded mode possible).
    Warning(String),
    /// Check failed fatally.
    Error(String),
    /// Check was skipped (not configured).
    Skipped,
}

impl CheckStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, CheckStatus::Ok)
    }
    pub fn is_done(&self) -> bool {
        !matches!(self, CheckStatus::Pending | CheckStatus::Running)
    }
    pub fn icon(&self) -> &'static str {
        match self {
            CheckStatus::Pending  => "⏳",
            CheckStatus::Running  => "🔄",
            CheckStatus::Ok       => "✅",
            CheckStatus::Warning(_) => "⚠️",
            CheckStatus::Error(_)   => "❌",
            CheckStatus::Skipped    => "⏭️",
        }
    }
    pub fn label(&self) -> String {
        match self {
            CheckStatus::Pending      => "等待中".to_string(),
            CheckStatus::Running      => "检测中…".to_string(),
            CheckStatus::Ok           => "正常".to_string(),
            CheckStatus::Warning(msg) => format!("警告: {}", msg),
            CheckStatus::Error(msg)   => format!("失败: {}", msg),
            CheckStatus::Skipped      => "跳过".to_string(),
        }
    }
}

// ── Individual check result ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvCheckItem {
    /// Short identifier (used for keying).
    pub id: &'static str,
    /// Human-readable display name.
    pub name: String,
    /// Description of what this check does.
    pub description: String,
    /// Result of the check.
    pub status: CheckStatus,
    /// How long the check took (ms).
    pub latency_ms: Option<u64>,
    /// Optional detail / fix hint.
    pub detail: Option<String>,
}

impl EnvCheckItem {
    pub fn new(id: &'static str, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: description.into(),
            status: CheckStatus::Pending,
            latency_ms: None,
            detail: None,
        }
    }
}

// ── Aggregate result ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct EnvCheckReport {
    pub items: Vec<EnvCheckItem>,
    /// Whether all critical checks passed.
    pub all_critical_ok: bool,
    /// Whether Ollama was auto-started by us.
    pub ollama_was_started: bool,
}

impl EnvCheckReport {
    pub fn new() -> Self {
        Self {
            items: vec![
                EnvCheckItem::new("ollama_running",  "Ollama 服务",      "检测 Ollama 是否运行，如未运行则自动启动"),
                EnvCheckItem::new("ollama_model",    "本地 AI 模型",     "确认默认 AI 模型已安装"),
                EnvCheckItem::new("wasmedge",        "WasmEdge 运行时",  "WASM 沙箱策略插件依赖"),
                EnvCheckItem::new("disk_space",      "磁盘空间",          "工作目录可用空间 ≥ 500MB"),
                EnvCheckItem::new("internet",        "网络连通",          "基础互联网连接"),
                EnvCheckItem::new("openai_api",      "OpenAI API",        "检测 OpenAI API Key 可用性"),
                EnvCheckItem::new("deepseek_api",    "DeepSeek API",      "检测 DeepSeek API Key 可用性"),
                EnvCheckItem::new("anthropic_api",   "Anthropic API",     "检测 Anthropic API Key 可用性"),
                EnvCheckItem::new("gateway",         "OpenClaw Gateway",  "本地 OpenClaw 服务连接"),
            ],
            all_critical_ok: false,
            ollama_was_started: false,
        }
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut EnvCheckItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Count items that have finished (any terminal status).
    pub fn finished_count(&self) -> usize {
        self.items.iter().filter(|i| i.status.is_done()).count()
    }

    pub fn total_count(&self) -> usize {
        self.items.len()
    }

    /// Progress 0.0..=1.0
    pub fn progress(&self) -> f32 {
        if self.items.is_empty() { return 1.0; }
        self.finished_count() as f32 / self.total_count() as f32
    }

    pub fn is_complete(&self) -> bool {
        self.items.iter().all(|i| i.status.is_done())
    }
}

// ── Check parameters passed in from app config ────────────────────────────────

#[derive(Debug, Clone)]
pub struct EnvCheckParams {
    pub ollama_endpoint: String,
    pub ollama_model: String,
    pub openai_api_key: Option<String>,
    pub deepseek_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub gateway_url: Option<String>,
    pub workspace_dir: String,
}

// ── Individual async check functions ─────────────────────────────────────────

/// Try to start Ollama and wait for it to become ready (up to 8s).
async fn try_start_ollama() -> bool {
    // Try `ollama serve` as a background process
    let spawn_result = tokio::process::Command::new("ollama")
        .arg("serve")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    if spawn_result.is_err() {
        return false;
    }

    // Poll until ready, up to 8 seconds
    for _ in 0..16 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if reqwest::Client::new()
            .get("http://localhost:11434/api/tags")
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .is_ok()
        {
            return true;
        }
    }
    false
}

pub async fn check_ollama(endpoint: &str) -> (CheckStatus, Option<u64>, bool) {
    let t0 = Instant::now();
    let url = format!("{}/api/tags", endpoint);

    // First attempt
    let reachable = reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .is_ok();

    if reachable {
        return (CheckStatus::Ok, Some(t0.elapsed().as_millis() as u64), false);
    }

    // Not reachable — attempt auto-start (only for localhost)
    if endpoint.contains("localhost") || endpoint.contains("127.0.0.1") {
        eprintln!("[ENV-CHECK] Ollama not running, attempting auto-start...");
        let started = try_start_ollama().await;
        let latency = t0.elapsed().as_millis() as u64;
        if started {
            return (
                CheckStatus::Ok,
                Some(latency),
                true, // was_started
            );
        } else {
            return (
                CheckStatus::Error("Ollama 未运行且自动启动失败，请手动运行 `ollama serve`".to_string()),
                Some(latency),
                false,
            );
        }
    }

    (
        CheckStatus::Error(format!("无法连接到 Ollama ({})", endpoint)),
        Some(t0.elapsed().as_millis() as u64),
        false,
    )
}

pub async fn check_ollama_model(endpoint: &str, model: &str) -> (CheckStatus, Option<u64>) {
    let t0 = Instant::now();
    let url = format!("{}/api/tags", endpoint);

    let resp = reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    let latency = Some(t0.elapsed().as_millis() as u64);

    match resp {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let models = body["models"].as_array().cloned().unwrap_or_default();
            let found = models.iter().any(|m| {
                m["name"].as_str().unwrap_or("").starts_with(model.split(':').next().unwrap_or(model))
            });
            if found {
                (CheckStatus::Ok, latency)
            } else {
                (
                    CheckStatus::Warning(format!("模型 {} 未找到，请运行: ollama pull {}", model, model)),
                    latency,
                )
            }
        }
        _ => (CheckStatus::Warning("无法列出模型 (Ollama 未就绪?)".to_string()), latency),
    }
}

pub async fn check_wasmedge() -> (CheckStatus, Option<u64>) {
    let t0 = Instant::now();
    let ok = tokio::process::Command::new("wasmedge")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    let latency = Some(t0.elapsed().as_millis() as u64);
    if ok {
        (CheckStatus::Ok, latency)
    } else {
        (
            CheckStatus::Warning("WasmEdge 未安装，WASM 沙箱策略功能不可用".to_string()),
            latency,
        )
    }
}

pub async fn check_disk_space(workspace_dir: &str) -> (CheckStatus, Option<u64>) {
    let t0 = Instant::now();
    // Ensure directory exists
    let _ = std::fs::create_dir_all(workspace_dir);

    // Use `df` to get available space in 512-byte blocks
    let output = tokio::process::Command::new("df")
        .arg("-k") // kilobytes
        .arg(workspace_dir)
        .output()
        .await;

    let latency = Some(t0.elapsed().as_millis() as u64);

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            // Parse the second line, 4th column (available KB)
            let avail_kb: u64 = text.lines().nth(1)
                .and_then(|line| line.split_whitespace().nth(3))
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let avail_mb = avail_kb / 1024;
            if avail_mb >= 500 {
                (CheckStatus::Ok, latency)
            } else {
                (CheckStatus::Warning(format!("磁盘可用空间仅 {}MB，建议 ≥ 500MB", avail_mb)), latency)
            }
        }
        Err(e) => (CheckStatus::Warning(format!("无法检测磁盘空间: {}", e)), latency),
    }
}

pub async fn check_internet() -> (CheckStatus, Option<u64>) {
    let t0 = Instant::now();
    // Try a reliable endpoint with short timeout
    let ok = reqwest::Client::new()
        .head("https://www.cloudflare.com")
        .timeout(Duration::from_secs(4))
        .send()
        .await
        .is_ok();
    let latency = Some(t0.elapsed().as_millis() as u64);
    if ok {
        (CheckStatus::Ok, latency)
    } else {
        (CheckStatus::Warning("无法访问外网，外部 AI API 可能不可用".to_string()), latency)
    }
}

pub async fn check_openai_api(api_key: &str) -> (CheckStatus, Option<u64>) {
    if api_key.is_empty() {
        return (CheckStatus::Skipped, None);
    }
    let t0 = Instant::now();
    let resp = reqwest::Client::new()
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(Duration::from_secs(8))
        .send()
        .await;
    let latency = Some(t0.elapsed().as_millis() as u64);
    match resp {
        Ok(r) if r.status().is_success() => (CheckStatus::Ok, latency),
        Ok(r) if r.status().as_u16() == 401 => {
            (CheckStatus::Error("OpenAI API Key 无效 (401 Unauthorized)".to_string()), latency)
        }
        Ok(r) => (CheckStatus::Warning(format!("OpenAI API 返回 HTTP {}", r.status())), latency),
        Err(e) => (CheckStatus::Warning(format!("OpenAI API 不可达: {}", e)), latency),
    }
}

pub async fn check_deepseek_api(api_key: &str) -> (CheckStatus, Option<u64>) {
    if api_key.is_empty() {
        return (CheckStatus::Skipped, None);
    }
    let t0 = Instant::now();
    let resp = reqwest::Client::new()
        .get("https://api.deepseek.com/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(Duration::from_secs(8))
        .send()
        .await;
    let latency = Some(t0.elapsed().as_millis() as u64);
    match resp {
        Ok(r) if r.status().is_success() => (CheckStatus::Ok, latency),
        Ok(r) if r.status().as_u16() == 401 => {
            (CheckStatus::Error("DeepSeek API Key 无效 (401)".to_string()), latency)
        }
        Ok(r) => (CheckStatus::Warning(format!("DeepSeek API 返回 HTTP {}", r.status())), latency),
        Err(e) => (CheckStatus::Warning(format!("DeepSeek API 不可达: {}", e)), latency),
    }
}

pub async fn check_anthropic_api(api_key: &str) -> (CheckStatus, Option<u64>) {
    if api_key.is_empty() {
        return (CheckStatus::Skipped, None);
    }
    let t0 = Instant::now();
    // Anthropic uses x-api-key header
    let resp = reqwest::Client::new()
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .timeout(Duration::from_secs(8))
        .send()
        .await;
    let latency = Some(t0.elapsed().as_millis() as u64);
    match resp {
        Ok(r) if r.status().is_success() || r.status().as_u16() == 404 => {
            // 404 means auth passed but endpoint might differ — treat as ok
            (CheckStatus::Ok, latency)
        }
        Ok(r) if r.status().as_u16() == 401 => {
            (CheckStatus::Error("Anthropic API Key 无效 (401)".to_string()), latency)
        }
        Ok(r) => (CheckStatus::Warning(format!("Anthropic API 返回 HTTP {}", r.status())), latency),
        Err(e) => (CheckStatus::Warning(format!("Anthropic API 不可达: {}", e)), latency),
    }
}

pub async fn check_gateway(gateway_url: &str) -> (CheckStatus, Option<u64>) {
    if gateway_url.is_empty() {
        return (CheckStatus::Skipped, None);
    }
    let t0 = Instant::now();
    let url = format!("{}/health", gateway_url.trim_end_matches('/'));
    let ok = reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    let latency = Some(t0.elapsed().as_millis() as u64);
    if ok {
        (CheckStatus::Ok, latency)
    } else {
        (CheckStatus::Warning("OpenClaw Gateway 不可达 (Gateway 功能不可用)".to_string()), latency)
    }
}

// ── Master run function (runs all checks, sends results back one by one) ──────

/// Result of a single check step, sent via AppMessage.
#[derive(Debug, Clone)]
pub struct EnvCheckStepResult {
    pub id: &'static str,
    pub status: CheckStatus,
    pub latency_ms: Option<u64>,
    pub detail: Option<String>,
    /// Whether Ollama was auto-started during this step.
    pub ollama_started: bool,
}

/// Run all checks sequentially and return results via a callback-friendly future.
/// Returns a Vec of step results in order.
pub async fn run_all_checks(params: EnvCheckParams) -> Vec<EnvCheckStepResult> {
    let mut results = Vec::new();

    // 1. Ollama service
    let (status, latency, ollama_started) = check_ollama(&params.ollama_endpoint).await;
    let ollama_ok = status.is_ok();
    results.push(EnvCheckStepResult {
        id: "ollama_running",
        status,
        latency_ms: latency,
        detail: if ollama_started { Some("Ollama 已自动启动".to_string()) } else { None },
        ollama_started,
    });

    // 2. Ollama model (only if Ollama is up)
    if ollama_ok {
        let (status, latency) = check_ollama_model(&params.ollama_endpoint, &params.ollama_model).await;
        results.push(EnvCheckStepResult {
            id: "ollama_model",
            status,
            latency_ms: latency,
            detail: None,
            ollama_started: false,
        });
    } else {
        results.push(EnvCheckStepResult {
            id: "ollama_model",
            status: CheckStatus::Skipped,
            latency_ms: None,
            detail: Some("Ollama 未就绪，跳过模型检测".to_string()),
            ollama_started: false,
        });
    }

    // 3. WasmEdge
    let (status, latency) = check_wasmedge().await;
    results.push(EnvCheckStepResult { id: "wasmedge", status, latency_ms: latency, detail: None, ollama_started: false });

    // 4. Disk space
    let (status, latency) = check_disk_space(&params.workspace_dir).await;
    results.push(EnvCheckStepResult { id: "disk_space", status, latency_ms: latency, detail: None, ollama_started: false });

    // 5. Internet
    let (status, latency) = check_internet().await;
    results.push(EnvCheckStepResult { id: "internet", status, latency_ms: latency, detail: None, ollama_started: false });

    // 6-8. External APIs (run concurrently)
    let openai_key  = params.openai_api_key.clone().unwrap_or_default();
    let deepseek_key = params.deepseek_api_key.clone().unwrap_or_default();
    let anthropic_key = params.anthropic_api_key.clone().unwrap_or_default();

    let (r_openai, r_deepseek, r_anthropic) = tokio::join!(
        check_openai_api(&openai_key),
        check_deepseek_api(&deepseek_key),
        check_anthropic_api(&anthropic_key),
    );

    results.push(EnvCheckStepResult { id: "openai_api", status: r_openai.0, latency_ms: r_openai.1, detail: None, ollama_started: false });
    results.push(EnvCheckStepResult { id: "deepseek_api", status: r_deepseek.0, latency_ms: r_deepseek.1, detail: None, ollama_started: false });
    results.push(EnvCheckStepResult { id: "anthropic_api", status: r_anthropic.0, latency_ms: r_anthropic.1, detail: None, ollama_started: false });

    // 9. Gateway
    let gateway_url = params.gateway_url.clone().unwrap_or_default();
    let (status, latency) = check_gateway(&gateway_url).await;
    results.push(EnvCheckStepResult { id: "gateway", status, latency_ms: latency, detail: None, ollama_started: false });

    results
}
