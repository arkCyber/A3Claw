# 云端 API 服务集成指南

## 概述

本指南介绍如何将 OpenAI、DeepSeek、Azure OpenAI 等云端 API 服务集成到 OpenClaw+ 作为冗余后端。

## 支持的云端服务

| 服务 | 端点 | 成本 | 性能 | 推荐场景 |
|------|------|------|------|----------|
| **OpenAI** | api.openai.com | 高 (~7¥/M tokens) | 优秀 | 生产环境最后保障 |
| **DeepSeek** | api.deepseek.com | 低 (~0.14¥/M tokens) | 良好 | 成本敏感场景 |
| **Azure OpenAI** | *.openai.azure.com | 中 | 优秀 | 企业级部署 |
| **自建服务** | 自定义 | 免费 | 可变 | 数据安全要求高 |

## 快速开始

### 1. 获取 API Key

#### OpenAI

1. 访问 https://platform.openai.com/api-keys
2. 点击 "Create new secret key"
3. 复制 API key (格式: `sk-...`)
4. 设置环境变量：
   ```bash
   export OPENAI_API_KEY="sk-your-key-here"
   ```

#### DeepSeek

1. 访问 https://platform.deepseek.com/api_keys
2. 注册账号并创建 API key
3. 复制 API key (格式: `sk-...`)
4. 设置环境变量：
   ```bash
   export DEEPSEEK_API_KEY="sk-your-key-here"
   ```

#### Azure OpenAI

1. 在 Azure Portal 创建 OpenAI 资源
2. 获取 API key 和 endpoint
3. 设置环境变量：
   ```bash
   export AZURE_OPENAI_API_KEY="your-key"
   export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
   ```

### 2. 配置后端

创建配置文件或使用代码配置：

```rust
use openclaw_inference::{InferenceConfig, BackendKind};
use std::time::Duration;

// OpenAI 配置
let openai_config = InferenceConfig {
    backend: BackendKind::OpenAiCompat,
    endpoint: "https://api.openai.com/v1".into(),
    model_name: "gpt-3.5-turbo".into(),
    api_key: std::env::var("OPENAI_API_KEY").ok(),
    max_tokens: 2048,
    temperature: 0.7,
    top_p: 0.95,
    inference_timeout: Duration::from_secs(60),
    circuit_breaker_threshold: 5,
    circuit_breaker_reset: Duration::from_secs(120),
    context_window: 16384,
    ..Default::default()
};

// DeepSeek 配置
let deepseek_config = InferenceConfig {
    backend: BackendKind::OpenAiCompat,
    endpoint: "https://api.deepseek.com/v1".into(),
    model_name: "deepseek-chat".into(),
    api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
    max_tokens: 2048,
    temperature: 0.7,
    top_p: 0.95,
    inference_timeout: Duration::from_secs(60),
    circuit_breaker_threshold: 5,
    circuit_breaker_reset: Duration::from_secs(120),
    context_window: 32768,
    ..Default::default()
};
```

### 3. 测试连接

运行多后端故障转移测试：

```bash
# 设置 API keys
export OPENAI_API_KEY="sk-..."
export DEEPSEEK_API_KEY="sk-..."

# 运行测试
cargo run --release -p openclaw-inference --example multi_backend_fallback
```

## 详细配置

### OpenAI API

**支持的模型**：
- `gpt-4` - 最强大，最贵
- `gpt-4-turbo` - 性能好，成本适中
- `gpt-3.5-turbo` - 快速，成本低（推荐）

**配置示例**：
```rust
InferenceConfig {
    backend: BackendKind::OpenAiCompat,
    endpoint: "https://api.openai.com/v1".into(),
    model_name: "gpt-3.5-turbo".into(),
    api_key: Some("sk-...".into()),
    max_tokens: 2048,
    temperature: 0.7,
    inference_timeout: Duration::from_secs(60),
    context_window: 16384,
    ..Default::default()
}
```

