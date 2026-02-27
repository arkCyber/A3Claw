//! 服务器管理器核心实现

use crate::types::*;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::SystemTime;
use sysinfo::{Pid, System};
use tokio::sync::RwLock;

/// 服务器管理器
pub struct ServerManager {
    servers: Arc<RwLock<HashMap<String, ServerInfo>>>,
    processes: Arc<RwLock<HashMap<String, Child>>>,
    system: Arc<RwLock<System>>,
}

impl ServerManager {
    /// 创建新的服务器管理器
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(System::new_all())),
        }
    }

    /// 添加服务器配置
    pub async fn add_server(&self, config: ServerConfig) -> Result<()> {
        let mut servers = self.servers.write().await;
        
        let info = ServerInfo {
            config: config.clone(),
            status: ServerStatus::Stopped,
            pid: None,
            uptime_seconds: None,
            last_health_check: None,
            health_check_success: false,
            error_message: None,
            cpu_usage: None,
            memory_mb: None,
        };
        
        servers.insert(config.id.clone(), info);
        Ok(())
    }

    /// 获取所有服务器信息
    pub async fn list_servers(&self) -> Vec<ServerInfo> {
        // 注意：部分服务器（例如 Ollama）可能由外部启动，无法通过 PID 跟踪。
        // 为了让 list/status/UI 刷新结果可信，这里对这类服务做一次轻量可达性探测。
        let snapshot: Vec<(String, String, ServerType)> = {
            let servers = self.servers.read().await;
            servers
                .values()
                .map(|s| (s.config.id.clone(), s.config.endpoint.clone(), s.config.server_type.clone()))
                .collect()
        };

        for (server_id, endpoint, server_type) in snapshot {
            if server_type != ServerType::Ollama {
                continue;
            }

            let reachable = Self::probe_http_health(&endpoint, &server_type).await;

            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(&server_id) {
                match reachable {
                    Ok(()) => {
                        server.status = ServerStatus::Running;
                        server.error_message = None;
                    }
                    Err(e) => {
                        // 对于外部服务，无法区分“未启动”和“启动但不可达”，这里统一标记为 Stopped 并带错误。
                        server.status = ServerStatus::Stopped;
                        server.error_message = Some(e.to_string());
                    }
                }
            }
        }

        let servers = self.servers.read().await;
        servers
            .values()
            .filter(|s| s.config.enabled)
            .cloned()
            .collect()
    }

    async fn probe_http_health(endpoint: &str, server_type: &ServerType) -> Result<()> {
        let url = match server_type {
            ServerType::Ollama => format!("{}/api/tags", endpoint),
            ServerType::LlamaCpp => format!("{}/health", endpoint),
            ServerType::OpenAI | ServerType::DeepSeek => format!("{}/models", endpoint),
            ServerType::Custom => format!("{}/health", endpoint),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(800))
            .build()?;

        let resp = client.get(url).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("HTTP {}", resp.status()))
        }
    }

    /// 获取单个服务器信息
    pub async fn get_server(&self, server_id: &str) -> Option<ServerInfo> {
        let servers = self.servers.read().await;
        servers.get(server_id).cloned()
    }

    /// 启动服务器
    pub async fn start_server(&self, server_id: &str) -> Result<ServerInfo> {
        let mut servers = self.servers.write().await;
        let server = servers.get_mut(server_id)
            .ok_or_else(|| anyhow!("Server not found: {}", server_id))?;

        if server.status == ServerStatus::Running {
            return Ok(server.clone());
        }

        server.status = ServerStatus::Starting;
        let config = server.config.clone();
        drop(servers);

        // 根据服务器类型启动
        let child = match config.server_type {
            ServerType::LlamaCpp => {
                self.start_llama_cpp_server(&config).await?
            }
            ServerType::Ollama => {
                return Err(anyhow!("Ollama should be started externally"));
            }
            _ => {
                return Err(anyhow!("Unsupported server type for auto-start"));
            }
        };

        let pid = child.id();
        
        // 保存进程
        let mut processes = self.processes.write().await;
        processes.insert(server_id.to_string(), child);
        drop(processes);

        // 更新服务器状态
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.status = ServerStatus::Running;
            server.pid = Some(pid);
            server.error_message = None;
        }

        Ok(servers.get(server_id).unwrap().clone())
    }

    /// 停止服务器
    pub async fn stop_server(&self, server_id: &str) -> Result<ServerInfo> {
        let mut servers = self.servers.write().await;
        let server = servers.get_mut(server_id)
            .ok_or_else(|| anyhow!("Server not found: {}", server_id))?;

        if server.status == ServerStatus::Stopped {
            return Ok(server.clone());
        }

        server.status = ServerStatus::Stopping;
        drop(servers);

        // 终止进程
        let mut processes = self.processes.write().await;
        if let Some(mut child) = processes.remove(server_id) {
            let _ = child.kill();
            let _ = child.wait();
        }
        drop(processes);

        // 更新状态
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.status = ServerStatus::Stopped;
            server.pid = None;
            server.uptime_seconds = None;
        }

        Ok(servers.get(server_id).unwrap().clone())
    }

    /// 重启服务器
    pub async fn restart_server(&self, server_id: &str) -> Result<ServerInfo> {
        self.stop_server(server_id).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        self.start_server(server_id).await
    }

    /// 健康检查
    pub async fn health_check(&self, server_id: &str) -> Result<HealthCheckResult> {
        let (endpoint, server_type) = {
            let servers = self.servers.read().await;
            let server = servers.get(server_id)
                .ok_or_else(|| anyhow!("Server not found: {}", server_id))?;
            (server.config.endpoint.clone(), server.config.server_type.clone())
        };

        let start = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        // 根据服务器类型选择正确的健康检查端点
        let health_url = match server_type {
            ServerType::Ollama => format!("{}/api/tags", endpoint),
            ServerType::LlamaCpp => format!("{}/health", endpoint),
            ServerType::OpenAI | ServerType::DeepSeek => format!("{}/models", endpoint),
            ServerType::Custom => format!("{}/health", endpoint),
        };

        let result = match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency_ms = start.elapsed().as_millis() as u64;
                HealthCheckResult {
                    server_id: server_id.to_string(),
                    healthy: true,
                    latency_ms: Some(latency_ms),
                    error: None,
                }
            }
            Ok(resp) => {
                HealthCheckResult {
                    server_id: server_id.to_string(),
                    healthy: false,
                    latency_ms: None,
                    error: Some(format!("HTTP {}", resp.status())),
                }
            }
            Err(e) => {
                HealthCheckResult {
                    server_id: server_id.to_string(),
                    healthy: false,
                    latency_ms: None,
                    error: Some(e.to_string()),
                }
            }
        };

        // 更新服务器信息
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.last_health_check = Some(SystemTime::now());
            server.health_check_success = result.healthy;
            if !result.healthy {
                server.error_message = result.error.clone();
            }
        }

        Ok(result)
    }

    /// 更新服务器资源使用情况
    pub async fn update_resource_usage(&self) {
        let mut system = self.system.write().await;
        system.refresh_all();

        let mut servers = self.servers.write().await;
        for server in servers.values_mut() {
            if let Some(pid) = server.pid {
                let pid_obj = Pid::from_u32(pid);
                if let Some(process) = system.process(pid_obj) {
                    server.cpu_usage = Some(process.cpu_usage());
                    server.memory_mb = Some(process.memory() / 1024 / 1024);
                }
            }
        }
    }

    /// 启动 llama.cpp 服务器
    async fn start_llama_cpp_server(&self, config: &ServerConfig) -> Result<Child> {
        let model_path = config.model_path.as_ref()
            .ok_or_else(|| anyhow!("Model path required for llama.cpp server"))?;

        let script_path = std::env::current_dir()?
            .join("scripts/start_llama_cpp_server.sh");

        let child = Command::new(script_path)
            .arg(model_path)
            .arg(config.port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}
