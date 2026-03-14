# WASI-NN Backend Integration Guide

## Overview

OpenClaw+ integrates **WasmEdge WASI-NN** for in-process AI inference with llama.cpp and GGUF models. This provides:

- ✅ **Zero external dependencies**: No HTTP servers required
- ✅ **Sandboxed execution**: WasmEdge VM isolation
- ✅ **Aerospace-grade reliability**: SHA-256 integrity checks, circuit breakers, comprehensive error handling
- ✅ **Native performance**: Direct llama.cpp integration via WASI-NN plugin
- ✅ **GGUF model support**: Quantized models (Q4_K_M, Q5_K_M, etc.)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ OpenClaw+ UI (Claw Terminal)                                │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ InferenceEngine (crates/inference)                          │
│  ├─ WasiNnBackend::infer()                                  │
│  └─ wasmedge_wasi_nn_infer()                                │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ WasmEdge Runtime (0.16.1)                                   │
│  ├─ WASI module (sandboxed I/O)                             │
│  ├─ wasi_nn plugin (GGML backend)                           │
│  └─ Embedded WASM module (openclaw-wasi-nn-infer)          │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ llama.cpp (via WASI-NN host functions)                      │
│  └─ GGUF model inference                                    │
└─────────────────────────────────────────────────────────────┘
```

## Prerequisites

### 1. Install WasmEdge with wasi_nn-ggml Plugin

```bash
# Install WasmEdge 0.16.1+ with wasi_nn-ggml plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh \
  | bash -s -- --plugins wasi_nn-ggml

# Verify installation
wasmedge --version
# Expected: wasmedge version 0.16.1 or higher

# Check plugin
ls ~/.wasmedge/plugin/
# Expected: libwasmedgePluginWasiNN.dylib (macOS) or libwasmedgePluginWasiNN.so (Linux)
```

### 2. Install Rust wasm32-wasip1 Target

```bash
rustup target add wasm32-wasip1
```

### 3. Download a GGUF Model

```bash
# Create models directory
mkdir -p models/gguf

# Download Qwen2.5-0.5B-Instruct (Q4_K_M quantization, ~400MB)
curl -L -o models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf"

# Verify download
ls -lh models/gguf/
```

## Building with WASI-NN Support

```bash
# Build inference crate with wasi-nn feature
cargo build --release -p openclaw-inference --features wasi-nn

# Build entire workspace (optional)
cargo build --release --features wasi-nn
```

The build process automatically:
1. Compiles `crates/wasi-nn-infer` to `wasm32-wasip1` target
2. Embeds the WASM binary into `openclaw-inference` via `include_bytes!`
3. Links against `wasmedge-sdk` 0.14 with `wasi_nn` feature

## Configuration

### InferenceConfig for WASI-NN

```rust
use openclaw_inference::{InferenceConfig, BackendKind};
use std::path::PathBuf;
use std::time::Duration;

let config = InferenceConfig {
    backend: BackendKind::WasiNn,
    
    // WASI-NN specific
    model_path: Some(PathBuf::from("models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf")),
    model_sha256: None, // Optional: "abc123..." for integrity check
    
    // HTTP backend fields (unused for WASI-NN)
    endpoint: "http://localhost:8080".into(),
    model_name: "qwen2.5-0.5b-instruct".into(),
    api_key: None,
    
    // Generation parameters
    max_tokens: 512,
    temperature: 0.7,
    top_p: 0.9,
    
    // Reliability
    inference_timeout: Duration::from_secs(120),
    circuit_breaker_threshold: 3,
    circuit_breaker_reset: Duration::from_secs(30),
    context_window: 8192, // Adjust based on model
};
```

### Environment Variables

```bash
# Optional: Override WasmEdge plugin path
export WASMEDGE_PLUGIN_PATH=/custom/path/to/plugins

# Optional: Enable WasmEdge debug logging
export WASMEDGE_LOG_LEVEL=debug
```

## Usage

### Programmatic API

```rust
use openclaw_inference::{InferenceEngine, InferenceRequest};
use openclaw_inference::types::ConversationTurn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create engine with WASI-NN config
    let engine = InferenceEngine::new(config)?;
    
    // Build request
    let request = InferenceRequest {
        request_id: 1,
        messages: vec![
            ConversationTurn {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            ConversationTurn {
                role: "user".to_string(),
                content: "What is Rust?".to_string(),
            },
        ],
        max_tokens_override: Some(256),
        temperature_override: Some(0.7),
        stream: false,
    };
    
    // Run inference
    let response = engine.infer(request).await?;
    println!("Response: {}", response.content);
    
    Ok(())
}
```

### UI Configuration

1. **Open Settings** in OpenClaw+ UI
2. **AI Provider** → Select "WASI-NN" (if available) or configure via config file
3. **Model Path** → Browse to `models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf`
4. **Save** and restart if needed

## Testing

### Run Integration Tests

```bash
# Run all WASI-NN integration tests
cargo test --features wasi-nn --test wasi_nn_integration

