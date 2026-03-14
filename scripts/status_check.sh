#!/bin/bash
# 快速状态检查脚本

echo "=========================================="
echo "OpenClaw+ 系统状态检查"
echo "=========================================="
echo ""

# 1. 检查进程
echo "📊 进程状态："
if pgrep -f "ollama.*serve" > /dev/null; then
    echo "✅ Ollama 运行中 (PID: $(pgrep -f 'ollama.*serve'))"
else
    echo "❌ Ollama 未运行"
fi

if pgrep -f "llama-server.*8080" > /dev/null; then
    echo "✅ llama.cpp 运行中 (PID: $(pgrep -f 'llama-server.*8080'))"
else
    echo "⚠️  llama.cpp 未运行"
fi

if pgrep -f "openclaw-plus" > /dev/null; then
    echo "✅ OpenClaw UI 运行中"
else
    echo "⚠️  OpenClaw UI 未运行"
fi

echo ""

# 2. 检查端口
echo "🌐 端口状态："
if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    echo "✅ Ollama API (11434) 可用"
else
    echo "❌ Ollama API (11434) 不可用"
fi

if curl -s http://localhost:8080/v1/models >/dev/null 2>&1; then
    echo "✅ llama.cpp API (8080) 可用"
else
    echo "⚠️  llama.cpp API (8080) 不可用"
fi

echo ""

# 3. 检查配置
echo "⚙️  配置状态："
CONFIG_FILE="$HOME/Library/Application Support/openclaw-plus/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    echo "✅ 配置文件存在"
    echo "   主引擎: $(grep '^provider = ' "$CONFIG_FILE" | cut -d'"' -f2)"
    echo "   模型: $(grep '^model = ' "$CONFIG_FILE" | cut -d'"' -f2)"
else
    echo "❌ 配置文件不存在"
fi

echo ""

# 4. 检查模型
echo "📦 模型状态："
if [ -f "models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf" ]; then
    echo "✅ GGUF 模型存在 ($(du -h models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf | cut -f1))"
else
    echo "⚠️  GGUF 模型不存在"
fi

echo ""
echo "=========================================="
echo "🎯 快速操作指南"
echo "=========================================="
echo ""
echo "启动 Ollama:     ollama serve"
echo "启动 llama.cpp:  ./scripts/start_llama_server.sh"
echo "停止 llama.cpp:  ./scripts/stop_llama_server.sh"
echo "启动 UI:         cargo run -p openclaw-ui --release"
echo "停止 UI:         pkill -f openclaw-plus"
echo ""
echo "测试命令:"
echo "  ./scripts/test_inference.sh"
echo ""
