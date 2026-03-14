//! 服务器控制命令行工具
//! 用于从 OpenClaw+ UI 或命令行管理推理服务器

use openclaw_server_manager::{ServerConfig, ServerManager};
use openclaw_server_manager::types::ServerType;
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    let manager = Arc::new(ServerManager::new());
    
    // 加载配置
    load_servers_from_config(&manager).await?;

    match command.as_str() {
        "list" | "ls" => {
            let json_output = args.get(2).map(|s| s.as_str()) == Some("--json");
            list_servers(&manager, json_output).await?;
        }
        "status" => {
            if args.len() < 3 {
                eprintln!("Usage: server-ctl status <server-id>");
                return Ok(());
            }
            show_status(&manager, &args[2]).await?;
        }
        "start" => {
            if args.len() < 3 {
                eprintln!("Usage: server-ctl start <server-id>");
                return Ok(());
            }
            start_server(&manager, &args[2]).await?;
        }
        "stop" => {
            if args.len() < 3 {
                eprintln!("Usage: server-ctl stop <server-id>");
                return Ok(());
            }
            stop_server(&manager, &args[2]).await?;
        }
        "restart" => {
            if args.len() < 3 {
                eprintln!("Usage: server-ctl restart <server-id>");
                return Ok(());
            }
            restart_server(&manager, &args[2]).await?;
        }
        "health" => {
            if args.len() < 3 {
                eprintln!("Usage: server-ctl health <server-id>");
                return Ok(());
            }
            check_health(&manager, &args[2]).await?;
        }
        "start-all" => {
            start_all_servers(&manager).await?;
        }
        "stop-all" => {
            stop_all_servers(&manager).await?;
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("OpenClaw+ Server Manager");
    println!();
    println!("Usage: server-ctl <command> [args]");
    println!();
    println!("Commands:");
    println!("  list, ls              列出所有服务器");
    println!("  status <id>           显示服务器状态");
    println!("  start <id>            启动服务器");
    println!("  stop <id>             停止服务器");
    println!("  restart <id>          重启服务器");
    println!("  health <id>           健康检查");
    println!("  start-all             启动所有已启用的服务器");
    println!("  stop-all              停止所有服务器");
    println!();
    println!("Examples:");
    println!("  server-ctl list");
    println!("  server-ctl start llama-cpp-backup");
    println!("  server-ctl health ollama-primary");
}

async fn load_servers_from_config(manager: &ServerManager) -> anyhow::Result<()> {
    // 从 config/servers.toml 加载配置
    let config_path = std::env::current_dir()?
        .join("config/servers.toml");
    
    if !config_path.exists() {
        // 使用默认配置
        add_default_servers(manager).await?;
        return Ok(());
    }

    #[derive(serde::Deserialize)]
    struct ServersToml {
        #[serde(default)]
        servers: Vec<ServerTomlEntry>,
    }

    #[derive(serde::Deserialize)]
    struct ServerTomlEntry {
        id: String,
        name: String,
        #[serde(rename = "type")]
        server_type: ServerType,
        endpoint: String,
        port: u16,
        #[serde(default)]
        enabled: bool,
        #[serde(default)]
        auto_start: bool,
        #[serde(default)]
        model_path: Option<String>,
    }

    let content = fs::read_to_string(&config_path)?;
    let parsed: ServersToml = toml::from_str(&content)?;

    if parsed.servers.is_empty() {
        return Ok(());
    }

    for s in parsed.servers {
        if !s.enabled {
            continue;
        }
        manager
            .add_server(ServerConfig {
                id: s.id,
                name: s.name,
                server_type: s.server_type,
                endpoint: s.endpoint,
                port: s.port,
                model_path: s.model_path,
                auto_start: s.auto_start,
                enabled: s.enabled,
            })
            .await?;
    }
    
    Ok(())
}

async fn add_default_servers(manager: &ServerManager) -> anyhow::Result<()> {
    // Ollama
    manager.add_server(ServerConfig {
        id: "ollama-primary".to_string(),
        name: "Ollama (主服务)".to_string(),
        server_type: ServerType::Ollama,
        endpoint: "http://localhost:11434".to_string(),
        port: 11434,
        model_path: None,
        auto_start: false,
        enabled: true,
    }).await?;

    // llama.cpp
    let model_path = std::env::current_dir()?
        .join("models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf")
        .to_string_lossy()
        .to_string();

    manager.add_server(ServerConfig {
        id: "llama-cpp-backup".to_string(),
        name: "llama.cpp (备份)".to_string(),
        server_type: ServerType::LlamaCpp,
        endpoint: "http://localhost:8080".to_string(),
        port: 8080,
        model_path: Some(model_path),
        auto_start: true,
        enabled: true,
    }).await?;

    Ok(())
}

async fn list_servers(manager: &ServerManager, json_output: bool) -> anyhow::Result<()> {
    let servers = manager.list_servers().await;
    
    if json_output {
        // JSON 输出格式，供 UI 使用
        #[derive(serde::Serialize)]
        struct ServerJson {
            server_id: String,
            server_type: String,
            name: String,
            endpoint: String,
            status: String,
            pid: Option<u32>,
            cpu_usage: Option<f32>,
            memory_mb: Option<u64>,
        }
        
        let json_servers: Vec<ServerJson> = servers.iter().map(|s| {
            ServerJson {
                server_id: s.config.id.clone(),
                server_type: format!("{:?}", s.config.server_type),
                name: s.config.name.clone(),
                endpoint: s.config.endpoint.clone(),
                status: format!("{:?}", s.status),
                pid: s.pid,
                cpu_usage: s.cpu_usage,
                memory_mb: s.memory_mb,
            }
        }).collect();
        
        println!("{}", serde_json::to_string(&json_servers)?);
        return Ok(());
    }
    
    // 人类可读的输出格式
    println!("=== 推理服务器列表 ===");
    println!();
    
    if servers.is_empty() {
        println!("没有配置的服务器");
        return Ok(());
    }

    for server in servers {
        let status_icon = match server.status {
            openclaw_server_manager::types::ServerStatus::Running => "🟢",
            openclaw_server_manager::types::ServerStatus::Stopped => "🔴",
            openclaw_server_manager::types::ServerStatus::Starting => "🟡",
            openclaw_server_manager::types::ServerStatus::Stopping => "🟠",
            openclaw_server_manager::types::ServerStatus::Error => "❌",
            _ => "⚪",
        };

        println!("{} {} ({})", status_icon, server.config.name, server.config.id);
        println!("   类型: {:?}", server.config.server_type);
        println!("   端点: {}", server.config.endpoint);
        println!("   状态: {:?}", server.status);
        
        if let Some(pid) = server.pid {
            println!("   PID: {}", pid);
        }
        
        if let Some(cpu) = server.cpu_usage {
            println!("   CPU: {:.1}%", cpu);
        }
        
        if let Some(mem) = server.memory_mb {
            println!("   内存: {} MB", mem);
        }
        
        if server.health_check_success {
            println!("   健康: ✅");
        } else if server.error_message.is_some() {
            println!("   健康: ❌ {}", server.error_message.as_ref().unwrap());
        }
        
        println!();
    }

    Ok(())
}

async fn show_status(manager: &ServerManager, server_id: &str) -> anyhow::Result<()> {
    let server = manager.get_server(server_id).await
        .ok_or_else(|| anyhow::anyhow!("服务器不存在: {}", server_id))?;

    println!("=== {} ===", server.config.name);
    println!();
    println!("ID:       {}", server.config.id);
    println!("类型:     {:?}", server.config.server_type);
    println!("端点:     {}", server.config.endpoint);
    println!("端口:     {}", server.config.port);
    println!("状态:     {:?}", server.status);
    println!("已启用:   {}", if server.config.enabled { "是" } else { "否" });
    println!("自动启动: {}", if server.config.auto_start { "是" } else { "否" });
    
    if let Some(model_path) = &server.config.model_path {
        println!("模型路径: {}", model_path);
    }
    
    if let Some(pid) = server.pid {
        println!("进程 PID: {}", pid);
    }
    
    if let Some(cpu) = server.cpu_usage {
        println!("CPU 使用: {:.1}%", cpu);
    }
    
    if let Some(mem) = server.memory_mb {
        println!("内存使用: {} MB", mem);
    }
    
    if let Some(uptime) = server.uptime_seconds {
        println!("运行时间: {} 秒", uptime);
    }
    
    println!();

    Ok(())
}

async fn start_server(manager: &ServerManager, server_id: &str) -> anyhow::Result<()> {
    println!("正在启动服务器: {}", server_id);
    
    let info = manager.start_server(server_id).await?;
    
    println!("✅ 服务器已启动: {}", info.config.name);
    println!("   端点: {}", info.config.endpoint);
    if let Some(pid) = info.pid {
        println!("   PID: {}", pid);
    }
    
    Ok(())
}

async fn stop_server(manager: &ServerManager, server_id: &str) -> anyhow::Result<()> {
    println!("正在停止服务器: {}", server_id);
    
    let info = manager.stop_server(server_id).await?;
    
    println!("✅ 服务器已停止: {}", info.config.name);
    
    Ok(())
}

async fn restart_server(manager: &ServerManager, server_id: &str) -> anyhow::Result<()> {
    println!("正在重启服务器: {}", server_id);
    
    let info = manager.restart_server(server_id).await?;
    
    println!("✅ 服务器已重启: {}", info.config.name);
    if let Some(pid) = info.pid {
        println!("   新 PID: {}", pid);
    }
    
    Ok(())
}

async fn check_health(manager: &ServerManager, server_id: &str) -> anyhow::Result<()> {
    println!("正在检查服务器健康状态: {}", server_id);
    
    let result = manager.health_check(server_id).await?;
    
    if result.healthy {
        println!("✅ 服务器健康");
        if let Some(latency) = result.latency_ms {
            println!("   延迟: {} ms", latency);
        }
    } else {
        println!("❌ 服务器不健康");
        if let Some(error) = result.error {
            println!("   错误: {}", error);
        }
    }
    
    Ok(())
}

async fn start_all_servers(manager: &ServerManager) -> anyhow::Result<()> {
    println!("正在启动所有已启用的服务器...");
    println!();
    
    let servers = manager.list_servers().await;
    let mut started = 0;
    let mut skipped = 0;
    
    for server in servers {
        if !server.config.enabled {
            println!("⏭️  跳过 {} (未启用)", server.config.name);
            skipped += 1;
            continue;
        }
        
        if !server.config.auto_start {
            println!("⏭️  跳过 {} (不支持自动启动)", server.config.name);
            skipped += 1;
            continue;
        }
        
        match manager.start_server(&server.config.id).await {
            Ok(_) => {
                println!("✅ 已启动: {}", server.config.name);
                started += 1;
            }
            Err(e) => {
                println!("❌ 启动失败 {}: {}", server.config.name, e);
            }
        }
    }
    
    println!();
    println!("完成: {} 个已启动, {} 个跳过", started, skipped);
    
    Ok(())
}

async fn stop_all_servers(manager: &ServerManager) -> anyhow::Result<()> {
    println!("正在停止所有服务器...");
    println!();
    
    let servers = manager.list_servers().await;
    let mut stopped = 0;
    
    for server in servers {
        match manager.stop_server(&server.config.id).await {
            Ok(_) => {
                println!("✅ 已停止: {}", server.config.name);
                stopped += 1;
            }
            Err(e) => {
                println!("❌ 停止失败 {}: {}", server.config.name, e);
            }
        }
    }
    
    println!();
    println!("完成: {} 个已停止", stopped);
    
    Ok(())
}
