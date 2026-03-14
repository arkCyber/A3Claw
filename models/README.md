# OpenClaw+ Model Storage

## Directory Structure

```
models/
├── gguf/          # GGUF model files for llama.cpp / WasmEdge WASI-NN
├── cache/         # Inference cache (KV-cache snapshots)
├── logs/          # Per-session inference logs
└── README.md      # This file
```

## Supported Backends

| Backend | Format | Notes |
|---------|--------|-------|
| Ollama  | Native | Recommended for development |
| llama.cpp HTTP | GGUF | Production / offline |
| WasmEdge WASI-NN | GGUF | Sandboxed in-process |

## Models

| Model | Size | Backend | SHA-256 |
|-------|------|---------|---------|
| qwen2.5:0.5b | ~400 MB | Ollama | (managed by Ollama) |

## Configuration

The inference engine reads `config/inference.toml` at startup.
See `crates/inference/src/types.rs` for all available fields.
