# Claw Terminal 对话功能使用指南

## 🎯 功能说明

Claw Terminal 是 OpenClaw+ 的命令行界面，支持两种模式：
1. **Shell 命令模式** - 执行系统命令
2. **数字员工对话模式** - 与 AI 数字员工进行对话

## ✅ 使用步骤

### 1. 启动应用

**重要**：必须使用 `run.sh` 启动才能支持中文输入！

```bash
cd /Users/arkSong/workspace/OpenClaw+
./scripts/run.sh
```

### 2. 进入 Claw Terminal

点击应用顶部的 **"Claw Terminal"** 标签页

### 3. 选择数字员工

在 Claw Terminal 页面顶部，你会看到一个数字员工选择器，显示：
- 🤖 选择数字员工（默认）
- 或当前选中的数字员工名称

点击选择器，会弹出可用的数字员工列表：
- **无 Agent** - Shell 命令模式
- **代码审查员 Alpha** - 代码审查专家
- **数据分析师 Insight** - 数据分析专家
- **知识库首席官 Librarian** - 知识管理专家
- **报告生成器 Scribe** - 报告生成专家
- **安全审计员 Guardian** - 安全审计专家

### 4. 开始对话

选择一个数字员工后：
1. 在底部输入框中输入你的问题
2. 按 **Enter** 键或点击 **发送** 按钮
3. 等待 AI 回复

## 📝 对话示例

### 示例 1：代码审查

1. 选择 **代码审查员 Alpha**
2. 输入：`请帮我审查这段代码的安全性`
3. 等待回复

### 示例 2：数据分析

1. 选择 **数据分析师 Insight**
2. 输入：`分析一下最近的用户增长趋势`
3. 等待回复

### 示例 3：知识查询

1. 选择 **知识库首席官 Librarian**
2. 输入：`OpenClaw+ 的架构是怎样的？`
3. 等待回复

## 🔍 常见问题排查

### 问题 1：输入后没有反应

**可能原因**：
- 没有选择数字员工
- Ollama 服务未运行
- 网络连接问题

**解决方案**：
```bash
# 1. 检查是否选择了数字员工
# 确保数字员工选择器显示的不是"选择数字员工"

# 2. 检查 Ollama 服务
curl http://localhost:11434/api/tags

# 3. 如果 Ollama 未运行，启动它
./scripts/start-ollama.sh

# 4. 重新运行测试
./tests/test_claw_terminal_chat.sh
```

### 问题 2：中文输入不工作

**原因**：使用了错误的启动方式

**解决方案**：
```bash
# 必须使用 run.sh 启动
./scripts/run.sh

# 不要使用以下方式：
# ./scripts/start-ui.sh  ❌
# cargo run -p openclaw-ui  ❌
```

参考：`docs/CHINESE_INPUT_GUIDE.md`

### 问题 3：回复很慢或超时

**可能原因**：
- 模型太大
- 系统资源不足
- 网络延迟

**解决方案**：
```bash
# 1. 使用更小的模型
# 编辑配置文件
vim ~/.config/openclaw-plus/config.toml

# 修改模型为：
# model = "qwen2.5:0.5b"  # 更快
# 而不是：
# model = "llama3.2:latest"  # 较慢

# 2. 检查系统资源
top

# 3. 重启 Ollama
pkill ollama
./scripts/start-ollama.sh
```

### 问题 4：回复内容不正确或不相关

**可能原因**：
- 问题描述不清晰
- 选择了错误的数字员工
- 对话历史混乱

**解决方案**：
1. **明确问题**：提供更详细的上下文
2. **选择正确的数字员工**：根据任务类型选择
3. **清除历史**：切换到其他数字员工再切换回来

### 问题 5：看不到数字员工选择器

**可能原因**：
- 数字员工配置文件缺失
- UI 未正确加载配置

**解决方案**：
```bash
# 1. 检查数字员工配置
ls -la agents/*.toml

# 2. 应该看到 5 个配置文件：
# - code_reviewer.toml
# - data_analyst.toml
# - knowledge_officer.toml
# - report_generator.toml
# - security_auditor.toml

# 3. 如果缺失，从备份恢复或重新创建

# 4. 重启 UI
pkill openclaw-plus
./scripts/run.sh
```

## 🔧 技术细节

### 对话流程

