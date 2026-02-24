//! Core data types for the inference engine.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Backend selection ─────────────────────────────────────────────────────────

/// Which inference backend to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    /// WasmEdge WASI-NN with llama.cpp plugin (in-process, sandboxed).
    /// Requires WasmEdge runtime + ggml/llama plugin installed.
    #[default]
    WasiNn,
    /// llama.cpp HTTP server (out-of-process, OpenAI-compatible API).
    LlamaCppHttp,
    /// Ollama local server (http://localhost:11434).
    Ollama,
    /// Any OpenAI-compatible HTTP endpoint.
    OpenAiCompat,
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendKind::WasiNn       => write!(f, "WasmEdge WASI-NN"),
            BackendKind::LlamaCppHttp => write!(f, "llama.cpp HTTP"),
            BackendKind::Ollama       => write!(f, "Ollama"),
            BackendKind::OpenAiCompat => write!(f, "OpenAI-compat"),
        }
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Full configuration for the inference engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Which backend to use.
    pub backend: BackendKind,

    // ── WASI-NN specific ──────────────────────────────────────────────────────
    /// Path to the GGUF model file (used by WasiNn backend).
    pub model_path: Option<std::path::PathBuf>,
    /// Expected SHA-256 hex digest of the model file (integrity check).
    pub model_sha256: Option<String>,

    // ── HTTP backend specific ─────────────────────────────────────────────────
    /// Base URL of the inference server (LlamaCppHttp / Ollama / OpenAiCompat).
    pub endpoint: String,
    /// Model name / tag to request from the server.
    pub model_name: String,
    /// Optional Bearer token for authenticated endpoints.
    pub api_key: Option<String>,

    // ── Generation parameters ─────────────────────────────────────────────────
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: f32,
    /// Top-p nucleus sampling threshold.
    pub top_p: f32,

    // ── Reliability / safety ──────────────────────────────────────────────────
    /// Hard timeout for a single inference call.
    pub inference_timeout: Duration,
    /// Number of consecutive failures before the circuit breaker opens.
    pub circuit_breaker_threshold: u32,
    /// How long the circuit breaker stays open before probing again.
    pub circuit_breaker_reset: Duration,
    /// Maximum number of tokens in the context window.
    pub context_window: u32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            backend:                    BackendKind::LlamaCppHttp,
            model_path:                 None,
            model_sha256:               None,
            endpoint:                   "http://localhost:8080".into(),
            model_name:                 "llama3".into(),
            api_key:                    None,
            max_tokens:                 2048,
            temperature:                0.7,
            top_p:                      0.95,
            inference_timeout:          Duration::from_secs(120),
            circuit_breaker_threshold:  3,
            circuit_breaker_reset:      Duration::from_secs(30),
            context_window:             4096,
        }
    }
}

// ── Request / Response ────────────────────────────────────────────────────────

/// A single message in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// `"system"` | `"user"` | `"assistant"`
    pub role: String,
    pub content: String,
}

/// A complete inference request with full audit metadata.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    /// Unique monotonic request ID for audit tracing.
    pub request_id: u64,
    /// Conversation history including the new user message.
    pub messages: Vec<ConversationTurn>,
    /// Per-request override for max tokens (None = use config default).
    pub max_tokens_override: Option<u32>,
    /// Per-request temperature override.
    pub temperature_override: Option<f32>,
    /// Whether to stream tokens back incrementally.
    pub stream: bool,
}

/// A completed inference response.
#[derive(Debug, Clone)]
pub struct InferenceResponse {
    /// Mirrors the request ID for correlation.
    pub request_id: u64,
    /// The generated text content.
    pub content: String,
    /// Number of prompt tokens consumed.
    pub prompt_tokens: Option<u32>,
    /// Number of completion tokens generated.
    pub completion_tokens: Option<u32>,
    /// Wall-clock latency of the inference call.
    pub latency_ms: u64,
    /// Which backend actually served this response.
    pub served_by: BackendKind,
}

/// A single streaming token chunk.
#[derive(Debug, Clone)]
pub struct StreamToken {
    /// Mirrors the request ID.
    pub request_id: u64,
    /// The token text fragment.
    pub delta: String,
    /// True if this is the final token (stream complete).
    pub done: bool,
}

// ── Model metadata ────────────────────────────────────────────────────────────

/// Metadata about a loaded / available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub backend: BackendKind,
    pub context_window: u32,
    pub parameter_count_b: Option<f32>,
    pub quantization: Option<String>,
    pub sha256_verified: bool,
}
