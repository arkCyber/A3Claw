# Qwen 3.5 9B 模型配置总结

**配置时间**: 2026-03-21 07:36  
**模型**: Qwen 3.5 9B  
**状态**: ✅ 配置完成，模型下载中

---

## 📥 模型下载

### 下载命令
```bash
ollama pull qwen3.5:9b
```

### 下载状态
- **模型大小**: 6.6 GB
- **当前进度**: 24% (1.6 GB / 6.6 GB)
- **预计剩余时间**: ~18 分钟
- **下载速度**: 4.5 MB/s

### 验证下载
下载完成后，运行以下命令验证：
```bash
ollama list
```

应该看到 `qwen3.5:9b` 在列表中。

---

## ⚙️ 已完成的配置修改

### 1. 默认模型设置 ✅

**文件**: `crates/ui/src/pages/assistant.rs`

**修改位置**: 第 98 行

**修改内容**:
```rust
// 之前
model: "qwen2:7b".to_string(),

// 修改后
model: "qwen3.5:9b".to_string(),
```

### 2. 预设模型列表 ✅

**文件**: `crates/ui/src/pages/assistant.rs`

**修改位置**: 第 1193 行

**修改内容**:
```rust
const PRESET_MODELS: &[&str] = &[
    "qwen3.5:9b",  // ← 新增，放在首位
    "qwen2:7b", "qwen2.5:7b", "qwen2.5:14b", "qwen2.5:32b",
    "llama3.2:3b", "llama3.2:latest",
    "mistral:7b", "deepseek-r1:7b", "deepseek-r1:14b",
    "phi3:mini", "phi3:medium", "gemma2:9b", "gemma2:27b",
    "codellama:7b", "codellama:13b",
];
```

### 3. 优化的参数配置 ✅

**文件**: `crates/ui/src/pages/assistant.rs`

**修改位置**: 第 100-101 行

**修改内容**:
```rust
// Temperature: 降低到 0.6 以获得更稳定的输出
temperature_str: "0.6".to_string(),

// Top-K: 提高到 50 以增加多样性
top_k_str: "50".to_string(),
```

### 4. 全局 AI 参数优化 ✅

**文件**: `crates/ui/src/app.rs`

**修改位置**: 第 1492-1493 行

**修改内容**:
```rust
// Max Tokens: 提高到 8192 以充分利用长上下文能力
openclaw_ai_max_tokens_input: "8192".to_string(),

// Temperature: 统一为 0.6
openclaw_ai_temperature_input: "0.6".to_string(),
```

### 5. 测试用例更新 ✅

**文件**: `crates/ui/src/pages/assistant.rs`

**修改位置**: 第 1435 行

**修改内容**:
```rust
const PRESETS: &[&str] = &[
    "qwen3.5:9b",  // ← 新增到测试
    "qwen2:7b", "qwen2.5:7b", "llama3.2:latest", "mistral:7b",
    "deepseek-r1:7b", "phi3:mini", "gemma2:9b", "codellama:7b",
];
```

---

## 🎯 优化参数说明

### Qwen 3.5 9B 的优势

1. **更强的推理能力**
   - 统一的视觉-语言基础
   - 在推理、编码、代理和视觉理解方面超越 Qwen 3

2. **长上下文支持**
   - 支持更长的上下文窗口
   - 因此我们将 max_tokens 提高到 8192

3. **更好的稳定性**
   - 降低 temperature 到 0.6 可以获得更一致的输出
   - 适合代码生成和技术问答

### 参数对比

| 参数            | 之前 (Qwen 2.5 7B) | 现在 (Qwen 3.5 9B) | 说明           |
| --------------- | ------------------ | ------------------ | -------------- |
| **模型**        | qwen2:7b           | qwen3.5:9b         | 升级到最新版本 |
| **Temperature** | 0.7                | 0.6                | 更稳定的输出   |
| **Top-K**       | 40                 | 50                 | 更多样化的选择 |
| **Max Tokens**  | 4096               | 8192               | 支持更长的响应 |

---

## 🚀 下一步操作

### 1. 等待下载完成

模型正在下载中，预计还需要 **18 分钟**。

你可以运行以下命令监控进度：
```bash
watch -n 5 'ollama list'
```

### 2. 验证模型安装

下载完成后，运行：
```bash
ollama list
```

应该看到：
```
NAME               ID              SIZE      MODIFIED
qwen3.5:9b         xxxxxxxxxx      6.6 GB    just now
qwen2.5:7b         845dbda0ea48    4.7 GB    10 days ago
llama3.1:8b        46e0c10c039e    4.9 GB    8 days ago
```

### 3. 测试模型

下载完成后，可以通过命令行测试：
```bash
ollama run qwen3.5:9b "你好，请介绍一下你自己"
```

或者在 OpenClaw+ 应用中：
1. 打开应用
2. 进入 **AI Assistant** 页面
3. 模型选择器中应该会看到 `qwen3.5:9b`（已自动设为默认）
4. 发送测试消息验证

### 4. 重新编译应用

配置已修改，需要重新编译：
```bash
cargo build --release -p openclaw-ui --bin openclaw-plus
```

或者直接运行：
```bash
cargo run -p openclaw-ui --bin openclaw-plus
```

---

## 📊 性能预期

使用 Qwen 3.5 9B 后，你应该会看到以下改进：

1. **更准确的代码生成**
   - 更好的语法理解
   - 更少的错误

2. **更强的推理能力**
   - 复杂问题的解答更准确
   - 逻辑链更清晰

3. **更好的多轮对话**
   - 更长的上下文记忆
   - 更连贯的对话

4. **更快的响应**
   - 9B 参数量在性能和速度之间取得平衡
   - 比 14B/32B 模型更快

---

## 🔧 故障排除

### 如果模型下载失败

1. 检查网络连接
2. 检查磁盘空间（需要至少 7 GB）
3. 重新运行下载命令：
   ```bash
   ollama pull qwen3.5:9b
   ```

### 如果应用中看不到新模型

1. 确认模型已下载：`ollama list`
2. 重启应用
3. 在 AI Assistant 页面点击 "Fetch Models" 按钮

### 如果性能不如预期

1. 检查系统资源（RAM 至少 16 GB 推荐）
2. 调整 temperature 参数（0.5-0.7 之间）
3. 调整 max_tokens（根据需要降低到 4096）

---

## 📝 配置文件位置

所有配置会保存在：
```
~/.config/openclaw-plus/assistant_config.json
```

如果需要手动编辑，可以直接修改此文件。

---

## ✅ 配置检查清单

- [x] Qwen 3.5 9B 模型下载中
- [x] 默认模型设置为 `qwen3.5:9b`
- [x] 预设模型列表已更新
- [x] Temperature 优化为 0.6
- [x] Top-K 优化为 50
- [x] Max Tokens 提高到 8192
- [x] 测试用例已更新
- [ ] 等待模型下载完成
- [ ] 验证模型可用
- [ ] 重新编译应用
- [ ] 测试新模型性能

---

**注意**: 所有配置修改已完成，模型下载完成后即可使用。应用会自动使用新的默认配置。