**成本估算**：
- GPT-3.5-turbo: $0.0005/1K tokens (输入), $0.0015/1K tokens (输出)
- 约 7¥/M tokens

### DeepSeek API

**支持的模型**：
- `deepseek-chat` - 通用对话模型
- `deepseek-coder` - 代码生成专用

**配置示例**：
```rust
InferenceConfig {
    backend: BackendKind::OpenAiCompat,
    endpoint: "https://api.deepseek.com/v1".into(),
    model_name: "deepseek-chat".into(),
    api_key: Some("sk-...".into()),
    max_tokens: 2048,
    temperature: 0.7,
    inference_timeout: Duration::from_secs(60),
    context_window: 32768,
    ..Default::default()
}
```

**成本估算**：
- DeepSeek-chat: ¥0.001/1K tokens (输入), ¥0.002/1K tokens (输出)
- 约 0.14¥/M tokens
- **比 OpenAI 便宜 50 倍！**

### Azure OpenAI

**配置示例**：
```rust
InferenceConfig {
    backend: BackendKind::OpenAiCompat,
    endpoint: "https://your-resource.openai.azure.com/openai/deployments/gpt-35-turbo".into(),
    model_name: "gpt-3.5-turbo".into(),
    api_key: Some("your-azure-key".into()),
    max_tokens: 2048,
    temperature: 0.7,
    inference_timeout: Duration::from_secs(60),
    context_window: 16384,
    ..Default::default()
}
```

**特点**：
- 企业级 SLA
- 数据隐私保护
- 区域部署选项
- 成本与 OpenAI 类似

## 多后端冗余策略

### 推荐的故障转移顺序

```
1. Ollama (本地, 免费, GPU)
   ↓ 失败
2. llama.cpp (本地备份, 免费, GPU)
   ↓ 失败
3. DeepSeek (云端, 低成本)
   ↓ 失败
4. OpenAI (云端, 高可用)
```

### 代码示例

参考 `examples/multi_backend_fallback.rs`：

```rust
// 尝试主服务
match primary_engine.infer(request.clone()).await {
    Ok(response) => return Ok(response),
    Err(e) => eprintln!("Primary failed: {}, trying backup...", e),
}

// 尝试备份服务
match backup_engine.infer(request.clone()).await {
    Ok(response) => return Ok(response),
    Err(e) => eprintln!("Backup failed: {}, trying cloud...", e),
}

// 尝试云端服务
cloud_engine.infer(request).await
```

## 安全最佳实践

### 1. API Key 管理

**❌ 不要这样做**：
```rust
// 硬编码在代码中
api_key: Some("sk-1234567890abcdef".into())
```

**✅ 应该这样做**：
```rust
// 从环境变量读取
api_key: std::env::var("OPENAI_API_KEY").ok()
```

### 2. 使用 .env 文件

创建 `.env` 文件（确保在 `.gitignore` 中）：
```bash
OPENAI_API_KEY=sk-your-key
DEEPSEEK_API_KEY=sk-your-key
```

加载环境变量：
```rust
// 在 main.rs 开头
dotenv::dotenv().ok();
```

### 3. 生产环境密钥管理

使用专业的密钥管理服务：
- **AWS Secrets Manager**
- **Azure Key Vault**
- **HashiCorp Vault**
- **Kubernetes Secrets**

### 4. 权限最小化

为 API key 设置适当的权限：
- 只授予必要的 API 访问权限
- 设置使用限额
- 定期轮换密钥
- 监控异常使用

## 成本控制

### 1. 设置使用限额

在 API 提供商控制台设置：
- 每月最大支出
- 每日请求限制
- 告警阈值

### 2. 本地优先策略

```rust
// 优先使用免费的本地服务
let backends = vec![
    ollama_config,      // 免费
    llama_cpp_config,   // 免费
    deepseek_config,    // 低成本
    openai_config,      // 高成本（最后保障）
];
```

