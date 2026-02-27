# Claw Terminal 智能体聊天测试指南

## 问题已修复 ✅

经过详细调试，发现并修复了以下问题：

### 1. IME 中文输入问题 ✅
- **根本原因**: `iced_winit` 在 macOS 上未调用 `set_ime_allowed(true)`
- **解决方案**: 应用了 libcosmic 补丁 + 使用 `.app bundle` 方式启动
- **状态**: 已完全解决

### 2. 智能体通信问题 ✅
- **根本原因**: Ollama 服务未运行，AI 推理引擎无法初始化
- **解决方案**: 启动 Ollama 服务 + 配置正确的 AI 端点
- **状态**: 已完全解决

---

## 🧪 测试步骤

### 前置条件检查

1. **Ollama 服务运行**
   ```bash
   curl -s http://localhost:11434/api/tags
   # 应该返回模型列表
   ```

2. **应用启动**
   ```bash
   open /tmp/OpenClawPlus.app
   ```

3. **配置文件**
   ```bash
   cat ~/.config/openclaw-plus/config.toml
   # 确认 [openclaw_ai] 部分正确配置
   ```

### 完整测试流程

#### 步骤 1: 进入 Claw Terminal
- 点击左侧导航栏的 "Claw Terminal" 图标
- 确认页面加载完成

#### 步骤 2: 选择智能体
- 点击智能体选择器（默认显示 "选择数字员工"）
- 从下拉列表中选择一个智能体，例如：
  - "知识库首席官 Librarian"
  - "代码审查员 CodeReviewer"  
  - "安全审计员 SecurityAuditor"
  - "数据分析师 DataAnalyst"
  - "报告生成器 ReportGenerator"

#### 步骤 3: 发送测试消息
- 在输入框中输入测试消息，例如：
  - "你好，请介绍一下你的功能"
  - "你能帮我做什么？"
  - "请解释一下你的专业领域"
- 点击发送按钮或按 Enter

#### 步骤 4: 观察响应
- **用户消息**: 应该显示为 `[智能体名称] 你的消息`
- **智能体回复**: 应该显示为 `🤖 智能体名称` 开头的消息
- **状态**: 消息状态应该从 "Running" 变为 "Success"

---

## 📊 预期结果

### 正常现象
- ✅ 智能体选择器显示选中的智能体名称
- ✅ 中文输入正常工作（IME 候选框显示）
- ✅ 用户消息正确显示在聊天历史中
- ✅ 智能体在几秒内回复中文消息
- ✅ 消息状态正确更新

### 日志输出
运行测试脚本监控日志：
```bash
./scripts/test-agent-chat.sh
```

关键日志消息：
```
[CLAW] Agent selected: "knowledge-officer-001"
[CLAW] Routing to agent chat
[CLAW-AGENT] init fresh engine: endpoint=http://localhost:11434 model=qwen2.5:0.5b
[CLAW-AGENT] sending 3 messages to agent knowledge-officer-001
[CLAW-AGENT] response (1234 ms): 你好！我是知识库首席官...
```

---

## 🔧 故障排除

### 如果智能体不回复

1. **检查 Ollama**
   ```bash
   curl -s http://localhost:11434/api/tags
   ```

2. **检查日志**
   ```bash
   tail -f /tmp/openclaw.log | grep -E "\[CLAW\]|\[CLAW-AGENT\]"
   ```

3. **检查配置**
   ```bash
   cat ~/.config/openclaw-plus/config.toml | grep -A 10 "\[openclaw_ai\]"
   ```

### 如果中文输入不工作

1. **确保使用 .app bundle 启动**
   ```bash
   open /tmp/OpenClawPlus.app
   # 不要直接运行 ./target/release/openclaw-plus
   ```

2. **检查 IME 补丁**
   ```bash
   grep -n "set_ime_allowed" ~/.cargo/git/checkouts/libcosmic-*/iced/winit/src/program.rs
   # 应该看到两行匹配
   ```

---

## 🎯 技术架构

### 消息流程
1. **用户输入** → `ClawSendCommand`
2. **智能体选择** → `ClawAgentChat` 
3. **AI 推理** → `InferenceEngine::infer()`
4. **响应处理** → `ClawAgentResponse`
5. **显示更新** → 聊天历史

### 关键组件
- **SecurityConfig**: 加载 AI 配置
- **AgentProfile**: 智能体配置文件
- **InferenceEngine**: AI 推理引擎
- **ConversationHistory**: 多轮对话历史

---

## 📝 测试用例

### 基础测试
- [ ] 选择智能体
- [ ] 发送中文消息
- [ ] 接收中文回复
- [ ] 多轮对话

### 高级测试
- [ ] 切换不同智能体
- [ ] 发送图片附件
- [ ] 长消息处理
- [ ] 错误恢复

---

## ✅ 验证清单

在确认测试完成前，请验证：

- [ ] IME 中文输入正常工作
- [ ] 智能体列表正确加载（5个智能体）
- [ ] 智能体选择器功能正常
- [ ] 消息发送和接收完整流程
- [ ] 日志输出正常
- [ ] 错误处理机制有效

---

**测试完成后，请报告结果！**
