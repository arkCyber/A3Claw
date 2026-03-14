# 🔧 OpenClaw UI 输入框焦点问题修复指南

## 问题描述

在使用 OpenClaw UI 时，输入焦点会跑到 Windsurf 的输入框，导致无法在 OpenClaw UI 中正常输入。

---

## 🎯 立即解决方案

### 方案 1: 使用独立终端启动（推荐）⭐

**不要在 Windsurf 内置终端启动 UI**，而是使用系统独立终端：

```bash
# 1. 打开 macOS 终端（Terminal.app 或 iTerm2）
# 2. 进入项目目录
cd /Users/arkSong/workspace/OpenClaw+

# 3. 使用启动脚本（自动检查 AI 服务）
./scripts/start_openclaw_ui.sh

# 或者直接运行
RUST_LOG=openclaw_ui=info cargo run -p openclaw-ui --release
```

**为什么这样有效**：
- ✅ 独立终端不会与 Windsurf 争夺输入焦点
- ✅ UI 窗口可以独立获取键盘输入
- ✅ 避免 IDE 的输入拦截

---

### 方案 2: 在 Windsurf 中使用但点击输入框

如果必须在 Windsurf 终端启动：

1. 启动 UI 后，**用鼠标点击 OpenClaw UI 的输入框**
2. 确保 OpenClaw UI 窗口在最前面
3. 每次输入前都点击一下输入框

---

### 方案 3: 使用快捷键重新获取焦点

在 OpenClaw UI 中：
- 按 `Tab` 键切换焦点
- 或者点击输入框区域

---

## 🤖 AI 服务修复

### 当前状态

✅ **Ollama 正在运行**（进程 6911）  
✅ **可用模型**: llama3.2:latest (3.2B)  
✅ **配置已修复**: 已更新为使用 llama3.2

### 配置文件位置

```
~/Library/Application Support/openclaw-plus/config.toml
```

### 已修复的配置

```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "llama3.2"          # 已从 qwen3.5:9b 修复为 llama3.2
api_key = ""
max_tokens = 4096
temperature = 0.7
stream = true
```

---

## 🚀 完整启动流程

### 步骤 1: 检查 AI 服务

```bash
# 检查 Ollama 是否运行
ps aux | grep ollama

# 如果没有运行，启动它
ollama serve &

# 检查可用模型
curl http://localhost:11434/api/tags
```

### 步骤 2: 使用启动脚本（推荐）

```bash
# 在独立终端中运行
cd /Users/arkSong/workspace/OpenClaw+
./scripts/start_openclaw_ui.sh
```

启动脚本会自动：
- ✅ 检查 Ollama 服务状态
- ✅ 验证可用模型
- ✅ 测试 AI 推理
- ✅ 启动 OpenClaw UI

### 步骤 3: 验证 AI 功能

在 OpenClaw UI 中：

1. 切换到 **AI Chat** 页面
2. 点击输入框
3. 输入测试消息：`你好，请介绍一下你自己`
4. 按 Enter 发送
5. 等待 AI 响应

---

## 🔍 故障排查

### 问题 1: AI 无响应

**检查**:
```bash
# 测试 Ollama API
curl http://localhost:11434/api/generate \
  -d '{"model":"llama3.2","prompt":"hi","stream":false}'
```

**解决**:
- 如果返回错误，重启 Ollama: `killall ollama && ollama serve &`
- 等待 3-5 秒让服务完全启动

### 问题 2: 模型不存在

**检查可用模型**:
```bash
ollama list
```

**下载模型**（如果需要）:
```bash
# 下载 llama3.2 (推荐，3.2B，快速)
ollama pull llama3.2

# 或下载 qwen2.5 (7B，更强大)
ollama pull qwen2.5:7b
```

### 问题 3: 输入框焦点丢失

**立即解决**:
1. 用鼠标点击 OpenClaw UI 的输入框
2. 确保 OpenClaw UI 窗口在最前面
3. 不要在输入时切换到其他窗口

**长期解决**:
- 使用独立终端启动 UI（不要用 Windsurf 终端）
- 或者使用 `cargo run --release` 后台运行

### 问题 4: 编译错误

**重新编译**:
```bash
cd /Users/arkSong/workspace/OpenClaw+
cargo clean -p openclaw-ui
cargo build -p openclaw-ui --release
```

---

## 📊 AI 模型选择建议

### 当前可用: llama3.2 (3.2B)

**优点**:
- ✅ 快速响应（~1-2秒）
- ✅ 内存占用小（~2GB）
- ✅ 适合日常对话

**缺点**:
- ⚠️ 能力有限
- ⚠️ 中文支持一般

### 推荐升级: qwen2.5:7b

**下载**:
```bash
ollama pull qwen2.5:7b
```

**修改配置**:
```toml
[openclaw_ai]
model = "qwen2.5:7b"  # 更改这一行
```

**优点**:
- ✅ 中文支持优秀
- ✅ 能力更强
- ✅ 适合复杂任务

**缺点**:
- ⚠️ 响应稍慢（~3-5秒）
- ⚠️ 内存占用大（~5GB）

---

## 🎯 快速启动命令

### 最简单的方式（推荐）

```bash
# 1. 打开 macOS 终端（不是 Windsurf）
# 2. 运行
cd /Users/arkSong/workspace/OpenClaw+ && ./scripts/start_openclaw_ui.sh
```

### 手动启动

```bash
# 终端 1: 启动 Ollama（如果未运行）
ollama serve

# 终端 2: 启动 OpenClaw UI
cd /Users/arkSong/workspace/OpenClaw+
RUST_LOG=openclaw_ui=info cargo run -p openclaw-ui --release
```

---

## ✅ 验证清单

启动后检查：

- [ ] Ollama 服务正在运行
- [ ] OpenClaw UI 窗口已打开
- [ ] 可以在输入框中输入文字
- [ ] AI Chat 页面可以发送消息
- [ ] AI 有响应返回
- [ ] Claw Terminal 可以执行命令

---

## 💡 使用技巧

### 输入焦点管理

1. **启动后立即点击输入框**
2. **每次切换页面后点击输入框**
3. **使用 Tab 键在控件间切换**

### AI 对话技巧

1. **简短明确的提问**
2. **等待完整响应**
3. **使用中文或英文都可以**

### 性能优化

1. **使用 release 模式**: `--release`
2. **关闭不需要的日志**: 移除 `RUST_LOG`
3. **使用更小的模型**: llama3.2 比 qwen2.5:7b 快

---

## 🆘 紧急救援

如果一切都不工作：

```bash
# 1. 停止所有服务
killall ollama
killall openclaw-plus

# 2. 清理并重启
cd /Users/arkSong/workspace/OpenClaw+
cargo clean -p openclaw-ui

# 3. 重启 Ollama
ollama serve > /tmp/ollama.log 2>&1 &
sleep 3

# 4. 验证模型
ollama list

# 5. 重新编译和启动
cargo build -p openclaw-ui --release
cargo run -p openclaw-ui --release
```

---

**最后更新**: 2026-03-09  
**状态**: ✅ AI 配置已修复，可以使用
