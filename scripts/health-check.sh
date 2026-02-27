#!/bin/bash
# OpenClaw+ 服务器健康检查脚本
# 定期检查所有服务器的健康状态

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVER_CTL="$PROJECT_ROOT/target/release/server-ctl"

echo "🏥 OpenClaw+ 服务器健康检查"
echo "================================"
echo ""

# 检查 server-ctl 是否存在
if [ ! -f "$SERVER_CTL" ]; then
    echo "❌ 错误: server-ctl 未找到"
    exit 1
fi

# 获取服务器列表
servers=$("$SERVER_CTL" list --json 2>/dev/null || echo "[]")

if [ "$servers" = "[]" ]; then
    echo "⚠️  没有配置的服务器"
    exit 0
fi

# 解析并检查每个服务器
echo "$servers" | jq -r '.[] | "\(.server_id)|\(.name)|\(.endpoint)|\(.status)"' | while IFS='|' read -r id name endpoint status; do
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📡 服务器: $name"
    echo "   ID: $id"
    echo "   端点: $endpoint"
    echo "   状态: $status"
    echo ""
    
    if [ "$status" = "Running" ]; then
        echo "   🔍 执行健康检查..."
        "$SERVER_CTL" health "$id" 2>&1 | sed 's/^/   /'
    else
        echo "   ⚠️  服务器未运行，跳过健康检查"
    fi
    echo ""
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ 健康检查完成"
