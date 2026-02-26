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
        match self.config.backend {
            BackendKind::Ollama => serde_json::json!({
                "model":       self.config.model_name,
                "messages":    msgs,
                "stream":      stream,
                "options": {
                    "num_predict": max_tokens,
                    "temperature": temperature,
                    "top_p":       self.config.top_p,
                }
            }),
            _ => serde_json::json!({
                "model":       self.config.model_name,
                "messages":    msgs,
                "max_tokens":  max_tokens,
                "temperature": temperature,
                "top_p":       self.config.top_p,
                "stream":      stream,
            }),
        }
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

        let prompt = build_wasi_nn_prompt(messages);

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

// ── WasmEdge WASI-NN inference helper ────────────────────────────────────────

/// Embedded WASM inference module compiled from crates/wasi-nn-infer at build time.
#[cfg(feature = "wasi-nn")]
static WASI_NN_INFER_WASM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/wasi_nn_infer.wasm"));

/// In-process llama.cpp inference via WasmEdge WASI-NN plugin.
///
/// Pipeline:
///   1. Load wasi_nn plugin from WasmEdge default paths.
///   2. Register GGUF model with alias "default" via nn_preload.
///   3. Write JSON payload to a sandboxed temp dir (WasiIoPair).
///   4. Build WasmEdge VM: WASI module + wasi_nn plugin module.
///   5. Run embedded WASM module (_start reads req, writes resp).
///   6. Parse and return the JSON response text.
///
/// All CPU-bound work runs in spawn_blocking to keep the async executor free.
#[cfg(feature = "wasi-nn")]
async fn wasmedge_wasi_nn_infer(
    model_path: &std::path::Path,
    prompt: &str,
    max_tokens: u32,
    temperature: f32,
    top_p: f32,
) -> Result<String, anyhow::Error> {
    use std::collections::HashMap;
    use wasmedge_sdk::{
        params,
        plugin::{ExecutionTarget, GraphEncoding, NNPreload, PluginManager},
        vm::SyncInst,
        wasi::WasiModule,
        Module, Store, Vm,
    };

    let model_path = model_path.to_path_buf();
    let prompt     = prompt.to_string();

    tokio::task::spawn_blocking(move || -> Result<String, anyhow::Error> {
        // 1. Load plugins; verify wasi_nn is available.
        PluginManager::load(None)?;
        let plugin_names = PluginManager::names();
        if !plugin_names.iter().any(|n| n.contains("wasi_nn")) {
            return Err(anyhow::anyhow!(
                "wasi_nn plugin not found. \
                 Install WasmEdge with: bash <(curl -sSf \
                 https://raw.githubusercontent.com/WasmEdge/WasmEdge/refs/heads/master/utils/install.sh) \
                 -- --plugins wasi_nn-ggml"
            ));
        }

        // 2. Register GGUF model under alias "default".
        PluginManager::nn_preload(vec![NNPreload::new(
            "default",
            GraphEncoding::GGML,
            ExecutionTarget::AUTO,
            &model_path,
        )]);

        // 3. Build JSON payload.
        let json_payload = serde_json::json!({
            "model":        "default",
            "prompt":       prompt,
            "n_predict":    max_tokens,
            "temperature":  temperature,
            "top_p":        top_p,
            "ctx_size":     4096,
            "n_gpu_layers": 0,
        }).to_string();

        // 4. Create sandboxed temp dir.
        let io = WasiIoPair::new(&json_payload)?;

        // 5. Configure WASI module.
        //    envs: "KEY=VALUE" strings.
        //    preopens: "GUEST:HOST" mapping (guest "." → host temp dir).
        let req_name  = io.req_path.file_name()
            .and_then(|n| n.to_str()).unwrap_or("request.json");
        let resp_name = io.resp_path.file_name()
            .and_then(|n| n.to_str()).unwrap_or("response.json");
        let env_req   = format!("OPENCLAW_REQ={}",  req_name);
        let env_resp  = format!("OPENCLAW_RESP={}", resp_name);
        let preopen   = format!(".:{}", io.dir.to_str().unwrap_or("/tmp"));

        let mut wasi_mod = WasiModule::create(
            Some(vec!["openclaw_wasi_nn_infer"]),
            Some(vec![env_req.as_str(), env_resp.as_str()]),
            Some(vec![preopen.as_str()]),
        )?;

        // 6. Create wasi_nn plugin instance.
        let mut wasi_nn_inst = PluginManager::create_plugin_instance(
            "wasi_nn", "wasi_nn",
        ).map_err(|e| anyhow::anyhow!("wasi_nn plugin instance failed: {:?}", e))?;

        // 7. Assemble VM: WasiModule.as_mut() → &mut Instance which is SyncInst.
        let mut instances: HashMap<String, &mut dyn SyncInst> = HashMap::new();
        instances.insert(wasi_mod.name().to_string(), wasi_mod.as_mut());
        instances.insert("wasi_nn".to_string(), &mut wasi_nn_inst);

        let mut vm = Vm::new(
            Store::new(None, instances)
                .map_err(|e| anyhow::anyhow!("Store::new failed: {:?}", e))?,
        );

        // 8. Load embedded WASM module and run _start.
        let wasm_mod = Module::from_bytes(None, WASI_NN_INFER_WASM)
            .map_err(|e| anyhow::anyhow!("WASM module load failed: {:?}", e))?;
        vm.register_module(None, wasm_mod)
            .map_err(|e| anyhow::anyhow!("register_module failed: {:?}", e))?;
        vm.run_func(None, "_start", params!())
            .map_err(|e| anyhow::anyhow!("_start failed: {:?}", e))?;

        // 9. Read and parse JSON response.
        let raw = std::fs::read_to_string(&io.resp_path)
            .map_err(|e| anyhow::anyhow!("read response failed: {}", e))?;
        parse_wasi_nn_response(&raw)
    })
    .await?
}

