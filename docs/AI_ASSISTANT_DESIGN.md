# OpenClaw AI 助手设计方案

## 一、需求分析

### 1.1 用户痛点

基于 WasmEdge 的 OpenClaw 使用门槛：

#### 配置复杂度
- **WasmEdge 环境配置**: 路径、权限、依赖
- **安全策略配置**: 网络白名单、文件系统权限、资源限制
- **RAG 知识库配置**: 文件夹监控、索引设置、OCR 参数
- **Agent 配置**: 技能注册、沙箱参数、超时设置
- **多语言支持**: i18n 配置、语言切换

#### 故障排除难度
- **WasmEdge 运行时错误**: WASI 权限、模块加载失败
- **网络连接问题**: TLS 证书、代理配置、白名单
- **文件系统错误**: 路径映射、权限不足、preopen 配置
- **性能问题**: 内存限制、超时、并发控制
- **日志分析**: 多层日志、错误码理解

#### 最佳实践缺失
- 如何配置生产环境安全策略
- 如何优化 RAG 索引性能
- 如何调试 WASM 模块
- 如何监控 Agent 运行状态

---

## 二、AI 助手功能设计

### 2.1 核心功能

#### 1. 智能配置向导
```
用户: "我想添加一个新的知识库文件夹"
助手: 
  1. 检测当前 RAG 配置状态
  2. 引导用户选择文件夹路径
  3. 建议扩展名过滤 (基于文件夹内容)
  4. 配置 Watch/Write 权限
  5. 设置索引优先级
  6. 自动保存配置并验证
```

#### 2. 故障诊断专家
```
用户: "WasmEdge 启动失败，提示 WASI error 8"
助手:
  1. 识别错误码 (error 8 = EBADF, Bad file descriptor)
  2. 分析可能原因:
     - preopen 路径不存在
     - 路径格式错误 (GUEST:HOST 顺序)
     - 文件权限不足
  3. 检查当前配置
  4. 提供修复步骤
  5. 验证修复结果
```

#### 3. 性能优化顾问
```
用户: "RAG 索引很慢，如何优化？"
助手:
  1. 分析当前配置 (chunk_size, batch_size, OCR)
  2. 检测文件类型分布
  3. 建议优化参数:
     - 大文件禁用 OCR
     - 调整 chunk_size (512-1024)
     - 增加 batch_size
  4. 预估性能提升
  5. 应用配置并监控
```

#### 4. 安全策略助手
```
用户: "如何配置生产环境的安全策略？"
助手:
  1. 评估当前安全级别
  2. 建议最佳实践:
     - 网络白名单 (最小权限原则)
     - 文件系统隔离
     - 资源限制 (内存、超时)
     - 敏感操作确认
  3. 生成安全配置模板
  4. 验证配置有效性
```

#### 5. 交互式文档
```
用户: "如何使用 Python skill？"
助手:
  1. 搜索相关文档和示例
  2. 展示代码片段
  3. 解释参数和返回值
  4. 提供实际用例
  5. 链接到完整文档
```

---

## 三、技术架构

### 3.1 AI 助手组件

```
┌─────────────────────────────────────────────────────┐
│                   OpenClaw UI                        │
│  ┌───────────────────────────────────────────────┐  │
│  │         AI Assistant Panel (Cosmic UI)        │  │
│  │  - Chat Interface                             │  │
│  │  - Quick Actions                              │  │
│  │  - Context-aware Suggestions                  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│              AI Assistant Engine                     │
│  ┌─────────────────┐  ┌─────────────────────────┐  │
│  │  Intent Parser  │  │  Context Analyzer       │  │
│  │  - NLU          │  │  - Config State         │  │
│  │  - Command      │  │  - Error Logs           │  │
│  │    Recognition  │  │  - System Status        │  │
│  └─────────────────┘  └─────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────┐   │
│  │         Knowledge Base (RAG)                │   │
│  │  - OpenClaw Documentation                   │   │
│  │  - WasmEdge Manual                          │   │
│  │  - Error Code Database                      │   │
│  │  - Best Practices                           │   │
│  │  - Troubleshooting Guide                    │   │
│  └─────────────────────────────────────────────┘   │
│                                                      │
│  ┌─────────────────────────────────────────────┐   │
│  │         Action Executor                     │   │
│  │  - Config Modifier                          │   │
│  │  - Diagnostic Runner                        │   │
│  │  - Validation Checker                       │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│              LLM Backend (可选)                      │
│  - 本地模型 (WasmEdge + WASI-NN)                    │
│  - 云端 API (OpenAI, Claude, etc.)                  │
│  - 混合模式 (本地优先 + 云端备份)                   │
└─────────────────────────────────────────────────────┘
```

