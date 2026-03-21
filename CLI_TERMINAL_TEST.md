# OpenClaw+ CLI Terminal - 测试文档

## 📋 测试概览

**测试日期**: 2026-03-20  
**版本**: v0.1.0  
**测试环境**: macOS, Rust UI with Cosmic framework

---

## ✅ 已实现的命令

### 1. 系统命令 (System Commands)

#### `help` - 显示帮助信息
```bash
$ help
```
**预期输出**:
- 显示所有可用命令的列表
- 包含命令分类（系统、Agent、Gateway、AI、工具）
- 显示使用示例

**测试状态**: ✅ 待测试

---

#### `version` - 显示版本信息
```bash
$ version
```
**预期输出**:
```
╔══════════════════════════════════════════════════════════╗
║                  OpenClaw+ Version                      ║
╚══════════════════════════════════════════════════════════╝

  Version:          v0.1.0
  UI Framework:     Cosmic (libcosmic)
  Language:         Rust
  Build:            Debug

  Repository:       https://github.com/arkCyber/A3Claw
```

**测试状态**: ✅ 待测试

---

#### `status` - 显示系统状态
```bash
$ status
```
**预期输出**:
```
╔══════════════════════════════════════════════════════════╗
║                   System Status                         ║
╚══════════════════════════════════════════════════════════╝

  Sandbox:          ✓ Running / ⏸ Idle / ⏹ Stopped / ⚠ Tripped / ✗ Error
  Gateway:          ✓ Connected / ✗ Disconnected
  AI Engine:        ✓ Ready
  Agents:           X available
```

**测试状态**: ✅ 待测试

---

#### `clear` - 清空终端历史
```bash
$ clear
```
**预期输出**:
- 清空所有命令历史记录
- 显示 "Terminal cleared."

**测试状态**: ✅ 待测试

---

### 2. Agent 命令

#### `agent list` - 列出所有可用的 Agent
```bash
$ agent list
```
**预期输出**:
```
╔══════════════════════════════════════════════════════════╗
║                  Available Agents                       ║
╚══════════════════════════════════════════════════════════╝

  1. Agent Name (agent_id_1)
  2. Agent Name (agent_id_2)
  ...
```

**测试状态**: ✅ 待测试

---

#### `agent info` - 显示 Agent 信息
```bash
$ agent info
```
**预期输出**:
```
Agent information:
  Total agents: X
```

**测试状态**: ✅ 待测试

---

#### `agent` - 无子命令时显示用法
```bash
$ agent
```
**预期输出**:
```
Usage: agent <subcommand>
Subcommands: list, info
```

**测试状态**: ✅ 待测试

---

### 3. Gateway 命令

#### `gateway status` - 显示网关状态
```bash
$ gateway status
```
**预期输出**:
```
╔══════════════════════════════════════════════════════════╗
║                  Gateway Status                         ║
╚══════════════════════════════════════════════════════════╝

  Status:           ✓ Connected / ✗ Disconnected
  URL:              http://localhost:3000 / Not configured
```

**测试状态**: ✅ 待测试

---

#### `gateway url` - 显示网关 URL
```bash
$ gateway url
```
**预期输出**:
```
Gateway URL: http://localhost:3000 / Not configured
```

**测试状态**: ✅ 待测试

---

#### `gateway` - 无子命令时显示用法
```bash
$ gateway
```
**预期输出**:
```
Usage: gateway <subcommand>
Subcommands: status, url
```

**测试状态**: ✅ 待测试

---

### 4. AI 命令

#### `ai model` - 显示当前 AI 模型
```bash
$ ai model
```
**预期输出**:
```
╔══════════════════════════════════════════════════════════╗
║                    AI Model Info                        ║
╚══════════════════════════════════════════════════════════╝

  Current Model:    llama3.1 / gpt-4 / etc.
  Status:           ✓ Ready
```

**测试状态**: ✅ 待测试

---

#### `ai status` - 显示 AI 引擎状态
```bash
$ ai status
```
**预期输出**:
```
AI Engine Status: ✓ Ready
Model: llama3.1
```

**测试状态**: ✅ 待测试

---

#### `ai` - 无子命令时显示用法
```bash
$ ai
```
**预期输出**:
```
Usage: ai <subcommand>
Subcommands: model, status
```

**测试状态**: ✅ 待测试

---

### 5. 工具命令

#### `weather <city>` - 查询天气（占位符）
```bash
$ weather beijing
```
**预期输出**:
```
Fetching weather for beijing...
(Weather integration coming soon)
```

