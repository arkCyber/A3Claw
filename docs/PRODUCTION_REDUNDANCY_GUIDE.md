# 生产环境冗余部署指南

## 概述

本指南介绍如何在生产环境中部署多后端冗余的 AI 推理服务，确保高可用性和故障转移能力。

## 架构设计

### 三层冗余架构

```
┌─────────────────────────────────────────────────────────┐
│  OpenClaw+ Application                                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  InferenceEngine (智能路由)                       │  │
│  │  - 健康检查                                        │  │
│  │  - 自动故障转移                                    │  │
│  │  - 负载均衡                                        │  │
│  └──────┬────────────┬────────────┬───────────────────┘  │
└─────────┼────────────┼────────────┼──────────────────────┘
          │            │            │
          ▼            ▼            ▼
    ┌─────────┐  ┌─────────┐  ┌──────────┐
    │ Primary │  │ Backup  │  │ Remote   │
    │ Ollama  │  │ llama   │  │ Server   │
    │ :11434  │  │ :8080   │  │ :8081    │
    └─────────┘  └─────────┘  └──────────┘
    本地主服务   本地备份      远程服务器
    GPU 加速     GPU 加速      GPU/云端
```

## 部署步骤

### 1. 主服务：Ollama (端口 11434)

**优势**：
- ✅ GPU 加速 (Metal)
- ✅ 模型管理简单
- ✅ 自动更新

**启动**：
```bash
# 安装
brew install ollama

# 启动服务
ollama serve

# 下载模型
ollama pull qwen3.5:9b
```

**健康检查**：
```bash
curl http://localhost:11434/api/tags
```

### 2. 备份服务：llama.cpp server (端口 8080)

**优势**：
- ✅ 轻量级
- ✅ 独立进程
- ✅ 本地冗余

**启动**：
```bash
# 使用提供的脚本
./scripts/start_llama_cpp_server.sh \
  models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  8080
```

**健康检查**：
```bash
curl http://localhost:8080/health
```

### 3. 远程服务器 (端口 8081)

**场景**：
- 🌐 多机房部署
- 🔄 跨地域冗余
- 📈 弹性扩展

**选项 A：另一台服务器运行 llama.cpp**
```bash
# 在远程服务器上
./scripts/start_llama_cpp_server.sh \
  /path/to/model.gguf \
  8081 \
  0.0.0.0  # 监听所有网络接口
```

**选项 B：云端 API 服务**
- OpenAI API
- Azure OpenAI
- 自建 GPU 集群

## 配置示例

### 单后端配置（简单）

```rust
use openclaw_inference::{InferenceConfig, BackendKind};

let config = InferenceConfig {
    backend: BackendKind::Ollama,
    endpoint: "http://localhost:11434".into(),
    model_name: "qwen3.5:9b".into(),
    max_tokens: 256,
    temperature: 0.7,
    inference_timeout: Duration::from_secs(30),
    // ...
};
```

### 多后端冗余配置（推荐）

