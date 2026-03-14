# 🎨 OpenClaw UI 模型显示指南

## 📍 如何查看和切换 AI 模型

OpenClaw UI 中的 AI 模型显示在多个位置，以下是完整的使用指南。

---

## 🔍 模型显示位置

### 1. **Settings 页面**（主要模型管理）

这是管理 AI 模型的主要位置。

**如何访问**:
1. 启动 OpenClaw.app
2. 点击左侧导航栏的 **⚙️ Settings**
3. 滚动到 **AI Model Management** 部分

**功能**:
- ✅ 查看所有已安装的模型
- ✅ 查看模型详细信息（大小、参数、量化级别）
- ✅ 切换活动模型（点击 "Use" 按钮）
- ✅ 删除不需要的模型
- ✅ 下载新模型
- ✅ 搜索/过滤模型

**显示内容**:
```
llama3.1:8b
● ACTIVE  8B  Q4_0  llama
💾 4.9 GB  ·  📅 2 hours ago
[✓ In Use]  [Delete]
```

---

### 2. **Claw Terminal 页面**（快速切换）

在 Claw Terminal 底部有模型选择器。

**如何访问**:
1. 点击左侧导航栏的 **Claw Terminal**
2. 查看输入框上方的模型选择器

**功能**:
- ✅ 快速查看可用模型
- ✅ 一键切换模型
- ✅ 当前选中的模型高亮显示

**显示样式**:
```
Model: [llama3.1:8b] [qwen2.5:7b] [llama3.2:latest]
       ^^^^^^^^^^^^^^  (高亮显示当前模型)
```

---

### 3. **AI Chat 页面**（预设模型）

AI Chat 页面显示预设的常用模型。

**如何访问**:
1. 点击左侧导航栏的 **AI Chat**
2. 查看顶部的模型选择器

**功能**:
- ✅ 快速切换到预设模型
- ✅ 适合快速测试不同模型

**预设模型列表**:
```
qwen2.5:0.5b
qwen2.5:1.5b
qwen2.5:3b
llama3.2:1b
llama3.2:3b
phi3.5:mini
gemma2:2b
```

**注意**: 这些是预设列表，如果您安装了其他模型（如 llama3.1:8b），需要在 Settings 页面切换。

---

## 🚀 查看 llama3.1:8b 的步骤

### 方法 1: Settings 页面（推荐）

1. **启动应用**
   ```bash
   open ~/Applications/OpenClaw.app
   ```

2. **进入 Settings**
   - 点击左侧导航栏的 **⚙️ Settings**

3. **刷新模型列表**
   - 点击 **⟳ Refresh** 按钮
   - 等待 1-2 秒加载

4. **查找 llama3.1:8b**
   - 在模型列表中找到 `llama3.1:8b`
   - 应该显示：
     ```
     llama3.1:8b
     ● ACTIVE  8B  Q4_0  llama
     💾 4.9 GB  ·  📅 2 hours ago
     [✓ In Use]  [Delete]
     ```

5. **确认激活**
   - 如果显示 **✓ In Use**，说明已经是当前模型
   - 如果显示 **Use** 按钮，点击它切换到该模型

---

### 方法 2: Claw Terminal 页面

1. **进入 Claw Terminal**
   - 点击左侧导航栏的 **Claw Terminal**

2. **查看模型选择器**
   - 在输入框上方应该看到：
     ```
     Model: [llama3.1:8b] [qwen2.5:7b] [llama3.2:latest]
     ```

3. **切换模型**
   - 点击想要使用的模型按钮
   - 当前模型会高亮显示

---

## 🔧 故障排查

### 问题 1: 看不到 llama3.1:8b

**原因**: 模型列表未刷新

**解决**:
1. 进入 **Settings** 页面
2. 点击 **⟳ Refresh** 按钮
3. 等待 1-2 秒

**验证模型存在**:
```bash
/opt/homebrew/bin/ollama list
```

应该看到：
```
NAME               ID              SIZE      MODIFIED
llama3.1:8b        46e0c10c039e    4.9 GB    2 minutes ago
```

---

### 问题 2: 模型列表为空

**原因**: Ollama 服务未运行或无法连接

**解决**:
```bash
# 检查 Ollama 服务
ps aux | grep ollama

# 如果未运行，启动它
/opt/homebrew/bin/ollama serve &

# 等待 2-3 秒后刷新 UI
```