### 3.2 知识库构建

#### RAG 知识库内容
```toml
# crates/config/assistant_knowledge_base.toml

[[documents]]
category = "configuration"
title = "WasmEdge 环境配置"
content = """
WasmEdge 需要正确的 preopen 配置...
格式: GUEST_PATH:HOST_PATH
示例: /workspace:/Users/dev/workspace
"""
tags = ["wasmedge", "preopen", "filesystem"]
priority = 10

[[documents]]
category = "troubleshooting"
title = "WASI Error 8 (EBADF) 解决方案"
content = """
错误原因: Bad file descriptor
常见场景:
1. preopen 路径不存在
2. 路径格式错误
3. 权限不足
解决步骤: ...
"""
tags = ["error", "wasi", "filesystem"]
priority = 9

[[documents]]
category = "best_practices"
title = "生产环境安全配置"
content = """
最小权限原则:
- 网络白名单仅包含必需域名
- 文件系统仅映射必需目录
- 设置合理的资源限制
示例配置: ...
"""
tags = ["security", "production"]
priority = 8
```

### 3.3 提示词模板

```rust
// crates/assistant/src/prompts.rs

pub const SYSTEM_PROMPT: &str = r#"
你是 OpenClaw AI 助手，专门帮助用户配置和使用基于 WasmEdge 的 OpenClaw 系统。

你的职责:
1. 理解用户的配置需求和问题
2. 提供清晰、准确的解决方案
3. 引导用户完成配置步骤
4. 诊断和修复常见问题
5. 推荐最佳实践

你的知识范围:
- OpenClaw 架构和配置
- WasmEdge 运行时和 WASI
- 安全策略和权限管理
- RAG 知识库配置
- 性能优化
- 故障排除

回答风格:
- 简洁明了，避免冗长
- 提供可执行的步骤
- 包含代码示例
- 解释原理和原因
- 友好和耐心
"#;

pub const CONFIG_WIZARD_PROMPT: &str = r#"
用户想要: {user_intent}
当前配置状态: {current_config}
系统环境: {system_info}

请提供配置向导:
1. 分析用户需求
2. 检查当前配置
3. 生成配置步骤
4. 提供配置代码
5. 验证配置有效性
"#;

pub const TROUBLESHOOTING_PROMPT: &str = r#"
错误信息: {error_message}
错误日志: {error_logs}
系统状态: {system_status}
相关配置: {related_config}

请诊断问题:
1. 识别错误类型
2. 分析根本原因
3. 提供修复步骤
4. 预防类似问题
"#;
```

---

## 四、实现方案

### 4.1 Phase 1: 基础架构 (1-2 周)

#### 创建 `crates/assistant` crate
```rust
// crates/assistant/src/lib.rs

pub struct OpenClawAssistant {
    knowledge_base: KnowledgeBase,
    intent_parser: IntentParser,
    context_analyzer: ContextAnalyzer,
    action_executor: ActionExecutor,
}

impl OpenClawAssistant {
    pub async fn process_query(
        &self,
        query: &str,
        context: &SystemContext,
    ) -> Result<AssistantResponse> {
        // 1. 解析意图
        let intent = self.intent_parser.parse(query)?;
        
        // 2. 分析上下文
        let analysis = self.context_analyzer.analyze(context)?;
        
        // 3. 检索知识库
        let knowledge = self.knowledge_base.search(&intent, &analysis)?;
        
        // 4. 生成响应
        let response = self.generate_response(intent, knowledge, analysis)?;
        
        // 5. 执行操作 (如果需要)
        if response.requires_action {
            self.action_executor.execute(&response.action)?;
        }
        
        Ok(response)
    }
}

pub struct AssistantResponse {
    pub text: String,
    pub actions: Vec<SuggestedAction>,
    pub code_snippets: Vec<CodeSnippet>,
    pub related_docs: Vec<DocumentLink>,
}

pub enum SuggestedAction {
    ModifyConfig { path: String, changes: ConfigChanges },
    RunDiagnostic { test: DiagnosticTest },
    OpenDocument { url: String },
    ShowExample { example_id: String },
}
```

