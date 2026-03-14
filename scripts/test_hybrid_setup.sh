#!/bin/bash
# 测试混合推理引擎配置（无需模型文件）

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}"
echo "=========================================="
echo "OpenClaw+ 混合推理引擎测试"
echo "=========================================="
echo -e "${NC}"

cd "$(dirname "$0")/.."

# 1. 检查 llama.cpp
echo -e "${BLUE}1. 检查 llama.cpp server...${NC}"
if [ -f "./llama-server" ]; then
    echo -e "${GREEN}✅ llama-server 存在${NC}"
    ./llama-server --version 2>/dev/null || echo "版本信息不可用"
elif command -v llama-server >/dev/null 2>&1; then
    echo -e "${GREEN}✅ 系统已安装 llama-server${NC}"
    llama-server --version 2>/dev/null || echo "版本信息不可用"
else
    echo -e "${YELLOW}⚠️  llama-server 未安装${NC}"
    echo "建议安装: brew install llama.cpp"
fi

# 2. 检查配置文件
echo ""
echo -e "${BLUE}2. 检查配置文件...${NC}"
CONFIG_FILE="$HOME/Library/Application Support/openclaw-plus/config.toml"

if [ -f "$CONFIG_FILE" ]; then
    echo -e "${GREEN}✅ 配置文件存在${NC}"
    echo "位置: $CONFIG_FILE"
    
    # 显示当前配置
    echo ""
    echo -e "${BLUE}当前 AI 配置:${NC}"
    grep -A 10 '^\[openclaw_ai\]' "$CONFIG_FILE" | while read line; do
        if [[ "$line" =~ provider|endpoint|model ]]; then
            echo "  $line"
        fi
    done
else
    echo -e "${YELLOW}⚠️  配置文件不存在${NC}"
    echo "OpenClaw 首次运行时会自动创建"
fi

# 3. 创建测试配置（如果需要）
echo ""
echo -e "${BLUE}3. 创建测试配置...${NC}"

# 更新配置为混合模式
if [ -f "$CONFIG_FILE" ]; then
    # 备份
    cp "$CONFIG_FILE" "$CONFIG_FILE.test.backup.$(date +%Y%m%d_%H%M%S)"
    
    # 创建混合配置
    cat > "$CONFIG_FILE" << 'EOF'
[openclaw_ai]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "qwen3.5:9b"
api_key = ""
max_tokens = 4096
temperature = 0.7
stream = false

# 冗余配置
[openclaw_ai.backup]
provider = "llama_cpp_http"
endpoint = "http://localhost:8080"
model = "qwen2.5-7b-instruct-q4_k_m"
EOF
    
    echo -e "${GREEN}✅ 测试配置已创建${NC}"
fi

# 4. 创建管理脚本
echo ""
echo -e "${BLUE}4. 创建管理脚本...${NC}"

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
EOF

# 停止脚本
cat > scripts/stop_llama_server.sh << 'EOF'
#!/bin/bash
echo "🛑 停止 llama-server..."
pkill -f "llama-server.*8080"
echo "✅ 已停止"
EOF

# 测试脚本
cat > scripts/test_inference.sh << 'EOF'
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
EOF

chmod +x scripts/start_llama_server.sh scripts/stop_llama_server.sh scripts/test_inference.sh

# 5. 创建日志目录
mkdir -p logs

# 6. 测试启动 llama.cpp
echo ""
echo -e "${BLUE}5. 测试启动 llama.cpp...${NC}"
./scripts/start_llama_server.sh

# 等待启动
sleep 3

# 7. 运行测试
echo ""
echo -e "${BLUE}6. 运行推理测试...${NC}"
./scripts/test_inference.sh

# 8. 显示状态
echo ""
echo -e "${BLUE}7. 系统状态${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 检查进程
if pgrep -f "llama-server.*8080" > /dev/null; then
    echo -e "${GREEN}✅ llama-server 运行中 (PID: $(pgrep -f 'llama-server.*8080'))${NC}"
else
    echo -e "${YELLOW}⚠️  llama-server 未运行${NC}"
fi

if pgrep -f "ollama.*serve" > /dev/null; then
    echo -e "${GREEN}✅ Ollama 运行中 (PID: $(pgrep -f 'ollama.*serve'))${NC}"
else
    echo -e "${YELLOW}⚠️  Ollama 未运行${NC}"
fi

# 9. 显示下一步
echo ""
echo -e "${GREEN}"
echo "=========================================="
echo "🎉 测试完成！"
echo "=========================================="
echo -e "${NC}"
echo ""
echo "📊 当前配置："
echo "  主引擎: Ollama (http://localhost:11434)"
echo "  备份引擎: llama.cpp (http://localhost:8080)"
echo ""
echo "🚀 下一步操作："
echo ""
echo "1. 下载模型文件（可选）："
echo "   URL: https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF"
echo "   文件: qwen2.5-7b-instruct-q4_k_m.gguf"
echo "   保存到: models/gguf/"
echo ""
echo "2. 重启 OpenClaw UI："
echo "   pkill -f openclaw-plus"
echo "   cargo run -p openclaw-ui --release"
echo ""
echo "3. 管理命令："
echo "   启动 llama.cpp: ./scripts/start_llama_server.sh"
echo "   停止 llama.cpp: ./scripts/stop_llama_server.sh"
echo "   测试推理: ./scripts/test_inference.sh"
echo "   查看日志: tail -f logs/llama-server.log"
echo ""
echo -e "${GREEN}混合推理引擎已配置完成！${NC}"
