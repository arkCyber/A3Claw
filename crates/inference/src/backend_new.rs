//! Backend implementations: HTTP (llama.cpp / Ollama / OpenAI-compat) and WASI-NN stub.

use crate::error::InferenceError;
use crate::types::{BackendKind, ConversationTurn, InferenceConfig, InferenceResponse, StreamToken};
use futures_util::StreamExt;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{d