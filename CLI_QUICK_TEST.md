# CLI Terminal 快速测试指南

## 🚀 快速测试流程

### 步骤 1: 打开 CLI Terminal
1. 启动 OpenClaw+ 应用
2. 在左侧边栏找到 **💻 CLI Terminal** 按钮
3. 点击进入 CLI 终端页面

---

### 步骤 2: 测试基础命令

#### 测试 1: Help 命令
```bash
$ help
```
✅ **预期**: 显示完整的命令帮助列表，包含所有分类

---

#### 测试 2: Version 命令
```bash
$ version
```
✅ **预期**: 显示版本信息框，包含 v0.1.0、Rust、Cosmic 等信息

---

#### 测试 3: Status 命令
```bash
$ status
```
✅ **预期**: 显示系统状态框，包含 Sandbox、Gateway、AI、Agents 状态

---

### 步骤 3: 测试 Agent 命令

#### 测试 4: Agent List
```bash
$ agent list
```
✅ **预期**: 显示所有可用的 Agent 列表（如果有的话）

---

#### 测试 5: Agent Info
```bash
$ agent info
```
✅ **预期**: 显示 Agent 总数信息

---

#### 测试 6: Agent 错误处理
```bash
$ agent
```
✅ **预期**: 显示用法提示 "Usage: agent <subcommand>"

---

### 步骤 4: 测试 Gateway 命令

#### 测试 7: Gateway Status
```bash
$ gateway status
```
✅ **预期**: 显示网关连接状态和 URL

---

#### 测试 8: Gateway URL
```bash
$ gateway url
```
✅ **预期**: 显示网关 URL 或 "Not configured"

---

### 步骤 5: 测试 AI 命令

#### 测试 9: AI Model
```bash
$ ai model
```
✅ **预期**: 显示当前 AI 模型信息

---

#### 测试 10: AI Status
```bash
$ ai status
```
✅ **预期**: 显示 AI 引擎状态和模型名称

---

### 步骤 6: 测试工具命令

#### 测试 11: Weather 命令
```bash
$ weather beijing
```
✅ **预期**: 显示 "Fetching weather for beijing..." 和占位符消息

---

#### 测试 12: News 命令
```bash
$ news
```
✅ **预期**: 显示 "Fetching latest news..." 和占位符消息

---

### 步骤 7: 测试错误处理

#### 测试 13: 未知命令
```bash
$ unknown_command
```
✅ **预期**: 显示红色错误消息 "Unknown command: unknown_command"

---

#### 测试 14: Clear 命令
```bash
$ clear
```
✅ **预期**: 清空所有历史记录

---

## 📋 测试检查清单

### 功能测试
- [ ] Help 命令显示完整帮助
- [ ] Version 命令显示版本信息
- [ ] Status 命令显示系统状态
- [ ] Agent list 显示 Agent 列表
- [ ] Agent info 显示 Agent 信息
- [ ] Gateway status 显示网关状态
- [ ] Gateway url 显示网关 URL
- [ ] AI model 显示模型信息
- [ ] AI status 显示 AI 状态
- [ ] Weather 命令显示占位符
- [ ] News 命令显示占位符
- [ ] 未知命令显示错误
- [ ] Clear 命令清空历史

### UI 测试
- [ ] 命令输入框正常工作
- [ ] $ 提示符正确显示
- [ ] 等宽字体正确应用
- [ ] Enter 键执行命令
- [ ] 执行按钮正常工作
- [ ] 历史记录正确显示
- [ ] 错误消息显示为红色
- [ ] 正常输出显示为绿色
- [ ] 状态指示器切换正常
- [ ] 清空按钮正常工作
- [ ] 滚动条正常工作

---

## 🎯 测试要点

### 1. 命令格式
- 所有命令都应该有美观的框架输出（╔═══╗）
- 错误消息应该用红色显示
- 成功消息应该用绿色显示

### 2. 状态显示
- Sandbox 状态应该显示实际状态（Idle/Running/Paused/Stopped/Tripped/Error）
- Gateway 状态应该显示连接状态
- Agent 数量应该显示实际数量

### 3. 错误处理
- 未知命令应该显示友好的错误提示
- 缺少参数的命令应该显示用法说明
- 错误消息应该清晰易懂

---

## ✅ 测试通过标准

所有测试项都应该：
1. ✅ 命令执行成功
2. ✅ 输出格式正确
3. ✅ 颜色显示正确
4. ✅ 状态指示器正常
5. ✅ 无崩溃或错误

---

## 📸 测试截图建议

建议截取以下场景的截图：
1. Help 命令输出
2. Version 命令输出
3. Status 命令输出
4. Agent list 输出
5. Gateway status 输出
6. AI model 输出
7. 错误消息显示
8. 完整的命令历史记录

---

**测试完成后，请在测试文档中记录结果！**
