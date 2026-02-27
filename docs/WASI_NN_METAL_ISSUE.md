# WASI-NN Metal Crash on Apple Silicon

## Problem

The current WasmEdge WASI-NN ggml plugin (v0.1.34.0) crashes on Apple Silicon with:

```
GGML_ASSERT(buf_dst) failed
/Users/runner/work/wasi-nn-ggml-plugin/.../ggml-metal-context.m:323
```

This occurs during model loading (`nn_preload`) when the plugin attempts to initialize Metal GPU acceleration.

## Root Cause

1. **Plugin is compiled with Metal support**: The installed plugin at `~/.wasmedge/plugin/libwasmedgePluginWasiNN.dylib` is linked against Metal frameworks:
   ```
   /System/Library/Frameworks/Metal.framework
   /System/Library/Frameworks/MetalKit.framework
   ```

2. **Metal initialization cannot be disabled at runtime**: 
   - Environment variables like `GGML_METAL_DISABLE` are not checked by the plugin
   - The `ExecutionTarget::CPU` parameter in `nn_preload` does not prevent Metal initialization
   - Metal context creation happens before any runtime configuration is applied

3. **Buffer allocation failure**: The crash suggests Metal is failing to allocate GPU buffers, possibly due to:
   - macOS version incompatibility (older Monterey/Ventura vs newer llama.cpp Metal implementation)
   - Memory fragmentation or system resource limits
   - Driver issues with specific GPU models

## Attempted Solutions (All Failed)

### 1. Environment Variables ❌
Tried passing to WASI guest:
- `GGML_METAL_DISABLE=1`
- `GGML_NO_METAL=1`
- `GGML_METAL_SHARED_BUFFERS_DISABLE=1`
- `GGML_METAL_FUSION_DISABLE=1`
- `GGML_METAL_CONCURRENCY_DISABLE=1`

**Result**: Metal still initializes and crashes.

### 2. ExecutionTarget::CPU ❌
Changed `nn_preload` to use `ExecutionTarget::CPU` instead of `AUTO`.

**Result**: Metal still initializes (this parameter only affects inference, not loading).

### 3. Recompile from Source ❌
Attempted to build WasmEdge with `-DWASMEDGE_PLUGIN_WASI_NN_GGML_LLAMA_METAL=OFF`.

**Result**: Network instability prevented downloading dependencies (simdjson, llama.cpp).

## Working Solution

### Option A: Use HTTP Backend (Temporary)

The inference engine supports HTTP backend as fallback. Configure `InferenceConfig` to use an external LLM API:

```rust
let config = InferenceConfig {
    backend_type: BackendType::Http,
    http_endpoint: Some("http://localhost:11434/v1/chat/completions".to_string()),
    // ... other fields
};
```

### Option B: Rebuild Plugin without Metal (Recommended)

When network is stable, rebuild the WASI-NN plugin:

```bash
# Clone WasmEdge source
git clone --depth 1 --branch 0.14.1 https://github.com/WasmEdge/WasmEdge.git
cd WasmEdge

# Build with Metal disabled (CPU-only)
cmake -GNinja -Bbuild -DCMAKE_BUILD_TYPE=Release \
  -DWASMEDGE_PLUGIN_WASI_NN_BACKEND="GGML" \
  -DWASMEDGE_PLUGIN_WASI_NN_GGML_LLAMA_METAL=OFF \
  -DWASMEDGE_PLUGIN_WASI_NN_GGML_LLAMA_BLAS=OFF \
  .

cmake --build build

# Replace the plugin
cp build/plugins/wasi_nn/libwasmedgePluginWasiNN.dylib \
   ~/.wasmedge/plugin/libwasmedgePluginWasiNN.dylib

# Verify Metal is removed
otool -L ~/.wasmedge/plugin/libwasmedgePluginWasiNN.dylib | grep -i metal
# Should return nothing
```

### Option C: Use Pre-built CPU-only Plugin

Download an older version (0.13.x) that may not have Metal enabled by default, or request a CPU-only build from WasmEdge team.

## Verification

After replacing the plugin, verify Metal dependencies are removed:

```bash
otool -L ~/.wasmedge/plugin/libwasmedgePluginWasiNN.dylib | grep -i metal
```

Expected output: (empty - no Metal frameworks listed)

Then run the demo:

```bash
cargo run --release --features wasi-nn -p openclaw-inference --example wasi_nn_inference_demo
```

Expected: Model loads successfully without Metal crash, inference runs on CPU.

## Performance Impact

CPU-only inference will be **significantly slower** than Metal GPU acceleration:
- **Metal (GPU)**: ~50-100 tokens/sec on M1/M2
- **CPU only**: ~5-15 tokens/sec

For production use on Apple Silicon, consider:
1. Fixing the Metal buffer allocation issue (requires debugging llama.cpp Metal backend)
2. Using external GPU-accelerated inference server (Ollama, llama.cpp server)
3. Deploying on Linux with CUDA support

## Related Files

- Plugin source: `crates/inference/src/backend.rs` (lines 378-388 for `nn_preload`)
- Demo: `crates/inference/examples/wasi_nn_inference_demo.rs`
- Current plugin: `~/.wasmedge/plugin/libwasmedgePluginWasiNN.dylib` (v0.1.34.0, 5.3MB)

## Status

**Current**: Blocked on Metal crash, cannot run WASI-NN inference on Apple Silicon.

**Next Steps**:
1. Wait for stable network connection
2. Rebuild plugin with Metal disabled
3. Verify Test 1 runs successfully with CPU inference
4. Document performance benchmarks
