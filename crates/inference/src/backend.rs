//! Backend implementations: HTTP (llama.cpp / Ollama / OpenAI-compat) and WASI-NN stub.

use crate::error::InferenceError;
use crate::types::{BackendKind, ConversationTurn, InferenceConfig, InferenceResponse, StreamToken};
use futures_util::StreamExt;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info};

// ── HTTP backend (llama.cpp / Ollama / OpenAI-compat) ─────────────────────────

#[derive(Clone)]
pub struct HttpBackend {
    client: reqwest::Client,
    config: InferenceConfig,
}

impl HttpBackend {
    pub fn new(config: InferenceConfig) -> Result<Self, InferenceError> {
        let client = reqwest::Client::builder()
            .timeout(config.inference_timeout)
            .build()?;
        Ok(Self { client, config })
    }

    fn chat_url(&self) -> String {
        match self.config.backend {
            BackendKind::Ollama => format!("{}/api/chat", self.config.endpoint),
            _ => format!("{}/v1/chat/completions", self.config.endpoint),
        }
    }

    fn build_body(
        &self,
        messages: &[ConversationTurn],
        max_tokens: u32,
        temperature: f32,
        stream: bool,
    ) -> serde_json::Value {
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
            .collect();
        serde_json::json!({
            "model":       self.config.model_name,
            "messages":    msgs,
            "max_tokens":  max_tokens,
            "temperature": temperature,
            "top_p":       self.config.top_p,
            "stream":      stream,
        })
    }

    /// Non-streaming inference — returns the full response text.
    pub async fn infer(
        &self,
        request_id: u64,
        messages: &[ConversationTurn],
        max_tokens: u32,
        temperature: f32,
    ) -> Result<InferenceResponse, InferenceError> {
        let url = self.chat_url();
        let body = self.build_body(messages, max_tokens, temperature, false);
        let t0 = Instant::now();
        debug!(request_id, url = %url, "sending inference request");

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req.send().await?;
        let status = resp.status();
        let latency_ms = t0.elapsed().as_millis() as u64;

        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(InferenceError::HttpError {
                backend: self.config.backend.to_string(),
                status: status.as_u16(),
                body: body_text,
            });
        }

        let json: serde_json::Value = resp.json().await?;

        // Support both OpenAI shape and Ollama shape.
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| json["message"]["content"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                tracing::error!(
                    request_id,
                    backend = %self.config.backend,
                    response_keys = ?json.as_object().map(|o| o.keys().collect::<Vec<_>>()),
                    "backend response missing content field"
                );
                InferenceError::StreamParse(
                    format!("missing content field in response: {}", &json.to_string()[..json.to_string().len().min(200)])
                )
            })?;

        let prompt_tokens     = json["usage"]["prompt_tokens"].as_u64().map(|v| v as u32);
        let completion_tokens = json["usage"]["completion_tokens"].as_u64().map(|v| v as u32);

        info!(request_id, latency_ms, backend = %self.config.backend, "inference completed");

        Ok(InferenceResponse {
            request_id,
            content,
            prompt_tokens,
            completion_tokens,
            latency_ms,
            served_by: self.config.backend.clone(),
        })
    }

    /// Streaming inference via SSE (OpenAI) or NDJSON (Ollama).
    pub async fn infer_stream(
        &self,
        request_id: u64,
        messages: &[ConversationTurn],
        max_tokens: u32,
        temperature: f32,
        tx: mpsc::Sender<StreamToken>,
    ) -> Result<(), InferenceError> {
        let url = self.chat_url();
        let body = self.build_body(messages, max_tokens, temperature, true);
        debug!(request_id, url = %url, "sending streaming inference request");

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(InferenceError::HttpError {
                backend: self.config.backend.to_string(),
                status: status.as_u16(),
                body: body_text,
            });
        }

        let mut stream = resp.bytes_stream();
        let is_ollama = matches!(self.config.backend, BackendKind::Ollama);

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            let text = String::from_utf8_lossy(&bytes);

            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }

                // OpenAI SSE: "data: {...}" or "data: [DONE]"
                let json_str = if line.starts_with("data: ") {
                    let payload = &line["data: ".len()..];
                    if payload == "[DONE]" {
                        let _ = tx.send(StreamToken {
                            request_id, delta: String::new(), done: true,
                        }).await;
                        return Ok(());
                    }
                    payload
                } else if is_ollama {
                    // Ollama NDJSON: one JSON object per line
                    line
                } else {
                    continue;
                };

                let json: serde_json::Value = serde_json::from_str(json_str)
                    .map_err(|e| InferenceError::StreamParse(e.to_string()))?;

                let delta = json["choices"][0]["delta"]["content"]
                    .as_str()
                    .or_else(|| json["message"]["content"].as_str())
                    .unwrap_or("")
                    .to_string();

                let done = json["done"].as_bool().unwrap_or(false)
                    || json["choices"][0]["finish_reason"].as_str() == Some("stop");

                if !delta.is_empty() || done {
                    if tx.send(StreamToken { request_id, delta, done }).await.is_err() {
                        break;
                    }
                }
                if done { return Ok(()); }
            }
        }

        // Stream ended without explicit done marker.
        let _ = tx.send(StreamToken { request_id, delta: String::new(), done: true }).await;
        Ok(())
    }
}

