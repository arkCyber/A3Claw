# OpenClaw+ 工具系统完整指南

## 📋 概述

OpenClaw+ 提供了一个完整的工具生态系统，支持 AI Agent 执行各种任务。本文档涵盖所有工具的实现、测试和优化。

---

## 🛠️ 工具分类与状态

### 1. 文件系统工具 (fs.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `fs.read` | ✅ 完整 | Gateway | Safe |
| `fs.write` | ✅ 完整 | Gateway | Confirm |
| `fs.list` | ✅ 完整 | Gateway | Safe |
| `fs.delete` | ✅ 完整 | Gateway | Confirm |
| `fs.mkdir` | ✅ 完整 | Gateway | Confirm |
| `fs.move` | ✅ 完整 | Gateway | Confirm |
| `fs.copy` | ✅ 完整 | Gateway | Confirm |
| `fs.stat` | ✅ 完整 | Gateway | Safe |
| `fs.exists` | ✅ 完整 | Gateway | Safe |

### 2. 命令执行工具 (exec, process.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `exec` | ✅ 完整 | `exec.rs` | Deny |
| `process.list` | ✅ 完整 | `exec.rs` | Safe |
| `process.poll` | ✅ 完整 | `exec.rs` | Safe |
| `process.log` | ✅ 完整 | `exec.rs` | Safe |
| `process.kill` | ✅ 完整 | `exec.rs` | Confirm |
| `process.clear` | ✅ 完整 | `exec.rs` | Safe |

### 3. 网页工具 (web.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `web.fetch` | ✅ 完整 | `web_fetch.rs` | Safe |
| `web_fetch` | ✅ 完整 | `web_fetch.rs` | Safe |
| `web.screenshot` | ⚠️ Stub | `browser.rs` | Safe |
| `web.navigate` | ⚠️ Stub | `browser.rs` | Confirm |
| `web.click` | ⚠️ Stub | `browser.rs` | Confirm |
| `web.fill` | ⚠️ Stub | `browser.rs` | Confirm |
| `web_search` | ✅ 完整 | AgentExecutor | Safe |

### 4. 图像处理工具

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `image` | ✅ 完整 | `image.rs` | Safe |

### 5. 定时任务工具 (cron.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `cron.add` | ✅ 完整 | `cron.rs` | Confirm |
| `cron.list` | ✅ 完整 | `cron.rs` | Safe |
| `cron.remove` | ✅ 完整 | `cron.rs` | Confirm |
| `cron.enable` | ✅ 完整 | `cron.rs` | Confirm |
| `cron.disable` | ✅ 完整 | `cron.rs` | Confirm |

### 6. 会话管理工具 (sessions.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `sessions.list` | ✅ 完整 | `sessions.rs` | Safe |
| `sessions.history` | ✅ 完整 | `sessions.rs` | Safe |
| `sessions.send` | ✅ 完整 | `sessions.rs` | Confirm |
| `sessions.spawn` | ✅ 完整 | `sessions.rs` | Confirm |
| `session.status` | ✅ 完整 | `sessions.rs` | Safe |
| `agents.list` | ✅ 完整 | `sessions.rs` | Safe |

### 7. 补丁应用工具

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `apply_patch` | ✅ 完整 | `apply_patch.rs` | Confirm |

### 8. 安全工具 (security.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `security.scan` | ✅ 完整 | Gateway | Safe |
| `security.report` | ✅ 完整 | Gateway | Safe |

### 9. 循环检测工具 (loop_detection.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `loop_detection.check` | ✅ 完整 | Gateway | Safe |
| `loop_detection.reset` | ✅ 完整 | Gateway | Confirm |

### 10. 网关工具 (gateway.*)

| 工具 | 状态 | 实现位置 | 安全级别 |
|------|------|---------|---------|
| `gateway.status` | ✅ 完整 | Gateway | Safe |
| `gateway.config` | ✅ 完整 | Gateway | Safe |

---

## 🚀 需要补全的工具

### 高优先级

1. **浏览器自动化后端** (`web.screenshot`, `web.navigate`, `web.click`, `web.fill`)
   - 当前状态: Stub 实现
   - 需要: Playwright/Puppeteer 集成
   - 优先级: 高
   - 预计工作量: 2-3 天

2. **邮件工具** (`email.*`)
   - 当前状态: 未实现
   - 需要: SMTP/IMAP 客户端
   - 优先级: 中
   - 预计工作量: 1-2 天

3. **日历工具** (`calendar.*`)
   - 当前状态: 未实现
   - 需要: CalDAV 集成
   - 优先级: 中
   - 预计工作量: 1-2 天

### 中优先级

4. **知识库工具** (`knowledge.*`)
   - 当前状态: 未实现
   - 需要: 向量数据库集成
   - 优先级: 中
   - 预计工作量: 2-3 天

