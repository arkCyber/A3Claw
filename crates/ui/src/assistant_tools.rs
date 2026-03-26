//! Assistant tool calling system
//!
//! Provides diagnostic and maintenance tools for the OpenClaw+ Assistant.

use serde::{Deserialize, Serialize};
use std::process::Command;

/// Detect which tool to execute based on user input keywords
pub fn detect_tool_trigger(input: &str) -> Option<String> {
    let input_lower = input.to_lowercase();
    
    // Update config
    if (input_lower.contains("配置") || input_lower.contains("config")) &&
       (input_lower.contains("修改") || input_lower.contains("更新") ||
        input_lower.contains("设置") || input_lower.contains("update") ||
        input_lower.contains("modify") || input_lower.contains("change")) {
        return Some("update_config".to_string());
    }
    
    // Auto start system
    if (input_lower.contains("自动启动") || input_lower.contains("auto start") ||
        input_lower.contains("启动系统") || input_lower.contains("start system")) {
        return Some("auto_start_system".to_string());
    }
    
    // Enable health monitor
    if (input_lower.contains("启用") || input_lower.contains("enable") ||
        input_lower.contains("开启") || input_lower.contains("打开")) &&
       (input_lower.contains("监控") || input_lower.contains("monitor") ||
        input_lower.contains("定时检测") || input_lower.contains("health check")) {
        return Some("enable_health_monitor".to_string());
    }
    
    // Disable health monitor
    if (input_lower.contains("禁用") || input_lower.contains("disable") ||
        input_lower.contains("关闭") || input_lower.contains("停止")) &&
       (input_lower.contains("监控") || input_lower.contains("monitor") ||
        input_lower.contains("定时检测") || input_lower.contains("health check")) {
        return Some("disable_health_monitor".to_string());
    }
    
    // Check Ollama health
    if input_lower.contains("ollama") && 
       (input_lower.contains("检查") || input_lower.contains("状态") || 
        input_lower.contains("check") || input_lower.contains("status") ||
        input_lower.contains("无法连接") || input_lower.contains("连接不上") ||
        input_lower.contains("cannot connect") || input_lower.contains("not working")) {
        return Some("check_ollama_health".to_string());
    }
    
    // Start Ollama
    if input_lower.contains("ollama") && 
       (input_lower.contains("启动") || input_lower.contains("start") ||
        input_lower.contains("开启") || input_lower.contains("运行")) {
        return Some("start_ollama_service".to_string());
    }
    
    // Check config
    if (input_lower.contains("配置") || input_lower.contains("config")) &&
       (input_lower.contains("检查") || input_lower.contains("check") ||
        input_lower.contains("问题") || input_lower.contains("错误") ||
        input_lower.contains("issue") || input_lower.contains("error")) {
        return Some("check_config".to_string());
    }
    
    // System status
    if (input_lower.contains("系统") || input_lower.contains("system")) &&
       (input_lower.contains("状态") || input_lower.contains("status") ||
        input_lower.contains("健康") || input_lower.contains("health") ||
        input_lower.contains("检查") || input_lower.contains("check")) {
        return Some("get_system_status".to_string());
    }
    
    // Provide guide
    if input_lower.contains("如何") || input_lower.contains("怎么") ||
       input_lower.contains("指南") || input_lower.contains("教程") ||
       input_lower.contains("how to") || input_lower.contains("guide") ||
       input_lower.contains("tutorial") || input_lower.contains("help me") {
        return Some("provide_guide".to_string());
    }
    
    None
}

/// Tool call request from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Available tools for the Assistant
#[derive(Debug, Clone)]
pub enum AssistantTool {
    CheckOllamaHealth,
    StartOllamaService,
    CheckConfig,
    UpdateConfig,
    AutoStartSystem,
    GetSystemStatus,
    ProvideGuide,
    EnableHealthMonitor,
    DisableHealthMonitor,
}

