# OpenClaw+ 系统就绪指南

## ✅ 系统状态

**所有组件已成功集成并运行在单一进程中！**

```
┌────────────────────────────────────────┐
│    OpenClaw+ (单一进程架构)           │
│                                        │
│  ✓ Cosmic UI (libcosmic)               │
│  ✓ Plugin Gateway (内嵌 HTTP 7878)    │
│  ✓ AgentExecutor (ReAct 推理)         │
│  ✓ WasmEdge Sandbox (安全执行)        │
│  ✓ 中文输入支持 (IME)                 │
└────────────────────────────────────────┘
```

---

## 🚀 启动系统

### 简化启动（推荐）

```bash
cd /Users/arkSong/workspace/OpenClaw+
./scripts/run.sh
```

**启动过程**:
1. 编译 UI (release 模式)
2. 创建 macOS .app bundle
3. 启动 UI 进程
4. 自动启动内嵌 Gateway (端口 7878)
5. 自动探测 Gateway 就绪状态
6. 启用中文输入支持

---

## 🧪 验证系统状态

### 1. 检查 Gateway 健康状态

```bash
curl http://localhost:7878/health
# 预期输出: {"status":"ok"}
```

### 2. 检查 Gateway 技能列表

```bash
curl http://localhost:7878/skills/list | jq
```

### 3. 检查 UI 进程

```bash
ps aux | grep openclaw-plus
```

---

## 💬 使用 Claw Terminal

### 基本对话测试

在 UI 的 **Claw Terminal** 页面中输入：

1. **简单对话**:
   ```
   你好
   ```
   预期: AI 回复问候

2. **文件系统操作**:
   ```
   列出当前目录
   ```
   预期: 显示目录内容

3. **网页搜索**:
   ```
   搜索 OpenClaw 相关信息
   ```
   预期: 执行网页搜索并返回结果

### 自动测试套件

点击 UI 中的 **"🧪 Auto Test"** 按钮运行完整测试套件：

- ✅ 基本对话
- ✅ 天气查询
- ✅ 网页搜索
- ✅ 文件系统操作
- ✅ Python 执行
- ✅ 网页抓取
- ✅ 知识库搜索
- ✅ 画布管理
- ✅ 安全状态
- ✅ 技能列表

---

## 🔧 技术架构

### 修复的核心问题

**之前的问题**:
- UI 和 Gateway 分离为独立进程
- UI 不启动 Gateway
- AgentExecutor 无法执行 skills
- Claw Terminal 卡住无响应

**修复方案**:
- UI 内嵌启动 Gateway (HTTP 服务器)
- 所有组件在单一进程中运行
- 自动探测 Gateway 就绪状态
- 无需手动启动外部进程

### 代码修改摘要

1. **添加依赖** (`crates/ui/Cargo.toml`):
   - `openclaw-plugin-gateway`
   - `axum` (HTTP 服务器)

2. **内嵌 Gateway** (`crates/ui/src/app.rs`):
   ```rust
   fn start_embedded_gateway(config: SecurityConfig) -> tokio::task::JoinHandle<()>
   ```

3. **自动探测** (`crates/ui/src/app.rs`):
   - 启动时延迟 1 秒后探测 Gateway
   - 自动设置 `gateway_url = http://localhost:7878`
   - 更新 UI 状态显示

---

## 📊 系统组件

### 1. UI 层 (libcosmic)
- **路径**: `crates/ui/`
- **功能**: 图形界面、用户交互
- **端口**: N/A (GUI)

### 2. Plugin Gateway (内嵌)
- **路径**: `crates/plugin/`
- **功能**: Skill 执行、安全检查、审计日志
- **端口**: 7878 (HTTP)
- **API**:
  - `GET /health` - 健康检查
  - `GET /skills/list` - 技能列表
  - `POST /hooks/before-skill` - 执行前检查
  - `POST /hooks/after-skill` - 执行后审计

