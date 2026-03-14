#!/bin/bash
# 使用已下载文件进行配置

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}"
echo "=========================================="
echo "OpenClaw+ 快速配置（使用已有文件）"
echo "=========================================="
echo -e "${NC}"

cd "$(dirname "$0")/.."

# 检查 llama.cpp
LLAMA_SERVER=""
if [ -f "./llama-server" ]; then
    LLAMA_SERVER="./llama-server"
    echo -e "${GREEN}✅ 找到项目中的 llama-server${NC}"
elif command -v llama-server >/dev/null 2>&1; then
    LLAMA_SERVER="$(which llama-server)"
    echo -e "${GREEN}✅ 找到系统安装的 llama-server${NC}"
else
    echo -e "${YELLOW}⚠️  未找到 llama-server${NC}"
    echo "请先安装 llama.cpp："
    echo "  brew install llama.cpp"
    echo "或手动下载到项目根目录"
    exit 1
fi

# 检查模型文件
MODEL_FILE="models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"
if [ ! -f "$MODEL_FILE" ]; then
    echo -e "${YELLOW}⚠️  模型文件不存在${NC}"
    echo "请下载模型文件到：$MODEL_FILE"
    echo "下载地址：https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF"
    exit 1
fi

echo -e "${GREEN}✅ 找到模型文件${NC}"
echo "大小: $(du -h "$MODEL_FILE" | cut -f1)"

# 创建管理脚本
mkdir -p scripts logs

# 启动脚本
cat > scripts/start_llama_server.sh << 'EOF'
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
    exit 1
fi

# 检查模型
MODEL_FILE="models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"
if [ ! -f "$MODEL_FILE" ]; then
    echo "❌ 模型文件不存在: $MODEL_FILE"
    exit 1
fi

# 检查是否已运行
if pgrep -f "llama-server.*8080" > /dev/null; then
    echo "✅ llama-server 已在运行"
    exit 0
fi

# 启动
echo "🚀 启动 llama-server..."
nohup "$LLAMA_SERVER" \
  -m "$MODEL_FILE" \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml \
  > logs/llama-server.log 2>&1 &

echo "✅ llama-server 已启动"
echo "   端口: 8080"
echo "   日志: logs/llama-server.log"
EOF

# 停止脚本
cat > scripts/stop_llama_server.sh << 'EOF'
#!/bin/bash
echo "🛑 停止 llama-server..."
pkill -f "llama-server.*8080"
echo "✅ 已停止"
EOF

chmod +x scripts/start_llama_server.sh scripts/stop_llama_server.sh

# 更新配置
CONFIG_FILE="$HOME/Library/Application Support/openclaw-plus/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    echo -e "${BLUE}更新配置文件...${NC}"
    cp "$CONFIG_FILE" "$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
    
    # 更新配置
    sed -i '' 's/^provider = .*/provider = "llama_cpp_http"/' "$CONFIG_FILE"
    sed -i '' 's|^endpoint = .*|endpoint = "http://localhost:8080"|' "$CONFIG_FILE"
    sed -i '' 's/^model = .*/model = "qwen2.5-7b-instruct-q4_k_m"/' "$CONFIG_FILE"
    
    echo -e "${GREEN}✅ 配置已更新${NC}"
else
    echo -e "${YELLOW}⚠️  配置文件不存在，首次运行时会创建${NC}"
fi

# 启动服务
echo ""
echo -e "${BLUE}启动 llama.cpp server...${NC}"
./scripts/start_llama_server.sh

# 等待启动
echo -e "${BLUE}等待服务启动...${NC}"
for i in {1..30}; do
    if curl -s http://localhost:8080/v1/models >/dev/null 2>&1; then
        echo -e "${GREEN}✅ 服务启动成功！${NC}"
        break
    fi
    echo -n "."
    sleep 1
done

echo ""
echo -e "${GREEN}"
echo "=========================================="
echo "🎉 配置完成！"
echo "=========================================="
echo -e "${NC}"
echo ""
echo "📊 配置详情："
echo "  主引擎: llama.cpp (端口 8080)"
echo "  备份引擎: Ollama (端口 11434)"
echo "  模型: Qwen2.5-7B-Instruct Q4_K_M"
echo ""
echo "🚀 下一步："
echo "1. 重启 OpenClaw UI："
echo "   pkill -f openclaw-plus"
echo "   cargo run -p openclaw-ui --release"
echo ""
echo "2. 测试功能："
echo "   🧪 Auto Test - 10 条核心功能测试"
echo "   📄 Page Test - 9 个页面自动切换"
echo ""
echo "3. 管理命令："
echo "   启动: ./scripts/start_llama_server.sh"
echo "   停止: ./scripts/stop_llama_server.sh"
echo "   日志: tail -f logs/llama-server.log"
echo ""
echo -e "${GREEN}现在可以使用更轻量的 llama.cpp 推理引擎了！${NC}"