### 3. 缓存响应

对于相同的请求，缓存响应避免重复调用：
```rust
use std::collections::HashMap;

let mut cache: HashMap<String, String> = HashMap::new();

if let Some(cached) = cache.get(&request_hash) {
    return Ok(cached.clone());
}
```

### 4. 监控成本

定期检查 API 使用情况：
```bash
# OpenAI 使用情况
curl https://api.openai.com/v1/usage \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

## 性能优化

### 1. 超时配置

根据服务特性设置合理的超时：
```rust
// 本地服务：短超时
inference_timeout: Duration::from_secs(30),

// 云端服务：长超时（考虑网络延迟）
inference_timeout: Duration::from_secs(60),
```

### 2. 并发控制

使用 Tokio 的 Semaphore 限制并发：
```rust
use tokio::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(10)); // 最多 10 个并发请求
let permit = semaphore.acquire().await?;
let response = engine.infer(request).await?;
drop(permit);
```

### 3. 重试策略

使用指数退避重试：
```rust
use tokio::time::{sleep, Duration};

for attempt in 0..3 {
    match engine.infer(request.clone()).await {
        Ok(response) => return Ok(response),
        Err(e) if attempt < 2 => {
            let backoff = Duration::from_secs(2_u64.pow(attempt));
            sleep(backoff).await;
        }
        Err(e) => return Err(e),
    }
}
```

## 故障排查

### OpenAI API 常见错误

**401 Unauthorized**：
```
原因: API key 无效或过期
解决: 检查 OPENAI_API_KEY 环境变量
```

**429 Rate Limit**：
```
原因: 超过速率限制
解决: 降低请求频率，或升级账户
```

**500 Server Error**：
```
原因: OpenAI 服务器错误
解决: 自动切换到备份服务
```

### DeepSeek API 常见错误

**余额不足**：
```
原因: 账户余额为 0
解决: 充值账户
```

**模型不存在**：
```
原因: model_name 拼写错误
解决: 使用 "deepseek-chat" 或 "deepseek-coder"
```

### 网络问题

**连接超时**：
```bash
# 测试网络连通性
curl -I https://api.openai.com/v1/models

# 使用代理
export https_proxy=http://proxy:port
```

## 测试清单

- [ ] OpenAI API key 已设置
- [ ] DeepSeek API key 已设置
- [ ] 多后端故障转移测试通过
- [ ] 成本监控已配置
- [ ] 使用限额已设置
- [ ] API key 已加入密钥管理
- [ ] .env 文件已加入 .gitignore
- [ ] 生产环境配置已验证

## 示例代码

完整示例见：
- `examples/multi_backend_fallback.rs` - 多后端故障转移
- `examples/http_backend_demo.rs` - HTTP 后端基础用法
- `config/inference_redundancy.toml` - 配置文件示例

## 总结

**推荐配置**：
1. **主服务**: Ollama (本地, 免费, GPU)
2. **本地备份**: llama.cpp (本地, 免费, GPU)
3. **云端备份 1**: DeepSeek (低成本, 高性价比)
4. **云端备份 2**: OpenAI (高可用, 最后保障)

**成本估算** (每月 100M tokens)：
- 仅本地服务: ¥0
- 本地 + DeepSeek: ~¥14
- 本地 + OpenAI: ~¥700

**可用性**：
- 单后端: ~99%
- 双后端: ~99.9%
- 四后端: ~99.99%

---

**快速测试命令**：

```bash
# 1. 设置 API keys
export OPENAI_API_KEY="sk-..."
export DEEPSEEK_API_KEY="sk-..."

# 2. 运行测试
cargo run --release -p openclaw-inference --example multi_backend_fallback

# 3. 查看结果
# ✅ 应该看到主服务（Ollama）成功响应
# 如果主服务失败，会自动尝试备份服务
```