5. **消息工具** (`message.*`)
   - 当前状态: 部分实现
   - 需要: 完善多渠道支持
   - 优先级: 中
   - 预计工作量: 1-2 天

6. **画布工具** (`canvas.*`)
   - 当前状态: 部分实现
   - 需要: 完善渲染和操作
   - 优先级: 低
   - 预计工作量: 2-3 天

---

## 🔒 安全策略

### 安全级别定义

```rust
pub enum SecurityLevel {
    Safe,      // 只读操作，无副作用
    Confirm,   // 需要用户确认的操作
    Deny,      // 默认禁止的危险操作
}
```

### SSRF 防护

```rust
// 网络请求 SSRF 检查
pub fn check_ssrf(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    
    // 禁止访问内网地址
    if let Some(host) = parsed.host_str() {
        if is_private_ip(host) {
            return Err("Access to private network is forbidden".to_string());
        }
    }
    
    Ok(())
}
```

### 沙箱隔离

```toml
[sandbox]
enabled = true
network_isolation = true
filesystem_isolation = true
resource_limits = true

[sandbox.limits]
max_memory_mb = 512
max_cpu_percent = 50
max_execution_time_sec = 300
max_file_size_mb = 100
```

---

## ⚡ 性能优化

### 1. 并发执行

```rust
// 并发执行多个工具调用
use tokio::task::JoinSet;

pub async fn execute_parallel(skills: Vec<Skill>) -> Vec<Result<String, String>> {
    let mut set = JoinSet::new();
    
    for skill in skills {
        set.spawn(async move {
            execute_skill(skill).await
        });
    }
    
    let mut results = Vec::new();
    while let Some(res) = set.join_next().await {
        results.push(res.unwrap());
    }
    
    results
}
```

### 2. 缓存机制

```rust
use lru::LruCache;
use std::sync::Mutex;

lazy_static! {
    static ref CACHE: Mutex<LruCache<String, String>> = 
        Mutex::new(LruCache::new(100));
}

pub async fn cached_fetch(url: &str) -> Result<String, String> {
    // 检查缓存
    if let Some(cached) = CACHE.lock().unwrap().get(url) {
        return Ok(cached.clone());
    }
    
    // 执行请求
    let result = fetch(url).await?;
    
    // 存入缓存
    CACHE.lock().unwrap().put(url.to_string(), result.clone());
    
    Ok(result)
}
```

### 3. 资源池管理

```rust
use deadpool::managed::{Pool, Manager};

// HTTP 客户端池
pub struct HttpClientManager;

impl Manager for HttpClientManager {
    type Type = reqwest::Client;
    type Error = String;
    
    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| e.to_string())?)
    }
    
    async fn recycle(&self, _: &mut Self::Type) -> Result<(), Self::Error> {
        Ok(())
    }
}

lazy_static! {
    static ref HTTP_POOL: Pool<HttpClientManager> = 
        Pool::builder(HttpClientManager)
            .max_size(20)
            .build()
            .unwrap();
}
```

### 4. 流式处理

```rust
use futures::stream::{Stream, StreamExt};

// 流式处理大文件
pub async fn process_large_file(path: &str) -> Result<(), String> {
    let file = tokio::fs::File::open(path).await
        .map_err(|e| e.to_string())?;
    
    let reader = tokio::io::BufReader::new(file);
    let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
    
    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        // 逐行处理，避免内存溢出
        process_line(&line).await?;
    }
    
    Ok(())
}
```

---

## 🧪 测试策略

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_web_fetch_success() {
        let client = reqwest::Client::new();
        let args = WebFetchArgs {
            url: "https://example.com",
            extract_mode: ExtractMode::Text,
            timeout_sec: 10,
        };
        
        let result = fetch(&client, &args).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_ssrf_protection() {
        assert!(check_ssrf("http://127.0.0.1").is_err());
        assert!(check_ssrf("http://192.168.1.1").is_err());
        assert!(check_ssrf("https://example.com").is_ok());
    }
}
```

### 集成测试

```rust
#[tokio::test]
async fn test_skill_execution_pipeline() {
    // 1. 创建 Agent
    let agent = Agent::new("test-agent");
    
    // 2. 执行技能
    let result = agent.execute_skill("web.fetch", json!({
        "url": "https://example.com"
    })).await;
    
    // 3. 验证结果
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Example Domain"));
}
```

### 性能测试

```rust
#[tokio::test]
async fn test_concurrent_execution_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    // 并发执行 100 个请求
    let mut tasks = Vec::new();
    for i in 0..100 {
        tasks.push(execute_skill(format!("task-{}", i)));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let duration = start.elapsed();
    
    // 验证性能
    assert!(duration.as_secs() < 10); // 应在 10 秒内完成
    assert_eq!(results.len(), 100);
}
```

### 安全测试

```rust
#[test]
fn test_path_traversal_protection() {
    assert!(validate_path("../../../etc/passwd").is_err());
    assert!(validate_path("/etc/passwd").is_err());
    assert!(validate_path("./workspace/file.txt").is_ok());
}

