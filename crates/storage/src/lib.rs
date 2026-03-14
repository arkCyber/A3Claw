//! # openclaw-storage
//!
//! 企业数字员工平台的持久化存储层。
//!
//! ## 模块结构
//! - [`db`]：SQLite 数据库连接与 schema 初始化
//! - [`agent_store`]：数字员工档案的 CRUD 操作
//! - [`run_store`]：任务执行记录（Task/Run/Step）的存储与查询
//! - [`audit_store`]：不可变审计事件流的写入与查询
//! - [`types`]：存储层专用类型（Run、Step、AuditEvent 等）

pub mod db;
pub mod agent_store;
pub mod run_store;
pub mod audit_store;
pub mod agent_manager;
pub mod types;

pub use db::Database;
pub use agent_store::AgentStore;
pub use run_store::RunStore;
pub use audit_store::AuditStore;
pub use agent_manager::{AgentManager, PlatformSummary};
pub use types::{
    RunRecord, RunStatus, StepRecord, StepKind,
    AuditEventRecord, AuditDecision,
};
