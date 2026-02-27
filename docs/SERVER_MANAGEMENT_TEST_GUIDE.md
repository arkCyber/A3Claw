# OpenClaw+ 服务器管理功能测试指南

## 📋 功能概述

OpenClaw+ 现已集成完整的推理服务器管理功能，支持通过 UI 界面和命令行工具管理本地和远程推理服务器。

## ✅ 已完成的功能

### 1. **服务器管理 UI**
- ✅ 在"通用设置"页面集成服务器管理部分
- ✅ 显示服务器列表（名称、类型、端点、状态）
- ✅ 服务器控制按钮（启动、停止、重启）
- ✅ 刷新服务器列表功能
- ✅ 实时状态更新

### 2. **命令行工具 (server-ctl)**
- ✅ 列出所有服务器 (`list`)
- ✅ JSON 格式输出 (`list --json`)
- ✅ 查看服务器状态 (`status <id>`)
- ✅ 启动服务器 (`start <id>`)
- ✅ 停止服务器 (`stop <id>`)
- ✅ 重启服务器 (`restart <id>`)
- ✅ 健康检查 (`health <id>`)
- ✅ 批量操作 (`start-all`, `stop-all`)

### 3. **后端集成**
- ✅ ServerManager 核心管理器
- ✅ 进程管理和监控
- ✅ 健康检查机制
- ✅ 资源使用统计（CPU、内存）
- ✅ 配置文件支持 (`config/servers.toml`)

## 🧪 测试步骤

### 步骤 1: 在 UI 中查看服务器管理界面

1. **启动 OpenClaw+ UI**
   ```bash
   ./target/release/openclaw-plus
   ```

2. **导航到通用设置**
   - 在左侧导航栏点击 "General Settings"（通用设置）
   - 向下滚动到 "Inference Server Management" 部分

3. **查看服务器列表**
   - 点击 "⟳ Refresh" 按钮刷新服务器列表
   - 应该看到两个服务器：
     - `llama.cpp (备份)` - http://localhost:8080
     - `Ollama (主服务)` - http://localhost:11434

### 步骤 2: 测试命令行工具

1. **列出所有服务器**
   ```bash
   ./target/release/server-ctl list
   ```
   
   预期输出：
   ```
   === 推理服务器列表 ===

   🔴 llama.cpp (备份) (llama-cpp-backup)
      类型: LlamaCpp
      端点: http://localhost:8080
      状态: Stopped

   🔴 Ollama (主服务) (ollama-primary)
      类型: Ollama
      端点: http://localhost:11434
      状态: Stopped
   ```

2. **JSON 格式输出（供 UI 使用）**
   ```bash
   ./target/release/server-ctl list --json
   ```
   
   预期输出：JSON 数组格式的服务器列表

3. **查看特定服务器状态**
   ```bash
   ./target/release/server-ctl status llama-cpp-backup
   ```

### 步骤 3: 测试服务器启动功能

**注意**：启动 llama.cpp 服务器需要以下条件：

1. **llama.cpp 服务器二进制文件**
   - 需要编译或下载 llama.cpp 的 `server` 可执行文件
   - 默认路径：系统 PATH 中的 `llama-server` 或 `server`

2. **模型文件**
   - 默认模型路径：`models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf`
   - 可以在 `config/servers.toml` 中修改模型路径

3. **启动服务器**
   ```bash
   # 通过命令行启动
   ./target/release/server-ctl start llama-cpp-backup
   
   # 或在 UI 中点击 "Start" 按钮
   ```

4. **验证服务器运行**
   ```bash
   # 检查进程
   ps aux | grep llama
   
   # 健康检查
   ./target/release/server-ctl health llama-cpp-backup
   
   # 或访问
   curl http://localhost:8080/health
   ```

### 步骤 4: 测试 Ollama 服务器

1. **确保 Ollama 已安装**
   ```bash
   which ollama
   ```

2. **启动 Ollama**
   ```bash
   # 通过 server-ctl
   ./target/release/server-ctl start ollama-primary
   
   # 或直接启动 Ollama
   ollama serve
   ```

3. **验证 Ollama 运行**
   ```bash
   curl http://localhost:11434/api/tags
   ```

## 📝 配置文件说明

### config/servers.toml

```toml
# 当前配置支持两个服务器：

[servers.llama-cpp-backup]
id = "llama-cpp-backup"
name = "llama.cpp (备份)"
type = "LlamaCpp"
endpoint = "http://localhost:8080"
port = 8080
model_path = "models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf"
auto_start = true
enabled = true

[servers.ollama-primary]
id = "ollama-primary"
name = "Ollama (主服务)"
type = "Ollama"
endpoint = "http://localhost:11434"
port = 11434
auto_start = false
enabled = true
```

## 🔧 故障排查

### 问题 1: UI 中看不到服务器列表

**解决方案**：
1. 检查 server-ctl 是否正常工作：
   ```bash
   ./target/release/server-ctl list --json
   ```
2. 查看 UI 日志：
   ```bash
   tail -f /tmp/openclaw.log
   ```
3. 确保 server-ctl 在正确的路径

### 问题 2: 无法启动 llama.cpp 服务器

**可能原因**：
- llama.cpp 服务器二进制文件不存在
- 模型文件路径不正确
- 端口 8080 已被占用

**解决方案**：
1. 检查 llama.cpp 安装：
   ```bash
   which llama-server
   ```
2. 检查模型文件：
   ```bash
   ls -lh models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf
   ```
3. 检查端口占用：
   ```bash
   lsof -i :8080
   ```

### 问题 3: 服务器状态不更新

**解决方案**：
- 点击 UI 中的 "⟳ Refresh" 按钮
- 或重新执行 `server-ctl list`

## 📊 当前状态

### ✅ 已实现
- [x] UI 界面集成
- [x] 服务器列表显示
- [x] JSON 输出支持
- [x] 启动/停止/重启控制
- [x] 命令行工具完整功能
- [x] 配置文件支持

### 🚧 待完善（可选）
- [ ] llama.cpp 二进制文件自动下载
- [ ] 模型文件管理
- [ ] 服务器日志查看
- [ ] 性能监控图表
- [ ] 自动健康检查
- [ ] 服务器配置编辑器

## 🎯 下一步建议

1. **准备 llama.cpp 环境**
   - 编译或下载 llama.cpp 服务器
   - 下载测试模型文件

2. **测试完整流程**
   - 启动 Ollama 服务器
   - 启动 llama.cpp 服务器
   - 在 UI 中验证状态
   - 测试推理功能

3. **集成到工作流**
   - 配置自动启动
   - 设置健康检查
   - 配置故障转移

## 📚 相关文档

- [服务器管理器实现总结](./SERVER_MANAGER_SUMMARY.md)
- [服务器管理器使用指南](./SERVER_MANAGER_GUIDE.md)
- [云端 API 集成指南](./CLOUD_API_INTEGRATION.md)

## 🎉 总结

OpenClaw+ 的服务器管理功能已经完整实现并集成到 UI 中！你现在可以：

1. ✅ 在 UI 中查看和管理所有推理服务器
2. ✅ 使用命令行工具进行批量操作
3. ✅ 监控服务器状态和资源使用
4. ✅ 通过配置文件管理服务器

**UI 已启动，请在"通用设置"页面中测试服务器管理功能！** 🚀