# Expected output:
# test test_wasi_nn_plugin_detection ... ok
# test test_wasi_nn_missing_model_error ... ok
# test test_wasi_io_pair_sandbox ... ok
# test test_sha256_integrity_check ... ok
# test test_json_response_parser ... ok
# test test_build_wasi_nn_prompt ... ok
# test result: ok. 6 passed; 0 failed
```

### End-to-End Test Script

```bash
# Run comprehensive E2E test
./scripts/test_wasi_nn_e2e.sh

# This script verifies:
# - WasmEdge installation
# - wasi_nn plugin availability
# - GGUF model file
# - Build with wasi-nn feature
# - Integration tests
```

## Performance Tuning

### Context Window

Edit `crates/inference/src/backend.rs`, line ~381:

```rust
let json_payload = serde_json::json!({
    "model":        "default",
    "prompt":       prompt,
    "n_predict":    max_tokens,
    "temperature":  temperature,
    "top_p":        top_p,
    "ctx_size":     8192,  // ← Increase for longer context (default: 4096)
    "n_gpu_layers": 0,     // ← Set >0 for GPU acceleration (requires CUDA/Metal)
});
```

### GPU Acceleration

For models with GPU support:

```rust
"n_gpu_layers": 33,  // Offload 33 layers to GPU (adjust based on VRAM)
```

**Note**: Requires WasmEdge built with CUDA or Metal support.

### Model Selection

| Model | Size | Quantization | Use Case |
|-------|------|--------------|----------|
| Qwen2.5-0.5B | ~400MB | Q4_K_M | Fast, low-resource |
| Qwen2.5-1.5B | ~1.2GB | Q4_K_M | Balanced |
| Llama-3.2-3B | ~2.5GB | Q5_K_M | High quality |
| Qwen2.5-7B | ~5GB | Q4_K_M | Production |

Download from:
- **Qwen**: https://huggingface.co/Qwen
- **Llama**: https://huggingface.co/meta-llama
- **Mistral**: https://huggingface.co/mistralai

## Troubleshooting

### Error: "wasi_nn plugin not found"

```bash
# Reinstall WasmEdge with plugin
bash <(curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh) \
  -- --plugins wasi_nn-ggml

# Verify plugin
ls ~/.wasmedge/plugin/libwasmedgePluginWasiNN.*
```

### Error: "ModelNotFound"

Check:
1. Model file path is correct
2. File exists: `ls -lh models/gguf/your-model.gguf`
3. Permissions: `chmod 644 models/gguf/*.gguf`

### Error: "WASM module load failed"

Rebuild with clean state:

```bash
cargo clean
cargo build --release -p openclaw-inference --features wasi-nn
```

### Slow Inference

1. **Use smaller model**: Q4_K_M instead of Q8_0
2. **Reduce context**: `ctx_size: 2048`
3. **Enable GPU**: `n_gpu_layers: 33`
4. **Reduce max_tokens**: `n_predict: 256`

## Security Considerations

### Sandboxing

- ✅ WASM module runs in WasmEdge VM sandbox
- ✅ File I/O restricted to preopened temp directory
- ✅ No network access from WASM module
- ✅ Automatic cleanup via RAII (`Drop` trait)

### Integrity Verification

Enable SHA-256 model verification:

```rust
model_sha256: Some("abc123def456...".to_string()),
```

Generate hash:

```bash
shasum -a 256 models/gguf/your-model.gguf
```

### Resource Limits

Configure in `InferenceConfig`:

```rust
inference_timeout: Duration::from_secs(60),  // Hard timeout
circuit_breaker_threshold: 3,                // Fail after 3 errors
context_window: 4096,                        // Max context tokens
```

## Advanced Topics

### Custom WASM Inference Module

The embedded WASM module source is at `crates/wasi-nn-infer/src/main.rs`. To customize:

1. Edit `main.rs` (e.g., add preprocessing, custom sampling)
2. Rebuild: `cargo build --release -p openclaw-inference --features wasi-nn`
3. The build script auto-compiles to WASM and embeds it

### Multiple Models

Load different models by changing `model_path` in config. The WASI-NN plugin supports dynamic model loading via `nn_preload`.

### Streaming Inference

Currently not supported. The WASM module returns complete responses. For streaming, use the HTTP backends (Ollama, LlamaCppHttp).

## References

- **WasmEdge**: https://wasmedge.org/
- **WASI-NN Spec**: https://github.com/WebAssembly/wasi-nn
- **llama.cpp**: https://github.com/ggerganov/llama.cpp
- **GGUF Format**: https://github.com/ggerganov/ggml/blob/master/docs/gguf.md

## License

WASI-NN integration code is licensed under MIT, same as OpenClaw+.
