# CLI Terminal - Bash 命令集成设计方案

**日期**: 2026-03-20  
**目标**: 实现真正的 bash 命令终端功能

---

## 🎯 设计目标

### 当前状态
- ✅ 基础命令系统（内置命令）
- ✅ ↑↓ 键历史导航（通过消息系统）
- ✅ Tab 键自动补全
- ❌ 无真正的键盘事件处理
- ❌ 无真正的 shell 命令执行

### 目标功能
1. **键盘事件处理**
   - ↑↓ 键：历史导航
   - ←→ 键：光标移动
   - Home/End：光标跳转
   - Ctrl+C：中断命令
   - Ctrl+L：清屏

2. **真正的 Shell 命令执行**
   - 执行系统 bash 命令（ls, cd, cat, grep 等）
   - 捕获命令输出（stdout/stderr）
   - 支持命令参数和选项
   - 工作目录管理

---

## 🏗️ 技术方案

### 方案 A: 混合模式（推荐）✅

**设计思路**:
- 内置命令优先（help, version, status 等）
- 未匹配的命令自动转发到系统 shell
- 保持航空航天级别的安全验证

**优点**:
- ✅ 保留现有内置命令
- ✅ 支持所有系统命令
- ✅ 安全性可控
- ✅ 用户体验最佳

**实现步骤**:
```rust
1. 检查命令是否为内置命令
2. 如果是内置命令 → 执行内置逻辑
3. 如果不是 → 使用 std::process::Command 执行
4. 捕获输出并显示
```

### 方案 B: 纯 Shell 模式

**设计思路**:
- 所有命令都转发到系统 shell
- 移除内置命令系统

**缺点**:
- ❌ 失去航空航天级别控制
- ❌ 安全性降低
- ❌ 无法自定义命令

**不推荐使用**

---

## 🔧 实现细节

### 1. 键盘事件处理

#### Cosmic 框架的键盘事件
```rust
// Cosmic 使用 iced 的事件系统
use cosmic::iced::keyboard::{KeyCode, Modifiers};
use cosmic::iced::Event;

// 在 text_input 中处理键盘事件
widget::text_input("", &state.command_input)
    .on_input(AppMessage::CliInputChanged)
    .on_submit(|_| AppMessage::CliExecuteCommand)
    // 需要添加自定义键盘事件处理
```

#### 问题：Cosmic text_input 限制
- `text_input` widget 不直接暴露键盘事件
- ↑↓ 键需要通过应用级别的 `subscription` 处理
- 或者通过消息系统间接处理

#### 解决方案：使用 Subscription
```rust
// 在 App 中添加键盘事件订阅
pub fn subscription(&self) -> Subscription<AppMessage> {
    cosmic::iced::keyboard::on_key_press(|key, modifiers| {
        match key {
            KeyCode::Up => Some(AppMessage::CliHistoryPrevious),
            KeyCode::Down => Some(AppMessage::CliHistoryNext),
            KeyCode::Tab => Some(AppMessage::CliApplyAutocomplete),
            KeyCode::L if modifiers.control() => Some(AppMessage::CliClearHistory),
            _ => None,
        }
    })
}
```

### 2. Shell 命令执行

#### 使用 std::process::Command
```rust
use std::process::{Command, Stdio};

fn execute_shell_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(stderr)
    }
}
```

#### 命令解析
```rust
fn parse_command(input: &str) -> (&str, Vec<&str>) {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.get(0).unwrap_or(&"");
    let args = parts.get(1..).unwrap_or(&[]).to_vec();
    (cmd, args)
}
```

### 3. 混合命令路由

