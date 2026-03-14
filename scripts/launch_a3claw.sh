#!/bin/bash
# 启动 A3Claw.app

set -e

APP_NAME="A3Claw"
BUNDLE_DIR="$HOME/Applications/${APP_NAME}.app"

echo "🚀 启动 A3Claw.app..."

# 检查 app 是否存在
if [ ! -d "$BUNDLE_DIR" ]; then
    echo "❌ 错误: 找不到 ${APP_NAME}.app"
    echo "   请先运行: ./scripts/create_a3claw_bundle.sh"
    exit 1
fi

# 启动应用
echo "📱 启动应用..."
open "$BUNDLE_DIR"

echo "✅ A3Claw.app 已启动！"
echo ""
echo "📋 应用信息："
echo "   - 名称: A3Claw"
echo "   - 位置: $BUNDLE_DIR"
echo "   - 版本: 1.0.0"
echo ""
echo "💡 提示："
echo "   - 如果应用没有响应，请检查 Ollama 服务是否运行"
echo "   - 日志文件位于: /tmp/ollama.log"
echo "   - 配置文件位于: ~/Library/Application Support/openclaw-plus/"
echo ""
