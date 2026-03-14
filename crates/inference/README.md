# `openclaw-inference` — Aerospace-Grade Local AI Inference Engine

**Author:** arksong2018@gmail.com

## Overview

`openclaw-inference` is a production-grade AI inference engine designed with aerospace reliability principles (DO-178C inspired). It provides a robust, fault-tolerant interface to local LLM backends with comprehensive safety mechanisms.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   InferenceEngine                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ CircuitBreaker│  │  AuditLog    │  │HealthMonitor │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                  │          │
│  ┌──────▼───────────────────────────────────▼───────┐  │
│  │              BackendRouter                        │  │
│  │  ┌─────────────────┐   ┌──────────────────────┐  │  │
│  │  │  WasiNnBackend  │   │  HttpBackend         │  │  │
│  │  │  (WasmEdge)     │   │  (llama.cpp/Ollama)  │  │  │
│  │  └─────────────────┘   └──────────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Design Principles

| Principle | Implementation |
|---|---|
| **Isolation** | Every inference request runs in a bounded async task with hard timeout |
| **Fault containment** | Circuit breaker per backend; failures do not cascade |
| **Determinism** | Request IDs, sequence numbers, full audit trail |
| **Integrity** | SHA-256 model file verification before loading |
| **Observability** | Structured tracing on every state transition |
| **Graceful degradation** | Fallback chain: WASI-NN → llama.cpp HTTP → Ollama → error |

## Supported Backends

### 1. **WasmEdge WASI-NN** (In-process, sandboxed)
- Runs GGUF models via WasmEdge's llama.cpp plugin
- SHA-256 integrity verification before model loading
- Feature-gated: `--features wasi-nn`
- **Status:** Stub implementation (executor not yet wired)

### 2. **llama.cpp HTTP** (Out-of-process)
- OpenAI-compatible API (`/v1/chat/completions`)
- Supports streaming via SSE
- Default endpoint: `http://localhost:8080`

### 3. **Ollama** (Out-of-process)
- Native Ollama API (`/api/chat`)
- NDJSON streaming
- Default endpoint: `http://localhost:11434`

### 4. **OpenAI-compatible** (Generic HTTP)
- Any OpenAI-compatible endpoint
- Useful for cloud providers or custom servers

## Key Features

### Circuit Breaker
- Per-backend circuit breakers prevent cascading failures
- States: `Closed` → `Open` → `HalfOpen`
- Configurable threshold and reset duration
- Automatic recovery probing

### Audit Log
- Every request, response, error, and state transition is logged
- Monotonic sequence numbers for total ordering
- Structured tracing integration

### Health Monitoring
- Periodic liveness probes for HTTP backends
- Health states: `Unknown` → `Healthy` → `Degraded` → `Unhealthy`
- Automatic backend selection based on health

### Timeout Protection
- Hard timeout per inference request
- Prevents resource exhaustion from slow/hung backends
- Configurable via `InferenceConfig::inference_timeout`

### Streaming Support
- SSE (Server-Sent Events) for OpenAI-compatible backends
- NDJSON for Ollama
- Async channel-based token delivery

## Usage Example

```rust
use openclaw_inference::{InferenceEngine, InferenceConfig, InferenceRequest, BackendKind};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InferenceConfig {
        backend: BackendKind::LlamaCppHttp,
        endpoint: "http://localhost:8080".into(),
        model_name: "llama3".into(),
        max_tokens: 2048,
        temperature: 0.7,
        inference_timeout: Duration::from_secs(120),
        circuit_breaker_threshold: 3,
        circuit_breaker_reset: Duration::from_secs(30),
        ..Default::default()
    };

    let engine = InferenceEngine::new(config)?;

    let request = InferenceRequest {
        request_id: 0,  // Auto-generated if 0
        messages: vec![
            ConversationTurn {
                role: "user".into(),
                content: "Explain quantum computing in simple terms.".into(),
            }
        ],
        max_tokens_override: None,
        temperature_override: None,
        stream: false,
    };

    let response = engine.infer(request).await?;
    println!("Response: {}", response.content);
    println!("Latency: {}ms", response.latency_ms);

    Ok(())
}
```

## Streaming Example

```rust
let mut request = InferenceRequest { /* ... */ stream: true };
let mut rx = engine.infer_stream(request).await?;

while let Some(token) = rx.recv().await {
    print!("{}", token.delta);
    if token.done {
        break;
    }
}
```

## Configuration

Key configuration parameters in `InferenceConfig`:

- `backend`: Which backend to use (WasiNn, LlamaCppHttp, Ollama, OpenAiCompat)
- `endpoint`: Base URL for HTTP backends
- `model_name`: Model identifier to request
- `max_tokens`: Maximum tokens to generate
- `temperature`: Sampling temperature (0.0 = deterministic, 1.0 = creative)
- `inference_timeout`: Hard timeout for inference calls
- `circuit_breaker_threshold`: Failures before circuit opens
- `circuit_breaker_reset`: Duration before probing recovery
- `context_window`: Maximum context window size

## Testing

```bash
cargo test -p openclaw-inference
```

Current test coverage:
- ✅ Engine creation
- ✅ Health status reporting
- ✅ Request ID generation
- ✅ Timeout enforcement

## Integration with OpenClaw+

This crate is part of the OpenClaw+ security-hardened plugin system:

1. **Sandbox integration**: Inference requests from sandboxed plugins are routed through the security layer
2. **Resource limits**: Token limits and timeout enforcement prevent DoS
3. **Audit trail**: All AI interactions are logged for compliance
4. **Offline-first**: WASI-NN backend enables fully offline operation

## Roadmap

- [ ] Wire up WasmEdge WASI-NN executor (currently stub)
- [ ] Add token counting and context window management
- [ ] Implement model caching and preloading
- [ ] Add batch inference support
- [ ] Prometheus metrics export
- [ ] Function calling / tool use support
- [ ] Multi-model routing and load balancing

## License

MIT

## Status

**Version:** 0.1.0  
**Compilation:** ✅ Passing  
**Tests:** ✅ 4/4 passing  
**Warnings:** 1 (unused field in WasiNnBackend)
