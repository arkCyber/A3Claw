#!/bin/bash
# OpenClaw UI 启动脚本 - 优化版
# 自动处理环境配置和服务检查

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 OpenClaw UI 启动中..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 1. 加载 Rust 环境
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
    echo "✅ Rust 环境已加载"
elif [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
    echo "✅ Rust 环境已加载"
fi

# 2. 检查 Ollama 服务
echo ""
if pgrep -x "ollama" > /dev/null; then
    echo "✅ Ollama 服务运行中"
else
    echo "⚠️  Ollama 未运行，正在启动..."
    /opt/homebrew/bin/ollama serve > /tmp/ollama.log 2>&1 &
    sleep 2
    echo "✅ Ollama 已启动"
fi

# 3. 显示 AI 配置
echo ""
echo "🤖 AI 配置："
CONFIG_FILE="$HOME/Library/Application Support/openclaw-plus/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    MODEL=$(grep '^model = ' "$CONFIG_FILE" | cut -d'"' -f2)
    ENDPOINT=$(grep '^endpoint = ' "$CONFIG_FILE" | cut -d'"' -f2)
    echo "   模型: $MODEL"
    echo "   端点: $ENDPOINT"
    
    # 验证模型存在
    if /opt/homebrew/bin/ollama list | grep -q "$MODEL"; then
        echo "   ✅ 模型已安装"
    else
        echo "   ⚠️  模型未找到，请运行: ollama pull $MODEL"
    fi
fi

# 4. 显示可用模型
echo ""
echo "📋 已安装的模型："
/opt/homebrew/bin/ollama list | tail -n +2 | while read line; do
    echo "   - $line"
done

# 5. 启动 UI
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎨 正在启动 OpenClaw UI..."
echo ""
echo "💡 使用提示："
echo "   - 点击输入框以获取焦点"
echo "   - 切换到 AI Chat 页面测试对话"
echo "   - 使用 Ctrl+C 停止应用"
echo "   - 日志位置: /tmp/openclaw.log"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cd "$(dirname "$0")"

# 启动 UI（带详细日志）
RUST_LOG=openclaw_ui=info,openclaw_inference=debug cargo run -p openclaw-ui --release
