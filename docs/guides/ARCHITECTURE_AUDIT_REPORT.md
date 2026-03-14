# OpenClaw+ 架构审计报告

## 问题根源

### 1. **核心架构问题：UI 和 Gateway 分离**

当前架构存在严重的设计缺陷：

```
当前架构（错误）:
┌─────────────┐              ┌──────────────────┐
│   UI 进程   │              │ Gateway 进程     │
│             │              │ (需要手动启动)   │
│ - Claw      │─────✗────────│ - 端口 7878      │
│ - AgentExec │  无法连接    │ - Skills 执行    │
└─────────────┘              └──────────────────┘
```

**问题**:
- UI 启动时没有启动 Plugin Gateway
- AgentExecutor 硬编码连接 `localhost:7878`
- Gateway 不运行 → AgentExecutor 卡住 → Claw Terminal 无响应

### 2. **代码证据**

#### UI 初始化 (app.rs:1430-1436)
```rust
let run_mode = if let Ok(url) = std::env::var("OPENCLAW_GATEWAY_URL") {
    RunMode::Plugin { gateway_url: url }
} else {
    RunMode::Embedded  // ← 默认是 Embedded 模式
};
```

**问题**: `Embedded` 模式下，UI **没有启动 Gateway**！

#### AgentExecutor 配置 (app.rs:6018-6028)
```rust
let cfg = ExecutorConfig {
    llm_endpoint: endpoint,
    model,
    api_key: self.config.openclaw_ai.api_key.clone(),
    temperature: self.config.openclaw_ai.temperature,
    max_tokens: self.config.openclaw_ai.max_tokens,
    is_ollama: matches!(self.config.openclaw_ai.provider, AiProvider::Ollama),
    gateway_port: 7878,  // ← 硬编码端口
    max_steps: 15,
    timeout_secs: 120,
};
```

#### SkillDispatcher 初始化 (executor.rs:279-325)
```rust
let gw_url = format!("http://localhost:{}", config.gateway_port);
// ...
let dispatcher = SkillDispatcher::new(&gw_url);
```

**问题**: AgentExecutor **总是尝试连接 Gateway**，即使在 Embedded 模式下！

### 3. **运行模式混淆**

代码中有两个不同的 "Embedded" 概念：

1. **UI 的 RunMode::Embedded** (app.rs)
   - 表示 UI 直接管理 WasmEdge 沙箱
   - 通过 flume 通道接收沙箱事件
   - **但不启动 Plugin Gateway**

2. **AgentExecutor 的需求**
   - **总是需要 Gateway** 来执行 skills
   - 通过 HTTP 调用 Gateway 的 `/hooks/before-skill` 等端点
   - 没有 Gateway 就无法工作

### 4. **订阅机制问题**

UI 的订阅逻辑 (app.rs:6578-6604):

```rust
fn subscription(&self) -> Subscription<Self::Message> {
    // Plugin-mode: poll the gateway's /skills/events endpoint every second.
    if let RunMode::Plugin { gateway_url } = &self.run_mode {
        // 只有 Plugin 模式才订阅 Gateway 事件
        // ...
    }
    
    // Embedded mode: receive events directly from the flume channel.
    // 只订阅沙箱事件，不启动 Gateway
}
```

**问题**: Embedded 模式下，UI 订阅沙箱事件，但 **AgentExecutor 无法执行 skills**！

---

## 正确的架构设计

### 方案 A: 真正的 Embedded 模式（推荐）

UI 应该在 Embedded 模式下**内嵌启动 Gateway**：

```
正确架构:
┌────────────────────────────────────────┐
│         UI 进程 (单一进程)             │
│                                        │
│  ┌──────────┐      ┌───────────────┐  │
│  │  Cosmic  │      │ Plugin Gateway│  │
│  │   UI     │      │ (内嵌 HTTP)   │  │
│  │          │      │ localhost:7878│  │
│  └──────────┘      └───────────────┘  │
│       │                    ▲           │
│       │                    │           │
│       ▼                    │           │
│  ┌──────────────────────────────────┐ │
│  │      AgentExecutor               │ │
│  │  (通过 HTTP 调用内嵌 Gateway)    │ │
│  └──────────────────────────────────┘ │
│       │                                │
│       ▼                                │
│  ┌──────────────────────────────────┐ │
│  │      WasmEdge Sandbox            │ │
│  └──────────────────────────────────┘ │
└────────────────────────────────────────┘
```

**实现要点**:
1. UI 在 `init()` 时启动内嵌的 HTTP Gateway (axum)
2. Gateway 在后台线程运行，监听 `localhost:7878`
3. AgentExecutor 通过 HTTP 调用 Gateway
4. 所有组件在同一进程内，无需外部依赖

### 方案 B: 简化架构（备选）

如果不想运行 HTTP Gateway，可以让 AgentExecutor **直接调用 skill 实现**：