// ── WASI-NN backend ───────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct WasiNnBackend {
    config: InferenceConfig,
}

impl WasiNnBackend {
    pub fn new(config: InferenceConfig) -> Self { Self { config } }

    /// In-process WASI-NN inference via WasmEdge ggml plugin.
    /// Only compiled when the `wasi-nn` feature is enabled.
    #[cfg(feature = "wasi-nn")]
    pub async fn infer(
        &self,
        request_id: u64,
        messages: &[ConversationTurn],
        max_tokens: u32,
        temperature: f32,
    ) -> Result<InferenceResponse, InferenceError> {
        let model_path = self.config.model_path.as_ref()
            .ok_or_else(|| InferenceError::ModelNotFound { path: "<none>".into() })?;

        if !model_path.exists() {
            return Err(InferenceError::ModelNotFound {
                path: model_path.display().to_string(),
            });
        }

        // SHA-256 integrity check before loading the model.
        if let Some(expected) = &self.config.model_sha256 {
            let actual = compute_sha256(model_path)?;
            if &actual != expected {
                return Err(InferenceError::IntegrityFailure {
                    expected: expected.clone(),
                    actual,
                });
            }
        }

        let t0 = Instant::now();
        info!(request_id, model = %model_path.display(), "WASI-NN inference started");

        let prompt = build_chat_prompt(messages);

        // Execute via WasmEdge WASI-NN ggml graph.
        let content = wasmedge_wasi_nn_infer(
            model_path,
            &prompt,
            max_tokens,
            temperature,
            self.config.top_p,
        ).await.map_err(|e| InferenceError::WasiNn(e.to_string()))?;

        let latency_ms = t0.elapsed().as_millis() as u64;
        info!(request_id, latency_ms, "WASI-NN inference completed");

        Ok(InferenceResponse {
            request_id,
            content,
            prompt_tokens: None,
            completion_tokens: None,
            latency_ms,
            served_by: BackendKind::WasiNn,
        })
    }

    /// Stub when `wasi-nn` feature is not enabled.
    #[cfg(not(feature = "wasi-nn"))]
    pub async fn infer(
        &self,
        _request_id: u64,
        _messages: &[ConversationTurn],
        _max_tokens: u32,
        _temperature: f32,
    ) -> Result<InferenceResponse, InferenceError> {
        Err(InferenceError::WasiNn(
            "WasmEdge WASI-NN not compiled in. \
             Rebuild with --features wasi-nn or use the LlamaCppHttp backend.".into(),
        ))
    }
}

// ── SHA-256 integrity helper ──────────────────────────────────────────────────

#[cfg(feature = "wasi-nn")]
fn compute_sha256(path: &std::path::Path) -> Result<String, InferenceError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}
