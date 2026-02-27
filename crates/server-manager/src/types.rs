//! 服务器管理类型定义

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// 服务器类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerType {
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "llama_cpp")]
    LlamaCpp,
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "custom")]
    Custom,
}

/// 服务器状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Error,
    Unknown,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub server_type: ServerType,
    pub endpoint: String,
    pub port: u16,
    pub model_path: Option<String>,
    pub auto_start: bool,
    pub enabled: bool,
}

/// 服务器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub config: ServerConfig,
    pub status: ServerStatus,
    pub pid: Option<u32>,
    pub uptime_seconds: Option<u64>,
    pub last_health_check: Option<SystemTime>,
    pub health_check_success: bool,
    pub error_message: Option<String>,
    pub cpu_usage: Option<f32>,
    pub memory_mb: Option<u64>,
}

/// 服务器操作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAction {
    pub server_id: String,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Start,
    Stop,
    Restart,
    HealthCheck,
}

/// 服务器操作响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerActionResponse {
    pub success: bool,
    pub message: String,
    pub server_info: Option<ServerInfo>,
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub server_id: String,
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}
