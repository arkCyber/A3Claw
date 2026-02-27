#!/bin/bash
# Ollama 服务器启动脚本
# Ollama 需要作为独立服务运行，此脚本帮助启动和管理

set -e

echo "🦙 Ollama 服务器启动脚本"
echo "================================"
echo ""

# 检查 Ollama 是否已安装
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama 未安装"
    echo ""
    echo "安装方法:"
    echo "  macOS:   brew install ollama"
    echo "  Linux:   curl https://ollama.ai/install.sh | sh"
    echo ""
    exit 1
fi

echo "✅ Ollama 已安装: $(which ollama)"
echo ""

# 检查 Ollama 是否已在运行
if curl -s http://localhost:11434/api/tags &> /dev/null; then
    echo "✅ Ollama 已在运行"
    echo ""
    echo "📦 已安装的模型:"
    ollama list
    echo ""
    echo "💡 提示: 使用以下命令管理 Ollama:"
    echo "  ollama list           # 列出模型"
    echo "  ollama pull <model>   # 下载模型"
    echo "  ollama run <model>    # 运行模型"
    exit 0
fi

echo "🚀 启动 Ollama 服务..."
echo ""
echo "⚠️  Ollama 将在后台运行"
echo "   日志位置: ~/.ollama/logs/"
echo ""

# 启动 Ollama 服务
ollama serve &
OLLAMA_PID=$!

echo "✅ Ollama 已启动 (PID: $OLLAMA_PID)"
echo ""

# 等待 Ollama 启动
echo "⏳ 等待 Ollama 服务就绪..."
for i in {1..30}; do
    if curl -s http://localhost:11434/api/tags &> /dev/null; then
        echo "✅ Ollama 服务已就绪！"
        echo ""
        break
    fi
    sleep 1
    echo -n "."
done
echo ""

# 检查是否有模型
echo "📦 检查已安装的模型:"
if ollama list | grep -q "NAME"; then
    ollama list
else
    echo "⚠️  未找到已安装的模型"
    echo ""
    echo "推荐下载以下轻量级模型:"
    echo "  ollama pull qwen2.5:0.5b    # 500MB, 快速"
    echo "  ollama pull llama3.2:1b     # 1.3GB, 平衡"
    echo "  ollama pull llama3.2:3b     # 2GB, 高质量"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Ollama 服务器已启动并运行"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📡 服务端点: http://localhost:11434"
echo "🔧 管理命令:"
echo "  ollama list           # 列出模型"
echo "  ollama pull <model>   # 下载模型"
echo "  ollama ps             # 查看运行中的模型"
echo ""
echo "🧪 测试推理:"
echo "  curl http://localhost:11434/api/generate -d '{\"model\":\"qwen2.5:0.5b\",\"prompt\":\"Hello\"}'"
echo ""
echo "🛑 停止服务:"
echo "  pkill ollama"
echo ""
