# WasmEdge 独立进程方案

## 问题分析

当前 WASI-NN CPU-only 模式的问题：
- 模型加载和推理在**同一个进程**中
- 即使配置了 16GB 内存限制，仍然 SIGKILL
- 问题不在于配置，而在于架构

## 解决方案：独立进程架构

### 方案对比

| 方案 | 描述 | 优势 | 实现难度 |
|------|------|------|----------|
| **方案 1: Ollama** | 使用 Ollama 作为独立服务 | ✅ 最简单<br>✅ GPU 加速<br>✅ 已验证 | ⭐ 简单 |
| **方案 2: llama.cpp server** | 原生 llama.cpp HTTP 服务器 | ✅ 轻量级<br>✅ 性能好 | ⭐⭐ 中等 |
| **方案 3: WasmEdge 独立进程** | 用 WasmEdge 运行 llama.cpp WASM | ✅ 隔离性好<br>⚠️ 仍可能有内存问题 | ⭐⭐⭐ 复杂 |

## 推荐方案：使用 Ollama（方案 1）

### 为什么推荐 Ollama？

1. **已经是独立进程** - Ollama 本身就是一个独立的服务进程
2. **内存隔离** - 完全独立的进程空间，不受 WASM 限制
3. **GPU 加速** - 支持 Metal (Apple Silicon) 和 CUDA
4. **已验证可用** - HTTP backend demo 测试通过
5. **易于管理** - 简单的命令行工具

### 快速开始

```bash
# 1. 安装 Ollama
brew install ollama

# 2. 启动 Ollama 服务（独立进程）
ollama serve

# 3. 下载模型
ollama pull qwen3.5:9b

# 4. 配置 OpenClaw+ 使用 Ollama
# 在 InferenceConfig 中设置：
#   backend: BackendKind::Ollama
#   endpoint: "http://localhost:11434"
#   model_name: "qwen3.5:9b"
```

### 架构图

```
┌─────────────────────────────────┐
│  OpenClaw+ (Rust 主进程)         │
│  ┌───────────────────────────┐  │
│  │  HTTP Backend             │  │
│  │  (reqwest client)         │  │
│  └──────────┬────────────────┘  │
└─────────────┼───────────────────┘
              │ HTTP
              ▼
┌─────────────────────────────────┐
│  Ollama (独立进程)               │
│  ┌───────────────────────────┐  │
│  │  llama.cpp (原生)         │  │
│  │  - 无内存限制              │  │
│  │  - GPU 加速 (Metal)       │  │
│  │  - 模型管理                │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

## 如果仍想尝试 WasmEdge 独立进程（方案 3）

### 理论可行性

创建一个独立的 WasmEdge 进程来运行 llama.cpp WASM 模块，理论上可以：
- 独享 4GB WASM 内存空间
- 进程隔离
- 崩溃不影响主程序

### 实际问题

1. **仍然受 WASM 限制** - WASM 32位地址空间最大 4GB
2. **模型加载仍在插件中** - WasmEdge WASI-NN 插件加载模型时仍可能失败
3. **复杂度高** - 需要构建 WASM HTTP 服务器
4. **性能损失** - WASM 运行时开销

### 如果要实现

需要以下步骤：

1. **创建 WASM HTTP 服务器模块**
   ```rust
   // 新建 crates/llama-http-server
   // 使用 WASI-NN + HTTP 库（如 hyper-wasi）
   ```

2. **构建为 WASM**
   ```bash
   cargo build --target wasm32-wasip1 --release
   ```

3. **用 WasmEdge 运行**
   ```bash
   wasmedge \
     --dir .:. \
     --env MODEL_PATH=/path/to/model.gguf \
     llama-http-server.wasm
   ```

4. **OpenClaw+ 连接到这个服务**
   ```rust
   InferenceConfig {
       backend: BackendKind::OpenAiCompat,
       endpoint: "http://localhost:8080".into(),
       // ...
   }
   ```

### 为什么不推荐

- ❌ **仍可能遇到相同的内存问题** - WASI-NN 插件加载模型时的限制依然存在
- ❌ **增加复杂度** - 需要维护额外的 WASM 模块
- ❌ **性能不如原生** - WASM 运行时开销
- ✅ **Ollama 更简单** - 已有成熟方案，无需重复造轮子

## 测试结果对比

| 方案 | 内存限制 | GPU 加速 | 性能 | 复杂度 | 状态 |
|------|----------|----------|------|--------|------|
| WASI-NN (嵌入式) | ❌ SIGKILL | ❌ | - | 低 | ❌ 失败 |
| WASI-NN (独立进程) | ⚠️ 可能失败 | ❌ | 低 | 高 | ⚠️ 未测试 |
| Ollama | ✅ 无限制 | ✅ Metal | 高 | 低 | ✅ 已验证 |
| llama.cpp server | ✅ 无限制 | ✅ Metal | 高 | 中 | ✅ 可用 |

## 结论

**推荐使用 Ollama（方案 1）**，原因：
1. ✅ 已验证可用
2. ✅ 性能最好（GPU 加速）
3. ✅ 最简单（无需额外开发）
4. ✅ 生产就绪

**不推荐** WasmEdge 独立进程方案，因为：
1. ❌ 仍可能遇到相同问题
2. ❌ 增加不必要的复杂度
3. ❌ 性能不如原生方案

## 快速验证

运行已有的 HTTP backend demo：

```bash
# 确保 Ollama 正在运行
ollama serve

# 运行 demo
cargo run --release -p openclaw-inference --example http_backend_demo
```

预期输出：
```
✓ Test 1: Simple question (4959ms)
✓ Test 2: Follow-up (334ms)
✓ Test 3: Code generation (2595ms)
```

---

**最终建议**：使用 Ollama 作为独立推理服务，OpenClaw+ 通过 HTTP 后端连接。这是最简单、最可靠、性能最好的方案。
