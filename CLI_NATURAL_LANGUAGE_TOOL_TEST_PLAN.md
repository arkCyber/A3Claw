# CLI Natural Language Tool Test Plan (CLI 入口自然语言测试)

## 目标
用 **CLI Terminal** 作为统一入口，以“自然语言/人类习惯”的方式覆盖项目内置工具与系统能力，验证：

- **命令解析**
- **工具功能正确性**
- **错误处理与可用性**
- **网络不可用/超时等异常路径**
- **输出可读性（多行、长输出）**

## 测试前提
- 在 UI 中打开 **CLI Terminal** 页面。
- 保证能联网（用于 `weather` / `news`）。
- 若你想验证离线行为：断网或用防火墙阻断网络。

## 通用验收标准（所有命令）
- **无崩溃**：执行任何命令都不应导致 UI 崩溃或卡死。
- **输出可解释**：失败要有可读错误信息（至少 1 行）。
- **执行后可继续输入**：命令返回后可立即输入下一条命令。
- **历史记录**：命令与输出应写入 history，支持 ↑↓ 回看。

---

# A. CLI 基础能力（像终端一样）

## A1. 帮助与发现能力
1. 输入：`help`
- 预期：
  - 显示命令分组：System / Agent / Gateway / AI / System Info / Tool / Shell
  - 至少包含：`weather <city>`、`news`、`agent list`、`gateway status`、`ai model`、`sysinfo`

2. 输入：`version`
- 预期：显示版本信息与仓库地址。

3. 输入：`clear`
- 预期：history 被清空（至少当前视图不再显示历史内容）。

## A2. 历史与编辑体验
1. 连续执行三条命令：
- `help`
- `version`
- `status`

2. 按 ↑ 三次
- 预期：输入框依次出现 `status` → `version` → `help`

3. 按 ↓ 三次
- 预期：输入框依次出现 `version` → `status` → 空输入

## A3. 自动补全（Tab）
1. 输入：`hel` 然后按 Tab
- 预期：变为 `help`

2. 输入：`agent l` 然后按 Tab
- 预期：变为 `agent list`

---

# B. 系统信息与状态工具

## B1. status
输入：`status`
- 预期：
  - 显示 Sandbox/Gateway/AI/Agents 信息
  - Gateway connected/disconnected 与当前配置一致

## B2. sysinfo
输入：`sysinfo`
- 预期：
  - OS/ARCH/FAMILY/Hostname 等字段存在
  - 不应 panic（即使取不到 HOSTNAME 也应显示 Unknown）

## B3. pwd / whoami / env / uptime
依次输入：
- `pwd`
- `whoami`
- `env`
- `uptime`

- 预期：
  - 这些命令若走 shell fallback，输出应与系统一致
  - 若某条命令在系统不可用，应给出错误行（stderr）

---

# C. Agent 工具（本地数据/配置）

## C1. agent list
输入：`agent list`
- 预期：
  - 若有 agent：输出列表含序号、display_name、id
  - 若没有 agent：输出 `No agents available.`

## C2. agent info
输入：`agent info`
- 预期：
  - 至少显示 `Total agents: N`

## C3. 错误子命令
输入：`agent foo`
- 预期：
  - 输出 Unknown subcommand
  - 提示可用 subcommands

---

# D. Gateway 工具（连接状态）

## D1. gateway status
输入：`gateway status`
- 预期：
  - 显示 Status: Connected/Disconnected
  - 显示 URL（未配置则 Not configured）

## D2. gateway url
输入：`gateway url`
- 预期：
  - 输出 Gateway URL: ...

## D3. 错误子命令
输入：`gateway foo`
- 预期：
  - Unknown gateway subcommand
  - 提示可用 subcommands

---

# E. AI 工具（模型信息）

## E1. ai model
输入：`ai model`
- 预期：
  - 显示 Current Model: ...
  - 显示 Ready

## E2. ai status
输入：`ai status`
- 预期：
  - 显示 AI Engine Status 与 Model

## E3. 错误子命令
输入：`ai foo`
- 预期：
  - Unknown ai subcommand
  - 提示可用 subcommands

---

# F. Tool 工具（网络能力）

## F1. weather（正常路径）
输入：`weather beijing`
- 预期：
  - 输出多行天气报告（包含温度/体感/湿度/风速 等至少若干项）
  - 不应出现 “coming soon”

## F2. weather（参数缺失）
输入：`weather`
- 预期：
  - Usage: weather <city>
  - Example: weather beijing

## F3. weather（未知城市）
输入：`weather asdfghjkl`
- 预期：
  - 返回可读错误（无法找到位置/地理编码失败等）

## F4. weather（断网/超时）
操作：断网后输入 `weather beijing`
- 预期：
  - 错误信息包含 HTTP 请求失败/timeout
  - CLI 不崩溃

## F5. news（正常路径）
输入：`news`
- 预期：
  - 输出包含来源（CNN/NPR/Reuters 之一）
  - 至少 3-5 条新闻标题

## F6. news（断网/源失败）
操作：断网后输入 `news`
- 预期：
  - 输出 `News tool error:`
  - 错误信息提到所有新闻源失败

---

# G. Shell fallback（bash/zsh 类行为）

## G1. 简单命令
输入：
- `echo hello`
- `date`
- `ls`

- 预期：
  - stdout 正常显示

## G2. 错误命令
输入：`some_command_that_does_not_exist`
- 预期：
  - 输出包含 `Failed to execute` 或 unknown command 提示

## G3. 安全限制（注入）
输入：
- `ls && whoami`
- `ls; whoami`
- `ls || whoami`

- 预期：
  - 被拒绝（Command chaining not allowed）

---

# H. 长输出与滚动

## H1. 长输出
输入：`env`
- 预期：
  - 输出很多行
  - 终端可滚动
  - 最新输出可见

## H2. 重复执行（压力）
操作：连续快速执行 20 次 `echo 123`
- 预期：
  - 不崩溃
  - history 增长
  - UI 不明显卡顿

---

# 你需要我继续补全的部分（可选）
如果你想把“自然语言”进一步升级为“像 ChatGPT 那样对话式测试”，我可以新增命令：

- `tool test all`：自动跑一组自检（对网络工具给出提示）
- `tool test weather` / `tool test news`：输出更结构化的诊断信息

你告诉我：更偏向 **纯手工验收清单**，还是 **一键自检命令**？
