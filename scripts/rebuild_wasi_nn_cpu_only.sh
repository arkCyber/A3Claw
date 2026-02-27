#!/bin/bash
set -e

echo "=== WasmEdge WASI-NN CPU-only Plugin Rebuild Script ==="
echo ""

# Configuration
WASMEDGE_VERSION="0.14.1"
BUILD_DIR="/tmp/wasmedge-cpu-build"
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
echo "Step 3: Configure build with Metal DISABLED..."
cmake -GNinja -Bbuild -DCMAKE_BUILD_TYPE=Release \
  -DWASMEDGE_PLUGIN_WASI_NN_BACKEND="GGML" \
  -DWASMEDGE_PLUGIN_WASI_NN_GGML_LLAMA_METAL=OFF \
  -DWASMEDGE_PLUGIN_WASI_NN_GGML_LLAMA_BLAS=OFF \
  .

echo ""
echo "Step 4: Build plugin (this will take 10-30 minutes)..."
cmake --build build --target wasmedgePluginWasiNN

echo ""
echo "Step 5: Install new plugin..."
BUILT_PLUGIN="$BUILD_DIR/build/plugins/wasi_nn/libwasmedgePluginWasiNN.dylib"
if [ -f "$BUILT_PLUGIN" ]; then
    cp "$BUILT_PLUGIN" "$PLUGIN_INSTALL_PATH"
    echo "✓ Installed CPU-only plugin to $PLUGIN_INSTALL_PATH"
else
    echo "✗ Build failed: plugin not found at $BUILT_PLUGIN"
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
