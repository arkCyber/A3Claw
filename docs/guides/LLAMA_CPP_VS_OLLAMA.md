# llama.cpp vs Ollama 对比分析

## 当前配置状态

### ✅ 已更新为 `qwen3.5:9b` 的文件
1. `~/Library/Application Support/openclaw-plus/config.toml`
2. `crates/security/src/config.rs` (代码默认值)
3. `agents/*.toml` (5个agent配置)
4. `config/servers.toml`
5. `config/inference_redundancy.toml`
6. `config/inference.toml`
7. `test_agent_profile.toml`

### 📊 推理引擎对比

| 特性 | llama.cpp | Ollama |
|------|-----------|--------|
| **安装难度** | ⭐⭐⭐⭐⭐ 单个二进制文件 | ⭐⭐⭐ 需要安装应用 |
| **模型格式** | GGUF (直接加载) | 需要 pull 下载 |
| **内存占用** | 更低 (Q4量化 ~4GB) | 较高 (~6.6GB) |
| **启动方式** | 可自动启动 | 需要手动启动 |
| **API兼容** | OpenAI 兼容 | Ollama 专有 API |
| **更新频率** | 手动更新模型文件 | `ollama pull` 自动更新 |
| **GPU支持** | 支持 (Metal/CUDA) | 支持 (Metal/CUDA) |
| **模型共享** | 文件系统直接复制 | 需要 push/pull |

## 推荐方案

### 方案 A：优先使用 llama.cpp（推荐）

**优势**：
- ✅ **无需等待 Ollama 更新**（当前 Ollama 0.7.0 不支持 Qwen3.5）
- ✅ **更轻量**：单个二进制 + GGUF 模型文件
- ✅ **可自动启动**：系统启动时自动运行
- ✅ **更低内存**：Q4 量化约 4GB vs Ollama 6.6GB
- ✅ **易于部署**：复制文件即可，无需安装

**需要做的**：
1. 下载 llama.cpp server (llama-server)
2. 下载 Qwen3.5-9B-Instruct GGUF 模型
3. 修改配置将 llama.cpp 设为主引擎

### 方案 B：继续使用 Ollama（当前方案）

**优势**：
- ✅ **模型管理简单**：`ollama pull/list/rm`
- ✅ **官方支持**：Ollama 官方维护
- ✅ **自动优化**：针对不同硬件自动优化

**劣势**：
- ❌ **需要更新 Ollama**：当前版本不支持 Qwen3.5
- ❌ **无法自动启动**：需要手动运行 `ollama serve`
- ❌ **内存占用更高**

## 实施建议

### 立即可行：切换到 llama.cpp

```bash
# 1. 下载 llama.cpp server
curl -L https://github.com/ggerganov/llama.cpp/releases/latest/download/llama-server-macos-arm64 -o llama-server
chmod +x llama-server

# 2. 下载 Qwen3.5-9B-Instruct GGUF 模型
mkdir -p models/gguf
curl -L https://huggingface.co/Qwen/Qwen3.5-9B-Instruct-GGUF/resolve/main/qwen3.5-9b-instruct-q4_k_m.gguf \
  -o models/gguf/qwen3.5-9b-instruct-q4_k_m.gguf

# 3. 启动 llama.cpp server
./llama-server -m models/gguf/qwen3.5-9b-instruct-q4_k_m.gguf \
  --port 8080 --host 0.0.0.0 -ngl 99 --ctx-size 8192
```

### 配置修改

修改 `~/Library/Application Support/openclaw-plus/config.toml`:

```toml
[openclaw_ai]
provider = "llama_cpp_http"  # 从 "ollama" 改为 "llama_cpp_http"
endpoint = "http://localhost:8080"
model = "qwen3.5-9b-instruct-q4_k_m"
```

## 混合方案（最佳实践）

使用冗余配置，两者都支持：

1. **Primary**: llama.cpp (自动启动，轻量)
2. **Backup**: Ollama (手动启动，更新后使用)

这样：
- 平时使用 llama.cpp（更轻量，自动启动）
- Ollama 更新后可以切换回去
- 如果一个失败，自动切换到另一个

## 结论

**建议立即切换到 llama.cpp**，原因：
1. 无需等待 Ollama 更新
2. 可以立即使用 Qwen3.5-9B
3. 更低的资源占用
4. 更方便的部署和管理

等 Ollama 更新到支持 Qwen3.5 的版本后，可以再评估是否切换回去。