---

### 问题 3: 刷新后仍然看不到

**原因**: 配置文件中的端点不正确

**检查配置**:
```bash
cat ~/Library/Application\ Support/openclaw-plus/config.toml | grep endpoint
```

应该显示：
```toml
endpoint = "http://localhost:11434"
```

**如果不正确，修复**:
```bash
# 编辑配置文件
nano ~/Library/Application\ Support/openclaw-plus/config.toml

# 确保这一行正确：
endpoint = "http://localhost:11434"
```

---

## 📊 当前配置状态

### ✅ 已完成

- ✅ llama3.1:8b 已下载（4.9 GB）
- ✅ 配置文件已更新使用 llama3.1:8b
- ✅ OpenClaw.app 已重新编译和部署
- ✅ 模型自动刷新已启用

### 📝 配置文件

位置: `~/Library/Application Support/openclaw-plus/config.toml`

```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "llama3.1:8b"
api_key = ""
max_tokens = 4096
temperature = 0.7
stream = true
```

---

## 🎯 使用建议

### 在 Settings 页面

**优点**:
- ✅ 完整的模型信息
- ✅ 可以管理（删除/下载）
- ✅ 可以搜索过滤
- ✅ 显示模型状态

**适合**:
- 首次配置
- 模型管理
- 查看详细信息

### 在 Claw Terminal 页面

**优点**:
- ✅ 快速切换
- ✅ 无需离开工作页面
- ✅ 一键操作

**适合**:
- 日常使用
- 快速测试不同模型
- 命令行工作流

### 在 AI Chat 页面

**优点**:
- ✅ 预设常用模型
- ✅ 适合新手

**限制**:
- ⚠️ 只显示预设列表
- ⚠️ llama3.1:8b 不在预设中

**建议**:
- 使用 Settings 或 Claw Terminal 切换到 llama3.1:8b

---

## 💡 最佳实践

### 1. 启动后立即刷新

每次启动 OpenClaw.app 后：
1. 进入 **Settings**
2. 点击 **⟳ Refresh**
3. 确认当前模型正确

### 2. 使用搜索功能

如果模型很多：
1. 在 Settings 页面使用搜索框
2. 输入 `llama3.1` 快速定位
3. 或输入 `8b` 查找所有 8B 模型

### 3. 验证模型切换

切换模型后：
1. 检查是否显示 **● ACTIVE** 标记
2. 在 Claw Terminal 测试一条命令
3. 观察响应时间和质量

---

## 🆘 需要帮助？

### 查看日志

```bash
# OpenClaw 日志
tail -f /tmp/openclaw.log

# Ollama 日志
tail -f /tmp/ollama.log
```

### 完全重置

如果遇到问题：
```bash
# 1. 停止所有服务
killall OpenClaw
killall ollama

# 2. 重启 Ollama
/opt/homebrew/bin/ollama serve &

# 3. 等待 2 秒
sleep 2

# 4. 重启 OpenClaw
open ~/Applications/OpenClaw.app

# 5. 进入 Settings 并刷新
```

---

## 📸 界面截图说明

### Settings 页面布局

```
┌─────────────────────────────────────────┐
│ ⚙️ Settings                             │
├─────────────────────────────────────────┤
│                                         │
│ AI Model Management                     │
│ ┌─────────────────────────────────────┐ │
│ │ 3 models installed    [⟳ Refresh]  │ │
│ ├─────────────────────────────────────┤ │
│ │ 🔍 Filter models...                 │ │
│ ├─────────────────────────────────────┤ │
│ │ llama3.1:8b                         │ │
│ │ ● ACTIVE  8B  Q4_0  llama          │ │
│ │ 💾 4.9 GB  ·  📅 2 hours ago        │ │
│ │                    [✓ In Use] [Del] │ │
│ ├─────────────────────────────────────┤ │
│ │ qwen2.5:7b                          │ │
│ │ 7B  Q4_0  qwen                      │ │
│ │ 💾 4.7 GB  ·  📅 2 days ago         │ │
│ │                    [Use] [Delete]   │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

---

**创建日期**: 2026-03-12  
**当前模型**: llama3.1:8b  
**状态**: ✅ 已配置并可用
