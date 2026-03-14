#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 综合功能测试套件
# 测试网页搜集、邮件、文件操作等所有功能
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TEST_DIR="${PROJECT_ROOT}/tests"
WORKSPACE_DIR="${HOME}/.openclaw-plus/workspace"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_test() { echo -e "${CYAN}[TEST]${NC}  $*"; }

# 创建测试报告文件
REPORT_FILE="${PROJECT_ROOT}/test_report_$(date +%Y%m%d_%H%M%S).txt"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee "${REPORT_FILE}"
echo "  OpenClaw+ 综合功能测试套件" | tee -a "${REPORT_FILE}"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')" | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"
echo "" | tee -a "${REPORT_FILE}"

# 检查前置条件
log_info "检查测试环境..." | tee -a "${REPORT_FILE}"

# 1. 检查 Ollama 服务
if ! curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    log_warn "Ollama 服务未运行，正在启动..." | tee -a "${REPORT_FILE}"
    "${SCRIPT_DIR}/start-ollama.sh" >/dev/null 2>&1
    sleep 3
fi
log_ok "Ollama 服务运行正常" | tee -a "${REPORT_FILE}"

# 2. 创建测试工作目录
mkdir -p "${WORKSPACE_DIR}/test_files"
log_ok "测试目录已创建" | tee -a "${REPORT_FILE}"

# 3. 检查 WasmEdge
if ! command -v wasmedge &>/dev/null; then
    log_error "WasmEdge 未安装" | tee -a "${REPORT_FILE}"
    exit 1
fi
log_ok "WasmEdge 已安装" | tee -a "${REPORT_FILE}"

echo "" | tee -a "${REPORT_FILE}"

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 测试 1: 网页信息搜集
log_test "运行网页信息搜集测试..." | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"

if [ -f "${TEST_DIR}/test_web_scraping.js" ]; then
    cd "${PROJECT_ROOT}"
    
    # 使用 WasmEdge 运行测试
    if wasmedge --dir /workspace:"${WORKSPACE_DIR}" \
                --env OPENCLAW_TEST=1 \
                /opt/homebrew/Cellar/wasmedge/0.14.1/lib/wasmedge/libwasmedge_quickjs.dylib \
                "${TEST_DIR}/test_web_scraping.js" 2>&1 | tee -a "${REPORT_FILE}"; then
        log_ok "网页搜集测试完成" | tee -a "${REPORT_FILE}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "网页搜集测试失败" | tee -a "${REPORT_FILE}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
else
    log_warn "网页搜集测试脚本不存在" | tee -a "${REPORT_FILE}"
fi

echo "" | tee -a "${REPORT_FILE}"

# 测试 2: 文件操作
log_test "运行文件操作测试..." | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"

if [ -f "${TEST_DIR}/test_file_operations.js" ]; then
    cd "${PROJECT_ROOT}"
    
    if wasmedge --dir /workspace:"${WORKSPACE_DIR}" \
                --env OPENCLAW_TEST=1 \
                /opt/homebrew/Cellar/wasmedge/0.14.1/lib/wasmedge/libwasmedge_quickjs.dylib \
                "${TEST_DIR}/test_file_operations.js" 2>&1 | tee -a "${REPORT_FILE}"; then
        log_ok "文件操作测试完成" | tee -a "${REPORT_FILE}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "文件操作测试失败" | tee -a "${REPORT_FILE}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
else
    log_warn "文件操作测试脚本不存在" | tee -a "${REPORT_FILE}"
fi

echo "" | tee -a "${REPORT_FILE}"

# 测试 3: 邮件功能（模拟）
log_test "运行邮件功能测试..." | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"

if [ -f "${TEST_DIR}/test_email.js" ]; then
    cd "${PROJECT_ROOT}"
    
    if node "${TEST_DIR}/test_email.js" 2>&1 | tee -a "${REPORT_FILE}"; then
        log_ok "邮件功能测试完成" | tee -a "${REPORT_FILE}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "邮件功能测试失败" | tee -a "${REPORT_FILE}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
else
    log_warn "邮件功能测试脚本不存在" | tee -a "${REPORT_FILE}"
fi

echo "" | tee -a "${REPORT_FILE}"

# 测试 4: 数字员工配置
log_test "验证数字员工配置..." | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"

AGENT_COUNT=$(ls -1 "${PROJECT_ROOT}/agents"/*.toml 2>/dev/null | wc -l | tr -d ' ')
if [ "${AGENT_COUNT}" -gt 0 ]; then
    log_ok "找到 ${AGENT_COUNT} 个数字员工配置" | tee -a "${REPORT_FILE}"
    
    for agent_file in "${PROJECT_ROOT}/agents"/*.toml; do
        agent_name=$(basename "${agent_file}" .toml)
        agent_display=$(grep "display_name" "${agent_file}" | cut -d'"' -f2)
        echo "  ✓ ${agent_name}: ${agent_display}" | tee -a "${REPORT_FILE}"
    done
    
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "未找到数字员工配置" | tee -a "${REPORT_FILE}"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo "" | tee -a "${REPORT_FILE}"

# 测试 5: AI 推理功能
log_test "测试 AI 推理功能..." | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"

TEST_RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "简单介绍一下你自己，不超过20个字。",
  "stream": false
}' 2>&1)

if echo "${TEST_RESPONSE}" | grep -q '"response"'; then
    RESPONSE_TEXT=$(echo "${TEST_RESPONSE}" | grep -o '"response":"[^"]*"' | cut -d'"' -f4 | head -c 50)
    log_ok "AI 推理测试成功" | tee -a "${REPORT_FILE}"
    echo "  响应: ${RESPONSE_TEXT}..." | tee -a "${REPORT_FILE}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "AI 推理测试失败" | tee -a "${REPORT_FILE}"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo "" | tee -a "${REPORT_FILE}"

# 生成最终测试报告
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"
echo "  测试结果汇总" | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"
echo "" | tee -a "${REPORT_FILE}"

echo "总测试数: ${TOTAL_TESTS}" | tee -a "${REPORT_FILE}"
echo "通过: ${PASSED_TESTS}" | tee -a "${REPORT_FILE}"
echo "失败: ${FAILED_TESTS}" | tee -a "${REPORT_FILE}"

if [ "${FAILED_TESTS}" -eq 0 ]; then
    echo "" | tee -a "${REPORT_FILE}"
    log_ok "✅ 所有测试通过！" | tee -a "${REPORT_FILE}"
    SUCCESS_RATE=100
else
    echo "" | tee -a "${REPORT_FILE}"
    log_warn "⚠️  部分测试失败" | tee -a "${REPORT_FILE}"
    SUCCESS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
fi

echo "成功率: ${SUCCESS_RATE}%" | tee -a "${REPORT_FILE}"
echo "" | tee -a "${REPORT_FILE}"
echo "详细报告已保存到: ${REPORT_FILE}" | tee -a "${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a "${REPORT_FILE}"

# 返回退出码
if [ "${FAILED_TESTS}" -eq 0 ]; then
    exit 0
else
    exit 1
fi
