#!/bin/bash
# 使用国内镜像下载 Qwen2.5-7B GGUF 模型

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

MODEL_FILE="$PROJECT_ROOT/models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"
mkdir -p models/gguf

echo "=========================================="
echo "下载 Qwen2.5-7B-Instruct GGUF 模型"
echo "=========================================="

# 方法 1: 使用 ModelScope 镜像（国内）
download_from_modelscope() {
    print_info "尝试从 ModelScope 下载..."
    local url="https://www.modelscope.cn/api/v1/models/Qwen/Qwen2.5-7B-Instruct-GGUF/repo?Revision=master&FilePath=qwen2.5-7b-instruct-q4_k_m.gguf"
    
    if curl -L --progress-bar --continue-at - -o "$MODEL_FILE" "$url"; then
        print_success "从 ModelScope 下载成功"
        return 0
    else
        print_warning "ModelScope 下载失败"
        return 1
    fi
}

# 方法 2: 使用魔搭社区镜像
download_from_moxing() {
    print_info "尝试从魔搭社区下载..."
    local url="https://download.openmmlab.com/mmodel/qwen/Qwen2.5-7B-Instruct-GGUF/qwen2.5-7b-instruct-q4_k_m.gguf"
    
    if curl -L --progress-bar --continue-at - -o "$MODEL_FILE" "$url"; then
        print_success "从魔搭社区下载成功"
        return 0
    else
        print_warning "魔搭社区下载失败"
        return 1
    fi
}

# 方法 3: 使用阿里云 OSS 镜像
download_from_aliyun() {
    print_info "尝试从阿里云 OSS 下载..."
    local url="https://openclaw-models.oss-cn-hangzhou.aliyuncs.com/qwen2.5-7b-instruct-q4_k_m.gguf"
    
    if curl -L --progress-bar --continue-at - -o "$MODEL_FILE" "$url"; then
        print_success "从阿里云 OSS 下载成功"
        return 0
    else
        print_warning "阿里云 OSS 下载失败"
        return 1
    fi
}

# 方法 4: 使用 aria2c 多线程下载
download_with_aria2c() {
    if ! command -v aria2c >/dev/null 2>&1; then
        print_warning "aria2c 未安装"
        return 1
    fi
    
    print_info "使用 aria2c 多线程下载..."
    local url="https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"
    
    if aria2c -x 16 -s 16 -c --continue=true \
        -d "$(dirname "$MODEL_FILE")" \
        -o "$(basename "$MODEL_FILE")" \
        "$url"; then
        print_success "aria2c 下载成功"
        return 0
    else
        print_warning "aria2c 下载失败"
        return 1
    fi
}

# 方法 5: 分片下载（如果大文件下载失败）
download_in_chunks() {
    print_info "尝试分片下载..."
    
    # 这里可以实现分片下载逻辑
    # 由于 GGUF 文件通常很大，分片下载比较复杂
    # 暂时跳过
    return 1
}

# 尝试各种下载方法
if [ -f "$MODEL_FILE" ]; then
    print_success "模型文件已存在"
    print_info "大小: $(du -h "$MODEL_FILE" | cut -f1)"
    exit 0
fi

# 尝试下载
METHODS=(
    "download_from_modelscope"
    "download_from_moxing"
    "download_from_aliyun"
    "download_with_aria2c"
)

for method in "${METHODS[@]}"; do
    echo ""
    print_info "尝试方法: $method"
    if $method; then
        # 验证下载的文件
        if [ -f "$MODEL_FILE" ]; then
            FILE_SIZE=$(du -h "$MODEL_FILE" | cut -f1)
            print_success "模型文件下载成功，大小: $FILE_SIZE"
            
            # 检查文件大小是否合理（应该大于 1GB）
            FILE_SIZE_BYTES=$(stat -f%z "$MODEL_FILE" 2>/dev/null || stat -c%s "$MODEL_FILE" 2>/dev/null || echo 0)
            if [ "$FILE_SIZE_BYTES" -gt 1000000000 ]; then
                print_success "文件大小验证通过"
                exit 0
            else
                print_warning "文件大小异常，可能下载不完整"
                rm -f "$MODEL_FILE"
            fi
        fi
    fi
done

# 所有方法都失败
echo ""
print_error "所有下载方法都失败"
echo ""
echo "请选择以下方式之一："
echo ""
echo "方式 1：浏览器下载（推荐）"
echo "  1. 打开浏览器访问："
echo "     https://www.modelscope.cn/models/Qwen/Qwen2.5-7B-Instruct-GGUF/files"
echo "  2. 下载文件：qwen2.5-7b-instruct-q4_k_m.gguf"
echo "  3. 保存到：$MODEL_FILE"
echo ""
echo "方式 2：使用 VPN 后重新运行脚本"
echo "  export https_proxy=http://127.0.0.1:7890"
echo "  ./scripts/download_model_with_mirrors.sh"
echo ""
echo "方式 3：使用 wget（如果 curl 失败）"
echo "  wget -c -O '$MODEL_FILE' \\"
echo "    'https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf'"
echo ""
echo "方式 4：下载较小的模型（Q4_K_M 约 4.4GB）"
echo "  如果 7B 模型太大，可以尝试 3B 模型："
echo "  URL: https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf"
echo ""

exit 1
