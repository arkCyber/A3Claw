# 🎨 OpenClaw.app 使用指南

## ✅ 已完成

OpenClaw.app 已成功创建并安装到您的 macOS 系统！

---

## 📍 应用位置

```
~/Applications/OpenClaw.app
```

---

## 🚀 启动方式

### 方法 1: 通过 Finder（推荐）

1. 打开 Finder
2. 按 `Cmd + Shift + G`
3. 输入：`~/Applications/`
4. 双击 `OpenClaw.app`

### 方法 2: 通过命令行

```bash
open ~/Applications/OpenClaw.app
```

### 方法 3: 添加到 Dock

1. 打开 Finder，进入 `~/Applications/`
2. 将 `OpenClaw.app` 拖到 Dock 栏
3. 以后可以直接从 Dock 启动

---

## 🤖 AI 配置

应用已配置使用 **qwen3.5:9b** 模型：

```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "qwen3.5:9b"
max_tokens = 4096
temperature = 0.7
stream = true
```

**配置文件位置**：
```
~/Library/Application Support/openclaw-plus/config.toml
```

---

## 💡 使用提示

### 首次启动

1. **自动启动 Ollama**：应用会自动检查并启动 Ollama 服务
2. **加载 Rust 环境**：启动脚本会自动加载必要的环境变量
3. **等待窗口出现**：首次启动可能需要几秒钟

### AI Chat 功能

1. 启动应用后，切换到 **AI Chat** 页面
2. 点击输入框
3. 输入您的问题（支持中文）
4. 按 Enter 发送
5. 等待 qwen3.5:9b 响应（通常 3-5 秒）

### Claw Terminal 功能

1. 切换到 **Claw Terminal** 页面
2. 输入命令或自然语言请求
3. 系统会智能执行或转发给 AI

### Auto Test 功能

1. 点击 **🧪 Auto Test** 按钮
2. 自动运行 10 个测试用例
3. 查看测试结果和性能数据

---

## 🔍 故障排查

### 问题 1: 应用无法启动

**检查**：
```bash
# 查看应用是否存在
ls -la ~/Applications/OpenClaw.app

# 检查权限
ls -la ~/Applications/OpenClaw.app/Contents/MacOS/
```

**解决**：
```bash
# 重新设置权限
chmod -R 755 ~/Applications/OpenClaw.app
chmod +x ~/Applications/OpenClaw.app/Contents/MacOS/*
```

### 问题 2: Ollama 服务未运行

**检查**：
```bash
ps aux | grep ollama
```

**手动启动**：
```bash
/opt/homebrew/bin/ollama serve &
```

### 问题 3: AI 无响应

**检查模型**：
```bash
/opt/homebrew/bin/ollama list
```

**测试推理**：
```bash
curl http://localhost:11434/api/generate \
  -d '{"model":"qwen3.5:9b","prompt":"你好","stream":false}'
```

### 问题 4: 输入焦点丢失

**解决**：
- 用鼠标点击输入框
- 确保 OpenClaw 窗口在最前面
- 不要在输入时切换到其他应用

---

## 🔧 重新构建应用

如果需要更新应用（比如代码有修改）：

```bash
cd /Users/arkSong/workspace/OpenClaw+

# 1. 重新编译
cargo build -p openclaw-ui --release

# 2. 重新创建 app bundle
./create_app_bundle.sh

# 3. 启动新版本
open ~/Applications/OpenClaw.app
```

---

## 📊 应用结构

```
OpenClaw.app/
├── Contents/
│   ├── Info.plist          # 应用元数据
│   ├── MacOS/
│   │   ├── OpenClaw        # 主二进制文件
│   │   └── OpenClaw-launcher  # 启动脚本
│   └── Resources/          # 资源文件（图标等）
```

---

## 🎯 功能特性

### ✅ 已实现

- ✅ **AI Chat**: 与 qwen3.5:9b 对话
- ✅ **Claw Terminal**: 智能命令执行
- ✅ **Auto Test**: 自动化测试套件
- ✅ **Page Test**: UI 页面测试
- ✅ **Dashboard**: 系统状态监控
- ✅ **Settings**: 配置管理
- ✅ **多语言**: 中文/英文支持

### 🎨 UI 特性

- ✅ **Cosmic 框架**: 现代化 UI
- ✅ **自适应轮询**: 智能响应时间优化
- ✅ **错误日志**: 完整的调试信息
- ✅ **状态管理**: 实时状态更新

---

## 📝 日志位置

应用运行日志：
```
/tmp/openclaw.log
```

Ollama 服务日志：
```
/tmp/ollama.log
```

查看日志：
```bash
# OpenClaw 日志
tail -f /tmp/openclaw.log

# Ollama 日志
tail -f /tmp/ollama.log
```

---

## 🆘 获取帮助

### 查看配置

```bash
cat ~/Library/Application\ Support/openclaw-plus/config.toml
```

### 检查服务状态

```bash
# Ollama 服务
ps aux | grep ollama

# 可用模型
/opt/homebrew/bin/ollama list

# 端口占用
lsof -i :11434
```

### 完全重置

如果遇到问题，可以完全重置：

```bash
# 1. 停止所有服务
killall OpenClaw
killall ollama

# 2. 删除应用
rm -rf ~/Applications/OpenClaw.app

# 3. 重新创建
cd /Users/arkSong/workspace/OpenClaw+
cargo build -p openclaw-ui --release
./create_app_bundle.sh

# 4. 重新启动
open ~/Applications/OpenClaw.app
```

---

## 🎉 开始使用

**现在就启动应用吧！**

```bash
open ~/Applications/OpenClaw.app
```

或者在 Finder 中双击 `OpenClaw.app`

---

**创建日期**: 2026-03-12  
**版本**: 1.0.0  
**AI 模型**: qwen3.5:9b  
**状态**: ✅ 准备就绪
