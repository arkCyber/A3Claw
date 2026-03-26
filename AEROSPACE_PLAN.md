# 航空航天级别实施计划

## 已完成 ✅
1. RAG 基础实现（TF-IDF，916行，完整测试）
2. 依赖配置（Cargo.toml 已添加 imap/lettre/caldav/qdrant）
3. 核心架构（AgentExecutor/ReAct/工具调用 100%对齐）

## 待实施（3个功能）

### 1. RAG-Qdrant 集成
- 文件：builtin_tools/rag_qdrant.rs
- 依赖：qdrant-client = "1.7"
- 测试：单元+集成+性能

### 2. 邮件集成
- 文件：builtin_tools/email_impl.rs  
- 依赖：imap="3.0", lettre="0.11"
- 功能：list/read/send/reply/delete

### 3. 日历集成
- 文件：builtin_tools/calendar_impl.rs
- 依赖：caldav="0.8"
- 功能：list/get/create/update/delete

## 测试标准
- 单元测试覆盖率 >80%
- 集成测试（Docker容器）
- 性能基准测试
- 错误注入测试

## 下一步
创建实现文件 → 编写测试 → 集成到 dispatch.rs
