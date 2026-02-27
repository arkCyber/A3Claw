//! REST API 接口

use crate::manager::ServerManager;
use crate::types::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

/// API 状态
pub struct ApiState {
    pub manager: Arc<ServerManager>,
}

/// 创建 API 路由
pub fn create_router(manager: Arc<ServerManager>) -> Router {
    let state = Arc::new(ApiState { manager });

    Router::new()
        .route("/api/servers", get(list_servers))
        .route("/api/servers/:id", get(get_server))
        .route("/api/servers/:id/start", post(start_server))
        .route("/api/servers/:id/stop", post(stop_server))
        .route("/api/servers/:id/restart", post(restart_server))
        .route("/api/servers/:id/health", get(health_check))
        .with_state(state)
}

/// 列出所有服务器
async fn list_servers(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<ServerInfo>>, StatusCode> {
    let servers = state.manager.list_servers().await;
    Ok(Json(servers))
}

/// 获取单个服务器信息
async fn get_server(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<ServerInfo>, StatusCode> {
    state.manager
        .get_server(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// 启动服务器
async fn start_server(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.manager.start_server(&id).await {
        Ok(info) => (StatusCode::OK, Json(ServerActionResponse {
            success: true,
            message: format!("Server {} started", id),
            server_info: Some(info),
        })),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ServerActionResponse {
            success: false,
            message: e.to_string(),
            server_info: None,
        })),
    }
}

/// 停止服务器
async fn stop_server(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.manager.stop_server(&id).await {
        Ok(info) => (StatusCode::OK, Json(ServerActionResponse {
            success: true,
            message: format!("Server {} stopped", id),
            server_info: Some(info),
        })),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ServerActionResponse {
            success: false,
            message: e.to_string(),
            server_info: None,
        })),
    }
}

/// 重启服务器
async fn restart_server(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.manager.restart_server(&id).await {
        Ok(info) => (StatusCode::OK, Json(ServerActionResponse {
            success: true,
            message: format!("Server {} restarted", id),
            server_info: Some(info),
        })),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ServerActionResponse {
            success: false,
            message: e.to_string(),
            server_info: None,
        })),
    }
}

/// 健康检查
async fn health_check(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.manager.health_check(&id).await {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(HealthCheckResult {
            server_id: id,
            healthy: false,
            latency_ms: None,
            error: Some(e.to_string()),
        })),
    }
}
