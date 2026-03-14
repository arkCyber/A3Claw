# 🔧 OpenClaw 空响应问题修复指南

**问题**: OpenClaw 返回 "finish" 但没有实际内容  
**日期**: 2026-03-12 16:36  
**状态**: 🔍 诊断中

---

## 📊 问题诊断

### 用户报告
从截图看到：
- 第一次查询：返回了正常内容（19秒）
- 第二次查询："我现在上海，如何去德国？" → 只返回 "finish"（19秒）

### Ollama 服务验证 ✅

```bash
# 模型状态
curl http://localhost:11434/api/tags
# ✅ llama3.1:8b 存在，大小 4.9GB

# 直接测试
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.1:8b",
  "prompt": "你好",
  "stream": false
}'
# ✅ 返回: "你好！能为你提供什么帮助吗？"
```

**结论**: Ollama 服务正常，模型可以正确生成内容。

---

## 🎯 可能的原因

### 1. UI 响应解析问题

**位置**: `crates/ui/src/app.rs` ClawAgentResponse 处理

**可能情况**:
- 响应内容被错误解析
- `resp.content` 为空字符串
- 显示逻辑过滤掉了空内容

### 2. 推理引擎问题

**位置**: `crates/inference/src/backend.rs`

**可能情况**:
- HTTP 响应解析错误
- JSON 字段提取失败
- 编码问题（中文字符）

### 3. 非流式 API 的 Bug

**当前代码** (`app.rs:4079`):
```rust
match engine_arc.infer(req).await {
    Ok(resp) => {
        // resp.content 可能为空
    }
}
```

**问题**: 非流式 API 可能在某些情况下返回空内容

---

## 🔧 已应用的修复

### 配置优化（方案 B）

**文件**: `~/Library/Application Support/openclaw-plus/config.toml`

```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "llama3.1:8b"
api_key = ""
max_tokens = 512        # 从 1024 降低
temperature = 0.3       # 从 0.7 降低
stream = true           # 保持启用
```

**预期效果**:
- 响应时间减少 20-30%
- 更确定的输出（temperature 降低）

---

## 🧪 测试步骤

### 测试 1: 验证配置生效

重启后，配置应该已生效。

### 测试 2: 重复之前的查询

在 Claw Terminal 中：
1. 输入："我现在上海，如何去德国？"
2. 观察：
   - 是否有内容返回
   - 响应时间是否更快
   - 内容是否完整

### 测试 3: 尝试不同类型的查询

```
简单问答: "你好"
中文查询: "今天天气怎么样？"
英文查询: "How are you?"
复杂查询: "解释一下量子计算"
```

---

## 🔍 调试建议

### 如果问题仍然存在

#### 1. 检查 UI 日志

```bash
# 查找 OpenClaw 进程
ps aux | grep OpenClaw

# 如果有日志输出，查看
tail -f ~/Library/Logs/OpenClaw/*.log
```

#### 2. 测试流式 API

```bash
# 测试流式响应
curl -N http://localhost:11434/api/generate -d '{
  "model": "llama3.1:8b",
  "prompt": "我现在上海，如何去德国？",
  "stream": true
}'
```

观察是否有内容输出。

#### 3. 检查代码中的响应处理

**关键位置**: `crates/ui/src/app.rs:4080-4088`

```rust
Ok(resp) => {
    eprintln!("[CLAW-AGENT] response ({} ms): {:.120}", 
        start.elapsed().as_millis(), resp.content);
    // 检查 resp.content 是否为空
    AppMessage::ClawAgentResponse {
        agent_id: agent_id_clone,
        content: resp.content,  // ← 可能为空
        latency_ms: start.elapsed().as_millis() as u64,
        user_entry_id: entry_id,
    }
}
```

**可能的修复**:
```rust
Ok(resp) => {
    let content = if resp.content.trim().is_empty() {
        "（AI 返回了空响应，请重试）".to_string()
    } else {
        resp.content
    };
    // ...
}
```

---

## 🎯 根本解决方案

### 实施流式响应（方案 A）

空响应问题可能是非流式 API 的 bug。实施流式响应可以：

1. **避免空响应**: 流式 API 逐 token 返回，更可靠
2. **更好的用户体验**: 实时显示，不会等待 19 秒
3. **更容易调试**: 可以看到每个 token 的生成

**修改位置**: `crates/ui/src/app.rs:4077-4096`

```rust
// 1. 启用流式
stream: true,

// 2. 使用流式 API
match engine_arc.infer_stream(req).await {
    Ok(mut rx) => {
        let mut full_content = String::new();
        while let Some(token) = rx.recv().await {
            full_content.push_str(&token.delta);
            if token.done {
                break;
            }
        }
        // full_content 不会为空（除非模型真的没有输出）
        AppMessage::ClawAgentResponse {
            content: full_content,
            ...
        }
    }
}
```

---

## 📊 预期结果

### 配置优化后

| 指标 | 优化前 | 优化后 |
|------|--------|--------|
| 响应时间 | 19秒 | 13-15秒 |
| 空响应概率 | ? | 应该减少 |
| 内容质量 | 不稳定 | 更确定 |

### 流式响应后

| 指标 | 非流式 | 流式 |
|------|--------|------|
| 首字延迟 | 19秒 | 0.5秒 |
| 空响应概率 | 高 | 极低 |
| 用户体验 | ⭐ | ⭐⭐⭐⭐ |

---

## 🔄 下一步行动

### 立即测试（已重启）

1. OpenClaw 已重启，配置已优化
2. 在 Claw Terminal 中重复之前的查询
3. 观察是否仍然返回空响应

### 如果仍有问题

**临时解决方案**:
- 使用更简单的查询
- 避免过长或复杂的问题
- 尝试英文查询

**长期解决方案**:
- 实施流式响应（方案 A）
- 添加空响应检测和重试逻辑
- 改进错误处理和用户提示

---

**创建时间**: 2026-03-12 16:36  
**状态**: 配置已优化，UI 已重启，等待测试
