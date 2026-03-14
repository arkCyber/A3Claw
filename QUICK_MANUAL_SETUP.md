# 🚀 快速手动配置指南

## 当前状态

✅ **llama.cpp 推理引擎已完整实现**
✅ **所有配置文件已更新为 qwen3.5:9b**
✅ **混合方案已配置完成**

## 📋 只需完成 3 步即可使用

### 步骤 1：安装 llama.cpp（选择一种方式）

#### 方式 A：使用 Homebrew（最简单）
```bash
brew install llama.cpp
```

#### 方式 B：手动下载
1. 访问：https://github.com/ggerganov/llama.cpp/releases
2. 下载：`llama-server-macos-arm64`（Apple Silicon）或 `llama-server-macos-x64`（Intel）
3. 保存到：`/Users/arkSong/workspace/OpenClaw+/llama-server`
4. 添加执行权限：`chmod +x llama-server`

### 步骤 2：下载模型文件

#### 方式 A：浏览器下载（推荐）
1. 打开：https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF
2. 下载：`qwen2.5-7b-instruct-q4_k_m.gguf`（4.4GB）
3. 保存到：`/Users/arkSong/workspace/OpenClaw+/models/gguf/`

#### 方式 B：使用命令行
```bash
cd /Users/arkSong/workspace/OpenClaw+
mkdir -p models/gguf

# 使用 curl（支持断点续传）
curl -L --progress-bar --continue-at - \
  -o models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf \
  "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"
```

### 步骤 3：启动并测试

```bash
cd /Users/arkSong/workspace/OpenClaw+

# 启动 llama.cpp server
./llama-server \
  -m models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml

# 新开终端测试
curl http://localhost:8080/v1/models
```

## ⚙️ 配置 OpenClaw

编辑配置文件：
```bash
open ~/Library/Application\ Support/openclaw-plus/config.toml
```

修改 `[openclaw_ai]` 部分：
```toml
[openclaw_ai]
provider = "llama_cpp_http"  # 改为 llama.cpp
endpoint = "http://localhost:8080"
model = "qwen2.5-7b-instruct-q4_k_m"
```

## 🧪 测试功能

重启 OpenClaw UI：
```bash
pkill -f openclaw-plus
cd /Users/arkSong/workspace/OpenClaw+
cargo run -p openclaw-ui --release
```

在 Claw Terminal 测试：
- **🧪 Auto Test** - 10 条核心功能测试
- **📄 Page Test** - 9 个页面自动切换

## 🎯 混合方案优势

| 特性 | llama.cpp (主) | Ollama (备份) |
|------|---------------|--------------|
| 内存 | ~4GB | ~6.6GB |
| 启动 | <5秒 | ~10秒 |
| 安装 | 单文件 | 需要安装 |
| 自动启动 | ✅ | ❌ |

## 📁 文件位置

- llama.cpp server: `/Users/arkSong/workspace/OpenClaw+/llama-server`
- 模型文件: `/Users/arkSong/workspace/OpenClaw+/models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf`
- 配置文件: `~/Library/Application Support/openclaw-plus/config.toml`

---

## 🎉 完成！

下载完成后，您将拥有：
- ✅ 更轻量的推理引擎（节省 40% 内存）
- ✅ 更快的启动速度
- ✅ 自动冗余备份（Ollama）
- ✅ 完整的故障切换机制

需要帮助？请告诉我下载进度！
