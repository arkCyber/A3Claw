# 混合推理引擎快速启动指南

## ✅ 系统检查结果

您的 OpenClaw+ 系统**已完整实现** llama.cpp 推理功能，包括：

- ✅ **HTTP Backend** - 完整的 llama.cpp/Ollama/OpenAI 支持
- ✅ **熔断器机制** - 自动故障检测和隔离
- ✅ **冗余切换** - 主后端失败时自动切换到备份
- ✅ **健康监控** - 实时监控所有后端状态
- ✅ **审计日志** - 完整的推理请求追踪

**代码位置**：
- `crates/inference/src/backend.rs` - HTTP 后端实现
- `crates/inference/src/circuit_breaker.rs` - 熔断器
- `crates/inference/src/engine.rs` - 推理引擎
- `examples/multi_backend_fallback.rs` - 冗余示例

---

## 🎯 混合方案配置（推荐）

```
Primary:  llama.cpp (http://localhost:8080) - 自动启动，轻量，4GB 内存
Backup:   Ollama    (http://localhost:11434) - 手动启动，Ollama 更新后使用
```

---

## 📥 步骤 1：下载 Qwen2.5-7B GGUF 模型

由于 Qwen3.5 的 GGUF 模型尚未发布，我们先使用 **Qwen2.5-7B-Instruct**（性能接近）。

### 方法 A：使用浏览器下载（推荐）

1. 打开浏览器访问：
   ```
   https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF
   ```

2. 点击 **Files and versions** 标签

3. 下载文件（选择一个）：
   - `qwen2.5-7b-instruct-q4_k_m.gguf` (4.4GB) - **推荐**，内存占用低
   - `qwen2.5-7b-instruct-q8_0.gguf` (7.7GB) - 更高质量

4. 将下载的文件移动到：
   ```
   /Users/arkSong/workspace/OpenClaw+/models/gguf/
   ```

### 方法 B：使用命令行下载

```bash
cd /Users/arkSong/workspace/OpenClaw+
mkdir -p models/gguf

# 下载 Q4_K_M 版本（推荐）
curl -L --progress-bar \
  "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf" \
  -o models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf
```

---

## 📥 步骤 2：下载 llama.cpp server

### 方法 A：使用 Homebrew（最简单）

```bash
brew install llama.cpp
# llama-server 将安装到 /opt/homebrew/bin/llama-server
```

### 方法 B：手动下载预编译版本

1. 访问：https://github.com/ggerganov/llama.cpp/releases
2. 下载最新的 macOS ARM64 版本
3. 解压并复制 `llama-server` 到项目根目录
4. 添加执行权限：`chmod +x llama-server`

---

## 🚀 步骤 3：启动 llama.cpp server

```bash
cd /Users/arkSong/workspace/OpenClaw+

# 使用项目提供的启动脚本
./scripts/start_llama_server.sh

# 或手动启动
./llama-server \
  -m models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml
```

### 验证 llama.cpp 运行

```bash
# 测试连接
curl http://localhost:8080/v1/models

# 测试推理
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5-7b-instruct-q4_k_m",
    "messages": [{"role": "user", "content": "你好，介绍一下你自己"}],
    "max_tokens": 100
  }'
```

---

## ⚙️ 步骤 4：配置 OpenClaw 使用 llama.cpp

编辑配置文件：
```bash
open ~/Library/Application\ Support/openclaw-plus/config.toml
```

修改 `[openclaw_ai]` 部分：

```toml
[openclaw_ai]
provider = "llama_cpp_http"  # 从 "ollama" 改为 "llama_cpp_http"
endpoint = "http://localhost:8080"
model = "qwen2.5-7b-instruct-q4_k_m"
api_key = ""
max_tokens = 4096
temperature = 0.7
stream = false
```

---

## 🧪 步骤 5：重启 OpenClaw UI 并测试

```bash
cd /Users/arkSong/workspace/OpenClaw+

# 停止当前 UI
pkill -f openclaw-plus

# 重新编译并启动
cargo run -p openclaw-ui --release
```

### 测试功能

在 Claw Terminal 页面：

1. **🧪 Auto Test** - 测试 10 条核心 AI 功能
2. **📄 Page Test** - 测试 9 个 UI 页面自动切换

---

## 🔄 冗余切换测试

### 测试场景 1：正常运行
- llama.cpp 运行 → 所有请求由 llama.cpp 处理
- 查看日志确认使用的是 llama.cpp

### 测试场景 2：主引擎故障
```bash
# 停止 llama.cpp
./scripts/stop_llama_server.sh

# 启动 Ollama（如果已更新到支持 Qwen3.5）
ollama serve

# OpenClaw 会自动切换到 Ollama 备份引擎
```

### 测试场景 3：自动恢复
```bash
# 重启 llama.cpp
./scripts/start_llama_server.sh

# 熔断器会自动恢复，流量切回 llama.cpp
```

---

## 📊 性能对比

| 指标 | llama.cpp (Q4_K_M) | Ollama |
|------|-------------------|--------|
| 内存占用 | ~4GB | ~6.6GB |
| 启动时间 | <5秒 | ~10秒 |
| 推理速度 | 快 | 快 |
| 安装难度 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| 模型管理 | 文件系统 | `ollama pull` |

---

## 🛠️ 管理命令

```bash
# 启动 llama.cpp
./scripts/start_llama_server.sh

# 停止 llama.cpp
./scripts/stop_llama_server.sh

# 查看日志
tail -f logs/llama-server.log

# 检查进程
ps aux | grep llama-server

# 测试连接
curl http://localhost:8080/v1/models
```

---

## ❓ 常见问题

### Q: 为什么使用 Qwen2.5-7B 而不是 Qwen3.5-9B？
A: Qwen3.5 的 GGUF 格式模型尚未发布。Qwen2.5-7B 性能已经很好，等 Qwen3.5 GGUF 发布后可以直接替换模型文件。

### Q: 如何切换回 Ollama？
A: 修改 `config.toml` 中的 `provider = "ollama"` 并重启 UI。

### Q: llama.cpp 和 Ollama 可以同时运行吗？
A: 可以！它们使用不同的端口（8080 vs 11434），系统会根据配置选择主引擎。

### Q: 如何更新模型？
A: 下载新的 GGUF 文件，替换 `models/gguf/` 中的文件，重启 llama-server。

---

## 🎉 完成！

现在您的系统已配置为：
- ✅ **主引擎**：llama.cpp (轻量、自动启动)
- ✅ **备份引擎**：Ollama (手动启动)
- ✅ **自动冗余切换**：主引擎故障时自动切换
- ✅ **熔断器保护**：防止级联故障

享受更快、更轻量的 AI 推理体验！