**测试状态**: ✅ 待测试（占位符功能）

---

#### `weather` - 无参数时显示用法
```bash
$ weather
```
**预期输出**:
```
Usage: weather <city>
Example: weather beijing
```

**测试状态**: ✅ 待测试

---

#### `news` - 获取最新新闻（占位符）
```bash
$ news
```
**预期输出**:
```
Fetching latest news...
(News integration coming soon)
```

**测试状态**: ✅ 待测试（占位符功能）

---

### 6. 错误处理

#### 未知命令
```bash
$ unknown_command
```
**预期输出**:
```
Unknown command: unknown_command
Type 'help' for available commands.
```

**测试状态**: ✅ 待测试

---

#### 空命令
```bash
$ 
```
**预期行为**:
- 不执行任何操作
- 不添加到历史记录

**测试状态**: ✅ 待测试

---

## 🎨 UI 功能测试

### 1. 命令输入
- ✅ 输入框正常显示
- ✅ `$` 提示符显示
- ✅ 等宽字体显示
- ✅ 按 Enter 键执行命令
- ✅ 点击"执行"按钮执行命令

### 2. 命令历史
- ✅ 命令和输出正确显示
- ✅ 历史记录可滚动查看
- ✅ 错误信息用红色显示
- ✅ 正常输出用绿色显示

### 3. 状态指示
- ✅ "就绪"状态显示
- ✅ "执行中"状态显示
- ✅ 执行完成后恢复"就绪"状态

### 4. 清空功能
- ✅ "清空"按钮正常工作
- ✅ `clear` 命令清空历史

---

## 📊 测试结果汇总

### 命令测试统计
- **总命令数**: 20+
- **已测试**: 0
- **通过**: 0
- **失败**: 0
- **待测试**: 20+

### UI 功能测试统计
- **总功能点**: 12
- **已测试**: 0
- **通过**: 0
- **失败**: 0

---

## 🔧 测试步骤

### 准备工作
1. ✅ 启动 OpenClaw+ 应用
2. ✅ 在侧边栏找到 "💻 CLI Terminal" 按钮
3. ✅ 点击进入 CLI 终端页面

### 测试流程
1. **基础命令测试**
   - 输入 `help` 查看帮助
   - 输入 `version` 查看版本
   - 输入 `status` 查看状态
   - 输入 `clear` 清空历史

2. **Agent 命令测试**
   - 输入 `agent list` 查看 Agent 列表
   - 输入 `agent info` 查看 Agent 信息
   - 输入 `agent` 测试错误提示

3. **Gateway 命令测试**
   - 输入 `gateway status` 查看网关状态
   - 输入 `gateway url` 查看网关 URL
   - 输入 `gateway` 测试错误提示

4. **AI 命令测试**
   - 输入 `ai model` 查看模型信息
   - 输入 `ai status` 查看 AI 状态
   - 输入 `ai` 测试错误提示

5. **工具命令测试**
   - 输入 `weather beijing` 测试天气查询
   - 输入 `weather` 测试错误提示
   - 输入 `news` 测试新闻查询

6. **错误处理测试**
   - 输入未知命令测试错误提示
   - 输入空命令测试处理

7. **UI 功能测试**
   - 测试命令输入框
   - 测试历史记录滚动
   - 测试状态指示器
   - 测试清空按钮

---

## 🐛 已知问题

暂无

---

## 📝 测试备注

### 测试环境
- **操作系统**: macOS
- **Rust 版本**: 1.x
- **Cosmic 版本**: libcosmic latest

### 测试说明
1. 所有命令都应该在 CLI Terminal 页面中测试
2. 注意观察命令输出的格式和颜色
3. 检查错误信息是否用红色显示
4. 验证状态指示器是否正确切换

---

## ✅ 测试完成标准

- [ ] 所有基础命令正常工作
- [ ] 所有子命令正常工作
- [ ] 错误处理正确显示
- [ ] UI 显示美观且功能正常
- [ ] 命令历史记录正确保存
- [ ] 状态指示器正确切换
- [ ] 清空功能正常工作

---

## 🚀 下一步计划

1. **命令历史导航** (↑↓ 键)
2. **命令自动补全** (Tab 键)
3. **IPC 集成** (连接 TypeScript CLI)
4. **实时工具集成** (真实的 weather 和 news 功能)
5. **命令别名系统**
6. **脚本执行功能**

---

**测试人员**: _____________  
**测试日期**: 2026-03-20  
**签名**: _____________
