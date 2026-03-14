#!/bin/bash
# OpenClaw UI 快速启动脚本
# 使用方法: ./QUICK_START.sh

echo "🚀 OpenClaw UI 快速启动"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 加载 Rust 环境变量
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
    echo "✅ Rust 环境已加载"
elif [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
    echo "✅ Rust 环境已加载"
else
    echo "❌ 找不到 Rust 环境，请先安装 Rust"
    echo "   运行: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# 检查 cargo 命令
if ! command -v cargo &> /dev/null; then
    echo "❌ cargo 命令不可用，请检查 Rust 安装"
    exit 1
fi

echo ""

# 检查 Ollama
if ! pgrep -x "ollama" > /dev/null; then
    echo "⚠️  Ollama 未运行，正在启动..."
    ollama serve > /tmp/ollama.log 2>&1 &
    sleep 3
fi

echo "✅ AI 服务已就绪"
echo ""
echo "🎨 正在启动 OpenClaw UI..."
echo ""

cd "$(dirname "$0")"
RUST_LOG=openclaw_ui=info cargo run -p openclaw-ui --release
