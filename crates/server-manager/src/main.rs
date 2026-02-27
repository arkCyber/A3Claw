//! 服务器管理器主程序

use openclaw_server_manager::{ServerManager, ServerConfig, api};
use openclaw_server_manager::types::ServerType;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🚀 OpenClaw+ Server Manager Starting...");
    println!();

    // 创建服务器管理器
    let manager = Arc::new(ServerManager::new());

    // 添加预配置的服务器
    add_default_servers(&manager).await?;

    // 创建 API 路由
    let api_router = api::create_router(manager.clone());

    // 创建完整的应用路由
    let app = axum::Router::new()
        .nest("/", api_router)
        .nest_service("/ui", ServeDir::new("crates/server-manager/ui"))
        .layer(CorsLayer::permissive());

    // 启动后台任务：定期更新资源使用情况
    let manager_clone = manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            manager_clone.update_resource_usage().await;
        }
    });

    // 启动 HTTP 服务器
    let addr = "127.0.0.1:9000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    println!("✅ Server Manager API running on http://{}", addr);
    println!("📊 Web UI available at http://{}/ui", addr);
    println!();
    println!("API Endpoints:");
    println!("  GET    /api/servers           - List all servers");
    println!("  GET    /api/servers/:id       - Get server info");
    println!("  POST   /api/servers/:id/start - Start server");
    println!("  POST   /api/servers/:id/stop  - Stop server");
    println!("  POST   /api/servers/:id/restart - Restart server");
    println!("  GET    /api/servers/:id/health - Health check");
    println!();

    axum::serve(listener, app).await?;

    Ok(())
}

/// 添加默认服务器配置
async fn add_default_servers(manager: &ServerManager) -> anyhow::Result<()> {
    // Ollama 主服务
    manager.add_server(ServerConfig {
        id: "ollama-primary".to_string(),
        name: "Ollama (Primary)".to_string(),
        server_type: ServerType::Ollama,
        endpoint: "http://localhost:11434".to_string(),
        port: 11434,
        model_path: None,
        auto_start: false,
        enabled: true,
    }).await?;

    // llama.cpp 备份服务
    let model_path = std::env::current_dir()?
        .join("models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf")
        .to_string_lossy()
        .to_string();

    manager.add_server(ServerConfig {
        id: "llama-cpp-backup".to_string(),
        name: "llama.cpp (Backup)".to_string(),
        server_type: ServerType::LlamaCpp,
        endpoint: "http://localhost:8080".to_string(),
        port: 8080,
        model_path: Some(model_path),
        auto_start: true,
        enabled: true,
    }).await?;

    // OpenAI (云端)
    manager.add_server(ServerConfig {
        id: "openai-cloud".to_string(),
        name: "OpenAI (Cloud)".to_string(),
        server_type: ServerType::OpenAI,
        endpoint: "https://api.openai.com/v1".to_string(),
        port: 443,
        model_path: None,
        auto_start: false,
        enabled: false,
    }).await?;

    // DeepSeek (云端)
    manager.add_server(ServerConfig {
        id: "deepseek-cloud".to_string(),
        name: "DeepSeek (Cloud)".to_string(),
        server_type: ServerType::DeepSeek,
        endpoint: "https://api.deepseek.com/v1".to_string(),
        port: 443,
        model_path: None,
        auto_start: false,
        enabled: false,
    }).await?;

    println!("✅ Added {} default server configurations", 4);

    Ok(())
}
