# IME 输入问题诊断指南

## 问题现象
在 Claw Terminal 输入框中输入中文时，字符出现在 Windsurf IDE 的输入框中，而不是 OpenClaw+ 的 Terminal 输入框。

## 已完成的修复

### 1. 添加 on_paste 处理
**文件**: `crates/ui/src/pages/claw_terminal.rs:413`
```rust
.on_paste(AppMessage::ClawInputChanged)
```

### 2. 添加 on_focus 事件
**文件**: `crates/ui/src/pages/claw_terminal.rs:414`
```rust
.on_focus(AppMessage::ClawInputFocused)
```

### 3. 移除循环焦点请求
**文件**: `crates/ui/src/app.rs:3073-3076`
```rust
AppMessage::ClawInputFocused => {
    tracing::info!("[IME] Claw Terminal input focused");
    self.claw_input_focused = true;
    // 移除了重复的 focus() 调用，避免破坏 IME 候选框
}
```

### 4. 切换页面时请求焦点
**文件**: `crates/ui/src/app.rs:1604-1608`
```rust
} else if page == NavPage::ClawTerminal {
    return cosmic::widget::text_input::focus(
        crate::pages::claw_terminal::CLAW_INPUT_ID.clone(),
    ).map(cosmic::Action::App);
}
```

### 5. 添加输入日志
**文件**: `crates/ui/src/app.rs:3067`
```rust
tracing::info!("[IME] ClawInputChanged: {:?} ({} chars)", s, s.chars().count());
```

## 测试步骤

1. **启动应用**
   ```bash
   ./target/release/openclaw-plus
   ```

2. **进入 Claw Terminal 页面**
   - 点击左侧导航栏的 "Claw Terminal" 图标

3. **点击输入框**
   - 确认输入框边框变为浅蓝色（表示获得焦点）

4. **切换中文输入法**
   - macOS: Ctrl+Space 或点击输入法图标

5. **输入拼音**
   - 例如输入 "nihao"

6. **观察现象**
   - 检查 IME 候选框是否显示
   - 检查选择汉字后出现在哪里

7. **查看日志**
   ```bash
   tail -f /tmp/openclaw.log | grep IME
   ```

## 预期日志输出

```
[IME] make_visible: logical_size=1200x820 ime_y=770
[IME] Focused: logical_size=1200x754 scale=2 ime_pos=(280,671)
[IME] Claw Terminal input focused
[IME] ClawInputChanged: "你" (1 chars)
[IME] ClawInputChanged: "你好" (2 chars)
```

## 问题根源分析

### Windsurf IDE 劫持 IME 输入的可能原因：

1. **窗口层级问题**
   - Windsurf 作为外层 IDE，可能在操作系统级别捕获了 IME 事件
   - OpenClaw+ 作为嵌入式应用，其窗口焦点可能不被操作系统识别为"真正的"焦点

2. **事件冒泡**
   - IME 输入事件可能从 OpenClaw+ 窗口冒泡到 Windsurf 窗口
   - Cosmic/Iced 框架可能没有正确阻止事件传播

3. **焦点管理**
   - macOS 的输入法系统可能将焦点仍然认为在 Windsurf 的输入框
   - 即使 OpenClaw+ 的输入框获得了应用内焦点，操作系统级别的焦点可能仍在 Windsurf

## 可能的解决方案

### 方案 A: 独立窗口运行
将 OpenClaw+ 作为独立应用运行，而不是在 Windsurf 中运行：
```bash
open /tmp/OpenClawPlus.app
```

### 方案 B: 检查日志确认事件流
如果日志显示 `ClawInputChanged` 被调用，说明应用接收到了输入，但显示有问题。
如果日志没有显示，说明事件根本没有到达应用。

### 方案 C: 操作系统级别的焦点控制
可能需要在 Cosmic/Iced 层面设置窗口属性，明确告诉操作系统这个窗口需要 IME 焦点。

## 下一步行动

1. **查看日志** - 确认 `ClawInputChanged` 是否被调用
2. **独立运行测试** - 尝试在 Windsurf 外部运行应用
3. **报告现象** - 详细描述测试结果

## 技术限制

**重要**: 如果 Windsurf IDE 在操作系统级别劫持了 IME 输入，这可能超出了应用代码层面能解决的范围。这是一个窗口系统和输入法路由的底层问题。

可能的最终解决方案是将 OpenClaw+ 作为独立应用运行，而不是嵌入在 Windsurf 中。
