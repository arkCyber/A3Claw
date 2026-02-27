//! # openclaw-agent-executor
//!
//! The execution engine that makes Digital Workers truly work.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                   AgentExecutor                          │
//! │                                                         │
//! │  ┌──────────────┐   ┌─────────────┐   ┌─────────────┐  │
//! │  │  TaskContext  │──▶│  ReActLoop  │──▶│ SkillDispatch│  │
//! │  └──────────────┘   └──────┬──────┘   └──────┬──────┘  │
//! │                            │                  │          │
//! │                     LLM Tool Call         Gateway HTTP   │
//! │                     (plan/observe)        /hooks/before  │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`context`]  — Per-agent task context: goal, memory, history, step log
//! - [`skill`]    — Canonical Skill definitions aligned with SkillRegistry
//! - [`dispatch`] — HTTP dispatch to Gateway + response handling
//! - [`react`]    — ReAct reasoning loop (Thought → Action → Observation)
//! - [`executor`] — Top-level executor: spawns loops, tracks state, reports progress
//! - [`bootstrap`]— OpenClaw workspace bootstrap: plugin-config.json, entry scripts
//! - [`session`]  — Per-agent session registry (maps session_id → AgentProfile)
//! - [`error`]    — Unified error type

pub mod bootstrap;
pub mod context;
pub mod dispatch;
pub mod error;
pub mod executor;
pub mod react;
pub mod session;
pub mod skill;

pub use executor::{AgentExecutor, ExecutorConfig, ExecutorEvent, ExecutorHandle};
pub use context::{TaskContext, TaskGoal, StepRecord, MemoryStore};
pub use skill::{Skill, SkillCategory, SkillSet, BUILTIN_SKILLS};
pub use session::{SessionRegistry, AgentSession, SessionId};
pub use error::ExecutorError;
