//! Configuration Management for OpenClaw+
//! 
//! This crate provides structured configuration management for all OpenClaw+ components,
//! separated by domain for better maintainability and modularity.

pub mod rag;

// Re-export main configuration types
pub use rag::{RagConfig, RagFolder, RagFile, RagSettings, IndexingStatus};
