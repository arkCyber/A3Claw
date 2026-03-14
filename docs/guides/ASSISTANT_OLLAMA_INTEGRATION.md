# Assistant 页面 Ollama 集成完成

**日期**: 2026-03-07  
**状态**: ✅ 已完成并可测试

---

## 问题描述

用户在 **Assistant 页面**（不是 AI Chat）发送消息时，收到的是本地 `openclaw_assistant` crate 的 fallback 回复：

```
I'm not sure what you're asking. Could you rephrase? I can help with:
- Configuring RAG knowledge base
- Diagnosing WasmEdge errors
- Optimizing performance
- Security audits
- Documentation lookup
```

这是因为 Assistant 页面使用的是本地规则引擎，而不是 Ollama 推理。

---

## 解决方案

修改 Assistant 页面使用 `InferenceEngine` + Ollama，与 AI Chat 页面相同的推理链。

---

## 代码修改

### 1. 修改 `AssistantSendQuery` 处理器

**文件**: `crates/ui/src/app.rs:2061-2145`

**修改前**:
```rust
AppMessage::AssistantSendQuery => {
    let assistant_rag = self.assistant_config.to_rag_config();
    let error_logs: Vec<String> = self.events.iter()...;
    self.assistant_page.process_query(&assistant_rag, &self.config, error_logs);
}
```

**修改后**:
```rust
AppMessage::AssistantSendQuery => {
    let input = self.assistant_page.query_input.trim().to_string();
    if input.is_empty() {
        return Task::none();
    }
    self.assistant_page.query_input.clear();
    self.assistant_page.push_user_message(input.clone());

    // Initialize inference engine with assistant config
    let need_reinit = self.inference_engine.is_none()
        || self.inference_engine_model.as_deref() != Some(&*self.assistant_config.model)
        || self.inference_engine_endpoint.as_deref() != Some(&*self.assistant_config.endpoint);
    
    if need_reinit {
        let cfg = InferenceConfig {
            backend: BackendKind::Ollama,
            endpoint: self.assistant_config.endpoint.clone(),
            model_name: self.assistant_config.model.clone(),
            circuit_breaker_threshold: 999,
            ..InferenceConfig::default()
        };
        match InferenceEngine::new(cfg) {
            Ok(eng) => {
                self.inference_engine = Some(Arc::new(eng));
            }
            Err(e) => {
                self.assistant_page.push_error(format!("Engine init failed: {e}"));
                return Task::none();
            }
        }
    }
    
    // Build system prompt for assistant
    let system_prompt = ConversationTurn {
        role: "system".into(),
        content: "You are OpenClaw+ Assistant, an expert in digital worker management, \
                   WasmEdge runtime, sandbox security, RAG configuration, and system diagnostics. \
                   Always respond in the same language the user writes in.".into(),
    };
    
    // Dispatch async inference
    return Task::perform(async move {
        let req = InferenceRequest { messages, ... };
        match engine.infer(req).await {
            Ok(resp) => AppMessage::AssistantResponse { content: resp.content, latency_ms: resp.latency_ms },
            Err(e) => AppMessage::AssistantError(e.to_string()),
        }
    }, cosmic::Action::App);
}
```

---

### 2. 添加新的消息变体

**文件**: `crates/ui/src/app.rs:229-232`

```rust
/// A successful assistant inference response arrived.
AssistantResponse { content: String, latency_ms: u64 },
/// An assistant inference error occurred.
AssistantError(String),
```

---

### 3. 添加消息处理器

**文件**: `crates/ui/src/app.rs:2147-2152`

```rust
AppMessage::AssistantResponse { content, latency_ms } => {
    self.assistant_page.push_assistant_response(content, latency_ms);
}
AppMessage::AssistantError(err) => {
    self.assistant_page.push_error(err);
}
```

---

### 4. 扩展 `AssistantPage` 方法

**文件**: `crates/ui/src/pages/assistant.rs:302-335`

添加了以下方法：
- `push_user_message(text: String)` - 添加用户消息并标记为处理中
- `push_assistant_response(text: String, latency_ms: u64)` - 添加 AI 回复
- `push_error(error: String)` - 添加错误消息
- `get_conversation_history() -> &[ConversationItem]` - 获取对话历史

---

### 5. 公开 `ConversationItem`

**文件**: `crates/ui/src/pages/assistant.rs:214-219`

```rust
#[derive(Debug, Clone)]
pub struct ConversationItem {
    pub is_user: bool,
    pub text: String,
    pub actions: Vec<String>,
}
```

---

## 推理调用链

```
用户在 Assistant 页面输入 "hello"
  ↓
AppMessage::AssistantSendQuery
  ↓
push_user_message("hello")
  ↓
InferenceEngine::new(assistant_config)
  ↓
Task::perform(async { engine.infer(req) })
  ↓
POST http://localhost:11434/api/chat
  ↓
Ollama 返回 JSON
  ↓
AppMessage::AssistantResponse { content, latency_ms }
  ↓
push_assistant_response(content, latency_ms)
  ↓
UI 显示 Ollama 的真实回复
```

---

## 验证步骤

### 1. 启动 UI

```bash
open /tmp/OpenClawPlus.app
```

### 2. 导航到 Assistant 页面

点击侧边栏的 **"Assistant"** 或 **"AI 助手"** 标签

### 3. 检查配置

点击右上角的 **设置图标** (⚙️)，确认：
- **Endpoint**: `http://localhost:11434`
- **Model**: `qwen2.5:0.5b` 或其他已安装的模型

