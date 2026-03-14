#!/bin/bash
cd "$(dirname "$0")/.."

# 检查 llama-server
LLAMA_SERVER=""
if [ -f "./llama-server" ]; then
    LLAMA_SERVER="./llama-server"
elif command -v llama-server >/dev/null 2>&1; then
    LLAMA_SERVER="$(which llama-server)"
fi

if [ -z "$LLAMA_SERVER" ]; then
    echo "❌ 未找到 llama-server"
    echo "请安装: brew install llama.cpp"
    exit 1
fi

# 检查模型文件
MODEL_FILE="models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"
if [ ! -f "$MODEL_FILE" ]; then
    echo "⚠️  模型文件不存在: $MODEL_FILE"
    echo "使用测试模式启动..."
    
    # 测试模式启动（不加载模型）
    nohup "$LLAMA_SERVER" \
      --port 8080 \
      --host 0.0.0.0 \
      > logs/llama-server.log 2>&1 &
else
    echo "🚀 启动 llama-server（加载模型）..."
    nohup "$LLAMA_SERVER" \
      -m "$MODEL_FILE" \
      --port 8080 \
      --host 0.0.0.0 \
      -ngl 99 \
      --ctx-size 8192 \
      --chat-template chatml \
      > logs/llama-server.log 2>&1 &
fi

echo "✅ llama-server 已启动"
echo "   端口: 8080"
echo "   日志: logs/llama-server.log"