```rust
async fn execute_command(command: String) -> AppMessage {
    let (cmd, args) = parse_command(&command);
    
    // 内置命令列表
    let builtin_commands = ["help", "version", "status", "clear", 
                           "sysinfo", "uptime", "whoami", "pwd", "env"];
    
    let output = if builtin_commands.contains(&cmd) {
        // 执行内置命令
        execute_builtin_command(cmd, &args)
    } else {
        // 执行系统 shell 命令
        match execute_shell_command(cmd, &args) {
            Ok(stdout) => vec![(stdout, false)],
            Err(stderr) => vec![(stderr, true)],
        }
    };
    
    AppMessage::CliCommandResult { command, output }
}
```

---

## 🔒 安全考虑

### 航空航天级别安全措施

1. **命令白名单模式**（可选）
```rust
// 只允许特定的系统命令
const ALLOWED_COMMANDS: &[&str] = &[
    "ls", "cat", "echo", "pwd", "whoami", "date", 
    "grep", "find", "head", "tail", "wc"
];

fn is_command_allowed(cmd: &str) -> bool {
    ALLOWED_COMMANDS.contains(&cmd)
}
```

2. **路径限制**
```rust
// 限制命令只能在特定目录执行
fn validate_working_directory(path: &Path) -> Result<(), String> {
    let allowed_base = Path::new("/Users/arkSong/workspace");
    if !path.starts_with(allowed_base) {
        return Err("Access denied: outside allowed directory".to_string());
    }
    Ok(())
}
```

3. **超时保护**
```rust
use std::time::Duration;

Command::new(cmd)
    .args(args)
    .timeout(Duration::from_secs(30))  // 30 秒超时
    .output()
```

4. **资源限制**
```rust
// 限制输出大小
const MAX_OUTPUT_SIZE: usize = 1024 * 1024; // 1MB

if stdout.len() > MAX_OUTPUT_SIZE {
    stdout.truncate(MAX_OUTPUT_SIZE);
    stdout.push_str("\n... (output truncated)");
}
```

---

## 📋 实现计划

### Phase 1: 键盘事件处理 ✅
1. 添加 `CliHistoryPrevious` 和 `CliHistoryNext` 消息
2. 在 `App::update` 中处理这些消息
3. 测试 ↑↓ 键历史导航

### Phase 2: Shell 命令执行 🔄
1. 实现 `execute_shell_command` 函数
2. 修改命令执行逻辑，添加混合路由
3. 测试基础 shell 命令（ls, pwd, echo）

### Phase 3: 高级功能 📅
1. 工作目录管理（cd 命令）
2. 环境变量支持
3. 命令管道（基础版）
4. 输出分页

### Phase 4: 安全加固 📅
1. 命令白名单（可选）
2. 路径限制
3. 资源限制
4. 审计日志

---

## 🎯 当前实现状态

### 已实现 ✅
- ✅ 历史导航消息（CliHistoryPrevious/Next）
- ✅ 历史导航逻辑（history_previous/next）
- ✅ 自动补全消息（CliApplyAutocomplete）
- ✅ 内置命令系统

### 待实现 🔄
- 🔄 键盘事件订阅（subscription）
- 🔄 Shell 命令执行
- 🔄 混合命令路由
- 🔄 工作目录管理

---

## 💡 使用示例

### 内置命令
```bash
$ help              # 内置命令
$ version           # 内置命令
$ sysinfo           # 内置命令
```

### 系统命令
```bash
$ ls -la            # 系统命令
$ cat README.md     # 系统命令
$ grep "test" *.rs  # 系统命令
$ pwd               # 系统命令（也是内置）
```

### 混合使用
```bash
$ help              # 内置：显示帮助
$ ls                # 系统：列出文件
$ sysinfo           # 内置：系统信息
$ cat file.txt      # 系统：查看文件
$ clear             # 内置：清空终端
```

---

## 🚀 下一步行动

1. **立即实现**：Shell 命令执行（混合模式）
2. **短期实现**：工作目录管理
3. **长期实现**：命令管道、输出分页

---

**结论**: 推荐使用**混合模式**，保留内置命令的同时支持真正的 shell 命令执行，既保证了安全性，又提供了完整的终端体验。
