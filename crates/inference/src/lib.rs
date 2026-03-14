//! # `openclaw-inference` — Aerospace-Grade Local AI Inference Engine
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Design Principles (Aerospace / DO-178C inspired)
//!
//! | Principle | Implementation |
//! |---|---|
//! | **Isolation** | Every inference request runs in a bounded async task with hard timeout |
//! | **Fault containment** | Circuit breaker per backend; failures do not cascade |
//! | **Determinism** | Request IDs, sequence numbers, full audit trail |
//! | **Integrity** | SHA-256 model file verification before loading |
//! | **Observability** | Structured tracing on every state transition |
//! | **Graceful degradation** | Fallback chain: WASI-NN → llama.cpp HTTP → error |
//!
//! ## Architecture
//!
//! ```text
//!  ┌─────────────────────────────────────────────────────────┐
//!  │                   InferenceEngine                        │
//!  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
//!  │  │ CircuitBreaker│  │  AuditLog    │  │HealthMonitor │  │
//!  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
//!  │         │                 │                  │          │
//!  │  ┌──────▼───────────────────────────────────▼───────┐  │
//!  │  │              BackendRouter                        │  │
//!  │  │  ┌─────────────────┐   ┌──────────────────────┐  │  │
//!  │  │  │  WasiNnBackend  │   │  LlamaCppBackend      │  │  │
//!  │  │  │  (WasmEdge)     │   │  (HTTP SSE streaming) │  │  │
//!  │  │  └─────────────────┘   └──────────────────────┘  │  │
//!  │  └───────────────────────────────────────────────────┘  │
//!  └─────────────────────────────────────────────────────────┘
//! ```

pub mod audit;
pub mod backend;
pub mod circuit_breaker;
pub mod engine;
pub mod error;
pub mod health;
pub mod types;

#[cfg(test)]
mod tests;

pub use engine::InferenceEngine;
pub use error::InferenceError;
pub use types::{
    BackendKind, InferenceConfig, InferenceRequest, InferenceResponse,
    ModelInfo, StreamToken,
};