1. **用户输入** → `ClawInputChanged` 消息
2. **选择数字员工** → `ClawSelectAgent` 消息
3. **发送消息** → `ClawSendCommand` 消息
4. **路由到 Agent Chat** → `ClawAgentChat` 消息
5. **初始化推理引擎** → `InferenceEngine::new()`
6. **构建对话历史** → 系统提示 + 历史对话 + 新消息
7. **调用 AI 推理** → `engine.infer(req).await`
8. **接收回复** → `ClawAgentResponse` 消息
9. **更新 UI** → 显示回复，滚动到底部

### 配置文件

**用户配置**: `~/.config/openclaw-plus/config.toml`

```toml
[openclaw_ai]
provider = "Ollama"
endpoint = "http://localhost:11434"
model = "qwen2.5:0.5b"
api_key = ""
```

**数字员工配置**: `agents/*.toml`

```toml
id = "code_reviewer"
display_name = "代码审查员 Alpha"
role = "CodeReviewer"
description = "专业的代码审查员"

[security]
network_access = "Deny"
file_read = "Allow"
file_write = "Deny"
shell_intercept = true
max_tokens = 2048
```

### 日志调试

查看运行时日志：

```bash
# 启动 UI 时会输出日志到终端
./scripts/run.sh

# 关键日志标记：
# [CLAW] Send command: ...
# [CLAW] Selected agent: ...
# [CLAW] Routing to agent chat
# [CLAW-AGENT] resolved: endpoint=... model=...
# [CLAW-AGENT] init fresh engine: ...
# [CLAW-AGENT] sending N messages to agent ...
# [CLAW-AGENT] response (XXX ms): ...
```

## 📊 性能优化

### 1. 使用更快的模型

```bash
# qwen2.5:0.5b - 最快，适合对话
# llama3.2:latest - 较慢，但更准确
```

### 2. 限制对话历史

代码已自动限制为最近 6 轮对话，每轮最多 800 字符

### 3. 调整超时时间

默认推理超时：120 秒

如需调整，修改 `crates/ui/src/app.rs`:
```rust
inference_timeout: std::time::Duration::from_secs(120),
```

## 🎯 最佳实践

### 1. 选择合适的数字员工

- **代码相关** → 代码审查员 Alpha
- **数据分析** → 数据分析师 Insight
- **文档查询** → 知识库首席官 Librarian
- **报告生成** → 报告生成器 Scribe
- **安全审计** → 安全审计员 Guardian

### 2. 提供清晰的上下文

❌ 不好：`这个怎么样？`  
✅ 好：`请帮我审查这段 Rust 代码的内存安全性`

### 3. 分步骤提问

对于复杂任务，分成多个小问题：
1. 先问总体思路
2. 再问具体实现
3. 最后问优化建议

### 4. 利用多轮对话

数字员工会记住最近 6 轮对话，可以：
- 追问细节
- 要求澄清
- 请求示例

## 🧪 测试命令

```bash
# 运行完整测试
./tests/test_claw_terminal_chat.sh

# 测试 Ollama API
curl -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5:0.5b",
    "prompt": "你好",
    "stream": false
  }'

# 检查配置
cat ~/.config/openclaw-plus/config.toml

# 检查数字员工
ls -la agents/*.toml
```

## 📚 相关文档

- `docs/CHINESE_INPUT_GUIDE.md` - 中文输入法使用指南
- `docs/libcosmic-patches.md` - libcosmic IME 补丁说明
- `FINAL_COMPLETE_SYSTEM_REPORT.md` - 完整系统状态报告
- `tests/test_claw_terminal_chat.sh` - 对话功能测试脚本

## 🎊 快速开始

```bash
# 1. 启动 Ollama
./scripts/start-ollama.sh

# 2. 运行测试（可选）
./tests/test_claw_terminal_chat.sh

# 3. 启动 UI
./scripts/run.sh

# 4. 使用 Claw Terminal
# - 点击 Claw Terminal 标签页
# - 选择一个数字员工
# - 输入问题
# - 按 Enter 发送
```

## ⚠️ 注意事项

1. **必须选择数字员工**才能进行对话，否则输入会被当作 Shell 命令
2. **必须使用 run.sh 启动**才能支持中文输入
3. **确保 Ollama 服务运行**在 `http://localhost:11434`
4. **对话历史**会保存在内存中，重启应用会清空
5. **每个数字员工**有独立的对话历史

---

**最后更新**: 2026-03-01  
**维护者**: arkSong (arksong2018@gmail.com)  
**许可**: MIT