#### 知识库管理
```rust
// crates/assistant/src/knowledge_base.rs

pub struct KnowledgeBase {
    documents: Vec<Document>,
    index: VectorIndex,
}

impl KnowledgeBase {
    pub fn load_from_rag_config(rag_config: &RagConfig) -> Result<Self> {
        // 使用现有的 RAG 配置加载文档
        // 包括 OpenClaw 文档、WasmEdge 手册等
    }
    
    pub fn search(&self, query: &str, context: &Context) -> Result<Vec<Document>> {
        // 向量检索 + 关键词过滤
        // 返回最相关的文档片段
    }
    
    pub fn add_document(&mut self, doc: Document) -> Result<()> {
        // 动态添加新文档到知识库
    }
}

pub struct Document {
    pub id: String,
    pub category: DocumentCategory,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub priority: u8,
    pub embedding: Option<Vec<f32>>,
}

pub enum DocumentCategory {
    Configuration,
    Troubleshooting,
    BestPractices,
    API,
    Tutorial,
}
```

### 4.2 Phase 2: UI 集成 (1 周)

#### 添加 AI 助手面板到 UI
```rust
// crates/ui/src/pages/assistant.rs

pub struct AssistantPage {
    chat_history: Vec<ChatMessage>,
    input_text: String,
    is_processing: bool,
    suggested_actions: Vec<SuggestedAction>,
}

impl AssistantPage {
    pub fn view<'a>(&'a self, lang: Language) -> Element<'a, AppMessage> {
        let chat_view = self.build_chat_view(lang);
        let input_area = self.build_input_area(lang);
        let quick_actions = self.build_quick_actions(lang);
        
        widget::column::with_children(vec![
            chat_view,
            input_area,
            quick_actions,
        ])
        .spacing(16)
        .into()
    }
    
    fn build_quick_actions<'a>(&'a self, lang: Language) -> Element<'a, AppMessage> {
        // 快捷操作按钮
        widget::row::with_children(vec![
            widget::button("配置 RAG")
                .on_press(AppMessage::AssistantQuickAction(QuickAction::ConfigureRAG)),
            widget::button("诊断问题")
                .on_press(AppMessage::AssistantQuickAction(QuickAction::RunDiagnostics)),
            widget::button("安全检查")
                .on_press(AppMessage::AssistantQuickAction(QuickAction::SecurityAudit)),
        ])
        .spacing(8)
        .into()
    }
}

pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: SystemTime,
    pub actions: Vec<SuggestedAction>,
}

pub enum MessageRole {
    User,
    Assistant,
    System,
}
```

#### AppMessage 扩展
```rust
// crates/ui/src/app.rs

pub enum AppMessage {
    // ... 现有消息 ...
    
    // AI 助手消息
    AssistantQuerySubmit(String),
    AssistantQueryResult(Result<AssistantResponse>),
    AssistantQuickAction(QuickAction),
    AssistantActionExecute(SuggestedAction),
    AssistantClearHistory,
}

impl OpenClawApp {
    fn handle_assistant_query(&mut self, query: String) -> Task<AppMessage> {
        let assistant = self.assistant.clone();
        let context = self.get_system_context();
        
        Task::perform(
            async move {
                assistant.process_query(&query, &context).await
            },
            |result| cosmic::Action::App(AppMessage::AssistantQueryResult(result))
        )
    }
}
```

### 4.3 Phase 3: 知识库内容 (持续)

