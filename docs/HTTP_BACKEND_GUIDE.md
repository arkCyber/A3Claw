# HTTP Backend Quick Start Guide

This guide shows how to use OpenClaw+ with HTTP-based inference backends as an alternative to WASI-NN.

## Why Use HTTP Backend?

- **Workaround for Metal crash**: If you're on Apple Silicon and experiencing WASI-NN Metal crashes
- **Remote inference**: Run the model on a different machine with better hardware
- **Easier setup**: No need to rebuild WasmEdge plugins
- **Production deployment**: More suitable for distributed systems

## Supported Backends

1. **llama.cpp HTTP server** - Lightweight, OpenAI-compatible
2. **Ollama** - User-friendly local LLM server
3. **OpenAI-compatible APIs** - Any service following OpenAI's API spec

---

## Option 1: llama.cpp Server

### Installation

```bash
# Clone llama.cpp
git clone https://github.com/ggerganov/llama.cpp.git
cd llama.cpp

# Build (macOS)
make

# Or with Metal GPU support (Apple Silicon)
make LLAMA_METAL=1
```

### Start Server

```bash
# Basic usage
./llama-server -m /path/to/your/model.gguf -c 4096 --port 8080

# With more options
./llama-server \
  -m /path/to/your/model.gguf \
  -c 4096 \
  --port 8080 \
  --threads 8 \
  --n-gpu-layers 35  # GPU acceleration (if available)
```

### OpenClaw+ Configuration

```rust
use openclaw_inference::{InferenceConfig, InferenceEngine, BackendKind};
use std::path::PathBuf;

let config = InferenceConfig {
    backend: BackendKind::LlamaCppHttp,
    endpoint: "http://localhost:8080".into(),
    model_name: "qwen2.5-0.5b-instruct".into(),
    
    // HTTP-specific settings
    api_key: None,  // Not needed for local server
    inference_timeout: std::time::Duration::from_secs(120),
    
    // Model parameters
    max_tokens: 512,
    temperature: 0.7,
    top_p: 0.9,
    context_window: 4096,
    
    // Circuit breaker
    circuit_breaker_threshold: 3,
    circuit_breaker_reset: std::time::Duration::from_secs(30),
    
    // WASI-NN fields (not used for HTTP)
    model_path: None,
    model_sha256: None,
};

let engine = InferenceEngine::new(config)?;
```

---

## Option 2: Ollama

### Installation

```bash
# macOS
brew install ollama

# Or download from https://ollama.ai
```

### Start Ollama

```bash
# Start Ollama service
ollama serve

# Pull a model (in another terminal)
ollama pull qwen2.5:0.5b
ollama pull llama3.2:1b
ollama pull gemma2:2b
```

### OpenClaw+ Configuration

```rust
let config = InferenceConfig {
    backend: BackendKind::Ollama,
    endpoint: "http://localhost:11434".into(),
    model_name: "qwen2.5:0.5b".into(),  // Use Ollama model name
    
    api_key: None,
    inference_timeout: std::time::Duration::from_secs(120),
    max_tokens: 512,
    temperature: 0.7,
    top_p: 0.9,
    context_window: 4096,
    circuit_breaker_threshold: 3,
    circuit_breaker_reset: std::time::Duration::from_secs(30),
    model_path: None,
    model_sha256: None,
};
```

---

## Option 3: OpenAI-Compatible APIs

Works with:
- OpenAI API
- Azure OpenAI
- Any OpenAI-compatible service (vLLM, Text Generation Inference, etc.)

### Configuration

```rust
let config = InferenceConfig {
    backend: BackendKind::OpenAiCompat,
    endpoint: "https://api.openai.com".into(),  // Or your custom endpoint
    model_name: "gpt-3.5-turbo".into(),
    
    api_key: Some("sk-...".into()),  // Required for OpenAI
    inference_timeout: std::time::Duration::from_secs(120),
    max_tokens: 512,
    temperature: 0.7,
    top_p: 0.9,
    context_window: 4096,
    circuit_breaker_threshold: 3,
    circuit_breaker_reset: std::time::Duration::from_secs(30),
    model_path: None,
    model_sha256: None,
};
```

---

## Running the Demo

```bash
# Make sure your HTTP server is running first!

# Run with HTTP backend
cargo run --release -p openclaw-inference --example wasi_nn_inference_demo
```

The demo will automatically use the HTTP backend if WASI-NN is unavailable.

---

## Performance Comparison

| Backend | Latency | Throughput | GPU Support | Sandboxing |
|---------|---------|------------|-------------|------------|
| WASI-NN (CPU) | Low | 5-15 tok/s | ❌ | ✅ Full |
| WASI-NN (Metal) | Low | 50-100 tok/s | ✅ | ✅ Full |
| llama.cpp HTTP | Medium | 50-100 tok/s | ✅ | ❌ |
| Ollama | Medium | 50-100 tok/s | ✅ | ❌ |
| OpenAI API | High | Variable | ☁️ | ❌ |

**Notes:**
- WASI-NN provides the best isolation but currently has Metal issues on Apple Silicon
- HTTP backends are easier to set up and support GPU acceleration
- For production, consider using HTTP backends on dedicated inference servers

---

## Troubleshooting

### Connection Refused

```
Error: reqwest::Error { kind: Request, ... }
```

**Solution:** Make sure your HTTP server is running:
```bash
# Check if server is listening
curl http://localhost:8080/health  # llama.cpp
curl http://localhost:11434/api/tags  # Ollama
```

### Model Not Found

```
Error: HttpError { status: 404, body: "model not found" }
```

**Solution:**
- For llama.cpp: Verify the model file path in the server command
- For Ollama: Run `ollama pull <model-name>` first

### Timeout

```
Error: Timeout { ... }
```

**Solution:**
- Increase `inference_timeout` in config
- Use a smaller model or reduce `max_tokens`
- Check server logs for performance issues

---

## Next Steps

Once you've verified HTTP backend works:

1. **Rebuild WASI-NN plugin** (when network is stable):
   ```bash
   ./scripts/rebuild_wasi_nn_cpu_only.sh
   ```

2. **Compare performance** between HTTP and WASI-NN backends

3. **Choose deployment strategy**:
   - Development: HTTP backend (easier)
   - Production: WASI-NN (better isolation) or HTTP (better performance)

---

## See Also

- [WASI-NN Status](./WASI_NN_STATUS.md) - Current WASI-NN integration status
- [Metal Issue](./WASI_NN_METAL_ISSUE.md) - Detailed Metal crash analysis
- [WASI-NN Guide](./WASI_NN_GUIDE.md) - Full WASI-NN documentation
