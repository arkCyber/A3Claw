#!/bin/bash
cd "$(dirname "$0")/.."

echo "🧪 测试推理引擎..."

# 测试 Ollama
echo "1. 测试 Ollama (端口 11434)..."
if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    echo "✅ Ollama 可用"
else
    echo "⚠️  Ollama 不可用"
fi

# 测试 llama.cpp
echo "2. 测试 llama.cpp (端口 8080)..."
if curl -s http://localhost:8080/v1/models >/dev/null 2>&1; then
    echo "✅ llama.cpp 可用"
    
    # 测试推理
    echo "3. 测试 llama.cpp 推理..."
    RESPONSE=$(curl -s -X POST http://localhost:8080/v1/chat/completions \
      -H "Content-Type: application/json" \
      -d '{
        "model": "test",
        "messages": [{"role": "user", "content": "hello"}],
        "max_tokens": 10
      }' 2>/dev/null || echo "error")
    
    if echo "$RESPONSE" | grep -q '"content"'; then
        echo "✅ llama.cpp 推理正常"
    else
        echo "⚠️  llama.cpp 推理测试失败（可能需要模型文件）"
    fi
else
    echo "⚠️  llama.cpp 不可用"
fi
