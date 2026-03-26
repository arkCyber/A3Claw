# OpenClaw+ 快速开始指南

## 📋 目录

- [系统要求](#系统要求)
- [安装步骤](#安装步骤)
- [首次运行](#首次运行)
- [基本使用](#基本使用)
- [测试验证](#测试验证)
- [常见问题](#常见问题)

---

## 系统要求

### 必需组件

- **操作系统**: macOS 11.0+, Linux, Windows 10+
- **Rust**: 1.75.0 或更高版本
- **WasmEdge**: 0.16.0 或更高版本
- **内存**: 至少 4GB RAM
- **磁盘空间**: 至少 2GB 可用空间

### 可选组件

- **Node.js**: 18.0+ (用于运行邮件测试案例)
- **Git**: 用于版本控制
- **Docker**: 用于容器化部署

---

## 安装步骤

### 1. 安装 WasmEdge

**macOS/Linux:**
```bash
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash
source $HOME/.wasmedge/env
```

**验证安装:**
```bash
wasmedge --version
# 应该显示: wasmedge version 0.16.1 或更高
```

### 2. 克隆项目

```bash
git clone https://github.com/yourusername/OpenClaw+.git
cd OpenClaw+
```

### 3. 构建项目

```bash
# 构建所有 crates
cargo build --release --workspace --exclude openclaw-wasi-nn-infer

# 或者只构建主程序
cargo build --release -p openclaw-plus
```

构建时间约 5-10 分钟（首次构建）。

### 4. 配置文件

配置文件会在首次运行时自动生成，位置：
- **macOS**: `~/Library/Application Support/openclaw-plus/config.toml`
- **Linux**: `~/.config/openclaw-plus/config.toml`
- **Windows**: `%APPDATA%\openclaw-plus\config.toml`

---

## 首次运行

### 1. 启动应用

```bash
# 从构建目录运行
./target/release/openclaw-plus

# 或者使用 cargo
cargo run --release -p openclaw-plus
```

### 2. 初始化配置

首次运行时，应用会自动：
- ✅ 创建配置目录
- ✅ 生成默认配置文件
- ✅ 创建工作区目录
- ✅ 初始化审计日志

### 3. 验证环境

应用启动后，会自动检查：
- WasmEdge 安装状态
- 配置文件完整性
- 工作区权限
- 网络连接

---

## 基本使用

### Dashboard 页面

**功能:**
- 查看沙箱运行状态
- 监控系统事件
- 控制沙箱生命周期

**操作:**
1. 点击 **"Start Sandbox"** 启动沙箱
2. 查看实时事件日志
3. 使用 **"Emergency Stop"** 紧急停止

### Assistant 页面

**功能:**
- 与 AI 助手交互
- 执行自动化任务
- 管理工作流

**操作:**
1. 输入任务描述
2. 点击 **"Execute"** 执行
3. 查看执行结果

### AI Chat 页面

**功能:**
- 与本地 AI 模型对话
- 配置 AI 参数
- 查看对话历史

**配置:**
```toml
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "qwen3.5:9b"
temperature = 0.7
```

### Claw Terminal 页面

**功能:**
- 执行命令行操作
- 自然语言模式
- 语音输入支持

**操作:**
1. 输入命令或自然语言描述
2. 切换 NL 模式（自然语言）
3. 使用语音输入（点击 🎙 按钮）

---

## 测试验证

### 运行单元测试

```bash
# 运行所有测试
cargo test --workspace --exclude openclaw-wasi-nn-infer

# 运行特定 crate 的测试
cargo test --package openclaw-sandbox
cargo test --package openclaw-security
cargo test --package openclaw-ui
```

**预期结果:**
- ✅ 72+ 个测试通过
- ✅ 0 失败
- ✅ 部分测试可能被忽略（需要外部依赖）

### 运行集成测试

```bash
# 沙箱集成测试
cargo test --package openclaw-sandbox --test integration_real_data

# 安全模块测试
cargo test --package openclaw-security --lib
```

### 运行测试案例

**1. 文件操作测试**
```bash
# 需要在 WasmEdge 环境中运行
# 通过 UI 的 Assistant 页面启动沙箱后自动执行
```

**2. 网页搜集测试**
```bash
# 测试 HTTPS 请求和 RSS 解析
# 位置: tests/test_web_scraping.js
```

**3. 综合测试**
```bash
# 运行所有测试案例
# 位置: run_test_cases.js
# 结果保存到: /workspace/test_report.txt
```

---

## 常见问题

### Q1: WasmEdge 未找到

**问题:** `wasmedge: command not found`

**解决:**
```bash
# 重新安装 WasmEdge
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash

# 添加到 PATH
echo 'export PATH="$HOME/.wasmedge/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Q2: 编译错误

**问题:** `error: could not compile openclaw-*`

**解决:**
```bash
# 清理构建缓存
cargo clean

# 更新依赖
cargo update

# 重新构建
cargo build --release
```

### Q3: 配置文件错误

**问题:** `Failed to load config`

**解决:**
```bash
# 删除旧配置
rm -rf ~/Library/Application\ Support/openclaw-plus/

# 重新启动应用，会自动生成新配置
./target/release/openclaw-plus
```

### Q4: 沙箱启动失败

**问题:** `Failed to start sandbox`

**解决:**
1. 检查 WasmEdge 安装: `wasmedge --version`
2. 检查配置文件: `cat ~/Library/Application\ Support/openclaw-plus/config.toml`
3. 检查工作区权限: `ls -la ~/.openclaw-plus/workspace/`
4. 查看日志: `tail -f ~/.openclaw-plus/audit.log`

### Q5: 网络请求失败

**问题:** `Network request failed`

**解决:**
1. 检查网络白名单配置
2. 确认目标域名在允许列表中
3. 测试网络连接: `curl https://feeds.npr.org/1001/rss.xml`

### Q6: UI 显示异常

**问题:** Tooltip 或其他 UI 元素显示不正常

**解决:**
```bash
# 重新编译 UI crate
cargo build --release -p openclaw-ui

# 重启应用
pkill openclaw-plus
./target/release/openclaw-plus
```

---

## 性能优化

### 内存优化

编辑配置文件，调整内存限制：
```toml
memory_limit_mb = 512  # 默认 512MB，可根据需要调整
```

### 网络优化

添加常用域名到白名单：
```toml
network_allowlist = [
    "api.openai.com",
    "api.anthropic.com",
    "your-domain.com",
]
```

### 日志优化

控制日志级别：
```bash
# 设置环境变量
export RUST_LOG=info  # 可选: error, warn, info, debug, trace
./target/release/openclaw-plus
```

---

## 下一步

- 📖 阅读 [完整文档](./OPENCLAW_TOOLS_COMPLETE_GUIDE.md)
- 🧪 运行 [测试案例](../tests/)
- 🔧 配置 [安全策略](./SECURITY.md)
- 🚀 部署到 [生产环境](./DEPLOYMENT.md)

---

## 获取帮助

- **GitHub Issues**: https://github.com/yourusername/OpenClaw+/issues
- **文档**: https://docs.openclaw.dev
- **社区**: https://community.openclaw.dev

---

**祝您使用愉快！** 🎉
