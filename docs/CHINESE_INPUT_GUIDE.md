# OpenClaw+ 中文输入法使用指南

## 🎯 问题说明

在 macOS 上，直接运行二进制文件会导致中文输入法不工作，因为 macOS 会将 IME（输入法）焦点保持在启动终端上，而不是应用窗口。

## ✅ 正确的解决方案

### 方法 1：使用 run.sh 脚本（推荐）

这是**唯一正确**的启动方式，可以确保中文输入法正常工作：

```bash
./scripts/run.sh
```

**工作原理**：
1. 自动构建应用
2. 创建 macOS .app bundle (`/tmp/OpenClawPlus.app`)
3. 写入 `Info.plist` 让 macOS 正确识别为 GUI 应用
4. 使用 `open` 命令启动 .app bundle
5. macOS 正确分配 IME 焦点到应用窗口

### 方法 2：手动创建 .app bundle

如果你想手动创建 .app bundle：

```bash
# 1. 构建应用
cargo build --release -p openclaw-ui

# 2. 创建 bundle 结构
mkdir -p /tmp/OpenClawPlus.app/Contents/MacOS
mkdir -p /tmp/OpenClawPlus.app/Contents/Resources

# 3. 复制二进制文件
cp target/release/openclaw-plus /tmp/OpenClawPlus.app/Contents/MacOS/

# 4. 创建 Info.plist
cat > /tmp/OpenClawPlus.app/Contents/Info.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>   <string>com.openclaw.plus</string>
  <key>CFBundleName</key>         <string>OpenClaw+</string>
  <key>CFBundleDisplayName</key>  <string>OpenClaw+</string>
  <key>CFBundleExecutable</key>   <string>openclaw-plus</string>
  <key>CFBundleVersion</key>      <string>0.1.0</string>
  <key>CFBundlePackageType</key>  <string>APPL</string>
  <key>LSUIElement</key>          <false/>
  <key>NSHighResolutionCapable</key> <true/>
  <key>NSPrincipalClass</key>     <string>NSApplication</string>
</dict>
</plist>
EOF

# 5. 启动应用
open /tmp/OpenClawPlus.app
```

## ❌ 错误的启动方式

以下方式会导致中文输入法**不工作**：

```bash
# ❌ 不要使用 start-ui.sh
./scripts/start-ui.sh

# ❌ 不要直接运行二进制文件
./target/release/openclaw-plus

# ❌ 不要使用 cargo run
cargo run --release -p openclaw-ui
```

**原因**：这些方式都是从终端直接启动二进制文件，macOS 会将 IME 焦点保持在终端上。

## 🔧 技术细节

### libcosmic IME 补丁

OpenClaw+ 已经应用了以下 libcosmic/iced 补丁来支持 IME：

1. **补丁 1**: IME 多字符提交修复
   - 位置: `~/.cargo/git/checkouts/libcosmic-*/*/src/widget/text_input/input.rs`
   - 功能: 支持输入"你好"等多字符中文

2. **补丁 2**: IME 启用
   - 位置: `~/.cargo/git/checkouts/libcosmic-*/*/iced/winit/src/program.rs`
   - 功能: 调用 `window.set_ime_allowed(true)` 启用 IME

3. **补丁 3**: IME 事件转发
   - 位置: `~/.cargo/git/checkouts/libcosmic-*/*/iced/winit/src/conversion.rs`
   - 功能: 将 IME Commit 事件转发到应用

4. **补丁 4**: 候选窗口位置
   - 位置: `~/.cargo/git/checkouts/libcosmic-*/*/iced/winit/src/program.rs`
   - 功能: 正确定位输入法候选窗口

### macOS .app Bundle 要求

macOS 需要以下 `Info.plist` 配置才能正确处理 IME：

- `CFBundleIdentifier`: 应用标识符
- `CFBundleExecutable`: 可执行文件名
- `LSUIElement`: 设置为 `false` 表示这是一个正常的 GUI 应用
- `NSPrincipalClass`: 设置为 `NSApplication` 表示这是一个 Cocoa 应用
- `NSHighResolutionCapable`: 支持 Retina 显示

## 📝 使用指南

### 启动应用

```bash
cd /Users/arkSong/workspace/OpenClaw+
./scripts/run.sh
```

### 测试中文输入

1. **切换到中文输入法**
   - 按 `Command + Space` 或点击输入法图标
   - 选择"简体拼音"或其他中文输入法

2. **在 Claw Terminal 输入中文**
   - 点击 Claw Terminal 标签页
   - 在输入框中输入拼音
   - 选择候选字
   - 按回车确认

3. **在 AI Chat 页面使用中文对话**
   - 点击 AI Chat 标签页
   - 输入中文问题
   - 发送消息

### 测试用例

尝试输入以下中文：
- 你好世界
- OpenClaw+ 数字员工平台
- 测试中文输入法功能
- 人工智能助手

## 🔍 故障排除

### 问题 1: 中文字符出现在终端而不是应用窗口

**原因**: 使用了错误的启动方式

**解决方案**: 使用 `./scripts/run.sh` 启动应用

### 问题 2: 候选窗口位置不正确

**原因**: 补丁 4 未正确应用

**解决方案**: 
```bash
./scripts/apply-ime-patches.sh
cargo build --release -p openclaw-ui
./scripts/run.sh
```

### 问题 3: 只能输入第一个字符

**原因**: 补丁 1 未正确应用

**解决方案**:
```bash
./scripts/apply-ime-patches.sh
cargo build --release -p openclaw-ui
./scripts/run.sh
```

### 问题 4: 输入法完全不工作

**原因**: 补丁 2 或 3 未正确应用

**解决方案**:
```bash
./scripts/apply-ime-patches.sh
cargo build --release -p openclaw-ui
./scripts/run.sh
```

## 🧪 测试脚本

运行中文输入法测试：

```bash
./tests/test_chinese_input.sh
```

这个脚本会检查：
- IME 补丁文档是否存在
- IME 补丁是否已应用
- UI 二进制是否已构建
- 应用是否正在运行
- 系统环境是否支持中文

## 📚 相关文档

- `docs/libcosmic-patches.md` - libcosmic IME 补丁详细说明
- `scripts/apply-ime-patches.sh` - IME 补丁应用工具
- `scripts/run.sh` - 正确的启动脚本
- `scripts/start-ui.sh` - 简单启动脚本（不支持中文输入）

## 🎊 总结

**记住**：在 macOS 上使用 OpenClaw+ 时，**必须**使用 `./scripts/run.sh` 启动应用才能让中文输入法正常工作！

这不是一个 bug，而是 macOS IME 系统的设计要求。直接运行二进制文件会导致 IME 焦点保持在终端上，只有通过 .app bundle 启动才能让 macOS 正确分配 IME 焦点到应用窗口。

---

**最后更新**: 2026-03-01  
**维护者**: arkSong (arksong2018@gmail.com)  
**许可**: MIT
