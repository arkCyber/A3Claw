#!/bin/bash
set -e

echo "=== WasmEdge WASI-NN CPU-only Plugin Rebuild Script (Minimal Dependencies) ==="
echo ""

# Configuration
WASMEDGE_VERSION="0.14.1"
BUILD_DIR="/tmp/wasmedge-minimal-build"
PLUGIN_INSTALL_PATH="$HOME/.wasmedge/plugin/libwasmedgePluginWasiNN.dylib"

echo "Step 1: Backup existing plugin..."
if [ -f "$PLUGIN_INSTALL_PATH" ]; then
    cp "$PLUGIN_INSTALL_PATH" "${PLUGIN_INSTALL_PATH}.metal-backup"
    echo "✓ Backed up to ${PLUGIN_INSTALL_PATH}.metal-backup"
else
    echo "⚠ No existing plugin found at $PLUGIN_INSTALL_PATH"
fi

echo ""
echo "Step 2: Clone WasmEdge source (this may take a while)..."
rm -rf "$BUILD_DIR"
git clone --depth 1 --branch "$WASMEDGE_VERSION" https://github.com/WasmEdge/WasmEdge.git "$BUILD_DIR"
cd "$BUILD_DIR"

echo ""
echo "Step 3: Configure build with Metal DISABLED and minimal dependencies..."

# WasmEdge CMake requires LLD; on macOS Homebrew only llvm@12 provides it by default.
# Prefer LLD_DIR so find_package(LLD) succeeds; set CMAKE_PREFIX_PATH for LLVM if needed.
CMAKE_EXTRA=""
if [ "$(uname -s)" = "Darwin" ]; then
  LLD_CFG=""
  for D in /opt/homebrew/opt/llvm@12 /opt/homebrew/opt/llvm /usr/local/opt/llvm@12 /usr/local/opt/llvm; do
    if [ -f "$D/lib/cmake/lld/LLDConfig.cmake" ]; then
      LLD_CFG="$D/lib/cmake/lld"
      CMAKE_EXTRA="-DLLD_DIR=$LLD_CFG -DCMAKE_PREFIX_PATH=$D"
      echo "Using LLD from: $LLD_CFG"
      break
    fi
  done
  if [ -z "$LLD_CFG" ]; then
    LLD_CFG=$(find /opt/homebrew/Cellar /usr/local/Cellar -name "LLDConfig.cmake" 2>/dev/null | head -1)
    if [ -n "$LLD_CFG" ]; then
      LLD_DIR="${LLD_CFG%/*}"
      LLVM_PREFIX="${LLD_DIR%/lib/cmake/lld}"
      CMAKE_EXTRA="-DLLD_DIR=$LLD_DIR -DCMAKE_PREFIX_PATH=$LLVM_PREFIX"
      echo "Using LLD from: $LLD_DIR"
    else
      echo "Warning: LLD not found. Install with: brew install llvm@12"
    fi
  fi
fi

# Avoid Homebrew spdlog/fmt so WasmEdge uses FetchContent (bundled); otherwise build fails with
# "no member named 'basic_runtime' in namespace 'fmt'". Use only our LLVM prefix for this run.
SAVED_CMAKE_PREFIX_PATH="${CMAKE_PREFIX_PATH:-}"
export CMAKE_PREFIX_PATH=""
CMAKE_EXTRA="$CMAKE_EXTRA -DCMAKE_IGNORE_PATH=/opt/homebrew/lib/cmake/Spdlog;/opt/homebrew/lib/cmake/fmt"