#### 创建文档集合
```
docs/assistant_kb/
├── configuration/
│   ├── wasmedge_setup.md
│   ├── rag_configuration.md
│   ├── security_policies.md
│   └── agent_configuration.md
├── troubleshooting/
│   ├── wasi_errors.md
│   ├── network_issues.md
│   ├── filesystem_errors.md
│   └── performance_problems.md
├── best_practices/
│   ├── production_deployment.md
│   ├── security_hardening.md
│   ├── performance_tuning.md
│   └── monitoring_logging.md
└── api/
    ├── skills_reference.md
    ├── config_api.md
    └── security_api.md
```

### 4.4 Phase 4: LLM 集成 (可选)

#### 本地模型 (WasmEdge + WASI-NN)
```rust
// crates/assistant/src/llm/local.rs

pub struct LocalLLM {
    model_path: PathBuf,
    wasm_runtime: WasmEdgeRuntime,
}

impl LocalLLM {
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        // 使用 WasmEdge + WASI-NN 运行本地模型
        // 例如: Llama 3.2 3B, Phi-3 Mini
    }
}
```

#### 云端 API 备份
```rust
// crates/assistant/src/llm/cloud.rs

pub struct CloudLLM {
    api_key: String,
    endpoint: String,
}

impl CloudLLM {
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        // 调用云端 API (OpenAI, Claude, etc.)
    }
}
```

#### 混合策略
```rust
// crates/assistant/src/llm/hybrid.rs

pub struct HybridLLM {
    local: LocalLLM,
    cloud: CloudLLM,
    strategy: LLMStrategy,
}

pub enum LLMStrategy {
    LocalFirst,      // 优先本地，失败时云端
    CloudFirst,      // 优先云端，失败时本地
    LocalOnly,       // 仅本地
    CloudOnly,       // 仅云端
    Balanced,        // 根据查询复杂度选择
}
```

---

## 五、示例场景

### 场景 1: 配置新知识库

**用户输入:**
```
"我想添加公司文档目录 /Users/work/company-docs 到知识库，
只索引 PDF 和 Markdown 文件"
```

**AI 助手响应:**
```
✓ 理解您的需求：添加知识库文件夹

当前 RAG 配置:
- 已有 2 个文件夹
- 总计 156 个文件已索引

建议配置:
1. 文件夹路径: /Users/work/company-docs
2. 名称: Company Documents
3. 扩展名过滤: pdf, md
4. Watch 监控: 启用 (自动检测文件变化)
5. Agent 写入: 禁用 (只读模式)

[应用配置] [自定义] [取消]
```

**点击"应用配置"后:**
```
✓ 配置已保存到 ~/.config/openclaw-plus/rag.toml
✓ 开始索引文件夹...
  - 发现 42 个 PDF 文件
  - 发现 18 个 Markdown 文件
  - 预计耗时: 2-3 分钟

[查看进度] [返回]
```

### 场景 2: 诊断 WasmEdge 错误

**用户输入:**
```
"Agent 启动失败，日志显示 WASI error 8"
```

**AI 助手响应:**
```
🔍 诊断结果: WASI Error 8 (EBADF - Bad File Descriptor)

根本原因:
您的 WasmEdge preopen 配置中的路径不存在:
  /workspace:/Users/dev/nonexistent

修复步骤:
1. 检查路径是否存在:
   $ ls -la /Users/dev/nonexistent
   
2. 如果路径不存在，创建它:
   $ mkdir -p /Users/dev/workspace
   
3. 或者修改配置为正确路径:
   打开: 设置 > 安全策略 > 文件系统权限
   修改: /workspace:/Users/dev/workspace

[自动修复] [手动修复] [查看详情]
```

**点击"自动修复":**
```
✓ 已创建目录: /Users/dev/workspace
✓ 已更新配置: ~/.config/openclaw-plus/config.toml
✓ 验证通过: WasmEdge 可以正常访问该路径

建议: 重启 Agent 以应用新配置

[重启 Agent] [完成]
```

### 场景 3: 性能优化建议

**用户输入:**
```
"RAG 索引太慢了，有什么优化方法？"
```

