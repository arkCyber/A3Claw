# OpenClaw+ 快速开始指南 🚀

## 🎯 当前状态

✅ **UI 已启动并运行**  
✅ **服务器管理功能已集成**  
✅ **命令行工具已就绪**  

## 📍 在 UI 中查看服务器管理

1. **打开 OpenClaw+ UI**（如果未启动）
   ```bash
   ./target/release/openclaw-plus
   ```

2. **导航到服务器管理**
   - 点击左侧导航栏的 **"General Settings"**（通用设置）
   - 向下滚动到 **"Inference Server Management"** 部分

3. **刷新服务器列表**
   - 点击 **"⟳ Refresh"** 按钮
   - 查看配置的服务器列表

## 🔧 命令行快速参考

```bash
# 列出所有服务器
./target/release/server-ctl list

# JSON 格式输出（供 UI 使用）
./target/release/server-ctl list --json

# 查看服务器状态
./target/release/server-ctl status llama-cpp-backup
./target/release/server-ctl status ollama-primary

# 启动服务器
./target/release/server-ctl start llama-cpp-backup
./target/release/server-ctl start ollama-primary

# 停止服务器
./target/release/server-ctl stop llama-cpp-backup

# 重启服务器
./target/release/server-ctl restart llama-cpp-backup

# 健康检查
./target/release/server-ctl health llama-cpp-backup

# 批量操作
./target/release/server-ctl start-all
./target/release/server-ctl stop-all
```

## 📋 配置的服务器

| 服务器 | ID | 端点 | 状态 |
|--------|----|----- |------|
| llama.cpp (备份) | `llama-cpp-backup` | http://localhost:8080 | Stopped |
| Ollama (主服务) | `ollama-primary` | http://localhost:11434 | Stopped |

## 📁 重要文件位置

- **UI 可执行文件**: `./target/release/openclaw-plus`
- **命令行工具**: `./target/release/server-ctl`
- **服务器配置**: `config/servers.toml`
- **UI 日志**: `/tmp/openclaw.log`

## 📚 详细文档

- **测试指南**: `docs/SERVER_MANAGEMENT_TEST_GUIDE.md`
- **项目状态**: `docs/PROJECT_STATUS_SUMMARY.md`
- **使用指南**: `docs/SERVER_MANAGER_GUIDE.md`

## 🎯 下一步操作

### 选项 1: 测试 UI 功能
1. 在 UI 中点击 "⟳ Refresh" 刷新服务器列表
2. 查看服务器状态显示
3. 尝试点击 Start/Stop 按钮（需要相应的服务器环境）

### 选项 2: 准备服务器环境
```bash
# 安装 Ollama（如果需要）
# macOS: brew install ollama
# 或访问 https://ollama.ai

# 启动 Ollama
ollama serve

# 在另一个终端测试
./target/release/server-ctl start ollama-primary
```

### 选项 3: 测试 llama.cpp
1. 编译或下载 llama.cpp 服务器
2. 准备模型文件
3. 更新 `config/servers.toml` 中的路径
4. 启动服务器

## 🐛 遇到问题？

1. **查看日志**
   ```bash
   tail -f /tmp/openclaw.log
   ```

2. **测试 server-ctl**
   ```bash
   ./target/release/server-ctl list
   ```

3. **检查端口占用**
   ```bash
   lsof -i :8080
   lsof -i :11434
   ```

## 💡 提示

- UI 会自动调用 `server-ctl` 工具
- 所有操作都有日志记录
- 支持 JSON 格式输出供程序使用
- 配置文件支持热重载

---

**准备就绪！开始测试你的服务器管理功能吧！** 🎉
