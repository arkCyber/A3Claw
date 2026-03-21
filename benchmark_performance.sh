#!/bin/bash
# OpenClaw+ 性能基准测试
# 测试优化前后的性能对比

set -e

BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  OpenClaw+ 性能基准测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo

# 测试 1: WasmEdge 版本检查性能
echo -e "${YELLOW}[1/4] WasmEdge 版本检查性能${NC}"
START=$(date +%s%N)
for i in {1..10}; do
    wasmedge --version > /dev/null 2>&1
done
END=$(date +%s%N)
DURATION=$(( (END - START) / 10000000 ))
echo -e "${GREEN}✓${NC} 平均耗时: ${DURATION}ms (10次调用)"

# 测试 2: WASM 文件验证性能
echo
echo -e "${YELLOW}[2/4] WASM 文件验证性能${NC}"
if [ -f "assets/wasmedge_quickjs.wasm" ]; then
    START=$(date +%s%N)
    for i in {1..100}; do
        # 模拟验证：读取文件元数据 + 前4字节
        stat assets/wasmedge_quickjs.wasm > /dev/null 2>&1
        head -c 4 assets/wasmedge_quickjs.wasm > /dev/null 2>&1
    done
    END=$(date +%s%N)
    DURATION=$(( (END - START) / 100000000 ))
    echo -e "${GREEN}✓${NC} 平均耗时: ${DURATION}ms (100次验证)"
else
    echo -e "${YELLOW}⚠${NC} QuickJS WASM 文件未找到，跳过测试"
fi

# 测试 3: 编译性能
echo
echo -e "${YELLOW}[3/4] Rust 编译性能${NC}"
START=$(date +%s)
SDKROOT=$(xcrun --show-sdk-path) BINDGEN_EXTRA_CLANG_ARGS="-isysroot $(xcrun --show-sdk-path)" \
    cargo build -p openclaw-sandbox --lib --quiet 2>&1 | grep -v "warning:" || true
END=$(date +%s)
DURATION=$((END - START))
echo -e "${GREEN}✓${NC} 编译耗时: ${DURATION}s"

# 测试 4: 单元测试性能
echo
echo -e "${YELLOW}[4/4] 单元测试性能${NC}"
START=$(date +%s)
SDKROOT=$(xcrun --show-sdk-path) BINDGEN_EXTRA_CLANG_ARGS="-isysroot $(xcrun --show-sdk-path)" \
    cargo test -p openclaw-sandbox --lib --quiet 2>&1 | grep -v "warning:" || true
END=$(date +%s)
DURATION=$((END - START))
echo -e "${GREEN}✓${NC} 测试耗时: ${DURATION}s"

echo
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  性能基准测试完成${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
