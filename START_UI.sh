#!/bin/bash
# OpenClaw UI 完整启动脚本（修复版）

echo "🚀 OpenClaw UI 启动"
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
echo "📡 检查 Ollama 服务..."
if ! pgrep -x "ollama" > /dev/null; then
    echo "⚠️  Ollama 未运行，正在启动..."
    /opt/homebrew/bin/ollama serve > /tmp/ollama.log 2>&1 &
    sleep 3
    echo "✅ Ollama 已启动"
else
    echo "✅ Ollama 正在运行"
fi

# 3. 检查 qwen3.5:9b 模型
echo ""
echo "🤖 检查 AI 模型..."
if /opt/homebrew/bin/ollama list | grep -q "qwen3.5"; then
    echo "✅ qwen3.5 模型已安装"
else
    echo "⚠️  qwen3.5:9b 模型未安装"
    echo ""
    read -p "是否现在下载 qwen3.5:9b 模型？(约 5.5GB) [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "📥 正在下载 qwen3.5:9b..."
        /opt/homebrew/bin/ollama pull qwen3.5:9b
        if [ $? -eq 0 ]; then
            echo "✅ 下载成功"
        else
            echo "❌ 下载失败，将使用 llama3.2"
            # 更新配置使用 llama3.2
            sed -i.bak 's/model = "qwen3.5:9b"/model = "llama3.2"/' \
                "$HOME/Library/Application Support/openclaw-plus/config.toml"
        fi
    else
        echo "⚠️  跳过下载，将使用已有的 llama3.2 模型"
        # 更新配置使用 llama3.2
        sed -i.bak 's/model = "qwen3.5:9b"/model = "llama3.2"/' \
            "$HOME/Library/Application Support/openclaw-plus/config.toml"
    fi
fi

# 4. 显示当前配置
echo ""
echo "📋 当前配置："
echo "   AI 模型: $(grep '^model = ' "$HOME/Library/Application Support/openclaw-plus/config.toml" | cut -d'"' -f2)"
echo "   端点: $(grep '^endpoint = ' "$HOME/Library/Application Support/openclaw-plus/config.toml" | cut -d'"' -f2)"
echo ""

# 5. 启动 UI
echo "🎨 正在启动 OpenClaw UI..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "💡 提示："
echo "   - 如果输入焦点丢失，请点击输入框"
echo "   - 使用 Ctrl+C 停止 UI"
echo "   - 日志: /tmp/openclaw.log"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cd "$(dirname "$0")"
RUST_LOG=openclaw_ui=info,openclaw_inference=debug cargo run -p openclaw-ui --release
