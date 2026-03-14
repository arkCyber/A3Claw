#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 沙箱安全功能测试
# 测试安全策略、权限控制、资源限制等
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
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_test() { echo -e "${CYAN}[TEST]${NC}  $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 沙箱安全功能测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# ============================================================================
# 测试 1: 安全配置验证
# ============================================================================
log_test "测试安全配置..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

CONFIG_PATH="${HOME}/.config/openclaw-plus/config.toml"

log_info "检查内存限制配置"
if grep -q "memory_limit" "${CONFIG_PATH}"; then
    MEMORY_LIMIT=$(grep "memory_limit" "${CONFIG_PATH}" | head -1 | awk '{print $3}')
    log_ok "内存限制配置存在: ${MEMORY_LIMIT}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "内存限制配置未找到"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查 shell 拦截配置"
if grep -q "intercept_shell" "${CONFIG_PATH}"; then
    SHELL_INTERCEPT=$(grep "intercept_shell" "${CONFIG_PATH}" | head -1 | awk '{print $3}')
    log_ok "Shell 拦截配置: ${SHELL_INTERCEPT}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "Shell 拦截配置未找到"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查网络白名单配置"
if grep -q "network_allowlist" "${CONFIG_PATH}"; then
    log_ok "网络白名单配置存在"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "网络白名单配置未找到"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查文件删除确认配置"
if grep -q "confirm_file_deletion" "${CONFIG_PATH}"; then
    CONFIRM_DELETE=$(grep "confirm_file_deletion" "${CONFIG_PATH}" | head -1 | awk '{print $3}')
    log_ok "文件删除确认配置: ${CONFIRM_DELETE}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "文件删除确认配置未找到"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 2: 数字员工安全边界
# ============================================================================
log_test "测试数字员工安全边界..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

for agent_file in "${PROJECT_ROOT}/agents"/*.toml; do
    agent_name=$(basename "${agent_file}" .toml)
    
    log_info "检查 ${agent_name} 的安全配置"
    
    # 检查内存限制
    if grep -q "memory_limit" "${agent_file}"; then
        log_ok "  ✓ ${agent_name}: 内存限制已配置"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ ${agent_name}: 缺少内存限制配置"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # 检查 shell 拦截
    if grep -q "intercept_shell" "${agent_file}"; then
        log_ok "  ✓ ${agent_name}: Shell 拦截已配置"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ ${agent_name}: 缺少 Shell 拦截配置"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # 检查网络白名单
    if grep -q "network_allowlist" "${agent_file}"; then
        log_ok "  ✓ ${agent_name}: 网络白名单已配置"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ ${agent_name}: 缺少网络白名单配置"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
done

echo ""

# ============================================================================
# 测试 3: Rust 安全模块测试
# ============================================================================
log_test "测试 Rust 安全模块..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "${PROJECT_ROOT}"

log_info "运行 openclaw-security 单元测试"
if cargo test -p openclaw-security --lib 2>&1 | tail -20 | grep -q "test result: ok"; then
    log_ok "openclaw-security 测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "openclaw-security 测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 4: 工作目录权限
# ============================================================================
log_test "测试工作目录权限..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

WORKSPACE_DIR="${HOME}/.openclaw-plus/workspace"

log_info "检查工作目录存在性"
if [ -d "${WORKSPACE_DIR}" ]; then
    log_ok "工作目录存在: ${WORKSPACE_DIR}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "工作目录不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查工作目录可写性"
TEST_FILE="${WORKSPACE_DIR}/security_test_$(date +%s).txt"
if echo "test" > "${TEST_FILE}" 2>/dev/null; then
    log_ok "工作目录可写"
    rm -f "${TEST_FILE}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "工作目录不可写"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查工作目录权限"
WORKSPACE_PERMS=$(stat -f "%OLp" "${WORKSPACE_DIR}" 2>/dev/null || stat -c "%a" "${WORKSPACE_DIR}" 2>/dev/null)
if [ -n "${WORKSPACE_PERMS}" ]; then
    log_ok "工作目录权限: ${WORKSPACE_PERMS}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "无法获取工作目录权限"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 5: 安全策略文档
# ============================================================================
log_test "检查安全策略文档..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查默认配置文件"
DEFAULT_CONFIG="${PROJECT_ROOT}/config/default.toml"
if [ -f "${DEFAULT_CONFIG}" ]; then
    log_ok "默认配置文件存在"
    
    # 检查关键安全配置
    if grep -q "\[security\]" "${DEFAULT_CONFIG}"; then
        log_ok "  ✓ 安全配置段存在"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ 安全配置段缺失"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
else
    log_error "默认配置文件不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

echo ""

# ============================================================================
# 生成测试报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  安全测试结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "测试类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "安全配置验证:     4 个测试"
echo "数字员工安全边界: 15 个测试 (5 个员工 × 3 项配置)"
echo "Rust 安全模块:    1 个测试"
echo "工作目录权限:     3 个测试"
echo "安全策略文档:     1 个测试"

echo ""
echo "总体统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "总测试数: ${TOTAL_TESTS}"
echo "通过: ${PASSED_TESTS}"
echo "失败: ${FAILED_TESTS}"

SUCCESS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
echo "成功率: ${SUCCESS_RATE}%"

echo ""
if [ "${FAILED_TESTS}" -eq 0 ]; then
    log_ok "✅ 所有安全测试通过！"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    log_warn "⚠️  部分安全测试失败"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