```rust
use openclaw_inference::{InferenceEngine, InferenceConfig, BackendKind};
use std::time::Duration;

// 主服务：Ollama
let primary_config = InferenceConfig {
    backend: BackendKind::Ollama,
    endpoint: "http://localhost:11434".into(),
    model_name: "qwen3.5:9b".into(),
    max_tokens: 256,
    temperature: 0.7,
    inference_timeout: Duration::from_secs(30),
    circuit_breaker_threshold: 3,
    circuit_breaker_reset: Duration::from_secs(60),
    context_window: 8192,
};

// 备份服务：llama.cpp server
let backup_config = InferenceConfig {
    backend: BackendKind::LlamaCppHttp,
    endpoint: "http://localhost:8080".into(),
    model_name: "qwen2.5-0.5b-instruct-q4_k_m".into(),
    max_tokens: 256,
    temperature: 0.7,
    inference_timeout: Duration::from_secs(30),
    circuit_breaker_threshold: 3,
    circuit_breaker_reset: Duration::from_secs(60),
    context_window: 8192,
};

// 远程服务：另一台服务器或云端
let remote_config = InferenceConfig {
    backend: BackendKind::OpenAiCompat,
    endpoint: "http://remote-server:8081".into(),
    model_name: "gpt-3.5-turbo".into(),
    max_tokens: 256,
    temperature: 0.7,
    inference_timeout: Duration::from_secs(30),
    circuit_breaker_threshold: 5,
    circuit_breaker_reset: Duration::from_secs(120),
    context_window: 8192,
};

// 创建引擎（带自动故障转移）
let engine = InferenceEngine::new(primary_config)?;

// 手动故障转移示例
async fn infer_with_fallback(
    request: InferenceRequest,
) -> Result<InferenceResponse, InferenceError> {
    // 尝试主服务
    let primary_engine = InferenceEngine::new(primary_config.clone())?;
    match primary_engine.infer(request.clone()).await {
        Ok(response) => return Ok(response),
        Err(e) => {
            eprintln!("Primary service failed: {}, trying backup...", e);
        }
    }
    
    // 尝试备份服务
    let backup_engine = InferenceEngine::new(backup_config.clone())?;
    match backup_engine.infer(request.clone()).await {
        Ok(response) => return Ok(response),
        Err(e) => {
            eprintln!("Backup service failed: {}, trying remote...", e);
        }
    }
    
    // 尝试远程服务
    let remote_engine = InferenceEngine::new(remote_config.clone())?;
    remote_engine.infer(request).await
}
```

## 监控和健康检查

### 健康检查脚本

创建 `scripts/health_check.sh`：

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== AI Inference Services Health Check ==="
echo ""

# Check Ollama
echo -n "Ollama (primary):  "
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✅ Running"
else
    echo "❌ Down"
fi

# Check llama.cpp server
echo -n "llama.cpp (backup): "
if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Running"
else
    echo "❌ Down"
fi

# Check remote server (if configured)
echo -n "Remote server:     "
if curl -s http://remote-server:8081/health > /dev/null 2>&1; then
    echo "✅ Running"
else
    echo "⚠️  Not configured or down"
fi

echo ""
```

### 定时监控

使用 cron 或 systemd timer：

```bash
# 每分钟检查一次
* * * * * /path/to/scripts/health_check.sh >> /var/log/ai-health.log 2>&1
```

## 性能对比

| 服务 | 首次推理 | 后续推理 | GPU 加速 | 内存占用 | 启动时间 |
|------|----------|----------|----------|----------|----------|
| Ollama | ~5s | 300-500ms | ✅ Metal | ~2GB | ~2s |
| llama.cpp | ~3s | 200-400ms | ✅ Metal | ~1.5GB | ~1s |
| 远程服务器 | 网络延迟 | 100-300ms | ✅ CUDA | 远程 | N/A |

## 故障转移策略

### 自动故障转移

当前 `InferenceEngine` 已内置熔断器机制：

```rust
circuit_breaker_threshold: 3,      // 3次失败后熔断
circuit_breaker_reset: Duration::from_secs(60),  // 60秒后重试
```

### 手动故障转移

```bash
# 停止主服务
pkill -f "ollama serve"

# 应用会自动切换到备份服务（如果实现了故障转移逻辑）

# 或手动切换配置
export INFERENCE_ENDPOINT="http://localhost:8080"
```

## 负载均衡

### 简单轮询

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

static BACKEND_INDEX: AtomicUsize = AtomicUsize::new(0);

fn get_next_backend() -> InferenceConfig {
    let backends = vec![primary_config, backup_config, remote_config];
    let index = BACKEND_INDEX.fetch_add(1, Ordering::Relaxed) % backends.len();
    backends[index].clone()
}
```

### 基于延迟的智能路由

