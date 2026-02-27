#!/bin/bash
# OpenClaw+ 服务器管理功能演示脚本
# 展示完整的服务器管理工作流程

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVER_CTL="$PROJECT_ROOT/target/release/server-ctl"

echo "🎬 OpenClaw+ 服务器管理功能演示"
echo "================================"
echo ""

# 检查 server-ctl 是否存在
if [ ! -f "$SERVER_CTL" ]; then
    echo "❌ 错误: server-ctl 未找到"
    echo "正在编译 server-ctl..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin server-ctl
    echo "✅ 编译完成"
    echo ""
fi

# 步骤 1: 列出所有服务器
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 步骤 1: 列出所有配置的服务器"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
"$SERVER_CTL" list
echo ""
read -p "按 Enter 继续..."
echo ""

# 步骤 2: 查看 JSON 格式输出
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 步骤 2: JSON 格式输出（UI 使用此格式）"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
"$SERVER_CTL" list --json | jq '.'
echo ""
read -p "按 Enter 继续..."
echo ""

# 步骤 3: 检查 Ollama 是否可用
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 步骤 3: 检查 Ollama 安装状态"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
if command -v ollama &> /dev/null; then
    echo "✅ Ollama 已安装: $(which ollama)"
    echo ""
    echo "📦 已安装的模型:"
    ollama list 2>/dev/null || echo "   (无法获取模型列表，Ollama 可能未运行)"
else
    echo "⚠️  Ollama 未安装"
    echo "   安装方法: brew install ollama"
fi
echo ""
read -p "按 Enter 继续..."
echo ""

# 步骤 4: 查看特定服务器状态
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔎 步骤 4: 查看 Ollama 服务器详细状态"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
"$SERVER_CTL" status ollama-primary
echo ""
read -p "按 Enter 继续..."
echo ""

# 步骤 5: 测试启动 Ollama（如果未运行）
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 步骤 5: 启动 Ollama 服务器"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "⚠️  注意: 这将在后台启动 Ollama 服务器"
read -p "是否继续? [y/N]: " confirm
if [[ "$confirm" =~ ^[Yy]$ ]]; then
    echo ""
    echo "正在启动 Ollama..."
    "$SERVER_CTL" start ollama-primary || echo "⚠️  启动失败（可能已在运行）"
    echo ""
    sleep 2
    echo "当前状态:"
    "$SERVER_CTL" status ollama-primary
else
    echo "跳过启动"
fi
echo ""
read -p "按 Enter 继续..."
echo ""

# 步骤 6: 健康检查
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🏥 步骤 6: 执行健康检查"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
"$SERVER_CTL" health ollama-primary || echo "⚠️  健康检查失败（服务器可能未运行）"
echo ""
read -p "按 Enter 继续..."
echo ""

# 步骤 7: 最终状态
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 步骤 7: 最终服务器状态"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
"$SERVER_CTL" list
echo ""

# 总结
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ 演示完成！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 下一步操作:"
echo "  1. 在 UI 中查看服务器管理界面:"
echo "     ./target/release/openclaw-plus"
echo "     然后导航到 General Settings > Inference Server Management"
echo ""
echo "  2. 使用命令行工具管理服务器:"
echo "     $SERVER_CTL list              # 列出服务器"
echo "     $SERVER_CTL start <id>        # 启动服务器"
echo "     $SERVER_CTL stop <id>         # 停止服务器"
echo "     $SERVER_CTL health <id>       # 健康检查"
echo ""
echo "  3. 测试推理功能:"
echo "     curl http://localhost:11434/api/generate -d '{\"model\":\"qwen2.5:0.5b\",\"prompt\":\"Hello\"}'"
echo ""
