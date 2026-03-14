//! Server Manager - 管理和控制推理服务器
//!
//! 提供 API 和 UI 来管理多个推理后端服务器

pub mod api;
pub mod manager;
pub mod types;

pub use manager::ServerManager;
pub use types::{ServerConfig, ServerStatus, ServerInfo};
