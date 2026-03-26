# OpenClaw+ 完整演示指南

## 🎯 概述

本指南将带你完整体验 OpenClaw+ 的服务器管理功能，从安装到实际使用。

## ✅ 已完成的功能

### 核心功能
- ✅ **UI 界面集成** - 在通用设置中完整的服务器管理界面
- ✅ **命令行工具** - `server-ctl` 完整功能
- ✅ **JSON 输出** - UI 和程序可以使用的结构化输出
- ✅ **服务器管理** - 启动、停止、重启、健康检查
- ✅ **实用脚本** - 自动化启动和健康检查脚本

### 支持的服务器类型
- ✅ **Ollama** - 本地 LLM 推理（外部启动）
- ✅ **llama.cpp** - 轻量级推理服务器（内部管理）
- ✅ **OpenAI API** - 云端 API（配置管理）
- ✅ **自定义服务器** - 灵活配置

## 🚀 完整演示流程

### 步骤 1: 准备环境

#### 1.1 确认编译状态
```bash
# 确认 UI 已编译
ls -lh target/release/openclaw-plus

# 确认 server-ctl 已编译
ls -lh target/release/server-ctl

# 如果未编译，执行：
cargo build --release -p openclaw-ui
cargo build --release --bin server-ctl
```

#### 1.2 检查 Ollama 安装
```bash
# 检查 Ollama
which ollama

# 如果未安装：
brew install ollama

# 查看已安装的模型
ollama list
```

### 步骤 2: 启动服务器

#### 2.1 启动 Ollama（推荐）
```bash
# 使用我们的脚本启动
./scripts/start-ollama.sh

# 或手动启动
ollama serve &

# 验证 Ollama 运行
curl http://localhost:11434/api/tags
```

#### 2.2 查看服务器列表
```bash
# 人类可读格式
./target/release/server-ctl list

# JSON 格式（UI 使用）
./target/release/server-ctl list --json | jq '.'
```

### 步骤 3: 在 UI 中测试

#### 3.1 启动 UI
```bash
./target/release/openclaw-plus
```

#### 3.2 导航到服务器管理
1. 点击左侧导航栏的 **"General Settings"**
2. 向下滚动到 **"Inference Server Management"** 部分
3. 点击 **"⟳ Refresh"** 刷新服务器列表

#### 3.3 查看服务器状态
你应该看到：
- **llama.cpp (备份)** - http://localhost:8080 - 🔴 Stopped
- **Ollama (主服务)** - http://localhost:11434 - 🟢 Running（如果已启动）

#### 3.4 测试控制按钮
- 对于 Running 的服务器：可以点击 **Stop** 或 **Restart**
- 对于 Stopped 的服务器：可以点击 **Start**

### 步骤 4: 命令行测试

#### 4.1 查看服务器详细状态
```bash
./target/release/server-ctl status ollama-primary
```

#### 4.2 健康检查
```bash
./target/release/server-ctl health ollama-primary
```

#### 4.3 使用健康检查脚本
```bash
./scripts/health-check.sh
```

### 步骤 5: 测试推理功能

#### 5.1 使用 Ollama 进行推理
```bash
# 简单测试
curl http://localhost:11434/api/generate -d '{
  "model": "qwen3.5:9b",
  "prompt": "Hello, how are you?",
  "stream": false
}'

# 或使用 ollama 命令
ollama run qwen3.5:9b "Hello, how are you?"
```

#### 5.2 在 UI 中测试 AI Chat
1. 导航到 **AI Chat** 页面
2. 确保 AI 后端配置为 Ollama
3. 发送测试消息

### 步骤 6: 完整演示脚本

运行完整的交互式演示：
```bash
./scripts/demo-server-management.sh
```

这个脚本会：
1. 列出所有服务器
2. 显示 JSON 格式输出
3. 检查 Ollama 安装状态
4. 查看服务器详细状态
5. 可选：启动 Ollama 服务器
6. 执行健康检查
7. 显示最终状态

## 📊 验证清单

### UI 功能验证
- [ ] UI 成功启动
- [ ] 能看到 "Inference Server Management" 部分
- [ ] 点击 Refresh 能刷新服务器列表
- [ ] 服务器状态正确显示（Running/Stopped）
- [ ] 控制按钮可以点击
- [ ] 服务器信息显示完整（名称、端点、状态）