### 3. AgentExecutor
- **路径**: `crates/agent-executor/`
- **功能**: ReAct 推理循环、LLM 调用、Skill 调度
- **依赖**: Plugin Gateway (HTTP)

### 4. WasmEdge Sandbox
- **路径**: `crates/sandbox/`
- **功能**: 安全执行环境、WASM 运行时
- **依赖**: WasmEdge SDK

---

## 🎯 支持的功能

### AI 对话
- ✅ 自然语言理解
- ✅ ReAct 推理循环
- ✅ 多轮对话
- ✅ 上下文记忆

### Skill 执行
- ✅ 文件系统操作 (fs.*)
- ✅ Shell 命令执行 (exec)
- ✅ 网页搜索 (search.web)
- ✅ 网页抓取 (web.fetch)
- ✅ Python 执行 (python.run)
- ✅ 知识库管理 (knowledge.*)
- ✅ 130+ 内置 skills

### 安全特性
- ✅ 沙箱隔离
- ✅ 权限控制
- ✅ 审计日志
- ✅ 网络白名单
- ✅ 文件访问控制

### 中文支持
- ✅ 中文输入法 (IME)
- ✅ 中文对话
- ✅ 中文技能名称

---

## 🐛 故障排查

### Gateway 显示 Offline

**症状**: UI 显示 "Gateway: Offline"

**解决方案**:
1. 等待 2-3 秒让 Gateway 完全启动
2. 手动探测: 在 Claw Terminal 输入 `gateway status`
3. 检查日志: 查看终端输出是否有错误

### Claw Terminal 卡住

**症状**: 显示 "正在分析任务，启动 ReAct 推理循环..." 不动

**解决方案**:
1. 检查 Gateway 是否运行: `curl http://localhost:7878/health`
2. 检查 Ollama 是否运行: `ollama list`
3. 检查模型是否正确: 确认使用 `qwen2.5:7b`

### 中文输入无效

**症状**: 无法输入中文

**解决方案**:
1. 确保使用 `./scripts/run.sh` 启动 (不要直接运行二进制)
2. 检查是否创建了 .app bundle
3. 重启 UI

---

## 📝 配置文件

### 主配置
**路径**: `~/Library/Application Support/openclaw-plus/config.toml`

**关键配置**:
```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "qwen2.5:7b"  # 必须使用支持工具调用的模型
temperature = 0.7
max_tokens = 4096
```

### 安全配置
```toml
[security]
memory_limit_mb = 512
intercept_shell = true
network_allowlist = ["*.ollama.ai", "localhost"]
```

---

## 🔄 开发工作流

### 修改代码后重新编译

```bash
# 只编译 UI
cargo build --release -p openclaw-ui

# 编译所有组件
cargo build --release --workspace

# 运行测试
cargo test --workspace --exclude openclaw-wasi-nn-infer
```

### 清理并重新构建

```bash
cargo clean
cargo build --release -p openclaw-ui
./scripts/run.sh
```

---

## 📚 相关文档

- **架构审计报告**: `ARCHITECTURE_AUDIT_REPORT.md`
- **测试指南**: `CLAW_TERMINAL_TEST_GUIDE.md`
- **AI 模型设置**: `AI_MODEL_SETUP.md`
- **输入焦点修复**: `INPUT_FOCUS_FIX.md`

---

## ✨ 下一步

系统已完全就绪，可以：

1. **测试 Claw Terminal**: 尝试各种对话和命令
2. **运行自动测试**: 点击 "Auto Test" 验证所有功能
3. **探索 Skills**: 查看 130+ 内置技能
4. **开发新功能**: 基于稳定的架构继续开发
5. **部署生产**: 系统已可用于实际场景

---

## 🎉 总结

**OpenClaw+ 现在是一个真正的单进程 AI Agent 平台！**

- ✅ 所有组件集成在一起
- ✅ 自动启动和配置
- ✅ 完整的中文支持
- ✅ 稳定的架构设计
- ✅ 丰富的功能特性

**立即开始使用 Claw Terminal 进行对话吧！** 🚀