```
简化架构:
┌────────────────────────────────────────┐
│         UI 进程 (单一进程)             │
│                                        │
│  ┌──────────┐                          │
│  │  Cosmic  │                          │
│  │   UI     │                          │
│  └──────────┘                          │
│       │                                │
│       ▼                                │
│  ┌──────────────────────────────────┐ │
│  │      AgentExecutor               │ │
│  │  (直接调用 skill 函数)           │ │
│  │  - 无需 HTTP Gateway             │ │
│  │  - 直接函数调用                  │ │
│  └──────────────────────────────────┘ │
│       │                                │
│       ▼                                │
│  ┌──────────────────────────────────┐ │
│  │      WasmEdge Sandbox            │ │
│  └──────────────────────────────────┘ │
└────────────────────────────────────────┘
```

**问题**: 需要重构 AgentExecutor 的 SkillDispatcher，去除 HTTP 依赖。

---

## 修复方案（推荐方案 A）

### 步骤 1: 在 UI 中内嵌启动 Gateway

修改 `crates/ui/src/app.rs`:

```rust
// 在 OpenClawApp 结构体中添加字段
pub struct OpenClawApp {
    // ... 现有字段 ...
    
    /// Embedded gateway server handle (if running in Embedded mode)
    gateway_handle: Option<tokio::task::JoinHandle<()>>,
}

// 在 init() 中启动 Gateway
impl Application for OpenClawApp {
    fn init(
        core: Core,
        flags: Self::Flags,
    ) -> (Self, Task<cosmic::app::Message<Self::Message>>) {
        // ... 现有初始化代码 ...
        
        // 如果是 Embedded 模式，启动内嵌 Gateway
        let gateway_handle = if matches!(run_mode, RunMode::Embedded) {
            Some(start_embedded_gateway(config.clone()))
        } else {
            None
        };
        
        let mut app = Self {
            // ... 现有字段 ...
            gateway_handle,
        };
        
        // ...
    }
}

// 新增函数：启动内嵌 Gateway
fn start_embedded_gateway(config: SecurityConfig) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        use openclaw_plugin_gateway::{router, state};
        
        let state = state::GatewayState::new(config);
        let app = router::build_router(state.clone());
        
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 7878));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind Gateway on port 7878");
        
        tracing::info!("Embedded Gateway listening on http://127.0.0.1:7878");
        state.set_ready();
        
        axum::serve(listener, app)
            .await
            .expect("Gateway server error");
    })
}
```

### 步骤 2: 等待 Gateway 就绪

在启动 Gateway 后，等待它就绪：

```rust
// 在 init() 返回的 Task 中添加
let startup_task = Task::perform(
    async {
        // 等待 Gateway 启动
        for _ in 0..20 {
            if let Ok(resp) = reqwest::get("http://localhost:7878/health").await {
                if resp.status().is_success() {
                    tracing::info!("Embedded Gateway is ready");
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // ... 其他启动任务 ...
    },
    |_| cosmic::Action::App(AppMessage::Noop),
);
```

### 步骤 3: 更新启动脚本

简化启动脚本，只需启动 UI：

```bash
#!/usr/bin/env bash
# run.sh — 启动 OpenClaw+ (所有组件在一个进程中)

cargo build --release -p openclaw-ui
./target/release/openclaw-plus
```

---

## 当前问题总结

| 问题 | 影响 | 严重性 |
|------|------|--------|
| UI 不启动 Gateway | AgentExecutor 无法执行 skills | **严重** |
| Claw Terminal 卡住 | 用户无法使用对话功能 | **严重** |
| 需要手动启动 Gateway | 用户体验差，容易出错 | **高** |
| 架构设计混乱 | 维护困难，难以理解 | **高** |
| 文档缺失 | 用户不知道如何正确启动 | **中** |

---

## 实施计划

1. ✅ **审计完成** - 识别架构问题
2. ⏳ **修复 UI** - 添加内嵌 Gateway 启动逻辑
3. ⏳ **测试验证** - 确保所有组件正常工作
4. ⏳ **更新文档** - 说明新的启动方式
5. ⏳ **清理代码** - 移除旧的手动启动脚本

---

## 结论

**当前系统无法工作的根本原因**:

OpenClaw+ 的架构设计存在根本性缺陷。UI 声称支持 "Embedded" 模式，但实际上：

1. **UI 不启动 Gateway** - Embedded 模式下没有启动 Plugin Gateway
2. **AgentExecutor 依赖 Gateway** - 所有 skill 执行都需要通过 HTTP 调用 Gateway
3. **组件分离** - UI 和 Gateway 被设计为独立进程，但没有自动启动机制
4. **用户困惑** - 用户不知道需要手动启动 Gateway

**解决方案**:

实施方案 A，在 UI 进程中内嵌启动 Plugin Gateway，实现真正的单进程架构。所有组件（UI、Gateway、AgentExecutor、Sandbox）在同一进程中运行，用户只需启动一个程序。
