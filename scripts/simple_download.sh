#!/bin/bash
# 简单直接下载方案

set -e

# 颜色
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}"
echo "=========================================="
echo "OpenClaw+ 模型下载助手"
echo "=========================================="
echo -e "${NC}"

cd "$(dirname "$0")/.."
mkdir -p models/gguf

MODEL_FILE="models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"

if [ -f "$MODEL_FILE" ]; then
    echo -e "${GREEN}✅ 模型文件已存在${NC}"
    echo "大小: $(du -h "$MODEL_FILE" | cut -f1)"
    exit 0
fi

echo -e "${BLUE}ℹ️  开始下载模型文件...${NC}"
echo -e "${YELLOW}文件大小约 4.4GB，请耐心等待${NC}"
echo ""

# 使用 aria2c 如果可用，否则使用 curl
if command -v aria2c >/dev/null 2>&1; then
    echo -e "${BLUE}使用 aria2c 多线程下载...${NC}"
    aria2c -x 16 -s 16 -c \
        -d models/gguf \
        -o qwen2.5-7b-instruct-q4_k_m.gguf \
        "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"
else
    echo -e "${BLUE}使用 curl 下载（支持断点续传）...${NC}"
    curl -L --progress-bar --continue-at - \
        -o "$MODEL_FILE" \
        "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"
fi

if [ -f "$MODEL_FILE" ]; then
    FILE_SIZE=$(du -h "$MODEL_FILE" | cut -f1)
    echo -e "${GREEN}✅ 下载成功！${NC}"
    echo "文件大小: $FILE_SIZE"
    
    # 检查文件大小
    SIZE_BYTES=$(stat -f%z "$MODEL_FILE" 2>/dev/null || stat -c%s "$MODEL_FILE" 2>/dev/null || echo 0)
    if [ "$SIZE_BYTES" -gt 1000000000 ]; then
        echo -e "${GREEN}✅ 文件大小验证通过${NC}"
    else
        echo -e "${YELLOW}⚠️  文件可能不完整${NC}"
    fi
else
    echo -e "${RED}❌ 下载失败${NC}"
    echo ""
    echo "请手动下载："
    echo "1. 访问: https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF"
    echo "2. 下载: qwen2.5-7b-instruct-q4_k_m.gguf"
    echo "3. 保存到: $(pwd)/$MODEL_FILE"
    exit 1
fi
