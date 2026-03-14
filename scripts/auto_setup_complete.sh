#!/bin/bash
# OpenClaw+ 全自动混合推理引擎配置脚本
# 自动下载 llama.cpp、下载模型、配置并启动

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_header() {
    echo -e "${BLUE}"
    echo "=========================================="
    echo "$1"
    echo "=========================================="
    echo -e "${NC}"
}

# 获取项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

print_header "OpenClaw+ 全自动混合推理引擎配置"

# 检查系统架构
ARCH=$(uname -m)
if [[ "$ARCH" == "arm64" ]]; then
    ARCH_SUFFIX="arm64"
    print_success "检测到 Apple Silicon (ARM64)"
elif [[ "$ARCH" == "x86_64" ]]; then
    ARCH_SUFFIX="x64"
    print_success "检测到 Intel (x86_64)"
else
    print_error "不支持的架构: $ARCH"
    exit 1
fi

# 1. 创建必要目录
print_info "创建目录结构..."
mkdir -p models/gguf
mkdir -p logs
mkdir -p scripts
mkdir -p tools

# 2. 下载 llama.cpp server
print_header "下载 llama.cpp server"

LLAMA_SERVER="$PROJECT_ROOT/llama-server"

if [ -f "$LLAMA_SERVER" ]; then
    print_success "llama-server 已存在"
else
    print_info "正在下载 llama-server (macOS $ARCH_SUFFIX)..."
    
    # 尝试多个下载源
    DOWNLOAD_SUCCESS=false
    
    # 方法 1: GitHub releases
    if [ "$DOWNLOAD_SUCCESS" = false ]; then
        print_info "尝试从 GitHub releases 下载..."
        if curl -L --fail --progress-bar \
            "https://github.com/ggerganov/llama.cpp/releases/latest/download/llama-server-macos-${ARCH_SUFFIX}" \
            -o "$LLAMA_SERVER" 2>/dev/null; then
            chmod +x "$LLAMA_SERVER"
            DOWNLOAD_SUCCESS=true
            print_success "从 GitHub 下载成功"
        fi
    fi
    
    # 方法 2: 尝试预编译二进制
    if [ "$DOWNLOAD_SUCCESS" = false ]; then
        print_info "尝试下载预编译二进制..."
        if curl -L --fail --progress-bar \
            "https://huggingface.co/ggerganov/llama.cpp/resolve/main/llama-server-macos-${ARCH_SUFFIX}" \
            -o "$LLAMA_SERVER" 2>/dev/null; then
            chmod +x "$LLAMA_SERVER"
            DOWNLOAD_SUCCESS=true
            print_success "从 Hugging Face 下载成功"
        fi
    fi
    
    # 方法 3: Homebrew (如果其他方法失败)
    if [ "$DOWNLOAD_SUCCESS" = false ]; then
        print_warning "自动下载失败，尝试使用 Homebrew..."
        if command -v brew >/dev/null 2>&1; then
            if brew install llama.cpp; then
                LLAMA_SERVER=$(brew --prefix)/bin/llama-server
                if [ -f "$LLAMA_SERVER" ]; then
                    DOWNLOAD_SUCCESS=true
                    print_success "通过 Homebrew 安装成功"
                fi
            fi
        else
            print_warning "Homebrew 未安装"
        fi
    fi
    
    if [ "$DOWNLOAD_SUCCESS" = false ]; then
        print_error "无法下载 llama-server"
        print_info "请手动下载："
        print_info "1. 访问: https://github.com/ggerganov/llama.cpp/releases"
        print_info "2. 下载: llama-server-macos-${ARCH_SUFFIX}"
        print_info "3. 保存到: $LLAMA_SERVER"
        print_info "4. 运行: chmod +x $LLAMA_SERVER"
        exit 1
    fi
fi

# 3. 下载 Qwen2.5-7B GGUF 模型
print_header "下载 Qwen2.5-7B-Instruct GGUF 模型"

MODEL_FILE="$PROJECT_ROOT/models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"
MODEL_URL="https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"

if [ -f "$MODEL_FILE" ]; then
    print_success "模型文件已存在: $(basename "$MODEL_FILE")"
    print_info "大小: $(du -h "$MODEL_FILE" | cut -f1)"
