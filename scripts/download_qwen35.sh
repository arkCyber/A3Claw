#!/bin/bash
# 下载 qwen3.5:9b 模型

echo "📥 正在下载 qwen3.5:9b 模型..."
echo "⚠️  注意：这个模型约 5.5GB，下载可能需要几分钟"
echo ""

# 使用完整路径调用 ollama
/opt/homebrew/bin/ollama pull qwen3.5:9b

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ qwen3.5:9b 下载成功！"
    echo ""
    echo "📋 已安装的模型："
    /opt/homebrew/bin/ollama list
else
    echo ""
    echo "❌ 下载失败，请检查网络连接"
    echo "   或手动运行: /opt/homebrew/bin/ollama pull qwen3.5:9b"
fi
