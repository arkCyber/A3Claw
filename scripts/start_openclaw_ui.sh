#!/bin/bash
# OpenClaw UI 启动脚本 - 自动检查和修复 AI 服务

set -e

echo "🚀 OpenClaw UI 启动检查..."

# 1. 检查 Ollama 服务
echo ""
echo "📡 检查 Ollama 服务状态..."
if ! pgrep -x "ollama" > /dev/null; then
    echo "⚠️  Ollama 未运行，正在启动..."
    ollama serve > /tmp/ollama.log 2>&1 &
    sleep 3
    echo "✅ Ollama 已启动"
else
    echo "✅ Ollama 正在运行"
fi

# 2. 检查可用模型
echo ""
echo "🤖 检查可用 AI 模型..."
MODELS=$(curl -s http://localhost:11434/api/tags 2>/dev/null | grep -o '"name":"[^"]*"' | cut -d'"' -f4 || echo "")

if [ -z "$MODELS" ]; then
    echo "❌ 无法连接到 Ollama，请检查服务状态"
    echo "   尝试手动启动: ollama serve"
    exit 1
fi

echo "✅ 可用模型:"
echo "$MODELS" | while read model; do
    echo "   - $model"
done

# 3. 检查配置文件中的模型
CONFIG_FILE="$HOME/Library/Application Support/openclaw-plus/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    CONFIG_MODEL=$(grep '^model = ' "$CONFIG_FILE" | cut -d'"' -f2)
    echo ""
    echo "📝 配置文件中的模型: $CONFIG_MODEL"
    
    # 检查配置的模型是否可用
    if echo "$MODELS" | grep -q "^${CONFIG_MODEL}"; then
        echo "✅ 配置的模型可用"
    else
        echo "⚠️  配置的模型 '$CONFIG_MODEL' 不可用"
        FIRST_MODEL=$(echo "$MODELS" | head -1)
        echo "   建议使用: $FIRST_MODEL"
        echo ""
        read -p "是否自动修复配置文件? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            sed -i.bak "s/model = \".*\"/model = \"$FIRST_MODEL\"/" "$CONFIG_FILE"
            echo "✅ 已更新配置文件使用 $FIRST_MODEL"
        fi
    fi
fi

# 4. 测试 AI 推理
echo ""
echo "🧪 测试 AI 推理..."
TEST_MODEL=$(echo "$MODELS" | head -1)
TEST_RESPONSE=$(curl -s http://localhost:11434/api/generate \
    -d "{\"model\":\"$TEST_MODEL\",\"prompt\":\"hi\",\"stream\":false}" \
    --max-time 10 2>/dev/null | grep -o '"response":"[^"]*"' | head -1 || echo "")

if [ -n "$TEST_RESPONSE" ]; then
    echo "✅ AI 推理测试成功"
else
    echo "⚠️  AI 推理测试失败，但服务正在运行"
    echo "   首次使用可能需要下载模型，请稍候..."
fi

# 5. 启动 OpenClaw UI
echo ""
echo "🎨 启动 OpenClaw UI..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 提示:"
echo "   - 如果输入框焦点丢失，请点击输入框重新获取焦点"
echo "   - 使用 Ctrl+C 停止 UI"
echo "   - 日志文件: /tmp/openclaw.log"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cd "$(dirname "$0")/.."
RUST_LOG=openclaw_ui=info,openclaw_inference=debug cargo run -p openclaw-ui --release