cmake -GNinja -Bbuild -DCMAKE_BUILD_TYPE=Release \
  $CMAKE_EXTRA \
  -DWASMEDGE_BUILD_TESTS=OFF \
  -DWASMEDGE_BUILD_TOOLS=OFF \
  -DWASMEDGE_BUILD_AOT_RUNTIME=OFF \
  -DWASMEDGE_BUILD_SHARED_LIB=OFF \
  -DWASMEDGE_BUILD_STATIC_LIB=ON \
  -DWASMEDGE_BUILD_PLUGINS=ON \
  -DWASMEDGE_PLUGIN_WASI_NN=ON \
  -DWASMEDGE_PLUGIN_WASI_NN_BACKEND="GGML" \
  -DWASMEDGE_PLUGIN_WASI_NN_GGML_LLAMA_METAL=OFF \
  -DWASMEDGE_PLUGIN_WASI_NN_GGML_LLAMA_BLAS=OFF \
  -DWASMEDGE_PLUGIN_WASI_CRYPTO=OFF \
  -DWASMEDGE_PLUGIN_WASI_LOGGING=OFF \
  -DWASMEDGE_PLUGIN_WASI_HTTP=OFF \
  -DWASMEDGE_PLUGIN_PROCESS=OFF \
  -DWASMEDGE_PLUGIN_TENSORFLOW=OFF \
  -DWASMEDGE_PLUGIN_TENSORFLOWLITE=OFF \
  -DWASMEDGE_PLUGIN_IMAGE=OFF \
  -DWASMEDGE_PLUGIN_OPENCVMINI=OFF \
  -DWASMEDGE_PLUGIN_ZLIB=OFF \
  -DWASMEDGE_PLUGIN_FFMPEG=OFF \
  .

echo ""
echo "Step 4: Build plugin (this will take 10-30 minutes)..."
[ -n "$SAVED_CMAKE_PREFIX_PATH" ] && export CMAKE_PREFIX_PATH="$SAVED_CMAKE_PREFIX_PATH" || unset CMAKE_PREFIX_PATH
cmake --build build --target wasmedgePluginWasiNN -j$(sysctl -n hw.ncpu)

echo ""
echo "Step 5: Install new plugin..."
BUILT_PLUGIN="$BUILD_DIR/build/plugins/wasi_nn/libwasmedgePluginWasiNN.dylib"
if [ -f "$BUILT_PLUGIN" ]; then
    cp "$BUILT_PLUGIN" "$PLUGIN_INSTALL_PATH"
    echo "✓ Installed CPU-only plugin to $PLUGIN_INSTALL_PATH"
else
    echo "✗ Build failed: plugin not found at $BUILT_PLUGIN"
    echo ""
    echo "Checking build directory contents..."
    find "$BUILD_DIR/build" -name "*.dylib" -o -name "*.so" | head -20
    exit 1
fi

echo ""
echo "Step 6: Verify Metal is removed..."
if otool -L "$PLUGIN_INSTALL_PATH" | grep -i metal; then
    echo "✗ WARNING: Metal frameworks still linked!"
    exit 1
else
    echo "✓ Metal frameworks successfully removed"
fi

echo ""
echo "Step 7: Check plugin info..."
file "$PLUGIN_INSTALL_PATH"
ls -lh "$PLUGIN_INSTALL_PATH"
echo ""
echo "Dependencies:"
otool -L "$PLUGIN_INSTALL_PATH"

echo ""
echo "=== Build Complete ==="
echo ""
echo "Next steps:"
echo "1. Test the plugin:"
echo "   cd /Users/arkSong/workspace/OpenClaw+"
echo "   cargo run --release --features wasi-nn -p openclaw-inference --example wasi_nn_inference_demo"
echo ""
echo "2. If it works, you can delete the backup:"
echo "   rm ${PLUGIN_INSTALL_PATH}.metal-backup"
echo ""
echo "3. If it fails, restore the backup:"
echo "   cp ${PLUGIN_INSTALL_PATH}.metal-backup $PLUGIN_INSTALL_PATH"
echo ""
echo "Note: CPU-only inference will be slower (~5-15 tokens/sec vs 50-100 with Metal)"
echo ""
echo "Cleanup: To remove build directory, run:"
echo "   rm -rf $BUILD_DIR"
