#!/bin/bash
# A3Claw UI 启动脚本

set -e

echo "🚀 启动 A3Claw UI..."

# 检查配置目录
CONFIG_DIR="$HOME/Library/Application Support/openclaw-plus"
if [ ! -d "$CONFIG_DIR" ]; then
    echo "📁 创建配置目录: $CONFIG_DIR"
    mkdir -p "$CONFIG_DIR"
fi

# 检查配置文件
CONFIG_FILE="$CONFIG_DIR/config.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    echo "📝 创建默认配置文件..."
    cat > "$CONFIG_FILE" << 'EOF'
# A3Claw 配置文件

# 工作区目录（智能体可以访问的目录）
workspace_dir = "$HOME/workspace"

# 网络白名单（允许访问的域名）
network_allowlist = [
    "api.openai.com",
    "api.anthropic.com",
    "api.github.com",
]

# Shell 命令白名单
shell_allowlist = [
    "ls",
    "cat",
    "echo",
    "pwd",
]

# 是否启用断路器
enable_circuit_breaker = true

# 断路器配置
[circuit_breaker]
failure_threshold = 5
timeout_seconds = 60
half_open_timeout_seconds = 30
EOF
    echo "✅ 配置文件已创建: $CONFIG_FILE"
fi

# 运行 UI
echo "🎨 启动 UI 界面..."
cd "$(dirname "$0")"
cargo run -p openclaw-ui --release
