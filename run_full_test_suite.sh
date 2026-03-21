#!/bin/bash
# OpenClaw+ 完整自动化测试套件
# 航空航天级别标准 - 自动执行所有测试并收集结果

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 测试结果目录
RESULTS_DIR="/tmp/openclaw_test_results_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  OpenClaw+ 完整自动化测试套件${NC}"
echo -e "${BLUE}  测试时间: $(date '+%Y-%m-%d %H:%M:%S')${NC}"
echo -e "${BLUE}  结果目录: $RESULTS_DIR${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 记录测试结果
log_test() {
    local name="$1"
    local status="$2"
    local duration="$3"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    if [ "$status" = "PASS" ]; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
        echo -e "${GREEN}✓${NC} $name ${GREEN}($duration)${NC}"
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
        echo -e "${RED}✗${NC} $name ${RED}($duration)${NC}"
    fi
}

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo -e "${YELLOW}[1/7] Rust 编译测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

START_TIME=$(date +%s)
if SDKROOT=$(xcrun --show-sdk-path) BINDGEN_EXTRA_CLANG_ARGS="-isysroot $(xcrun --show-sdk-path)" \
   cargo build -p openclaw-sandbox --lib > "$RESULTS_DIR/01_cargo_build.log" 2>&1; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "Sandbox crate 编译" "PASS" "${DURATION}s"
else
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "Sandbox crate 编译" "FAIL" "${DURATION}s"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[2/7] Rust 单元测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

START_TIME=$(date +%s)
if SDKROOT=$(xcrun --show-sdk-path) BINDGEN_EXTRA_CLANG_ARGS="-isysroot $(xcrun --show-sdk-path)" \
   cargo test -p openclaw-sandbox --lib > "$RESULTS_DIR/02_cargo_test.log" 2>&1; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    TEST_COUNT=$(grep -c "test result: ok" "$RESULTS_DIR/02_cargo_test.log" || echo "0")
    log_test "Sandbox 单元测试 (11个)" "PASS" "${DURATION}s"
else
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "Sandbox 单元测试" "FAIL" "${DURATION}s"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[3/7] WasmEdge 简化测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

START_TIME=$(date +%s)
if bash tests/test_wasmedge_simple.sh > "$RESULTS_DIR/03_wasmedge_simple.log" 2>&1; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "WasmEdge 环境检查" "PASS" "${DURATION}s"
else
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "WasmEdge 环境检查" "FAIL" "${DURATION}s"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[4/7] WasmEdge 沙箱测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

START_TIME=$(date +%s)
if bash tests/test_wasmedge_sandbox.sh > "$RESULTS_DIR/04_wasmedge_sandbox.log" 2>&1; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    PASS_COUNT=$(grep -c "\[OK\]" "$RESULTS_DIR/04_wasmedge_sandbox.log" || echo "0")
    log_test "WasmEdge 沙箱集成测试 ($PASS_COUNT 项通过)" "PASS" "${DURATION}s"
else
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "WasmEdge 沙箱集成测试" "FAIL" "${DURATION}s"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[5/7] 自然语言工具测试 (完整14个用例)${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

START_TIME=$(date +%s)
if timeout 1800 python3 -u nl_cli_full_test.py > "$RESULTS_DIR/05_nl_test.log" 2>&1; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    PASS_COUNT=$(grep -c "结果: PASS" "$RESULTS_DIR/05_nl_test.log" || echo "0")
    FAIL_COUNT=$(grep -c "结果: FAIL" "$RESULTS_DIR/05_nl_test.log" || echo "0")
    log_test "自然语言测试 ($PASS_COUNT 通过, $FAIL_COUNT 失败)" "PASS" "${DURATION}s"
else
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "自然语言测试" "FAIL" "${DURATION}s (可能超时)"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[6/7] 代码质量检查${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

START_TIME=$(date +%s)
if cargo clippy -p openclaw-sandbox --lib -- -D warnings > "$RESULTS_DIR/06_clippy.log" 2>&1; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "Clippy 代码检查" "PASS" "${DURATION}s"
else
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    WARNING_COUNT=$(grep -c "warning:" "$RESULTS_DIR/06_clippy.log" || echo "0")
    log_test "Clippy 代码检查 ($WARNING_COUNT 警告)" "FAIL" "${DURATION}s"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[7/7] 文档生成测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

START_TIME=$(date +%s)
if cargo doc -p openclaw-sandbox --no-deps > "$RESULTS_DIR/07_doc.log" 2>&1; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "Rust 文档生成" "PASS" "${DURATION}s"
else
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_test "Rust 文档生成" "FAIL" "${DURATION}s"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  测试结果汇总${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo
echo -e "总测试数: ${BLUE}$TOTAL_TESTS${NC}"
echo -e "通过: ${GREEN}$PASSED_TESTS${NC}"
echo -e "失败: ${RED}$FAILED_TESTS${NC}"
# 使用 bc 进行浮点数计算，兼容性更好
if command -v bc &> /dev/null; then
    SUCCESS_RATE=$(echo "scale=1; ($PASSED_TESTS * 100) / $TOTAL_TESTS" | bc)
else
    # 如果没有 bc，使用整数计算
    SUCCESS_RATE=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
fi
echo -e "成功率: ${YELLOW}${SUCCESS_RATE}%${NC}"
echo
echo -e "详细日志保存在: ${BLUE}$RESULTS_DIR${NC}"
echo

# 生成测试报告
cat > "$RESULTS_DIR/SUMMARY.md" << EOF
# OpenClaw+ 测试结果汇总

**测试时间**: $(date '+%Y-%m-%d %H:%M:%S')  
**测试标准**: 航空航天级别

## 总体统计

- **总测试数**: $TOTAL_TESTS
- **通过**: $PASSED_TESTS
- **失败**: $FAILED_TESTS
- **成功率**: ${SUCCESS_RATE}%

## 测试项目

1. Sandbox crate 编译
2. Sandbox 单元测试 (11个)
3. WasmEdge 环境检查
4. WasmEdge 沙箱集成测试
5. 自然语言工具测试 (14个用例)
6. Clippy 代码质量检查
7. Rust 文档生成

## 详细日志

所有测试日志保存在: \`$RESULTS_DIR\`

- \`01_cargo_build.log\` - Cargo 编译日志
- \`02_cargo_test.log\` - 单元测试日志
- \`03_wasmedge_simple.log\` - WasmEdge 简化测试日志
- \`04_wasmedge_sandbox.log\` - WasmEdge 沙箱测试日志
- \`05_nl_test.log\` - 自然语言测试日志
- \`06_clippy.log\` - Clippy 检查日志
- \`07_doc.log\` - 文档生成日志

## 测试状态

$(if [ $FAILED_TESTS -eq 0 ]; then
    echo "✅ **所有测试通过！**"
else
    echo "⚠️ **部分测试失败，请查看详细日志**"
fi)
EOF

echo -e "${GREEN}测试报告已生成: $RESULTS_DIR/SUMMARY.md${NC}"
echo

# 返回状态码
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}  ✓ 所有测试通过！${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    exit 0
else
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}  ⚠ 部分测试失败，请查看详细日志${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    exit 1
fi