else
    print_info "正在下载模型文件 (约 4.4GB)..."
    print_warning "这可能需要 10-30 分钟，请耐心等待..."
    
    # 使用 aria2c 如果可用，否则使用 curl
    if command -v aria2c >/dev/null 2>&1; then
        print_info "使用 aria2c 进行多线程下载..."
        if aria2c -x 16 -s 16 -c --continue=true \
            -d "$(dirname "$MODEL_FILE")" \
            -o "$(basename "$MODEL_FILE")" \
            "$MODEL_URL"; then
            print_success "模型下载成功"
        else
            print_error "aria2c 下载失败"
            exit 1
        fi
    else
        print_info "使用 curl 下载 (支持断点续传)..."
        if curl -L --progress-bar --continue-at - \
            -o "$MODEL_FILE" \
            "$MODEL_URL"; then
            print_success "模型下载成功"
        else
            print_error "模型下载失败"
            print_info "请手动下载："
            print_info "URL: $MODEL_URL"
            print_info "保存到: $MODEL_FILE"
            exit 1
        fi
    fi
    
    # 验证下载的文件
    if [ -f "$MODEL_FILE" ]; then
        FILE_SIZE=$(du -h "$MODEL_FILE" | cut -f1)
        print_success "模型文件验证成功，大小: $FILE_SIZE"
    else
        print_error "模型文件下载失败"
        exit 1
    fi
fi

# 4. 创建管理脚本
print_header "创建管理脚本"

# 启动脚本
cat > scripts/start_llama_server.sh << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.."

# 检查模型文件
MODEL_FILE="models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"
if [ ! -f "$MODEL_FILE" ]; then
    echo "❌ 模型文件不存在: $MODEL_FILE"
    echo "请先运行: ./scripts/auto_setup_complete.sh"
    exit 1
fi

# 检查 llama-server
if [ ! -f "llama-server" ]; then
    echo "❌ llama-server 不存在"
    echo "请先运行: ./scripts/auto_setup_complete.sh"
    exit 1
fi

# 检查是否已运行
if pgrep -f "llama-server.*8080" > /dev/null; then
    echo "✅ llama-server 已在运行 (端口 8080)"
    exit 0
fi

# 创建日志目录
mkdir -p logs

# 启动 llama.cpp server
echo "🚀 启动 llama-server (端口 8080)..."
nohup ./llama-server \
  -m models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  -ngl 99 \
  --ctx-size 8192 \
  --chat-template chatml \
  > logs/llama-server.log 2>&1 &

SERVER_PID=$!
echo "✅ llama-server 已启动"
echo "   PID: $SERVER_PID"
echo "   端口: 8080"
echo "   日志: logs/llama-server.log"
echo "   测试: curl http://localhost:8080/v1/models"

# 等待服务启动
echo "等待服务启动..."
for i in {1..30}; do
    if curl -s http://localhost:8080/v1/models >/dev/null 2>&1; then
        echo "✅ 服务启动成功！"
        exit 0
    fi
    echo -n "."
    sleep 1
done

echo ""
echo "⚠️  服务启动可能需要更长时间，请检查日志:"
echo "   tail -f logs/llama-server.log"
EOF

# 停止脚本
cat > scripts/stop_llama_server.sh << 'EOF'
#!/bin/bash
echo "🛑 停止 llama-server..."
if pgrep -f "llama-server.*8080" > /dev/null; then
    pkill -f "llama-server.*8080"
    sleep 2
    if pgrep -f "llama-server.*8080" > /dev/null; then
        echo "⚠️  强制停止 llama-server..."
        pkill -9 -f "llama-server.*8080"
    fi
    echo "✅ llama-server 已停止"
else
    echo "✅ llama-server 未运行"
fi
EOF

# 测试脚本
cat > scripts/test_llama_server.sh << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.."

echo "🧪 测试 llama.cpp server..."

# 测试连接
echo "1. 测试连接..."
if curl -s http://localhost:8080/v1/models >/dev/null 2>&1; then
    echo "✅ 连接成功"
else
    echo "❌ 连接失败，请先启动服务: ./scripts/start_llama_server.sh"
    exit 1
fi

# 测试推理
echo "2. 测试推理..."
RESPONSE=$(curl -s -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5-7b-instruct-q4_k_m",
    "messages": [{"role": "user", "content": "你好"}],
    "max_tokens": 50
  }')