```rust
async fn select_fastest_backend() -> InferenceConfig {
    // 并发 ping 所有后端
    let (primary_latency, backup_latency, remote_latency) = tokio::join!(
        measure_latency("http://localhost:11434"),
        measure_latency("http://localhost:8080"),
        measure_latency("http://remote-server:8081"),
    );
    
    // 选择延迟最低的
    if primary_latency < backup_latency && primary_latency < remote_latency {
        primary_config
    } else if backup_latency < remote_latency {
        backup_config
    } else {
        remote_config
    }
}
```

## 部署检查清单

### 本地开发环境

- [ ] Ollama 已安装并运行
- [ ] 模型已下载 (`ollama pull qwen3.5:9b`)
- [ ] HTTP backend demo 测试通过
- [ ] 健康检查脚本可用

### 生产环境（单机）

- [ ] Ollama 主服务运行（端口 11434）
- [ ] llama.cpp 备份服务运行（端口 8080）
- [ ] 两个服务使用不同的模型副本（避免文件锁）
- [ ] 配置了自动重启（systemd/launchd）
- [ ] 设置了监控和告警
- [ ] 日志轮转已配置

### 生产环境（多机）

- [ ] 主服务器运行 Ollama
- [ ] 备份服务器运行 llama.cpp
- [ ] 远程服务器或云端 API 已配置
- [ ] 网络防火墙规则已设置
- [ ] 负载均衡器已配置（可选）
- [ ] 跨机房网络延迟已测试
- [ ] 故障转移流程已演练

## 故障排查

### Ollama 无法启动

```bash
# 检查端口占用
lsof -i :11434

# 查看日志
tail -f ~/.ollama/logs/server.log

# 重启服务
pkill ollama && ollama serve
```

### llama.cpp server 崩溃

```bash
# 检查模型文件
ls -lh models/gguf/

# 检查内存
vm_stat

# 降低 GPU 层数
llama-server --model model.gguf --n-gpu-layers 0
```

### 远程服务器连接失败

```bash
# 测试网络连通性
ping remote-server

# 测试端口
telnet remote-server 8081

# 检查防火墙
sudo iptables -L
```

## 最佳实践

### 1. 模型版本管理

```bash
# 为不同服务使用相同模型的不同副本
models/
├── ollama/          # Ollama 管理的模型
├── llama-cpp/       # llama.cpp 使用的模型
│   └── qwen2.5-0.5b-instruct-q4_k_m.gguf
└── remote/          # 远程服务器的模型
```

### 2. 配置管理

使用环境变量或配置文件：

```bash
# .env
PRIMARY_ENDPOINT=http://localhost:11434
BACKUP_ENDPOINT=http://localhost:8080
REMOTE_ENDPOINT=http://remote-server:8081
```

### 3. 监控指标

关键指标：
- 请求成功率
- 平均响应时间
- P95/P99 延迟
- 错误率
- 服务可用性

### 4. 容量规划

| 并发请求 | 推荐配置 |
|----------|----------|
| < 10 | 单个 Ollama 实例 |
| 10-50 | Ollama + llama.cpp 备份 |
| 50-100 | 多个 llama.cpp 实例 + 负载均衡 |
| > 100 | 分布式集群 + 云端 API |

## 总结

**推荐的生产部署方案**：

1. **主服务**：Ollama (GPU 加速，易管理)
2. **本地备份**：llama.cpp server (轻量级，独立进程)
3. **远程冗余**：另一台服务器或云端 API (跨地域容灾)

这种三层架构提供了：
- ✅ 高可用性（99.9%+）
- ✅ 故障自动转移
- ✅ 性能优化（GPU 加速）
- ✅ 成本可控（本地优先）
- ✅ 灵活扩展

---

**快速开始**：

```bash
# 1. 启动主服务
ollama serve

# 2. 启动备份服务（新终端）
./scripts/start_llama_cpp_server.sh models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf 8080

# 3. 运行健康检查
./scripts/health_check.sh

# 4. 测试推理
cargo run --release -p openclaw-inference --example http_backend_demo
```
