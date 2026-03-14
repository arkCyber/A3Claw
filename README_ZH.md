# OpenClaw+ 🛡️

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![WasmEdge](https://img.shields.io/badge/WasmEdge-0.14%2B-blue.svg)](https://wasmedge.org/)

> **AI 智能体安全平台** — 基于 WasmEdge 沙箱、可视化工作流编辑器和企业级安全控制的综合性 AI 智能体执行平台。

[English](README.md) | [中文](README_ZH.md)

## ✨ 核心特性

OpenClaw+ 在不修改 OpenClaw 源代码的情况下，将其包装在 WasmEdge WASI 沙箱中，提供：

- 🔒 **文件系统隔离** — 仅允许访问配置的工作区目录；自动阻止敏感路径如 `.ssh/` 和 `/etc/passwd`
- 🌐 **网络访问控制** — 出站连接仅限于明确的 LLM API 主机白名单
- 💻 **Shell 命令拦截** — 每次 shell 执行尝试都需要用户明确批准（人在回路中）
- 🗑️ **文件删除保护** — 删除前显示确认对话框，防止意外数据丢失
- 📊 **实时监控仪表板** — 原生 libcosmic UI 实时可视化所有沙箱事件
- 🔴 **断路器** — 当异常阈值超标时自动触发并终止沙箱（过多拒绝、危险命令或内存过载）
- 📝 **审计日志** — 所有操作以 NDJSON 格式持久化，便于事后审查
- 🤖 **AI 助手** — 内置 AI 助手，支持系统诊断、优化建议和安全审计
- 🎨 **可视化工作流** — 拖拽式流程编辑器，轻松构建复杂的 AI 工作流
- 🔌 **插件系统** — 丰富的插件生态，支持自定义扩展

## 🏗️ 架构设计

```
┌─────────────────────────────────────────────┐
│         libcosmic 监控 UI                    │
│  仪表板 | 事件日志 | 设置 | AI 助手 | 确认   │
└──────────────┬──────────────────────────────┘
               │ 事件流 / 控制命令
┌──────────────▼──────────────────────────────┐
│      Rust 安全层 (openclaw-security)         │
│  策略引擎 | 拦截器 | 审计日志 | 断路器       │
└──────────────┬──────────────────────────────┘
               │ WASI 系统调用拦截
┌──────────────▼──────────────────────────────┐
│     WasmEdge 沙箱 (openclaw-sandbox)        │
│  WasmEdge-QuickJS | WASI 能力映射           │
│  Node.js 安全 Shim (预脚本)                 │
└──────────────┬──────────────────────────────┘
               │ 受控文件系统视图
┌──────────────▼──────────────────────────────┐
│        OpenClaw 源码（未修改）               │
│  在标准 Node.js 环境中运行                   │
└─────────────────────────────────────────────┘
```

## 📦 项目结构

```
OpenClaw+/
├── Cargo.toml                    # 工作区根配置
├── openclaw.plugin.json          # OpenClaw 插件清单
├── config/
│   └── default.toml              # 默认安全配置模板
├── crates/
│   ├── security/                 # 安全策略引擎（核心库）
│   ├── sandbox/                  # WasmEdge 主机进程（嵌入模式）
│   ├── plugin/                   # OpenClaw 插件网关（插件模式）
│   ├── ui/                       # libcosmic 监控 UI
│   ├── assistant/                # AI 助手引擎
│   ├── agent-executor/           # 智能体执行引擎
│   ├── inference/                # AI 推理后端
│   ├── storage/                  # 数据存储层
│   └── voice/                    # 语音交互模块
├── agents/                       # 智能体配置文件
├── docs/                         # 详细文档
└── scripts/                      # 构建和部署脚本
```

## 🚀 快速开始

### 前置要求

```bash
# 安装 WasmEdge（嵌入模式必需）
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash

# macOS
brew install pkg-config

# Ubuntu / Debian
sudo apt-get install -y libwayland-dev libxkbcommon-dev pkg-config cmake \
    libfontconfig1-dev libfreetype6-dev
```

### 构建

```bash
git clone https://github.com/arksong2018/openclaw-plus
cd openclaw-plus

# 构建所有 crate
cargo build --release
```

### 方式 A — 嵌入模式（独立运行）

```bash
# 将 OpenClaw 打包为单个 JS 文件
./scripts/bundle_openclaw.sh

# 运行监控 UI（进程内启动 WasmEdge 沙箱）
cargo run --release -p openclaw-ui
```

### 方式 B — 插件模式（推荐给 OpenClaw 用户）

```bash
# 向 OpenClaw 注册插件
openclaw plugins install ./openclaw.plugin.json

# 验证插件已加载
openclaw plugins list
```

### 配置安全策略

编辑 `~/.config/openclaw-plus/config.toml`：

```toml
# OpenClaw 构建输出路径（仅嵌入模式）
openclaw_entry = "/path/to/openclaw/dist/index.js"

# 沙箱工作区目录（映射到沙箱内的 /workspace）
workspace_dir = "/path/to/your/workspace"

# 网络白名单
network_allowlist = [
    "api.openai.com",
    "api.anthropic.com",
    "api.deepseek.com",
    "openrouter.ai",
]

# 断路器阈值
[circuit_breaker]
denial_window_secs      = 10
max_denials_per_window  = 20
max_dangerous_commands  = 3
```

## 🎯 核心功能

### 1. AI 助手

内置智能 AI 助手，提供：

- 🔍 **系统诊断** — 分析安全事件并提供修复建议
- ⚡ **性能优化** — 审查配置并提出性能改进建议
- 🛡️ **安全审计** — 检查安全策略漏洞或过宽配置
- 📚 **RAG 知识库** — 检查知识库配置和索引状态

### 2. 可视化工作流编辑器

- 拖拽式节点编辑
- 实时预览和调试
- 支持复杂的条件分支和循环
- 内置丰富的节点库

### 3. 智能体管理

- 多智能体配置和管理
- 智能体对话历史
- 任务执行监控
- 审计日志回放

### 4. 安全控制

#### 技能风险等级

每个 OpenClaw 技能在执行前都会被分类：

| 风险等级 | 示例 | 操作 |
| --- | --- | --- |
| **安全** | `fs.readFile`, `search.query`, `knowledge.retrieve` | 无需提示直接允许 |
| **确认** | `fs.writeFile`, `email.send`, `web.navigate` | 暂停直到用户批准 |
| **拒绝** | `shell.exec`, `process.spawn`, `system.reboot` | 无条件阻止 |

#### 默认安全策略

| 操作 | 默认行为 | 说明 |
| --- | --- | --- |
| 文件读取 | ✅ 允许（工作区内） | 工作区外路径被拒绝 |
| 文件写入 | ✅ 允许（工作区内） | 工作区外路径被拒绝 |
| 文件删除 | ⏳ 确认 | 显示对话框；30 秒后自动拒绝 |
| 网络请求 | ✅ 允许（白名单主机） | 其他主机被拒绝 |
| Shell 执行 | ⏳ 确认 | 高风险命令立即拒绝 |
| 敏感路径 | 🚫 拒绝 | 硬编码安全规则 |

## 🤖 AI 推理后端

OpenClaw+ 集成了多种 AI 推理后端：

- **WASI-NN** (WasmEdge + llama.cpp)：进程内推理，支持 GGUF 模型
- **LlamaCppHttp**：llama.cpp 服务器 HTTP API
- **Ollama**：本地 Ollama 服务器
- **OpenAI 兼容**：OpenAI、Anthropic、DeepSeek、Gemini 等

### WASI-NN 快速开始

```bash
# 安装带 wasi_nn 插件的 WasmEdge
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh \
  | bash -s -- --plugins wasi_nn-ggml

# 下载 GGUF 模型
mkdir -p models/gguf
curl -L -o models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf"

# 使用 wasi-nn 特性构建
cargo build --release --features wasi-nn

# 运行测试
cargo test --features wasi-nn --test wasi_nn_integration
```

## 🛠️ 技术栈

- **运行时**: [WasmEdge](https://wasmedge.org/) 0.16+ (WASI + QuickJS + wasi_nn)
- **UI 框架**: [libcosmic](https://github.com/pop-os/libcosmic) (基于 iced)
- **编程语言**: Rust 2021 Edition
- **异步运行时**: Tokio
- **AI 推理**: wasmedge-sdk 0.14 + llama.cpp (通过 WASI-NN)
- **配置**: TOML
- **分发**: GitHub Actions + cargo-dist

## 📚 文档

- [快速开始指南](docs/QUICK_START.md)
- [部署指南](docs/DEPLOYMENT.md)
- [安全策略配置](docs/SECURITY_POLICY.md)
- [AI 助手使用指南](docs/AI_ASSISTANT.md)
- [插件开发指南](docs/PLUGIN_DEVELOPMENT.md)
- [树莓派部署](docs/RASPBERRY_PI_DEPLOYMENT.md)

## 🤝 贡献

我们欢迎各种形式的贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

### 贡献者

感谢所有为 OpenClaw+ 做出贡献的开发者！

## 📧 联系方式

- **作者**: arksong2018@gmail.com
- **问题反馈**: [GitHub Issues](https://github.com/arksong2018/openclaw-plus/issues)
- **讨论**: [GitHub Discussions](https://github.com/arksong2018/openclaw-plus/discussions)

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- [WasmEdge](https://wasmedge.org/) - 高性能 WebAssembly 运行时
- [libcosmic](https://github.com/pop-os/libcosmic) - 现代化 UI 框架
- [Rust](https://www.rust-lang.org/) - 系统编程语言
- 所有为开源社区做出贡献的开发者

## 🌟 Star History

如果这个项目对你有帮助，请给我们一个 ⭐️！

---

**用 ❤️ 构建，由 OpenClaw+ 团队维护**