#[test]
fn test_command_injection_protection() {
    assert!(validate_command("ls; rm -rf /").is_err());
    assert!(validate_command("ls && cat /etc/passwd").is_err());
    assert!(validate_command("ls -la").is_ok());
}
```

---

## 📊 监控和日志

### 结构化日志

```rust
use tracing::{info, warn, error, debug};

#[tracing::instrument]
pub async fn execute_skill(name: &str, args: Value) -> Result<String, String> {
    info!(skill = name, "Executing skill");
    
    let start = Instant::now();
    let result = match name {
        "web.fetch" => web_fetch(args).await,
        _ => Err(format!("Unknown skill: {}", name)),
    };
    
    let duration = start.elapsed();
    
    match &result {
        Ok(_) => info!(skill = name, duration_ms = duration.as_millis(), "Skill succeeded"),
        Err(e) => error!(skill = name, error = %e, "Skill failed"),
    }
    
    result
}
```

### 指标收集

```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref SKILL_EXECUTIONS: Counter = 
        register_counter!("skill_executions_total", "Total skill executions").unwrap();
    
    static ref SKILL_DURATION: Histogram = 
        register_histogram!("skill_duration_seconds", "Skill execution duration").unwrap();
}

pub async fn execute_with_metrics(skill: &str) -> Result<String, String> {
    SKILL_EXECUTIONS.inc();
    
    let timer = SKILL_DURATION.start_timer();
    let result = execute_skill(skill).await;
    timer.observe_duration();
    
    result
}
```

---

## 🔧 配置管理

### 配置文件结构

```toml
# ~/.config/openclaw-plus/config.toml

[openclaw_ai]
provider = "Ollama"
endpoint = "http://localhost:11434"
model = "qwen2.5:0.5b"
timeout_sec = 60

[browser]
enabled = true
engine = "playwright"
headless = false
timeout_ms = 30000

[security]
ssrf_protection = true
path_traversal_protection = true
command_injection_protection = true
max_file_size_mb = 100

[performance]
max_concurrent_skills = 10
cache_enabled = true
cache_size = 100
connection_pool_size = 20

[sandbox]
enabled = true
network_isolation = true
filesystem_isolation = true

[sandbox.limits]
max_memory_mb = 512
max_cpu_percent = 50
max_execution_time_sec = 300

[logging]
level = "info"
format = "json"
output = "stderr"
```

---

## 📚 最佳实践

### 1. 错误处理

```rust
// 使用 Result 类型
pub async fn safe_execute(skill: &str) -> Result<String, SkillError> {
    skill_execute(skill)
        .await
        .map_err(|e| SkillError::ExecutionFailed(e))
}

// 自定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Security violation: {0}")]
    SecurityViolation(String),
}
```

### 2. 资源清理

```rust
pub struct SkillExecutor {
    client: reqwest::Client,
}

impl Drop for SkillExecutor {
    fn drop(&mut self) {
        // 清理资源
        debug!("Cleaning up SkillExecutor resources");
    }
}
```

### 3. 超时控制

```rust
use tokio::time::{timeout, Duration};

pub async fn execute_with_timeout(skill: &str, timeout_sec: u64) -> Result<String, String> {
    timeout(
        Duration::from_secs(timeout_sec),
        execute_skill(skill)
    )
    .await
    .map_err(|_| "Skill execution timeout".to_string())?
}
```

---

## 🎯 路线图

### Phase 1: 核心工具完善 (当前)
- ✅ 文件系统工具
- ✅ 命令执行工具
- ✅ 网页抓取工具
- ⚠️ 浏览器自动化（需要完善）

### Phase 2: 扩展工具实现 (1-2 周)
- 📧 邮件工具
- 📅 日历工具
- 📚 知识库工具
- 💬 消息工具

### Phase 3: 高级功能 (2-4 周)
- 🎨 画布工具
- 🔊 语音工具
- 🎥 视频处理工具
- 🤖 多 Agent 协作

### Phase 4: 性能和安全优化 (持续)
- ⚡ 性能优化
- 🔒 安全加固
- 📊 监控完善
- 📖 文档完善

---

## 📖 相关文档

- [OpenClaw 官方文档](https://docs.openclaw.ai)
- [工具 API 参考](https://docs.openclaw.ai/tools)
- [安全策略](docs/SECURITY.md)
- [性能优化指南](docs/PERFORMANCE.md)
- [测试指南](docs/TESTING.md)

---

**最后更新**: 2026-03-02  
**维护者**: OpenClaw+ Team  
**许可**: MIT
