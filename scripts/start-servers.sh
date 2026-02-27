#!/bin/bash
# OpenClaw+ 服务器启动脚本
# 自动启动所有配置的推理服务器

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVER_CTL="$PROJECT_ROOT/target/release/server-ctl"

echo "🚀 OpenClaw+ 服务器启动脚本"
echo "================================"
echo ""

# 检查 server-ctl 是否存在
if [ ! -f "$SERVER_CTL" ]; then
    echo "❌ 错误: server-ctl 未找到"
    echo "请先编译项目: cargo build --release --bin server-ctl"
    exit 1
fi

# 列出所有服务器
echo "📋 当前配置的服务器:"
"$SERVER_CTL" list
echo ""

# 询问用户要启动哪些服务器
echo "请选择要启动的服务器:"
echo "  1) 启动所有服务器"
echo "  2) 仅启动 Ollama"
echo "  3) 仅启动 llama.cpp"
echo "  4) 自定义选择"
echo "  0) 退出"
echo ""
read -p "请输入选项 [0-4]: " choice

case $choice in
    1)
        echo ""
        echo "🔄 启动所有服务器..."
        "$SERVER_CTL" start-all
        ;;
    2)
        echo ""
        echo "🔄 启动 Ollama 服务器..."
        "$SERVER_CTL" start ollama-primary
        ;;
    3)
        echo ""
        echo "🔄 启动 llama.cpp 服务器..."
        "$SERVER_CTL" start llama-cpp-backup
        ;;
    4)
        echo ""
        read -p "请输入服务器 ID (例如: ollama-primary): " server_id
        if [ -n "$server_id" ]; then
            echo "🔄 启动服务器: $server_id"
            "$SERVER_CTL" start "$server_id"
        else
            echo "❌ 服务器 ID 不能为空"
            exit 1
        fi
        ;;
    0)
        echo "👋 退出"
        exit 0
        ;;
    *)
        echo "❌ 无效选项"
        exit 1
        ;;
esac

echo ""
echo "✅ 操作完成！"
echo ""
echo "📊 当前服务器状态:"
"$SERVER_CTL" list
echo ""
echo "💡 提示:"
echo "  - 查看服务器状态: $SERVER_CTL status <server-id>"
echo "  - 停止服务器: $SERVER_CTL stop <server-id>"
echo "  - 健康检查: $SERVER_CTL health <server-id>"
