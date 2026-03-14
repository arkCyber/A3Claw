# Assistant 工具调用设计方案

## 问题分析

Ollama 的 `qwen2.5:0.5b` 模型不支持原生 tool calling API，但我们可以通过以下方式实现：

## 方案：基于关键词触发的工具调用

### 架构

```
用户: "检查 Ollama 状态"
  ↓
Assistant 识别关键词 → 触发工具
  ↓
UI 执行工具 → 获取结果
  ↓
将结果添加到对话历史
  ↓
Assistant 基于结果生成回复
```

### 关键词映射

| 用户输入关键词 | 触发工具 | 说明 |
|---------------|---------|------|
| "检查 Ollama"、"Ollama 状态" | check_ollama_health | 检查 Ollama 服务 |
| "启动 Ollama"、"Ollama 启动" | start_ollama_service | 启动 Ollama |
| "检查配置"、"配置文件" | check_config | 检查配置 |
| "系统状态"、"健康检查" | get_system_status | 系统整体状态 |
| "如何"、"怎么"、"指南" | provide_guide | 操作指导 |

### 实现步骤

1. **在 AssistantSendQuery 中添加关键词检测**
   - 检查用户输入是否包含工具触发关键词
   - 如果匹配，先执行工具获取结果
   - 将工具结果作为系统消息添加到对话历史
   - 然后调用 Ollama 生成回复

2. **增强系统提示词**
   - 告诉 Assistant 它可以访问系统诊断工具
   - 当收到工具结果时，基于结果提供建议

3. **UI 反馈**
   - 显示工具执行状态（"正在检查 Ollama..."）
   - 显示工具结果
   - 显示 AI 的分析和建议

### 示例对话流程

**用户**: "Ollama 无法连接，怎么办？"

**系统检测**: 包含 "Ollama" → 触发 `check_ollama_health`

**工具执行**: 
```
❌ Ollama 服务未启动
端点: http://localhost:11434
错误: 无法连接
建议: 运行 'ollama serve' 启动服务
```

**添加到对话**:
```
[system] 工具执行结果: check_ollama_health
❌ Ollama 服务未启动...
```

**Assistant 回复**:
```
我检测到 Ollama 服务当前未启动。这是导致无法连接的原因。

解决方法：
1. 打开终端
2. 运行命令: ollama serve
3. 等待服务启动（约 2-3 秒）
4. 返回 OpenClaw+ 重试

如果需要，我可以尝试自动启动服务。请问是否需要我帮您启动？
```

**用户**: "是的，请启动"

**系统检测**: 包含 "启动" → 触发 `start_ollama_service`

**工具执行**: "✅ Ollama 服务已成功启动"

**Assistant 回复**:
```
太好了！Ollama 服务已经成功启动。现在您可以：
1. 在 AI Chat 页面开始对话
2. 模型列表应该会自动显示
3. 如果没有显示，请刷新页面

需要其他帮助吗？
```

## 优势

1. **简单可靠** - 不依赖模型的 tool calling 能力
2. **用户友好** - 自然语言触发，无需特殊命令
3. **可扩展** - 容易添加新工具
4. **透明** - 用户可以看到工具执行过程

## 实现清单

- [x] 创建 assistant_tools.rs 模块
- [ ] 在 AssistantSendQuery 中添加关键词检测
- [ ] 添加工具执行逻辑
- [ ] 增强系统提示词
- [ ] 添加 UI 状态显示
- [ ] 测试完整流程