impl AssistantTool {
    /// Get all available tools as JSON schema for system prompt
    pub fn get_tools_schema() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "check_ollama_health",
                    "description": "Check if Ollama service is running and accessible. Returns status, endpoint, and available models.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "start_ollama_service",
                    "description": "Attempt to start the Ollama service if it's not running.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "check_config",
                    "description": "Check OpenClaw+ configuration file for issues. Returns config path, validity, and any errors.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "config_key": {
                                "type": "string",
                                "description": "Specific config key to check (optional)"
                            }
                        }
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_system_status",
                    "description": "Get overall system health status including Ollama, config, and UI state.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "update_config",
                    "description": "Update OpenClaw+ configuration file with new values.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "key": {
                                "type": "string",
                                "description": "Configuration key to update (e.g., 'ollama_endpoint', 'ollama_model')"
                            },
                            "value": {
                                "type": "string",
                                "description": "New value for the configuration key"
                            }
                        },
                        "required": ["key", "value"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "auto_start_system",
                    "description": "Automatically start all required system components (Ollama service, etc.).",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "enable_health_monitor",
                    "description": "Enable periodic health monitoring (checks system status every N minutes).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "interval_minutes": {
                                "type": "number",
                                "description": "Check interval in minutes (default: 5)"
                            }
                        }
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "disable_health_monitor",
                    "description": "Disable periodic health monitoring.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "provide_guide",
                    "description": "Provide step-by-step guide for a specific operation.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "topic": {
                                "type": "string",
                                "description": "The topic to provide guidance on (e.g., 'setup_ollama', 'configure_ai_chat', 'use_assistant')"
                            }
                        },
                        "required": ["topic"]
                    }
                }
            }),
        ]
    }

    /// Execute a tool call
    pub async fn execute(tool_call: &ToolCall) -> ToolResult {
        let result = match tool_call.name.as_str() {
            "check_ollama_health" => Self::check_ollama_health().await,
            "start_ollama_service" => Self::start_ollama_service().await,
            "check_config" => Self::check_config(&tool_call.arguments).await,
            "update_config" => Self::update_config(&tool_call.arguments).await,
            "auto_start_system" => Self::auto_start_system().await,
            "get_system_status" => Self::get_system_status().await,
            "provide_guide" => Self::provide_guide(&tool_call.arguments).await,
            "enable_health_monitor" => Self::enable_health_monitor(&tool_call.arguments).await,
            "disable_health_monitor" => Self::disable_health_monitor().await,
            _ => Err(format!("Unknown tool: {}", tool_call.name)),
        };

        match result {
            Ok(output) => ToolResult {
                tool_call_id: tool_call.id.clone(),
                success: true,
                output,
                error: None,
            },
            Err(error) => ToolResult {
                tool_call_id: tool_call.id.clone(),
                success: false,
                output: String::new(),
                error: Some(error),
            },
        }
    }

    /// Check Ollama service health
    async fn check_ollama_health() -> Result<String, String> {
        let endpoint = "http://localhost:11434";
        
        // Try to connect to Ollama
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        match client.get(format!("{}/api/tags", endpoint)).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let models = json["models"].as_array()
                                .map(|arr| arr.len())
                                .unwrap_or(0);
                            Ok(format!(
                                "✅ Ollama 服务运行正常\n\
                                 端点: {}\n\
                                 状态: 在线\n\
                                 已安装模型: {} 个",
                                endpoint, models
                            ))
                        }
                        Err(e) => Ok(format!(
                            "⚠️ Ollama 服务响应异常\n\
                             端点: {}\n\
                             错误: 无法解析模型列表 ({})",
                            endpoint, e
                        )),
                    }
                } else {
                    Ok(format!(
                        "⚠️ Ollama 服务响应异常\n\
                         端点: {}\n\
                         HTTP 状态: {}",
                        endpoint, resp.status()
                    ))
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    Ok(format!(
                        "❌ Ollama 服务无响应\n\
                         端点: {}\n\
                         错误: 连接超时\n\
                         建议: 请检查 Ollama 是否已启动",
                        endpoint
                    ))
                } else if e.is_connect() {
                    Ok(format!(
                        "❌ Ollama 服务未启动\n\
                         端点: {}\n\
                         错误: 无法连接\n\
                         建议: 运行 'ollama serve' 启动服务",
                        endpoint
                    ))
                } else {
                    Ok(format!(
                        "❌ Ollama 服务检查失败\n\
                         端点: {}\n\
                         错误: {}",
                        endpoint, e
                    ))
                }
            }
        }
    }

    /// Start Ollama service
    async fn start_ollama_service() -> Result<String, String> {
        // Try to start Ollama in background
        match Command::new("ollama")
            .arg("serve")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => {
                // Wait a bit for service to start
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                
                // Verify it started
                match Self::check_ollama_health().await {
                    Ok(status) if status.contains("运行正常") => {
                        Ok("✅ Ollama 服务已成功启动".to_string())
                    }
                    Ok(status) => {
                        Ok(format!("⚠️ Ollama 服务启动中，请稍候...\n\n{}", status))
                    }
                    Err(e) => {
                        Ok(format!("⚠️ Ollama 服务启动失败\n错误: {}", e))
                    }
                }
            }
            Err(e) => {
                Ok(format!(
                    "❌ 无法启动 Ollama 服务\n\
                     错误: {}\n\
                     建议: 请手动运行 'ollama serve' 或检查 Ollama 是否已安装",
                    e
                ))
            }
        }
    }

    /// Check configuration
    async fn check_config(args: &serde_json::Value) -> Result<String, String> {
        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("openclaw-plus")
            .join("config.toml");

        if !config_path.exists() {
            return Ok(format!(
                "⚠️ 配置文件不存在\n\
                 路径: {}\n\
                 建议: 首次运行时会自动创建默认配置",
                config_path.display()
            ));
        }

        match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                match toml::from_str::<toml::Value>(&content) {
                    Ok(config) => {
                        let config_key = args.get("config_key")
                            .and_then(|v| v.as_str());
                        
                        if let Some(key) = config_key {
                            if let Some(value) = config.get(key) {
                                Ok(format!(
                                    "✅ 配置项 '{}' 存在\n\
                                     值: {:?}",
                                    key, value
                                ))
                            } else {
                                Ok(format!(
                                    "⚠️ 配置项 '{}' 不存在\n\
                                     可用配置项: {:?}",
                                    key, config.as_table().map(|t| t.keys().collect::<Vec<_>>())
                                ))
                            }
                        } else {
                            Ok(format!(
                                "✅ 配置文件有效\n\
                                 路径: {}\n\
                                 配置项数量: {}",
                                config_path.display(),
                                config.as_table().map(|t| t.len()).unwrap_or(0)
                            ))
                        }
                    }
                    Err(e) => {
                        Ok(format!(
                            "❌ 配置文件格式错误\n\
                             路径: {}\n\
                             错误: {}\n\
                             建议: 检查 TOML 语法",
                            config_path.display(), e
                        ))
                    }
                }
            }
            Err(e) => {
                Ok(format!(
                    "❌ 无法读取配置文件\n\
                     路径: {}\n\
                     错误: {}",
                    config_path.display(), e
                ))
            }
        }
    }

    /// Get overall system status
    async fn get_system_status() -> Result<String, String> {
        let mut status = String::from("📊 OpenClaw+ 系统状态\n\n");

        // Check Ollama
        status.push_str("【Ollama 服务】\n");
        match Self::check_ollama_health().await {
            Ok(health) => status.push_str(&format!("{}\n\n", health)),
            Err(e) => status.push_str(&format!("错误: {}\n\n", e)),
        }

        // Check config
        status.push_str("【配置文件】\n");
        match Self::check_config(&serde_json::json!({})).await {
            Ok(config) => status.push_str(&format!("{}\n\n", config)),
            Err(e) => status.push_str(&format!("错误: {}\n\n", e)),
        }

        // UI status
        status.push_str("【UI 状态】\n");
        status.push_str("✅ UI 运行正常\n");
        status.push_str(&format!("进程 ID: {}\n", std::process::id()));

        Ok(status)
    }

    /// Provide operation guide
    async fn provide_guide(args: &serde_json::Value) -> Result<String, String> {
        let topic = args.get("topic")
            .and_then(|v| v.as_str())
            .ok_or("缺少 topic 参数")?;

        let guide = match topic {
            "setup_ollama" => {
                "📖 Ollama 安装和配置指南\n\n\
                 1️⃣ 安装 Ollama\n\
                    macOS: brew install ollama\n\
                    Linux: curl https://ollama.ai/install.sh | sh\n\n\
                 2️⃣ 启动 Ollama 服务\n\
                    ollama serve\n\n\
                 3️⃣ 下载模型\n\
                    ollama pull qwen3.5:9b\n\
                    ollama pull llama3.2\n\n\
                 4️⃣ 验证安装\n\
                    ollama list\n\
                    curl http://localhost:11434/api/tags\n\n\
                 5️⃣ 在 OpenClaw+ 中使用\n\
                    - 打开 AI Chat 或 Assistant 页面\n\
                    - 模型会自动检测\n\
                    - 开始对话"
            }
            "configure_ai_chat" => {
                "📖 AI Chat 配置指南\n\n\
                 1️⃣ 导航到 AI Chat 页面\n\
                    点击侧边栏 'AI Chat' 标签\n\n\
                 2️⃣ 检查模型选择器\n\
                    应自动显示已安装的模型\n\
                    如: qwen3.5:9b, llama3.2\n\n\
                 3️⃣ 配置端点（可选）\n\
                    默认: http://localhost:11434\n\
                    可在设置中修改\n\n\
                 4️⃣ 开始对话\n\
                    输入消息并按 Enter\n\
                    支持中文和英文\n\n\
                 5️⃣ 常见问题\n\
                    - 模型列表为空: 检查 Ollama 服务\n\
                    - 无响应: 检查网络连接\n\
                    - Fallback 回复: 重启 UI"
            }
            "use_assistant" => {
                "📖 Assistant 使用指南\n\n\
                 1️⃣ 导航到 Assistant 页面\n\
                    点击侧边栏 'Assistant' 标签\n\n\
                 2️⃣ 功能介绍\n\
                    - 系统诊断和维护\n\
                    - WasmEdge 运行时支持\n\
                    - RAG 知识库配置\n\
                    - 安全审计\n\n\
                 3️⃣ 快捷操作\n\
                    - Diagnose: 诊断系统问题\n\
                    - Optimize: 性能优化建议\n\
                    - Audit: 安全审计\n\
                    - RAG: 知识库管理\n\n\
                 4️⃣ 配置 Assistant\n\
                    点击设置图标 ⚙️\n\
                    - 修改 Ollama 端点\n\
                    - 选择模型\n\
                    - 调整参数\n\n\
                 5️⃣ 工具调用\n\
                    Assistant 可以:\n\
                    - 检查 Ollama 状态\n\
                    - 启动服务\n\
                    - 检查配置\n\
                    - 提供操作指南"
            }
            "troubleshooting" => {
                "📖 故障排除指南\n\n\
                 【问题 1: Ollama 无法连接】\n\
                 ✓ 检查服务: ollama serve\n\
                 ✓ 检查端口: curl http://localhost:11434\n\
                 ✓ 重启服务: pkill ollama && ollama serve\n\n\
                 【问题 2: 模型列表为空】\n\
                 ✓ 检查模型: ollama list\n\
                 ✓ 下载模型: ollama pull qwen3.5:9b\n\
                 ✓ 刷新 UI: 切换页面\n\n\
                 【问题 3: AI 返回 Fallback】\n\
                 ✓ 检查模型名称是否正确\n\
                 ✓ 重启 UI\n\
                 ✓ 查看日志\n\n\
                 【问题 4: 配置文件错误】\n\
                 ✓ 路径: ~/Library/Application Support/openclaw-plus/config.toml\n\
                 ✓ 验证 TOML 语法\n\
                 ✓ 删除并重新生成\n\n\
                 【问题 5: UI 无响应】\n\
                 ✓ 重启应用\n\
                 ✓ 检查日志\n\
                 ✓ 清空缓存"
            }
            _ => {
                return Ok(format!(
                    "⚠️ 未知主题: {}\n\n\
                     可用主题:\n\
                     - setup_ollama: Ollama 安装配置\n\
                     - configure_ai_chat: AI Chat 配置\n\
                     - use_assistant: Assistant 使用\n\
                     - troubleshooting: 故障排除",
                    topic
                ));
            }
        };

        Ok(guide.to_string())
    }

    /// Update configuration file
    async fn update_config(args: &serde_json::Value) -> Result<String, String> {
        let key = args.get("key")
            .and_then(|v| v.as_str())
            .ok_or("缺少 key 参数")?;
        let value = args.get("value")
            .and_then(|v| v.as_str())
            .ok_or("缺少 value 参数")?;

        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("openclaw-plus")
            .join("config.toml");

        // Create config dir if not exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("无法创建配置目录: {}", e))?;
        }

        // Read existing config or create new
        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("无法读取配置文件: {}", e))?;
            toml::from_str::<toml::Value>(&content)
                .map_err(|e| format!("配置文件格式错误: {}", e))?
        } else {
            toml::Value::Table(toml::map::Map::new())
        };

        // Update the key
        if let Some(table) = config.as_table_mut() {
            table.insert(key.to_string(), toml::Value::String(value.to_string()));
        }

        // Write back to file
        let content = toml::to_string_pretty(&config)
            .map_err(|e| format!("无法序列化配置: {}", e))?;
        std::fs::write(&config_path, content)
            .map_err(|e| format!("无法写入配置文件: {}", e))?;

        Ok(format!(
            "✅ 配置已更新\n\
             路径: {}\n\
             配置项: {} = {}",
            config_path.display(), key, value
        ))
    }

    /// Auto start all system components
    async fn auto_start_system() -> Result<String, String> {
        let mut status = String::from("🚀 正在启动系统组件...\n\n");

        // 1. Check and start Ollama
        status.push_str("【1/2】Ollama 服务\n");
        match Self::check_ollama_health().await {
            Ok(health) if health.contains("运行正常") => {
                status.push_str("✅ 已在运行\n\n");
            }
            _ => {
                status.push_str("⏳ 正在启动...\n");
                match Self::start_ollama_service().await {
                    Ok(result) => {
                        status.push_str(&format!("{}\n\n", result));
                    }
                    Err(e) => {
                        status.push_str(&format!("❌ 启动失败: {}\n\n", e));
                    }
                }
            }
        }

        // 2. Verify configuration
        status.push_str("【2/2】配置文件\n");
        match Self::check_config(&serde_json::json!({})).await {
            Ok(config) if config.contains("有效") => {
                status.push_str("✅ 配置正常\n\n");
            }
            Ok(config) => {
                status.push_str(&format!("⚠️ {}\n\n", config));
            }
            Err(e) => {
                status.push_str(&format!("❌ 配置检查失败: {}\n\n", e));
            }
        }

        status.push_str("━━━━━━━━━━━━━━━━━━━━━━\n");
        status.push_str("✅ 系统启动流程完成！\n\n");
        status.push_str("建议操作：\n");
        status.push_str("1. 前往 AI Chat 页面测试对话\n");
        status.push_str("2. 检查模型列表是否显示\n");
        status.push_str("3. 如有问题，查看详细日志");

        Ok(status)
    }

    /// Enable health monitoring
    async fn enable_health_monitor(args: &serde_json::Value) -> Result<String, String> {
        let interval = args.get("interval_minutes")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        // Store monitor config in a file
        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("openclaw-plus")
            .join("health_monitor.toml");

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("无法创建配置目录: {}", e))?;
        }

        let monitor_config = toml::toml! {
            enabled = true
            interval_minutes = interval
            last_check = 0
        };

        let content = toml::to_string_pretty(&monitor_config)
            .map_err(|e| format!("无法序列化监控配置: {}", e))?;
        std::fs::write(&config_path, content)
            .map_err(|e| format!("无法写入监控配置: {}", e))?;

        Ok(format!(
            "✅ 健康监控已启用\n\n\
             检测间隔: {} 分钟\n\
             监控内容:\n\
             - Ollama 服务状态\n\
             - 配置文件有效性\n\
             - UI 运行状态\n\n\
             配置文件: {}\n\n\
             注意: 监控将在下次 UI 启动时生效",
            interval, config_path.display()
        ))
    }

    /// Disable health monitoring
    async fn disable_health_monitor() -> Result<String, String> {
        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("openclaw-plus")
            .join("health_monitor.toml");

        if config_path.exists() {
            std::fs::remove_file(&config_path)
                .map_err(|e| format!("无法删除监控配置: {}", e))?;
            Ok(format!(
                "✅ 健康监控已禁用\n\n\
                 配置文件已删除: {}\n\n\
                 监控将在下次 UI 启动时停止",
                config_path.display()
            ))
        } else {
            Ok("ℹ️ 健康监控未启用\n\n无需禁用".to_string())
        }
    }
}
