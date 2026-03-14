# 🤖 OpenClaw AI 模型设置指南

## 当前状态

✅ **配置已更新**: 使用 `qwen3.5:9b`  
⚠️ **模型未下载**: 需要下载约 5.5GB 的模型文件

---

## 🚀 快速启动（推荐）

### 方法 1: 使用自动启动脚本

```bash
cd /Users/arkSong/workspace/OpenClaw+
./START_UI.sh
```

这个脚本会：
1. ✅ 自动加载 Rust 环境
2. ✅ 检查并启动 Ollama 服务
3. ✅ 检查 qwen3.5:9b 模型是否存在
4. ✅ 如果不存在，询问是否下载
5. ✅ 启动 OpenClaw UI

---

## 📥 手动下载 qwen3.5:9b 模型

### 选项 1: 使用下载脚本

```bash
cd /Users/arkSong/workspace/OpenClaw+
./download_qwen35.sh
```

### 选项 2: 直接使用 Ollama 命令

```bash
/opt/homebrew/bin/ollama pull qwen3.5:9b
```

**下载信息**:
- 模型大小: 约 5.5GB
- 下载时间: 取决于网络速度（通常 5-15 分钟）
- 存储位置: `~/.ollama/models/`

---

## 🔄 备选方案：使用已有的 llama3.2

如果不想下载 qwen3.5:9b，可以使用已安装的 llama3.2：

### 临时使用 llama3.2

启动脚本会自动检测，如果 qwen3.5 不存在会询问是否使用 llama3.2

### 永久切换到 llama3.2

编辑配置文件：
```bash
nano ~/Library/Application\ Support/openclaw-plus/config.toml
```

修改这一行：
```toml
model = "llama3.2"  # 从 qwen3.5:9b 改为 llama3.2
```

---

## 📊 模型对比

### qwen3.5:9b（推荐用于中文）

**优点**:
- ✅ 中文支持优秀
- ✅ 能力强大（9B 参数）
- ✅ 适合复杂任务
- ✅ 代码生成能力好

**缺点**:
- ⚠️ 文件较大（5.5GB）
- ⚠️ 响应稍慢（3-5秒）
- ⚠️ 内存占用大（~6GB）

### llama3.2（已安装，快速）

**优点**:
- ✅ 已安装，无需下载
- ✅ 响应快速（1-2秒）
- ✅ 内存占用小（~2GB）
- ✅ 适合简单对话

**缺点**:
- ⚠️ 中文支持一般
- ⚠️ 能力有限（3.2B 参数）
- ⚠️ 不适合复杂任务

---

## 🛠️ 完整启动流程

### 步骤 1: 下载模型（首次使用）

```bash
# 方法 A: 使用脚本
cd /Users/arkSong/workspace/OpenClaw+
./download_qwen35.sh

# 方法 B: 直接命令
/opt/homebrew/bin/ollama pull qwen3.5:9b
```

### 步骤 2: 验证模型已安装

```bash
/opt/homebrew/bin/ollama list
```

应该看到：
```
NAME              ID              SIZE      MODIFIED
qwen3.5:9b        ...             5.5 GB    ...
llama3.2:latest   ...             2.0 GB    ...
```

### 步骤 3: 启动 OpenClaw UI

```bash
cd /Users/arkSong/workspace/OpenClaw+
./START_UI.sh
```

---

## 🔍 故障排查

### 问题 1: 下载失败

**错误**: `connection refused` 或 `timeout`

**解决**:
```bash
# 检查网络连接
ping ollama.ai

# 重试下载
/opt/homebrew/bin/ollama pull qwen3.5:9b

# 或使用代理（如果需要）
export HTTP_PROXY=http://your-proxy:port
export HTTPS_PROXY=http://your-proxy:port
/opt/homebrew/bin/ollama pull qwen3.5:9b
```

### 问题 2: 模型下载很慢

**解决**:
- 使用有线网络而不是 WiFi
- 关闭其他下载任务
- 或者先使用 llama3.2，稍后再下载

### 问题 3: 磁盘空间不足

**检查空间**:
```bash
df -h ~
```

**清理空间**:
```bash
# 删除不需要的 Ollama 模型
/opt/homebrew/bin/ollama rm <model-name>

# 或清理 Docker/其他缓存
```

### 问题 4: Ollama 服务未运行

**检查**:
```bash
ps aux | grep ollama
```

**启动**:
```bash
/opt/homebrew/bin/ollama serve > /tmp/ollama.log 2>&1 &
```

---

## 📝 配置文件位置

```
~/Library/Application Support/openclaw-plus/config.toml
```

**当前 AI 配置**:
```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "qwen3.5:9b"          # ← 这里是模型名称
api_key = ""
max_tokens = 4096
temperature = 0.7
stream = true
```

---

## ✅ 验证 AI 功能

启动 UI 后：

1. 切换到 **AI Chat** 页面
2. 点击输入框
3. 输入测试消息：
   ```
   你好，请用中文介绍一下你自己
   ```
4. 按 Enter 发送
5. 等待 AI 响应

**预期响应时间**:
- qwen3.5:9b: 3-5 秒
- llama3.2: 1-2 秒

---

## 🎯 推荐配置

### 如果您主要使用中文

```toml
model = "qwen3.5:9b"  # 推荐
```

### 如果您需要快速响应

```toml
model = "llama3.2"    # 更快但中文支持一般
```

### 如果您需要平衡

考虑下载 `qwen2.5:7b`（中等大小，中文好，速度适中）:
```bash
/opt/homebrew/bin/ollama pull qwen2.5:7b
```

然后修改配置：
```toml
model = "qwen2.5:7b"
```

---

## 🚀 立即开始

**最简单的方式**:

```bash
cd /Users/arkSong/workspace/OpenClaw+
./START_UI.sh
```

脚本会自动处理一切！

---

**最后更新**: 2026-03-09  
**状态**: ✅ 配置已更新为 qwen3.5:9b
