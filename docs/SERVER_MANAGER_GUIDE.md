# 服务器管理器使用指南

## 概述

服务器管理器是 OpenClaw+ 的推理服务器管理工具，与 Ollama 管理配置一样，在通用管理中配置和控制。

## 功能特性

- ✅ 统一管理多个推理后端（Ollama、llama.cpp、OpenAI、DeepSeek）
- ✅ 自动启动/停止服务器
- ✅ 健康检查和资源监控
- ✅ 故障自动转移
- ✅ 命令行控制工具
- ✅ 配置文件管理

## 配置文件

服务器配置文件位于 `config/servers.toml`，与其他配置文件一起管理。

### 配置示例

```toml
# Ollama 主服务
[[servers]]
id = "ollama-primary"
name = "Ollama (主服务)"
type = "ollama"
endpoint = "http://localhost:11434"
port = 11434
enabled = true
auto_start = false
model = "qwen2.5:0.5b"

# llama.cpp 备份服务
[[servers]]
id = "llama-cpp-backup"
name = "llama.cpp (备份)"
type = "llama_cpp"
endpoint = "http://localhost:8080"
port = 8080
enabled = true
auto_start = true
model_path = "models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf"
threads = 8
```

## 命令行工具

### 安装

```bash
# 编译服务器管理工具
cargo build --release -p openclaw-server-manager --bin server-ctl

# 创建快捷方式（可选）
ln -s target/release/server-ctl /usr/local/bin/server-ctl
```

### 基本命令

#### 列出所有服务器

```bash
server-ctl list
# 或
server-ctl ls
```

输出示例：
```
=== 推理服务器列表 ===

🟢 Ollama (主服务) (ollama-primary)
   类型: Ollama
   端点: http://localhost:11434
   状态: Running
   健康: ✅

🔴 llama.cpp (备份) (llama-cpp-backup)
   类型: LlamaCpp
   端点: http://localhost:8080
   状态: Stopped
```

#### 查看服务器状态

```bash
server-ctl status ollama-primary
```

#### 启动服务器

```bash
# 启动单个服务器
server-ctl start llama-cpp-backup

# 启动所有已启用的服务器
server-ctl start-all
```

#### 停止服务器

```bash
# 停止单个服务器
server-ctl stop llama-cpp-backup

# 停止所有服务器
server-ctl stop-all
```

#### 重启服务器

```bash
server-ctl restart llama-cpp-backup
```

#### 健康检查

```bash
server-ctl health ollama-primary
```

输出示例：
```
正在检查服务器健康状态: ollama-primary
✅ 服务器健康
   延迟: 15 ms
```

## 从 OpenClaw+ UI 调用

服务器管理器可以从 OpenClaw+ 的通用管理界面调用，与 Ollama 管理一样。

### 集成方式

1. **配置管理**：编辑 `config/servers.toml`
2. **命令调用**：通过 UI 调用 `server-ctl` 命令
3. **状态显示**：UI 显示服务器状态和资源使用

### UI 操作示例

```javascript
// 从 UI 调用服务器管理命令
const { exec } = require('child_process');

// 列出服务器
exec('server-ctl list', (error, stdout, stderr) => {
    if (error) {
        console.error(`错误: ${error}`);
        return;
    }
    console.log(stdout);
});

// 启动服务器
exec('server-ctl start llama-cpp-backup', (error, stdout, stderr) => {
    if (error) {
        console.error(`启动失败: ${error}`);
        return;
    }
    console.log('服务器已启动');
});
```

## 使用场景

### 场景 1：开发环境

```bash
# 只启动 Ollama
server-ctl start ollama-primary
```

### 场景 2：生产环境（本地冗余）

```bash
# 启动 Ollama + llama.cpp 双保险
server-ctl start ollama-primary
server-ctl start llama-cpp-backup
```

### 场景 3：自动启动所有服务

```bash
# 启动所有已启用且支持自动启动的服务器
server-ctl start-all
```

### 场景 4：健康检查

```bash
# 检查所有服务器
for server in ollama-primary llama-cpp-backup; do
    server-ctl health $server
done
```

## 故障排查

### llama.cpp 启动失败

**问题**：`server-ctl start llama-cpp-backup` 失败

**解决方案**：

1. 检查 llama.cpp 是否已安装：
   ```bash
   which llama-server
   # 或
   ls -la scripts/start_llama_cpp_server.sh
   ```

2. 手动安装 llama.cpp：
   ```bash
   brew install llama.cpp
   ```

3. 检查模型文件是否存在：
   ```bash
   ls -lh models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf
   ```

### Ollama 显示不健康

**问题**：`server-ctl health ollama-primary` 显示不健康

**解决方案**：

1. 确认 Ollama 正在运行：
   ```bash
   ps aux | grep ollama
   ```

2. 手动启动 Ollama：
   ```bash
   ollama serve
   ```

3. 检查端口是否被占用：
   ```bash
   lsof -i :11434
   ```

### 服务器列表为空

**问题**：`server-ctl list` 显示没有服务器

**解决方案**：

1. 检查配置文件是否存在：
   ```bash
   cat config/servers.toml
   ```

2. 如果不存在，工具会使用默认配置，确保在项目根目录运行

## 高级功能

### 资源监控

服务器管理器会自动监控每个服务器的资源使用情况：

- CPU 使用率
- 内存使用量
- 运行时间
- 进程 PID

查看资源使用：
```bash
server-ctl status llama-cpp-backup
```

### 健康检查配置

在 `config/servers.toml` 中配置健康检查：

```toml
[health_check]
interval_seconds = 30  # 检查间隔
timeout_seconds = 5    # 超时时间
retry_count = 3        # 重试次数
```

### 故障转移配置

配置自动故障转移优先级：

```toml
[failover]
enabled = true
priority = [
    "ollama-primary",
    "llama-cpp-backup",
    "deepseek-cloud",
    "openai-cloud"
]
```

## 最佳实践

### 1. 配置管理

- ✅ 将 `config/servers.toml` 加入版本控制
- ✅ 使用环境变量管理 API keys
- ✅ 为不同环境创建不同配置文件

### 2. 服务器启动顺序

推荐启动顺序：
1. Ollama（主服务）
2. llama.cpp（本地备份）
3. 云端服务（按需）

### 3. 监控和告警

- 定期运行健康检查
- 监控资源使用情况
- 设置告警阈值

### 4. 安全性

- 不要在配置文件中硬编码 API keys
- 使用环境变量或密钥管理服务
- 限制服务器访问权限

## 与 OpenClaw+ UI 集成

服务器管理器设计为与 OpenClaw+ UI 无缝集成：

1. **配置界面**：在通用管理中编辑 `servers.toml`
2. **控制面板**：通过 UI 按钮调用 `server-ctl` 命令
3. **状态显示**：实时显示服务器状态和资源使用
4. **日志查看**：查看服务器日志和错误信息

## 总结

服务器管理器提供了统一的方式来管理所有推理后端，与 Ollama 管理配置一样简单易用。

**快速开始**：

```bash
# 1. 编译工具
cargo build --release -p openclaw-server-manager --bin server-ctl

# 2. 查看服务器列表
./target/release/server-ctl list

# 3. 启动备份服务器
./target/release/server-ctl start llama-cpp-backup

# 4. 检查健康状态
./target/release/server-ctl health llama-cpp-backup
```

**从 UI 使用**：
- 在 OpenClaw+ 通用管理中配置服务器
- 使用 UI 按钮控制服务器启动/停止
- 查看实时状态和资源监控
