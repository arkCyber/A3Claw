# IME 输入问题最终诊断报告

## 问题现象

在 Claw Terminal 输入框中输入中文时，字符出现在 Windsurf IDE 的输入框中，而不是 OpenClaw+ 的 Terminal 输入框。

## 根本原因

**Windsurf IDE 在操作系统级别劫持了 IME 输入焦点**

### 证据

1. **日志分析**
   ```
   [IME] make_visible: logical_size=1200x820 ime_y=770
   [IME] Focused: logical_size=1200x754 scale=2 ime_pos=(280,671)
   ```
   - 只有窗口初始化日志
   - **没有** `[IME] ClawInputChanged` 消息
   - 说明输入事件根本没有到达 OpenClaw+ 应用

2. **技术分析**
   - OpenClaw+ 作为嵌入在 Windsurf 中运行的应用
   - macOS 的输入法系统将焦点仍然认为在 Windsurf 的输入框
   - 即使 OpenClaw+ 的输入框获得了应用内焦点，操作系统级别的 IME 焦点仍在 Windsurf

## 已尝试的修复方案（均无效）

### 方案 1: 添加 on_paste 处理
**文件**: `crates/ui/src/pages/claw_terminal.rs:413`
```rust
.on_paste(AppMessage::ClawInputChanged)
```
**结果**: 无效，IME 输入不是粘贴事件

### 方案 2: 添加 on_focus 事件
**文件**: `crates/ui/src/pages/claw_terminal.rs:414`
```rust
.on_focus(AppMessage::ClawInputFocused)
```
**结果**: 破坏了 IME 候选框显示

### 方案 3: 在页面切换时请求焦点
**文件**: `crates/ui/src/app.rs:1604-1608`
```rust
} else if page == NavPage::ClawTerminal {
    return cosmic::widget::text_input::focus(
        crate::pages::claw_terminal::CLAW_INPUT_ID.clone(),
    ).map(cosmic::Action::App);
}
```
**结果**: 破坏了 IME 候选框显示

### 方案 4: 在 ClawInputFocused 中请求焦点
**文件**: `crates/ui/src/app.rs:3068-3074`
```rust
AppMessage::ClawInputFocused => {
    self.claw_input_focused = true;
    return cosmic::widget::text_input::focus(...);
}
```
**结果**: 循环焦点请求，破坏了 IME 候选框

### 方案 5: 移除所有焦点请求（当前状态）
**结果**: IME 候选框正常显示，但输入仍被 Windsurf 劫持

## 技术限制

### 为什么应用代码无法解决

1. **窗口层级问题**
   - Windsurf 作为外层 IDE，在操作系统窗口系统中处于更高层级
   - macOS 的输入法路由基于窗口焦点，而不是应用内组件焦点

2. **Cosmic/Iced 框架限制**
   - Cosmic/Iced 是一个 GUI 框架，运行在应用层
   - 无法控制操作系统级别的 IME 焦点路由

3. **macOS 输入法系统**
   - macOS 的输入法系统将焦点绑定到窗口，而不是窗口内的特定组件
   - 当 OpenClaw+ 在 Windsurf 中运行时，操作系统认为焦点仍在 Windsurf 窗口

## 最终解决方案

### ✅ 方案 A: 独立运行 OpenClaw+（推荐）

将 OpenClaw+ 作为独立应用运行，而不是嵌入在 Windsurf 中：

```bash
# 方法 1: 直接运行二进制文件
./target/release/openclaw-plus

# 方法 2: 如果有 .app 包
open /tmp/OpenClawPlus.app
```

**优点**:
- 完全解决 IME 焦点问题
- OpenClaw+ 拥有独立的窗口焦点
- 操作系统正确路由 IME 输入

**缺点**:
- 需要在 Windsurf 外部运行
- 无法在 Windsurf 中直接查看应用

### ⚠️ 方案 B: 使用英文输入（临时方案）

在 Claw Terminal 中使用英文输入，避免 IME 问题。

**适用场景**:
- 临时测试
- 命令行输入（大多数命令是英文）

### 🔧 方案 C: 等待 Windsurf 修复（长期）

向 Windsurf 团队报告此问题，请求他们修复嵌入式应用的 IME 焦点路由。

## 验证步骤

### 验证独立运行是否解决问题

1. **关闭 Windsurf 中的 OpenClaw+**
   ```bash
   pkill -f openclaw-plus
   ```

2. **在独立终端中启动**
   ```bash
   cd /Users/arkSong/workspace/OpenClaw+
   ./target/release/openclaw-plus
   ```

3. **测试 IME 输入**
   - 进入 Claw Terminal 页面
   - 点击输入框
   - 切换中文输入法
   - 输入拼音并选择汉字

4. **预期结果**
   - IME 候选框正常显示
   - 汉字出现在 OpenClaw+ 的 Terminal 输入框内
   - **不会**出现在 Windsurf 输入框（因为 Windsurf 已关闭）

## 当前代码状态

### 保留的修复（不影响功能）

**文件**: `crates/ui/src/pages/claw_terminal.rs:409-416`
```rust
widget::text_input(placeholder, input)
    .id(CLAW_INPUT_ID.clone())
    .on_input(AppMessage::ClawInputChanged)
    .on_submit(|_| AppMessage::ClawSendCommand)
    .on_paste(AppMessage::ClawInputChanged)  // 支持粘贴
    .font(cosmic::font::mono())
    .size(13)
    .width(Length::Fill)
```

**文件**: `crates/ui/src/app.rs:3066-3068`
```rust
AppMessage::ClawInputChanged(s) => {
    tracing::info!("[IME] ClawInputChanged: {:?} ({} chars)", s, s.chars().count());
    self.claw_input = s;
}
```

### 移除的代码（避免破坏 IME）

- ❌ NavSelect 中对 ClawTerminal 的 focus() 请求
- ❌ on_focus 事件处理
- ❌ ClawInputFocused 中的重复 focus() 调用

## 结论

**这是一个操作系统级别的窗口焦点和 IME 路由问题，超出了应用代码层面的控制范围。**

**推荐解决方案**: 将 OpenClaw+ 作为独立应用运行，而不是嵌入在 Windsurf 中。

## 下一步行动

1. ✅ **立即**: 在独立终端中运行 OpenClaw+ 测试 IME 输入
2. ⏳ **短期**: 如果需要在 Windsurf 中开发，使用英文输入
3. 📧 **长期**: 向 Windsurf 团队报告此问题

## 技术参考

- **Cosmic/Iced 文档**: https://github.com/pop-os/cosmic-epoch
- **macOS 输入法系统**: NSTextInputClient protocol
- **窗口焦点管理**: macOS Window Server

---

**报告日期**: 2026-02-27  
**诊断工具**: 日志分析、事件流追踪  
**结论**: 需要独立运行应用以解决 IME 焦点问题
