# 智谱AI配置指南

## 配置信息

- **API端点**: `https://open.bigmodel.cn/api/paas/v4`
- **API Key**: `3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk`
- **推荐模型**:
  - `glm-4-flash` - 高速推理模型
  - `glm-4` - 高性能大模型

## 配置步骤

### 1. 设置环境变量

```bash
# 在 ~/.bashrc 或 ~/.zshrc 中添加
export ZHIPU_API_KEY="3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk"

# 重新加载配置
source ~/.bashrc  # 或 source ~/.zshrc
```

### 2. 启用智谱AI服务

编辑 `config/servers.toml`，将智谱AI启用：

```toml
[[servers]]
id = "zhipu-cloud"
name = "智谱AI (云端)"
type = "zhipu"
endpoint = "https://open.bigmodel.cn/api/paas/v4"
port = 443
enabled = true  # 改为 true
auto_start = false
model = "glm-4-flash"
# api_key 从环境变量 ZHIPU_API_KEY 读取
```

### 3. 编译并运行

```bash
# 编译项目
cargo build --release

# 运行OpenClaw+
./target/release/openclaw-ui
```

## 使用方法

### 在UI中使用

1. 打开OpenClaw+ UI
2. 进入"AI设置"页面
3. 选择"智谱AI (云端)"作为LLM后端
4. 选择模型：`glm-4-flash` 或 `glm-4`
5. 开始对话

### 在代码中使用

```rust
// 创建智谱AI配置
let zhipu_config = ReactConfig {
    endpoint: "https://open.bigmodel.cn/api/paas/v4".to_string(),
    model: "glm-4-flash".to_string(),
    api_key: Some("3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk".to_string()),
    temperature: 0.2,
    max_tokens: 2048,
    is_ollama: false,
};

// 创建LLM客户端
let llm_client = LlmClient::new(zhipu_config);
```

## 模型特性

### glm-4-flash
- **特点**: 高速推理，响应快
- **适用**: 实时对话、简单任务
- **成本**: 较低

### glm-4
- **特点**: 高性能，能力强
- **适用**: 复杂推理、长文本处理
- **成本**: 较高

## 故障转移

智谱AI已配置在故障转移链中，优先级为3：

```
1. ollama-primary (本地Ollama)
2. llama-cpp-backup (本地llama.cpp)
3. zhipu-cloud (智谱AI) ← 新增
4. deepseek-cloud (DeepSeek)
5. openai-cloud (OpenAI)
```

当本地服务不可用时，系统会自动切换到智谱AI。

## 注意事项

1. **API密钥安全**: 请妥善保管API密钥，不要提交到代码仓库
2. **网络要求**: 需要能够访问 `open.bigmodel.cn`
3. **配额限制**: 注意智谱AI的API调用配额限制
4. **费用**: 根据使用量计费，请关注费用情况

## 测试连接

```bash
# 测试智谱AI连接
curl -X POST "https://open.bigmodel.cn/api/paas/v4/chat/completions" \
  -H "Authorization: Bearer 3a27cba615f24a979fef006b3cb2487f.mXxpSoA7Vd0NDyUk" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "glm-4-flash",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 100
  }'
```

## 支持的功能

- ✅ 文本生成
- ✅ 工具调用 (Tool Calls)
- ✅ 流式响应
- ✅ 温度控制
- ✅ 最大Token限制
- ✅ 错误处理和重试

---

**配置完成！您现在可以使用智谱AI的强大能力了！** 🚀
