# OpenClaw AI Assistant

智能助手，用于 OpenClaw+ 配置、故障排除和性能优化。

## 🎯 核心功能

- **智能配置向导**: 引导用户配置 RAG 知识库、WasmEdge 环境
- **故障诊断专家**: 自动识别和解决 WASI 错误、配置问题
- **性能优化顾问**: 分析配置瓶颈，提供优化建议
- **安全策略助手**: 审计安全配置，推荐最佳实践
- **文档查询系统**: 快速检索相关文档和示例

## 📦 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
openclaw-assistant = { path = "../assistant" }
```

## 🚀 快速开始

```rust
use openclaw_assistant::{OpenClawAssistant, SystemContext};

// 创建助手
let assistant = OpenClawAssistant::new()?;

// 创建系统上下文
let context = SystemContext::new()
    .with_rag_config(rag_config)
    .with_security_config(security_config);

// 处理用户查询
let response = assistant.process_query(
    "How to add a new knowledge base folder?",
    &context,
)?;

// 显示响应
println!("{}", response.text);

// 执行建议操作
for action in response.actions {
    println!("Suggested action: {:?}", action);
}
```

## 📚 使用场景

### 场景 1: 配置 RAG 知识库

```rust
let response = assistant.process_query(
    "Add new RAG folder",
    &context,
)?;

// 响应包含:
// - 配置步骤说明
// - 当前配置状态
// - 建议的操作 (ConfigureRAG action)
```

### 场景 2: 诊断 WASI 错误

```rust
let context = SystemContext::new()
    .with_error_logs(vec![
        "WASI error 8: Bad file descriptor".to_string()
    ]);

let response = assistant.process_query(
    "WasmEdge error 8",
    &context,
)?;

// 响应包含:
// - 错误原因 (EBADF)
// - 常见场景
// - 修复步骤
// - 诊断操作 (RunDiagnostic action)
```

### 场景 3: 优化性能

```rust
let response = assistant.process_query(
    "RAG indexing is too slow",
    &context,
)?;

// 响应包含:
// - 性能分析
// - 优化建议
// - 预期提升
// - 优化操作 (OptimizeRAG action)
```

### 场景 4: 安全审计

```rust
let response = assistant.process_query(
    "Run security audit",
    &context,
)?;

// 响应包含:
// - 安全问题列表
// - 风险评估
// - 修复建议
// - 安全操作 (ApplySecurity action)
```

## 🏗️ 架构

```
OpenClawAssistant
├── IntentParser      # 意图识别
├── KnowledgeBase     # 知识库管理
├── ContextAnalyzer   # 上下文分析
└── ActionExecutor    # 操作执行
```

### Intent Parser (意图识别)

支持的意图类型：
- `ConfigureRAG`: RAG 配置相关
- `DiagnoseError`: 错误诊断
- `OptimizePerformance`: 性能优化
- `SecurityAudit`: 安全审计
- `QueryDocumentation`: 文档查询
- `Unknown`: 未知意图

### Knowledge Base (知识库)

内置文档：
- WasmEdge Filesystem Preopen 配置
- WASI Error 8 (EBADF) 故障排除
- WASI Error 2 (ENOENT) 故障排除
- RAG 性能优化指南
- 生产环境安全配置

### Context Analyzer (上下文分析)

分析指标：
- RAG 配置状态 (文件夹数、文件数、设置)
- 安全配置状态 (网络白名单、文件系统权限)
- 错误日志模式
- 系统信息

### Action Executor (操作执行)

支持的操作：
- `ConfigureRAG`: 修改 RAG 配置
- `RunDiagnostic`: 运行诊断测试
- `OptimizeRAG`: 应用优化参数
- `ApplySecurity`: 应用安全模板
- `OpenDocument`: 打开相关文档

## 🧪 测试

运行所有测试：

```bash
cargo test -p openclaw-assistant
```

运行特定测试：

```bash
# 单元测试
cargo test -p openclaw-assistant --lib

# 集成测试
cargo test -p openclaw-assistant --test integration_test
```

测试覆盖：
- **24 tests** (13 单元测试 + 11 集成测试)
- **0 failures**
- **代码覆盖率 85%+**

## 📊 性能

- Intent 解析: < 1ms
- 知识库搜索: < 5ms
- 上下文分析: < 1ms
- 响应生成: < 10ms
- **总响应时间: < 20ms**

## 🔧 扩展

### 添加新的意图类型

```rust
// 在 intent.rs 中添加新的 Intent 变体
pub enum Intent {
    // ... 现有意图
    CustomIntent { params: String },
}

// 在 IntentParser 中添加关键词模式
IntentPattern {
    keywords: vec!["custom", "keyword"],
    intent_type: IntentType::CustomIntent,
}
```

### 添加新的知识文档

```rust
kb.add_document(Document {
    id: "custom-doc".to_string(),
    category: DocumentCategory::Tutorial,
    title: "Custom Documentation".to_string(),
    content: "Your content here...".to_string(),
    tags: vec!["custom".to_string()],
    priority: 5,
});
```

## 📖 文档

- [设计文档](../../docs/AI_ASSISTANT_DESIGN.md)
- [测试报告](../../docs/AI_ASSISTANT_TEST_REPORT.md)

## 🤝 贡献

欢迎贡献！请确保：
1. 所有测试通过
2. 添加新功能的测试
3. 更新相关文档

## 📄 许可证

MIT License
