#!/usr/bin/env bash
#
# Start llama.cpp native server as backup inference service
# This provides redundancy for production environments
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

MODEL_PATH="${1:-}"
PORT="${2:-8080}"
HOST="${3:-127.0.0.1}"
THREADS="${4:-$(sysctl -n hw.ncpu)}"

if [ -z "$MODEL_PATH" ]; then
    echo "Usage: $0 <model.gguf> [port] [host] [threads]"
    echo ""
    echo "Example (local backup):"
    echo "  $0 models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf 8080"
    echo ""
    echo "Example (remote server):"
    echo "  $0 models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf 8080 0.0.0.0"
    echo ""
    exit 1
fi

if [ ! -f "$MODEL_PATH" ]; then
    echo "❌ Model file not found: $MODEL_PATH"
    exit 1
fi

MODEL_ABS=$(cd "$(dirname "$MODEL_PATH")" && pwd)/$(basename "$MODEL_PATH")

echo "=== Starting llama.cpp Server (Backup Service) ==="
echo "Model:   $MODEL_ABS"
echo "Host:    $HOST"
echo "Port:    $PORT"
echo "Threads: $THREADS"
echo ""

# Check if llama.cpp server is installed
LLAMA_SERVER=""

# Try Homebrew installation
if command -v llama-server &> /dev/null; then
    LLAMA_SERVER="llama-server"
# Try local build
elif [ -f "$PROJECT_ROOT/build/llama.cpp/build/bin/llama-server" ]; then
    LLAMA_SERVER="$PROJECT_ROOT/build/llama.cpp/build/bin/llama-server"
else
    echo "❌ llama.cpp server not found. Installing..."
    echo ""
    
    # Install via Homebrew
    if command -v brew &> /dev/null; then
        echo "Installing llama.cpp via Homebrew..."
        brew install llama.cpp
        LLAMA_SERVER="llama-server"
    else
        # Build from source
        echo "Building llama.cpp from source..."
        mkdir -p "$PROJECT_ROOT/build"
        cd "$PROJECT_ROOT/build"
        
        if [ ! -d "llama.cpp" ]; then
            git clone https://github.com/ggerganov/llama.cpp.git
        fi
        
        cd llama.cpp
        git pull
        
        mkdir -p build
        cd build
        
        # Build with Metal support on macOS
        cmake .. \
            -DCMAKE_BUILD_TYPE=Release \
            -DLLAMA_METAL=ON \
            -DLLAMA_BUILD_SERVER=ON
        
        cmake --build . --config Release -j"$THREADS"
        
        LLAMA_SERVER="$PROJECT_ROOT/build/llama.cpp/build/bin/llama-server"
    fi
fi

if [ ! -x "$LLAMA_SERVER" ] && [ ! -f "$LLAMA_SERVER" ]; then
    echo "❌ Failed to install llama.cpp server"
    exit 1
fi

echo "✅ Using: $LLAMA_SERVER"
echo ""

# Start the server
echo "🚀 Starting server on http://$HOST:$PORT"
echo "   OpenAI-compatible API endpoint: http://$HOST:$PORT/v1"
echo "   Health check: http://$HOST:$PORT/health"
echo ""
echo "   Press Ctrl+C to stop"
echo ""

# Run with optimal settings
exec "$LLAMA_SERVER" \
    --model "$MODEL_ABS" \
    --host "$HOST" \
    --port "$PORT" \
    --threads "$THREADS" \
    --ctx-size 8192 \
    --n-gpu-layers 999 \
    --flash-attn \
    --no-mmap \
    --log-disable