// ── Sandboxed temp I/O dir ────────────────────────────────────────────────────

/// Temp directory pair for sandboxed WASM file exchange: req.json in, resp.json out.
#[cfg(feature = "wasi-nn")]
struct WasiIoPair {
    dir:       std::path::PathBuf,
    req_path:  std::path::PathBuf,
    resp_path: std::path::PathBuf,
}

#[cfg(feature = "wasi-nn")]
impl WasiIoPair {
    fn new(request_json: &str) -> Result<Self, anyhow::Error> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir()
            .join(format!("openclaw_wasi_nn_{}_{}", std::process::id(), ts));
        std::fs::create_dir_all(&dir)?;
        let req_path  = dir.join("request.json");
        let resp_path = dir.join("response.json");
        std::fs::write(&req_path, request_json.as_bytes())?;
        std::fs::File::create(&resp_path)?;
        Ok(Self { dir, req_path, resp_path })
    }
}

#[cfg(feature = "wasi-nn")]
impl Drop for WasiIoPair {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// ── Response parser ───────────────────────────────────────────────────────────

/// Parse the WASM module output: `{"ok":true,"text":"..."}`.
#[cfg(feature = "wasi-nn")]
fn parse_wasi_nn_response(raw: &str) -> Result<String, anyhow::Error> {
    let v: serde_json::Value = serde_json::from_str(raw.trim())
        .map_err(|e| anyhow::anyhow!(
            "invalid JSON from WASM module: {} — raw snippet: {:?}",
            e, &raw[..raw.len().min(200)]
        ))?;
    if v["ok"].as_bool() == Some(true) {
        Ok(v["text"].as_str().unwrap_or("").to_string())
    } else {
        let err = v["error"].as_str().unwrap_or("unknown error");
        Err(anyhow::anyhow!("WASI-NN inference error: {}", err))
    }
}

// ── ChatML prompt builder ─────────────────────────────────────────────────────

/// Build a ChatML-format prompt string from a conversation history.
#[cfg(feature = "wasi-nn")]
fn build_wasi_nn_prompt(messages: &[ConversationTurn]) -> String {
    let mut out = String::new();
    for m in messages {
        out.push_str(&m.content);
        out.push('\n');
    }
    out
}