**AI 助手响应:**
```
📊 性能分析报告

当前配置:
- Chunk Size: 1000 (默认)
- Chunk Overlap: 200 (默认)
- Batch Size: 100 (默认)
- OCR: 启用 (对所有 PDF)

发现的问题:
1. 大量大型 PDF 文件 (平均 15MB)
2. OCR 处理耗时占 80%
3. Batch size 偏小

优化建议:
1. 对大文件禁用 OCR:
   max_file_size_mb: 10 (超过 10MB 跳过 OCR)
   
2. 增加 Batch Size:
   batch_size: 200 (提升并发)
   
3. 调整 Chunk Size:
   chunk_size: 800 (减少 chunk 数量)

预期提升: 索引速度提升 3-4 倍

[应用优化] [自定义参数] [了解更多]
```

---

## 六、实施优先级

### P0 (必须实现)
- ✅ 基础架构 (AssistantEngine, KnowledgeBase)
- ✅ UI 集成 (Chat 界面, 快捷操作)
- ✅ 核心知识库 (配置、故障排除)
- ✅ 意图识别 (配置、诊断、查询)

### P1 (重要)
- 🔄 上下文分析 (读取当前配置和状态)
- 🔄 操作执行 (自动修复配置)
- 🔄 代码生成 (配置模板、示例代码)
- 🔄 多语言支持 (中英文)

### P2 (增强)
- ⏳ 本地 LLM 集成 (WasmEdge + WASI-NN)
- ⏳ 云端 API 集成 (OpenAI, Claude)
- ⏳ 学习功能 (从用户反馈学习)
- ⏳ 高级诊断 (性能分析、安全审计)

### P3 (未来)
- 💡 语音交互
- 💡 可视化配置编辑器
- 💡 自动化测试生成
- 💡 社区知识库共享

---

## 七、成本收益分析

### 收益
1. **降低使用门槛**: 新用户 5 分钟上手 (vs 1 小时)
2. **减少支持成本**: 80% 常见问题自助解决
3. **提升用户满意度**: 即时帮助，无需查文档
4. **加速问题解决**: 故障诊断时间减少 70%
5. **推广最佳实践**: 自动推荐安全配置

### 成本
1. **开发时间**: 4-6 周 (Phase 1-3)
2. **维护成本**: 持续更新知识库
3. **计算资源**: 本地模型需要 2-4GB RAM
4. **API 费用**: 云端 LLM (可选)

### ROI
- 开发投入: 1 人月
- 用户时间节省: 每用户 2-3 小时/月
- 支持成本降低: 50-80%
- **预计 3 个月收回成本**

---

## 八、下一步行动

### 立即开始 (本周)
1. ✅ 创建设计文档 (本文档)
2. 🔄 搭建 `crates/assistant` 基础架构
3. 🔄 实现简单的规则引擎 (无 LLM)
4. 🔄 集成到 UI (基础 Chat 界面)

### 短期目标 (2 周内)
1. 完成核心知识库 (20+ 文档)
2. 实现配置向导功能
3. 实现故障诊断功能
4. 添加快捷操作

### 中期目标 (1 个月内)
1. 集成本地 LLM (可选)
2. 完善上下文分析
3. 实现自动修复
4. 多语言支持

### 长期目标 (3 个月内)
1. 社区知识库
2. 高级诊断工具
3. 性能优化建议
4. 安全审计功能

---

## 九、总结

为 OpenClaw 配备 AI 助手是**非常必要且可行**的：

✅ **必要性**
- 显著降低使用门槛
- 提升用户体验
- 减少支持成本
- 推广最佳实践

✅ **可行性**
- 技术栈成熟 (RAG, 规则引擎)
- 可以渐进式实现 (先规则后 LLM)
- 复用现有基础设施 (RAG 配置)
- 开发成本可控 (4-6 周)

✅ **优先级**
- **P0**: 基础架构 + UI + 核心知识库
- **P1**: 上下文分析 + 自动修复
- **P2**: LLM 集成 (可选)

**建议立即启动 Phase 1 开发！**