### 命令行功能验证
- [ ] `server-ctl list` 正常工作
- [ ] `server-ctl list --json` 输出有效 JSON
- [ ] `server-ctl status <id>` 显示详细信息
- [ ] `server-ctl health <id>` 执行健康检查
- [ ] 脚本可以正常执行

### 推理功能验证
- [ ] Ollama 服务器可以启动
- [ ] 可以通过 API 进行推理
- [ ] UI 中的 AI Chat 可以使用
- [ ] 模型列表正确显示

## 🎬 演示场景

### 场景 1: 日常使用
```bash
# 1. 启动 Ollama
./scripts/start-ollama.sh

# 2. 启动 UI
./target/release/openclaw-plus

# 3. 在 UI 中查看和管理服务器
# 4. 使用 AI Chat 进行对话
```

### 场景 2: 开发调试
```bash
# 1. 查看所有服务器状态
./target/release/server-ctl list

# 2. 检查特定服务器
./target/release/server-ctl status ollama-primary

# 3. 执行健康检查
./scripts/health-check.sh

# 4. 查看日志
tail -f /tmp/openclaw.log
```

### 场景 3: 自动化部署
```bash
# 1. 使用启动脚本
./scripts/start-servers.sh

# 2. 验证所有服务器
./scripts/health-check.sh

# 3. 启动 UI
./target/release/openclaw-plus
```

## 🔧 故障排查

### 问题 1: UI 中看不到服务器列表
**解决方案**：
```bash
# 测试 server-ctl
./target/release/server-ctl list --json

# 检查 UI 日志
tail -f /tmp/openclaw.log

# 确保 server-ctl 在正确位置
ls -l target/release/server-ctl
```

### 问题 2: Ollama 无法启动
**解决方案**：
```bash
# 检查 Ollama 是否已安装
which ollama

# 检查端口占用
lsof -i :11434

# 手动启动 Ollama
ollama serve

# 验证运行
curl http://localhost:11434/api/tags
```

### 问题 3: 健康检查失败
**解决方案**：
```bash
# 确认服务器正在运行
./target/release/server-ctl list

# 手动测试端点
curl http://localhost:11434/api/tags

# 查看详细状态
./target/release/server-ctl status ollama-primary
```

## 📝 配置说明

### 服务器配置文件
位置：`config/servers.toml`

```toml
# Ollama 配置（外部启动）
[servers.ollama-primary]
id = "ollama-primary"
name = "Ollama (主服务)"
type = "Ollama"
endpoint = "http://localhost:11434"
port = 11434
auto_start = false  # Ollama 需要外部启动
enabled = true

# llama.cpp 配置（内部管理）
[servers.llama-cpp-backup]
id = "llama-cpp-backup"
name = "llama.cpp (备份)"
type = "LlamaCpp"
endpoint = "http://localhost:8080"
port = 8080
model_path = "models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf"
auto_start = true  # 可以由 server-ctl 启动
enabled = true
```

## 🎉 成功标准

演示成功的标志：

1. ✅ UI 正常启动并显示服务器管理界面
2. ✅ 可以在 UI 中看到配置的服务器
3. ✅ 服务器状态正确显示（Running/Stopped）
4. ✅ 可以通过 UI 或命令行刷新服务器列表
5. ✅ Ollama 服务器可以正常运行
6. ✅ 可以通过 API 进行推理测试
7. ✅ 所有脚本可以正常执行
8. ✅ 健康检查功能正常工作

## 📚 相关文档

- [快速开始指南](../QUICK_START.md)
- [服务器管理测试指南](./SERVER_MANAGEMENT_TEST_GUIDE.md)
- [项目状态总结](./PROJECT_STATUS_SUMMARY.md)
- [服务器管理器使用指南](./SERVER_MANAGER_GUIDE.md)

## 💡 下一步

完成演示后，你可以：

1. **配置更多服务器**
   - 添加 OpenAI API 配置
   - 添加 DeepSeek API 配置
   - 配置自定义推理服务器

2. **集成到工作流**
   - 设置自动启动
   - 配置健康检查定时任务
   - 集成到 CI/CD 流程

3. **扩展功能**
   - 添加更多推理后端
   - 实现自动故障转移
   - 添加性能监控

---

**准备好了吗？开始你的 OpenClaw+ 服务器管理之旅！** 🚀
