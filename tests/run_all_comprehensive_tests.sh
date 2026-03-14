#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 完整测试套件运行器
# 运行所有测试并生成综合报告
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_test() { echo -e "${CYAN}[TEST]${NC}  $*"; }
log_section() { echo -e "${MAGENTA}[====]${NC} $*"; }

REPORT_FILE="${PROJECT_ROOT}/ALL_TESTS_REPORT_$(date +%Y%m%d_%H%M%S).txt"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 完整测试套件"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "  报告文件: ${REPORT_FILE}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 初始化报告
cat > "${REPORT_FILE}" << EOF
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
OpenClaw+ 完整测试报告
生成时间: $(date '+%Y-%m-%d %H:%M:%S')
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

EOF

# 测试统计
TOTAL_TEST_SUITES=0
PASSED_TEST_SUITES=0
FAILED_TEST_SUITES=0

# ============================================================================
# 测试 1: 真实功能综合测试
# ============================================================================
log_section "运行真实功能综合测试..."
echo ""

if [ -f "${SCRIPT_DIR}/comprehensive_test.sh" ]; then
    TOTAL_TEST_SUITES=$((TOTAL_TEST_SUITES + 1))
    
    log_info "执行 comprehensive_test.sh"
    if bash "${SCRIPT_DIR}/comprehensive_test.sh" >> "${REPORT_FILE}" 2>&1; then
        log_ok "✅ 真实功能综合测试通过"
        PASSED_TEST_SUITES=$((PASSED_TEST_SUITES + 1))
    else
        log_error "❌ 真实功能综合测试失败"
        FAILED_TEST_SUITES=$((FAILED_TEST_SUITES + 1))
    fi
else
    log_error "comprehensive_test.sh 不存在"
fi

echo ""

# ============================================================================
# 测试 2: 数字员工对话测试
# ============================================================================
log_section "运行数字员工对话测试..."
echo ""

if [ -f "${SCRIPT_DIR}/test_agent_conversation.sh" ]; then
    TOTAL_TEST_SUITES=$((TOTAL_TEST_SUITES + 1))
    
    log_info "执行 test_agent_conversation.sh"
    if bash "${SCRIPT_DIR}/test_agent_conversation.sh" >> "${REPORT_FILE}" 2>&1; then
        log_ok "✅ 数字员工对话测试通过"
        PASSED_TEST_SUITES=$((PASSED_TEST_SUITES + 1))
    else
        log_error "❌ 数字员工对话测试失败"
        FAILED_TEST_SUITES=$((FAILED_TEST_SUITES + 1))
    fi
else
    log_error "test_agent_conversation.sh 不存在"
fi

echo ""

# ============================================================================
# 测试 3: 性能压力测试
# ============================================================================
log_section "运行性能压力测试..."
echo ""

if [ -f "${SCRIPT_DIR}/test_performance.sh" ]; then
    TOTAL_TEST_SUITES=$((TOTAL_TEST_SUITES + 1))
    
    log_info "执行 test_performance.sh"
    if bash "${SCRIPT_DIR}/test_performance.sh" >> "${REPORT_FILE}" 2>&1; then
        log_ok "✅ 性能压力测试通过"
        PASSED_TEST_SUITES=$((PASSED_TEST_SUITES + 1))
    else
        log_error "❌ 性能压力测试失败"
        FAILED_TEST_SUITES=$((FAILED_TEST_SUITES + 1))
    fi
else
    log_error "test_performance.sh 不存在"
fi

echo ""

# ============================================================================
# 测试 4: 沙箱安全测试
# ============================================================================
log_section "运行沙箱安全测试..."
echo ""

if [ -f "${SCRIPT_DIR}/test_sandbox_security.sh" ]; then
    TOTAL_TEST_SUITES=$((TOTAL_TEST_SUITES + 1))
    
    log_info "执行 test_sandbox_security.sh"
    if bash "${SCRIPT_DIR}/test_sandbox_security.sh" >> "${REPORT_FILE}" 2>&1; then
        log_ok "✅ 沙箱安全测试通过"
        PASSED_TEST_SUITES=$((PASSED_TEST_SUITES + 1))
    else
        log_error "❌ 沙箱安全测试失败"
        FAILED_TEST_SUITES=$((FAILED_TEST_SUITES + 1))
    fi
else
    log_error "test_sandbox_security.sh 不存在"
fi

echo ""

# ============================================================================
# 测试 5: 插件网关测试
# ============================================================================
log_section "运行插件网关测试..."
echo ""

if [ -f "${SCRIPT_DIR}/test_plugin_gateway.sh" ]; then
    TOTAL_TEST_SUITES=$((TOTAL_TEST_SUITES + 1))
    
    log_info "执行 test_plugin_gateway.sh"
    if bash "${SCRIPT_DIR}/test_plugin_gateway.sh" >> "${REPORT_FILE}" 2>&1; then
        log_ok "✅ 插件网关测试通过"
        PASSED_TEST_SUITES=$((PASSED_TEST_SUITES + 1))
    else
        log_error "❌ 插件网关测试失败"
        FAILED_TEST_SUITES=$((FAILED_TEST_SUITES + 1))
    fi
else
    log_error "test_plugin_gateway.sh 不存在"
fi

echo ""

# ============================================================================
# 生成最终报告
# ============================================================================
log_section "生成最终测试报告..."
echo ""

cat >> "${REPORT_FILE}" << EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
最终测试统计
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

测试套件总数: ${TOTAL_TEST_SUITES}
通过套件数:   ${PASSED_TEST_SUITES}
失败套件数:   ${FAILED_TEST_SUITES}
成功率:       $((PASSED_TEST_SUITES * 100 / TOTAL_TEST_SUITES))%

测试完成时间: $(date '+%Y-%m-%d %H:%M:%S')

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  测试套件执行完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "测试套件统计:"
echo "  总数: ${TOTAL_TEST_SUITES}"
echo "  通过: ${PASSED_TEST_SUITES}"
echo "  失败: ${FAILED_TEST_SUITES}"
echo "  成功率: $((PASSED_TEST_SUITES * 100 / TOTAL_TEST_SUITES))%"
echo ""
echo "详细报告已保存到: ${REPORT_FILE}"
echo ""

if [ "${FAILED_TEST_SUITES}" -eq 0 ]; then
    log_ok "🎉 所有测试套件通过！"
    exit 0
else
    log_error "⚠️  部分测试套件失败，请查看报告"
    exit 1
fi