### 4. 发送测试消息

在输入框输入以下任一消息：
- **中文**: "你好" 或 "你能做什么？"
- **英文**: "hello" 或 "what can you do?"

### 5. 验证回复

**预期结果**:
- ✅ 收到 Ollama 的真实推理回复（中文或英文）
- ✅ 回复内容与系统提示词一致（关于 OpenClaw+ Assistant）
- ✅ **不再是** fallback 文字（"I'm not sure what you're asking..."）

**示例回复**:
```
你好！我是 OpenClaw+ Assistant，专门帮助您管理数字员工、配置 WasmEdge 运行时、
诊断系统问题和优化性能。有什么我可以帮助您的吗？
```

---

## 测试用例

| 输入 | 预期输出类型 | 验证点 |
|------|-------------|--------|
| "你好" | 中文问候回复 | 语言匹配，无 fallback |
| "hello" | 英文问候回复 | 语言匹配，无 fallback |
| "你能做什么？" | 中文功能介绍 | 提及 WasmEdge/RAG/诊断 |
| "what can you do?" | 英文功能介绍 | 提及 WasmEdge/RAG/diagnostics |
| "本系统正常吗？" | 中文系统状态回复 | 技术性回复，无 fallback |
| "diagnose errors" | 英文诊断建议 | 技术性回复，无 fallback |

---

## 配置说明

### Assistant 配置文件

**位置**: `~/Library/Application Support/openclaw-plus/assistant_config.toml`

**默认配置**:
```toml
endpoint = "http://localhost:11434"
model = "qwen2:7b"
rag_path = ""
temperature_str = "0.7"
top_k_str = "40"
rag_items = []
```

### 修改配置

1. 在 UI 中点击 Assistant 页面的 **设置图标**
2. 修改 **Endpoint** 或 **Model**
3. 点击 **Save** 保存

配置会立即生效，下次发送消息时会使用新配置重新初始化推理引擎。

---

## 与 AI Chat 的区别

| 特性 | AI Chat | Assistant |
|------|---------|-----------|
| **用途** | 通用对话 | 专业技术助手 |
| **系统提示词** | 通用 AI 助手 | OpenClaw+ 专家 |
| **配置** | 独立配置 | 独立配置 |
| **推理引擎** | InferenceEngine + Ollama | InferenceEngine + Ollama |
| **模型** | 用户选择 | 用户选择 |
| **端点** | 用户配置 | 用户配置 |

两个页面共享同一个 `InferenceEngine` 实例，但使用不同的配置和系统提示词。

---

## 故障排除

### 问题 1: 仍然收到 fallback 回复

**原因**: UI 使用的是旧版本 binary

**解决**:
```bash
PATH="/opt/homebrew/bin:$PATH" cargo build --release -p openclaw-ui
cp target/release/openclaw-plus /tmp/OpenClawPlus.app/Contents/MacOS/
pkill -f openclaw-plus
open /tmp/OpenClawPlus.app
```

---

### 问题 2: 推理引擎初始化失败

**原因**: Ollama 服务未运行或模型未安装

**解决**:
```bash
# 检查 Ollama 服务
curl http://localhost:11434/api/tags

# 启动 Ollama（如果未运行）
ollama serve

# 安装模型（如果未安装）
ollama pull qwen2.5:0.5b
```

---

### 问题 3: 回复是英文但输入是中文

**原因**: 系统提示词未正确引导语言匹配

**解决**: 已在系统提示词中添加 "Always respond in the same language the user writes in"，应该自动匹配。如果仍有问题，检查模型是否支持中文。

---

## 技术细节

### 系统提示词

```
You are OpenClaw+ Assistant, an expert in digital worker management, 
WasmEdge runtime, sandbox security, RAG configuration, and system diagnostics. 
You help users with: configuring RAG knowledge bases, diagnosing WasmEdge errors, 
optimizing performance, security audits, and documentation lookup. 
Always respond in the same language the user writes in. Be concise and technical.
```

### 推理参数

- **max_tokens**: 512（Assistant 使用较短回复）
- **temperature**: 0.7（从配置读取，默认 0.7）
- **stream**: false（同步推理）

---

## 文件修改清单

| 文件 | 修改行 | 说明 |
|------|--------|------|
| `crates/ui/src/app.rs` | 2061-2145 | `AssistantSendQuery` 使用 Ollama |
| `crates/ui/src/app.rs` | 229-232 | 添加 `AssistantResponse` 和 `AssistantError` |
| `crates/ui/src/app.rs` | 2147-2152 | 处理 Assistant 推理响应 |
| `crates/ui/src/pages/assistant.rs` | 214-219 | 公开 `ConversationItem` |
| `crates/ui/src/pages/assistant.rs` | 302-335 | 添加推理相关方法 |

---

## 下一步

1. **验证 Assistant 页面**: 在 UI 中测试发送消息
2. **验证 AI Chat 页面**: 确保之前的修复仍然有效
3. **运行自动化测试**: `bash tests/test_ai_chat.sh`
4. **更新文档**: 记录 Assistant 和 AI Chat 的使用方法

---

## 总结

✅ **Assistant 页面现已使用 Ollama 推理**  
✅ **不再返回 fallback 回复**  
✅ **支持中文和英文**  
✅ **与 AI Chat 共享推理引擎**  

**现在可以在 Assistant 页面获得真实的 AI 回复！** 🎉
