# 混合推理引擎配置指南

## ✅ 系统已具备的功能

您的 OpenClaw+ 系统**已完整实现** llama.cpp 推理引擎，包括：

### 核心功能
1. **HTTP Backend** - 支持 llama.cpp/Ollama/OpenAI 等多种后端
2. **熔断器机制** - 每个后端独立的故障检测和隔离
3. **自动冗余切换** - 主后端失败时自动切换到备份后端
4. **健康监控** - 实时监控所有后端的健康状态
5. **审计日志** - 完整的推理请求审计追踪

### 代码位置
- `crates/inference/src/backend.rs` - HTTP 后端实现
- `crates/inference/src/circuit_breaker.rs` - 熔断器实现
- `crates/inference/src/engine.rs` - 推理引擎和冗余切换
- `examples/multi_backend_fallback.rs` - 多后端冗余示例

---

## 🚀 混合方案配置步骤

### 方案架构
```
Primary:  llama.cpp (http://localhost:8080) - 自动启动，轻量
Backup:   Ollama    (http://localhost:11434) - 手动启动，更新后使用
```

### 步骤 1：下载 llama.cpp server

```bash
cd /Users/arkSong/workspace/OpenClaw+

# 方法 1：从 GitHub Releases 下载（推荐）
curl -L https://github.com/ggerganov/llama.cpp/releases/download/b4315/llama-b4315-bin-macos-arm64.zip -o llama.zip
unzip llama.zip
chmod +x llama-server

# 方法 2：使用 Homebrew
brew install llama.cpp
```

### 步骤 2：下载 Qwen3.5-9B GGUF 模型

```bash
# 创建模型目录
mkdir -p models/gguf

# 下载 Q4_K_M 量化版本（约 5.5GB，推荐）
curl -L "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf" \
  -o models/gguf/qwen3.5-9b-instruct-q4_k_m.gguf

# 或下载 Q8_0 版本（约 9GB，更高质量）
# curl -L "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q8_0.gguf" \
#   -o models/gguf/qwen3.5-9b-instruct-q8_0.gguf
```

**注意**：Qwen3.5 的 GGUF 模型可能还未发布，可以先使用 Qwen2.5-7B 作为替代。

### 步骤 3：启动 llama.cpp server

```bash
# 启动 llama.cpp server（端口 8080）
./llama-server \
  -m models/gguf/qwen3.5-9b-instruct-q4_k_m.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml

# 后台运行
nohup ./llama-server \
  -m models/gguf/qwen3.5-9b-instruct-q4_k_m.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml \
  > llama-server.log 2>&1 &
```

### 步骤 4：配置混合方案

修改 `~/Library/Application Support/openclaw-plus/config.toml`:

```toml
[openclaw_ai]
provider = "llama_cpp_http"  # 主引擎：llama.cpp
endpoint = "http://localhost:8080"
model = "qwen3.5-9b-instruct-q4_k_m"
api_key = ""
max_tokens = 4096
temperature = 0.7
stream = false
```

### 步骤 5：配置冗余备份（可选）

创建或修改 `config/inference_redundancy.toml`:

```toml
# 主服务：llama.cpp (自动启动)
[primary]
backend = "llama_cpp_http"
endpoint = "http://localhost:8080"
model_name = "qwen3.5-9b-instruct-q4_k_m"
max_tokens = 4096
temperature = 0.7
top_p = 0.95
inference_timeout_secs = 30
circuit_breaker_threshold = 3
circuit_breaker_reset_secs = 60
context_window = 8192

# 备份服务：Ollama (手动启动)
[backup_local]
backend = "ollama"
endpoint = "http://localhost:11434"
model_name = "qwen3.5:9b"  # Ollama 更新后使用
max_tokens = 4096
temperature = 0.7
top_p = 0.95
inference_timeout_secs = 30
circuit_breaker_threshold = 3
circuit_breaker_reset_secs = 60
context_window = 8192
```

---

## 🧪 测试混合方案

### 测试 llama.cpp 连接

```bash
# 测试 llama.cpp server 是否正常
curl http://localhost:8080/v1/models

# 测试推理
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3.5-9b-instruct-q4_k_m",
    "messages": [{"role": "user", "content": "你好"}],
    "max_tokens": 100
  }'
```

### 测试冗余切换

1. **正常情况**：llama.cpp 运行，所有请求由 llama.cpp 处理
2. **故障情况**：停止 llama.cpp，系统自动切换到 Ollama
3. **恢复情况**：重启 llama.cpp，熔断器自动恢复，流量切回

---

## 📊 混合方案优势

| 特性 | llama.cpp (主) | Ollama (备份) |
|------|---------------|--------------|
| **启动** | 自动启动 | 手动启动 |
| **内存** | ~4GB (Q4) | ~6.6GB |
| **模型** | GGUF 文件 | pull 下载 |
| **更新** | 手动替换文件 | `ollama pull` |
| **用途** | 日常使用 | 备份/测试 |

---

## 🔧 自动启动脚本（可选）

创建 `scripts/start_llama_server.sh`:

```bash
#!/bin/bash
cd /Users/arkSong/workspace/OpenClaw+

# 检查是否已运行
if pgrep -f "llama-server" > /dev/null; then
    echo "llama-server already running"
    exit 0
fi

# 启动 llama.cpp server
nohup ./llama-server \
  -m models/gguf/qwen3.5-9b-instruct-q4_k_m.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml \
  > llama-server.log 2>&1 &

echo "llama-server started on port 8080"
```

```bash
chmod +x scripts/start_llama_server.sh
```

---

## 🎯 下一步

1. 下载 llama.cpp server 和 Qwen GGUF 模型
2. 启动 llama.cpp server
3. 修改配置文件切换到 llama.cpp
4. 重启 OpenClaw UI
5. 测试 🧪 Auto Test 和 📄 Page Test

等 Ollama 更新到支持 Qwen3.5 的版本后，可以保留作为备份引擎。