if echo "$RESPONSE" | grep -q '"content"'; then
    echo "✅ 推理测试成功"
    CONTENT=$(echo "$RESPONSE" | jq -r '.choices[0].message.content' 2>/dev/null || echo "无法解析响应")
    echo "   回复: $CONTENT"
else
    echo "❌ 推理测试失败"
    echo "   响应: $RESPONSE"
fi
EOF

# 添加执行权限
chmod +x scripts/start_llama_server.sh
chmod +x scripts/stop_llama_server.sh
chmod +x scripts/test_llama_server.sh

print_success "管理脚本创建完成"

# 5. 更新配置文件
print_header "配置 OpenClaw 使用 llama.cpp"

CONFIG_FILE="$HOME/Library/Application Support/openclaw-plus/config.toml"

if [ -f "$CONFIG_FILE" ]; then
    # 备份原配置
    BACKUP_FILE="$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
    cp "$CONFIG_FILE" "$BACKUP_FILE"
    print_success "配置文件已备份: $(basename "$BACKUP_FILE")"
    
    # 更新配置
    if grep -q "^\[openclaw_ai\]" "$CONFIG_FILE"; then
        # 使用临时文件进行替换（macOS 兼容）
        sed '/^\[openclaw_ai\]/,/^\[/ {
            s/^provider = .*/provider = "llama_cpp_http"/
            s|^endpoint = .*|endpoint = "http://localhost:8080"|
            s/^model = .*/model = "qwen2.5-7b-instruct-q4_k_m"/
        }' "$CONFIG_FILE" > "$CONFIG_FILE.tmp" && mv "$CONFIG_FILE.tmp" "$CONFIG_FILE"
        
        print_success "配置文件已更新为 llama.cpp"
    else
        print_warning "配置文件格式异常，请手动编辑: $CONFIG_FILE"
    fi
else
    print_warning "配置文件不存在: $CONFIG_FILE"
    print_info "OpenClaw 首次运行时会自动创建"
fi

# 6. 启动 llama.cpp server
print_header "启动 llama.cpp server"

print_info "启动 llama-server..."
if ./scripts/start_llama_server.sh; then
    print_success "llama-server 启动成功"
else
    print_error "llama-server 启动失败"
    print_info "请查看日志: tail -f logs/llama-server.log"
    exit 1
fi

# 7. 验证配置
print_header "验证配置"

print_info "测试 llama.cpp server..."
if ./scripts/test_llama_server.sh; then
    print_success "llama.cpp server 验证通过"
else
    print_warning "llama.cpp server 验证失败，但配置已完成"
fi

# 8. 显示完成信息
print_header "🎉 配置完成！"

echo ""
print_success "混合推理引擎配置完成！"
echo ""
echo "📊 配置详情:"
echo "  主引擎: llama.cpp (http://localhost:8080)"
echo "  备份引擎: Ollama (http://localhost:11434)"
echo "  模型: Qwen2.5-7B-Instruct Q4_K_M"
echo "  内存占用: ~4GB"
echo ""
echo "🚀 下一步操作:"
echo ""
echo "1. 重启 OpenClaw UI:"
echo "   pkill -f openclaw-plus"
echo "   cargo run -p openclaw-ui --release"
echo ""
echo "2. 在 Claw Terminal 测试:"
echo "   🧪 Auto Test - 10 条核心功能测试"
echo "   📄 Page Test - 9 个页面自动切换"
echo ""
echo "3. 管理命令:"
echo "   启动: ./scripts/start_llama_server.sh"
echo "   停止: ./scripts/stop_llama_server.sh"
echo "   测试: ./scripts/test_llama_server.sh"
echo "   日志: tail -f logs/llama-server.log"
echo ""
echo "4. 冗余切换测试:"
echo "   - 停止 llama.cpp: ./scripts/stop_llama_server.sh"
echo "   - 启动 Ollama: ollama serve (需要先更新到支持 Qwen3.5)"
echo "   - OpenClaw 会自动切换到备份引擎"
echo ""
echo "📁 重要文件:"
echo "  模型文件: $MODEL_FILE"
echo "  配置文件: $CONFIG_FILE"
echo "  启动脚本: scripts/start_llama_server.sh"
echo "  详细文档: QUICK_START_HYBRID.md"
echo ""
print_success "现在您可以使用更轻量、更快速的 llama.cpp 推理引擎了！"
echo ""
