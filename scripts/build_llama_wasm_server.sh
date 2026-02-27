#!/usr/bin/env bash
#
# Build llama.cpp WASM server for WasmEdge
# This creates an independent WasmEdge process dedicated to llama.cpp inference
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build/llama-wasm"
LLAMA_REPO="https://github.com/ggerganov/llama.cpp.git"
LLAMA_DIR="$BUILD_DIR/llama.cpp"

echo "=== Building llama.cpp WASM Server for WasmEdge ==="
echo ""
echo "This will create an independent WasmEdge process for llama.cpp inference"
echo "with dedicated 4GB memory space, solving the memory limitation issue."
echo ""

# Step 1: Check prerequisites
echo "Step 1: Checking prerequisites..."

if ! command -v wasmedge &> /dev/null; then
    echo "❌ WasmEdge not found. Please install it first:"
    echo "   curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash"
    exit 1
fi

if ! command -v cmake &> /dev/null; then
    echo "❌ CMake not found. Please install it:"
    echo "   brew install cmake"
    exit 1
fi

# Check for Emscripten (needed for WASM build)
if ! command -v emcc &> /dev/null; then
    echo "⚠️  Emscripten not found. Installing via Homebrew..."
    if command -v brew &> /dev/null; then
        brew install emscripten
    else
        echo "❌ Please install Emscripten manually:"
        echo "   https://emscripten.org/docs/getting_started/downloads.html"
        exit 1
    fi
fi

echo "✅ All prerequisites found"
echo ""

# Step 2: Clone llama.cpp if needed
echo "Step 2: Preparing llama.cpp source..."
mkdir -p "$BUILD_DIR"

if [ -d "$LLAMA_DIR" ]; then
    echo "llama.cpp already exists, updating..."
    cd "$LLAMA_DIR"
    git pull
else
    echo "Cloning llama.cpp..."
    git clone --depth 1 "$LLAMA_REPO" "$LLAMA_DIR"
    cd "$LLAMA_DIR"
fi

echo "✅ Source ready"
echo ""

# Step 3: Build llama.cpp for WASM
echo "Step 3: Building llama.cpp WASM module..."
echo "This may take 10-20 minutes..."

mkdir -p build-wasm
cd build-wasm

# Configure with Emscripten
emcmake cmake .. \
    -DCMAKE_BUILD_TYPE=Release \
    -DLLAMA_METAL=OFF \
    -DLLAMA_CUDA=OFF \
    -DLLAMA_BLAS=OFF \
    -DLLAMA_BUILD_TESTS=OFF \
    -DLLAMA_BUILD_EXAMPLES=ON \
    -DLLAMA_BUILD_SERVER=ON

# Build
emmake make -j$(sysctl -n hw.ncpu) llama-server

if [ ! -f "bin/llama-server.wasm" ]; then
    echo "❌ Build failed: llama-server.wasm not found"
    exit 1
fi

echo "✅ Build successful"
echo ""

# Step 4: Copy to project directory
echo "Step 4: Installing WASM server..."
INSTALL_DIR="$PROJECT_ROOT/bin/llama-wasm"
mkdir -p "$INSTALL_DIR"

cp bin/llama-server.wasm "$INSTALL_DIR/"
cp bin/llama-server.js "$INSTALL_DIR/" 2>/dev/null || true

echo "✅ Installed to: $INSTALL_DIR"
echo ""

# Step 5: Create launcher script
echo "Step 5: Creating launcher script..."

cat > "$INSTALL_DIR/start-llama-server.sh" << 'EOF'
#!/usr/bin/env bash
#
# Start llama.cpp WASM server with WasmEdge
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODEL_PATH="${1:-}"
PORT="${2:-8080}"

if [ -z "$MODEL_PATH" ]; then
    echo "Usage: $0 <model.gguf> [port]"
    echo ""
    echo "Example:"
    echo "  $0 /path/to/qwen2.5-0.5b-instruct-q4_k_m.gguf 8080"
    exit 1
fi

if [ ! -f "$MODEL_PATH" ]; then
    echo "❌ Model file not found: $MODEL_PATH"
    exit 1
fi

echo "=== Starting llama.cpp WASM Server ==="
echo "Model: $MODEL_PATH"
echo "Port: $PORT"
echo "Memory: 4GB (dedicated WasmEdge process)"
echo ""

# Run with WasmEdge
# --dir maps host filesystem to WASM
# --env sets environment variables
wasmedge \
    --dir .:. \
    --env LLAMA_ARG_MODEL="$MODEL_PATH" \
    --env LLAMA_ARG_HOST="127.0.0.1" \
    --env LLAMA_ARG_PORT="$PORT" \
    --env LLAMA_ARG_CTX_SIZE="8192" \
    "$SCRIPT_DIR/llama-server.wasm"
EOF

chmod +x "$INSTALL_DIR/start-llama-server.sh"

echo "✅ Launcher created"
echo ""

# Step 6: Print usage instructions
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ llama.cpp WASM Server built successfully!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📦 Installation:"
echo "   Location: $INSTALL_DIR"
echo "   Binary:   llama-server.wasm"
echo "   Launcher: start-llama-server.sh"
echo ""
echo "🚀 Quick Start:"
echo ""
echo "   # Start the server"
echo "   $INSTALL_DIR/start-llama-server.sh \\"
echo "     $PROJECT_ROOT/models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf \\"
echo "     8080"
echo ""
echo "   # Test the server"
echo "   curl http://localhost:8080/v1/chat/completions \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{"
echo "       \"model\": \"gpt-3.5-turbo\","
echo "       \"messages\": [{\"role\": \"user\", \"content\": \"Hello!\"}]"
echo "     }'"
echo ""
echo "💡 Advantages:"
echo "   ✅ Independent WasmEdge process (4GB dedicated memory)"
echo "   ✅ Process isolation (crashes don't affect main app)"
echo "   ✅ OpenAI-compatible API"
echo "   ✅ Can run multiple instances for load balancing"
echo ""
echo "📝 Next Steps:"
echo "   1. Start the WASM server with your model"
echo "   2. Configure OpenClaw+ to use HTTP backend:"
echo "      backend: BackendKind::OpenAiCompat"
echo "      endpoint: \"http://localhost:8080\""
echo ""
