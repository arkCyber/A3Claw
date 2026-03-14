#!/bin/bash
# OpenClaw+ 混合推理引擎自动配置脚本
# llama.cpp (主引擎) + Ollama (备份)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

echo "=========================================="
echo "OpenClaw+ 混合推理引擎配置"
echo "=========================================="
echo ""

# 1. 创建模型目录
echo "📁 创建模型目录..."
mkdir -p models/gguf
mkdir -p logs

# 2. 下载 llama.cpp server
echo ""
echo "📥 下载 llama.cpp server..."
if [ ! -f "llama-server" ]; then
    echo "正在下载 llama-server (macOS ARM64)..."
    
    # 尝试从 GitHub Releases 下载
    if curl -L --fail "https://github.com/ggerganov/llama.cpp/releases/latest/download/llama-server" \
        -o llama-server 2>/dev/null; then
        chmod +x llama-server
        echo "✅ llama-server 下载成功"
    else
        echo "⚠️  自动下载失败，请手动下载："
        echo "   1. 访问 https://github.com/ggerganov/llama.cpp/releases"
        echo "   2. 下载 llama-server (macOS ARM64 版本)"
        echo "   3. 放置到: $PROJECT_ROOT/llama-server"
        echo "   4. 运行: chmod +x llama-server"
    fi
else
    echo "✅ llama-server 已存在"
fi

# 3. 下载 Qwen2.5-7B GGUF 模型
echo ""
echo "📥 下载 Qwen2.5-7B-Instruct GGUF 模型..."
MODEL_FILE="models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"

if [ ! -f "$MODEL_FILE" ]; then
    echo "正在下载模型文件 (约 4.4GB)..."
    echo "这可能需要 10-30 分钟，请耐心等待..."
    
    # 使用 Hugging Face 镜像下载
    if curl -L --progress-bar \
        "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf" \
        -o "$MODEL_FILE"; then
        echo "✅ 模型下载成功: $MODEL_FILE"
    else
        echo "❌ 模型下载失败"
        echo "请手动下载："
        echo "   URL: https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"
        echo "   保存到: $PROJECT_ROOT/$MODEL_FILE"
        exit 1
    fi
else
    echo "✅ 模型文件已存在: $MODEL_FILE"
fi

# 4. 创建启动脚本
echo ""
echo "📝 创建 llama.cpp 启动脚本..."
cat > scripts/start_llama_server.sh << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.."

# 检查是否已运行
if pgrep -f "llama-server.*8080" > /dev/null; then
    echo "llama-server 已在运行 (端口 8080)"
    exit 0
fi

# 启动 llama.cpp server
echo "启动 llama-server (端口 8080)..."
nohup ./llama-server \
  -m models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml \
  > logs/llama-server.log 2>&1 &

echo "✅ llama-server 已启动"
echo "   端口: 8080"
echo "   日志: logs/llama-server.log"
echo "   测试: curl http://localhost:8080/v1/models"
EOF

chmod +x scripts/start_llama_server.sh

# 5. 创建停止脚本
cat > scripts/stop_llama_server.sh << 'EOF'
#!/bin/bash
echo "停止 llama-server..."
pkill -f "llama-server.*8080"
echo "✅ llama-server 已停止"
EOF

chmod +x scripts/stop_llama_server.sh

# 6. 更新配置文件
echo ""
echo "⚙️  配置混合推理引擎..."

CONFIG_FILE="$HOME/Library/Application Support/openclaw-plus/config.toml"

if [ -f "$CONFIG_FILE" ]; then
    # 备份原配置
    cp "$CONFIG_FILE" "$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
    
    # 更新配置为 llama.cpp
    if grep -q "^\[openclaw_ai\]" "$CONFIG_FILE"; then
        # 使用 sed 更新配置（macOS 兼容）
        sed -i '' 's/^provider = .*/provider = "llama_cpp_http"/' "$CONFIG_FILE"
        sed -i '' 's|^endpoint = .*|endpoint = "http://localhost:8080"|' "$CONFIG_FILE"
        sed -i '' 's/^model = .*/model = "qwen2.5-7b-instruct-q4_k_m"/' "$CONFIG_FILE"
        
        echo "✅ 配置文件已更新为 llama.cpp"
    fi
else
    echo "⚠️  配置文件不存在: $CONFIG_FILE"
fi

# 7. 显示完成信息
echo ""
echo "=========================================="
echo "✅ 混合推理引擎配置完成！"
echo "=========================================="
echo ""
echo "下一步操作："
echo ""
echo "1. 启动 llama.cpp server:"
echo "   ./scripts/start_llama_server.sh"
echo ""
echo "2. 测试连接:"
echo "   curl http://localhost:8080/v1/models"
echo ""
echo "3. 重启 OpenClaw UI:"
echo "   cargo run -p openclaw-ui --release"
echo ""
echo "4. 测试功能:"
echo "   - 🧪 Auto Test (10 条核心功能测试)"
echo "   - 📄 Page Test (9 个页面自动切换)"
echo ""
echo "配置详情:"
echo "  主引擎: llama.cpp (http://localhost:8080)"
echo "  备份引擎: Ollama (http://localhost:11434)"
echo "  模型: Qwen2.5-7B-Instruct Q4_K_M"
echo "  内存占用: ~4GB"
echo ""
echo "管理命令:"
echo "  启动: ./scripts/start_llama_server.sh"
echo "  停止: ./scripts/stop_llama_server.sh"
echo "  日志: tail -f logs/llama-server.log"
echo ""
