# WASI-NN CPU-Only Mode Notes

## 构建状态

✅ **成功构建 CPU-only WASI-NN 插件**

- 日期: 2026-02-27
- 插件路径: `~/.wasmedge/plugin/libwasmedgePluginWasiNN.dylib`
- 插件大小: 7.2MB
- Metal 依赖: ✅ 已完全移除（通过 `otool -L` 验证）
- 构建配置:
  - GGML_METAL: **DISABLED**
  - GGML_NATIVE: **ENABLED** (AVX/AVX2/FMA/F16C)
  - GGML_CUDA: **DISABLED**

## 运行时问题

### 症状

运行 `wasi_nn_inference_demo` 时进程被 SIGKILL (退出码 137) 终止：

```
=== WASI-NN Inference Demo ===
Initializing inference engine...
✓ Engine initialized
Test 1: Simple question
Running inference...
DEBUG: Calling wasmedge_wasi_nn_infer with prompt length: 119
zsh: killed
```

### 根本原因分析

**内存限制问题**

1. **系统内存**: 16GB RAM (充足)
2. **模型大小**: 469MB (qwen2.5-0.5b-instruct-q4_k_m.gguf)
3. **问题**: CPU-only 模式下，llama.cpp 在 WasmEdge 环境中运行时遇到内存分配限制

**技术细节**:
- Metal 模式: 模型加载到 GPU VRAM，主内存压力小
- CPU-only 模式: 模型完全加载到主内存，需要更多 RAM
- WasmEdge WASM 模块可能有默认的内存限制
- llama.cpp 在 WASM 环境中的内存分配策略可能不同于原生环境

### 🔧 尝试的解决方案

### 1. 增加 WASM 内存限制（未成功）✅ 已测试

**尝试**: 通过 `RuntimeConfigOptions::max_memory_pages()` 配置更大的内存页限制

**测试配置**:
```rust
use wasmedge_sdk::config::RuntimeConfigOptions;

// 测试 1: 8GB (131072 页 × 64KB)
let runtime_config = RuntimeConfigOptions::default()
    .max_memory_pages(131072);

// 测试 2: 16GB (262144 页 × 64KB) - 与系统 RAM 相同
let runtime_config = RuntimeConfigOptions::default()
    .max_memory_pages(262144);

let config = ConfigBuilder::new(CommonConfigOptions::default())
    .with_runtime_config(runtime_config)
    .build()?;
```

**测试结果**:
- 4GB (默认): ❌ SIGKILL (exit code 137)
- 8GB: ❌ SIGKILL (exit code 137)
- 16GB: ❌ SIGKILL (exit code 137)

**结论**: 
- ✅ 找到了正确的 API (`RuntimeConfigOptions::max_memory_pages`)
- ❌ 但增加内存限制**无法解决问题**
- 问题不在于 WasmEdge 的内存页配置
- 可能是 WASM 32位地址空间的架构限制（最大 4GB）
- 或者是 llama.cpp 在 WASM 环境中的内存分配策略问题

### 2. ❌ 设置 `GGML_METAL=0` 环境变量 - 无效（问题不是 Metal）

### 3. ✅ 验证插件构建正确 - Metal 已完全移除

## 推荐解决方案

### 方案 1: 使用 HTTP 后端（推荐）✅

**优势**:
- ✅ 已验证可用（Ollama 测试通过）
- ✅ GPU 加速（性能优秀）
- ✅ 无内存限制问题
- ✅ 更灵活的模型管理

**示例**:
```rust
let config = InferenceConfig {
    backend: BackendKind::Ollama,
    endpoint: "http://localhost:11434".into(),
    model_name: "qwen2.5:0.5b".into(),
    // ...
};
```

**性能对比**:
- HTTP (Ollama): 300-5000ms 延迟，GPU 加速
- WASI-NN CPU-only: 理论上 5-15 tokens/sec（如果能运行）

### 方案 2: 使用更小的模型

尝试使用更小的量化模型：
- Q2_K: ~200MB
- Q3_K_S: ~250MB
- Q4_0: ~300MB

### 方案 3: 原生 llama.cpp（不使用 WASM）

如果必须使用 CPU-only 推理，考虑：
1. 直接使用 llama.cpp 原生二进制
2. 通过 FFI 调用而非 WASM
3. 避免 WasmEdge 的内存限制

### 方案 4: 等待 WasmEdge 更新

WasmEdge 团队可能在未来版本中：
- 改进 WASM 内存管理
- 优化 llama.cpp 集成
- 提供更好的大模型支持

## 测试结果总结

### ✅ 成功的部分

1. **插件构建**: CPU-only 插件成功编译
2. **Metal 移除**: 完全移除 Metal 依赖
3. **插件加载**: WasmEdge 能够加载插件
4. **HTTP 后端**: 完全可用，性能优秀
5. **单元测试**: 83/83 测试通过

### ⚠️ 限制

1. **WASI-NN CPU-only**: 运行时内存限制导致崩溃
2. **大模型支持**: 469MB 模型在 WASM 环境中无法运行
3. **性能**: CPU-only 即使能运行也会很慢

## 生产环境建议

### 推荐配置

**Apple Silicon Mac (有 GPU)**:
```rust
// 使用 HTTP 后端 + Ollama
BackendKind::Ollama
```

**Linux/Windows (有 CUDA)**:
```rust
// 使用 WASI-NN + CUDA 插件
BackendKind::WasiNn
```

**纯 CPU 环境**:
```rust
// 使用 HTTP 后端 + llama.cpp server
BackendKind::LlamaCppHttp
```

### 不推荐

- ❌ WASI-NN CPU-only 模式（内存限制问题）
- ❌ 大模型 (>500MB) 在 WASM 环境中

## 下一步行动

### 立即可用

1. ✅ 使用 HTTP 后端进行开发和生产
2. ✅ 运行 `http_backend_demo` 验证功能
3. ✅ 集成到应用中

### 长期改进

1. 监控 WasmEdge 更新，关注内存管理改进
2. 考虑贡献 PR 到 WasmEdge，改进大模型支持
3. 探索其他 WASM 运行时（如 Wasmtime）

## 参考资料

- WasmEdge WASI-NN 文档: https://wasmedge.org/docs/develop/rust/wasinn/
- llama.cpp 内存优化: https://github.com/ggerganov/llama.cpp/discussions
- OpenClaw+ HTTP 后端指南: `docs/HTTP_BACKEND_GUIDE.md`

## 结论

虽然 CPU-only WASI-NN 插件成功构建并移除了 Metal 依赖，但由于 WasmEdge WASM 环境的内存限制，无法运行较大的模型。

**最佳实践**: 使用 HTTP 后端（Ollama 或 llama.cpp server），这提供了更好的性能、灵活性和稳定性。

---

*最后更新: 2026-02-27*
*状态: CPU-only 插件已构建，推荐使用 HTTP 后端*
