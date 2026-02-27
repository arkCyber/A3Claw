#!/usr/bin/env bash
#
# Start a dedicated WasmEdge process for llama.cpp inference
# This provides an independent 4GB memory space for model loading
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

MODEL_PATH="${1:-}"
PORT="${2:-8080}"

if [ -z "$MODEL_PATH" ]; then
    echo "Usage: $0 <model.gguf> [port]"
    echo ""
    echo "Example:"
    echo "  $0 $PROJECT_ROOT/models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf 8080"
    echo ""
    echo "This starts a dedicated WasmEdge process for llama.cpp inference,"
    echo "providing an independent 4GB memory space to solve the memory limitation."
    exit 1
fi

if [ ! -f "$MODEL_PATH" ]; then
    echo "❌ Model file not found: $MODEL_PATH"
    exit 1
fi

MODEL_ABS=$(cd "$(dirname "$MODEL_PATH")" && pwd)/$(basename "$MODEL_PATH")

echo "=== Starting WasmEdge llama.cpp Service ==="
echo "Model: $MODEL_ABS"
echo "Port: $PORT"
echo "Memory: 4GB (dedicated process)"
echo ""

# Check if wasmedge is installed
if ! command -v wasmedge &> /dev/null; then
    echo "❌ WasmEdge not found. Please install it:"
    echo "   curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash"
    exit 1
fi

# Use WasmEdge's built-in llama.cpp plugin with a simple HTTP wrapper
# We'll create a minimal WASM module that uses WASI-NN and provides HTTP API

WASM_MODULE="$PROJECT_ROOT/bin/llama-http-server.wasm"

if [ ! -f "$WASM_MODULE" ]; then
    echo "⚠️  WASM HTTP server not found. Building..."
    "$SCRIPT_DIR/build_llama_http_server.sh"
fi

# Run the dedicated WasmEdge process
echo "🚀 Starting server on http://localhost:$PORT"
echo "   Press Ctrl+C to stop"
echo ""

wasmedge \
    --dir .:. \
    --env MODEL_PATH="$MODEL_ABS" \
    --env HTTP_PORT="$PORT" \
    --env CTX_SIZE="8192" \
    "$WASM_MODULE"
