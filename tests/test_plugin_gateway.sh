#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 插件网关和技能系统测试
# 测试插件 API、技能执行、技能注册等
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_test() { echo -e "${CYAN}[TEST]${NC}  $*"; }

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 插件网关和技能系统测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# ============================================================================
# 测试 1: 插件网关 Rust 模块测试
# ============================================================================
log_test "测试插件网关 Rust 模块..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "${PROJECT_ROOT}"

log_info "运行 openclaw-plugin-gateway 单元测试"
if cargo test -p openclaw-plugin-gateway --lib 2>&1 | tail -20 | grep -q "test result: ok"; then
    log_ok "openclaw-plugin-gateway 测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "openclaw-plugin-gateway 测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 2: 技能文档和示例
# ============================================================================
log_test "检查技能文档和示例..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

SKILLS_GUIDE="${PROJECT_ROOT}/assets/openclaw/SKILLS_GUIDE.md"
SDK_DIR="${PROJECT_ROOT}/assets/openclaw/sdk"
EXAMPLES_DIR="${PROJECT_ROOT}/assets/openclaw/examples"

log_info "检查技能指南文档"
if [ -f "${SKILLS_GUIDE}" ]; then
    log_ok "技能指南文档存在"
    
    # 检查文档内容
    if grep -q "Skill" "${SKILLS_GUIDE}"; then
        log_ok "  ✓ 文档包含技能说明"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "  ✗ 文档内容不完整"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
else
    log_error "技能指南文档不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

log_info "检查 SDK 目录"
if [ -d "${SDK_DIR}" ]; then
    log_ok "SDK 目录存在"
    
    # 检查 skills.js
    if [ -f "${SDK_DIR}/skills.js" ]; then
        log_ok "  ✓ skills.js 存在"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "  ✗ skills.js 不存在"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
else
    log_error "SDK 目录不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

log_info "检查示例目录"
if [ -d "${EXAMPLES_DIR}" ]; then
    EXAMPLE_COUNT=$(ls -1 "${EXAMPLES_DIR}"/*_skills.js 2>/dev/null | wc -l | tr -d ' ')
    log_ok "示例目录存在，找到 ${EXAMPLE_COUNT} 个技能示例文件"
    
    if [ "${EXAMPLE_COUNT}" -gt 0 ]; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
else
    log_error "示例目录不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

echo ""

# ============================================================================
# 测试 3: 技能分类检查
# ============================================================================
log_test "检查技能分类..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "${EXAMPLES_DIR}" ]; then
    log_info "检查各类技能示例文件"
    
    SKILL_CATEGORIES=(
        "fs_skills.js:文件系统技能"
        "exec_skills.js:执行技能"
        "web_skills.js:网络技能"
        "agent_skills.js:代理技能"
        "cron_skills.js:定时任务技能"
        "sessions_skills.js:会话技能"
        "messaging_skills.js:消息技能"
    )
    
    for category in "${SKILL_CATEGORIES[@]}"; do
        filename=$(echo "${category}" | cut -d':' -f1)
        desc=$(echo "${category}" | cut -d':' -f2)
        
        if [ -f "${EXAMPLES_DIR}/${filename}" ]; then
            log_ok "  ✓ ${desc} (${filename})"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            log_error "  ✗ ${desc} (${filename}) 缺失"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    done
fi

echo ""

# ============================================================================
# 测试 4: 代码质量检查
# ============================================================================
log_test "检查代码质量..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查插件网关源代码"
PLUGIN_SRC="${PROJECT_ROOT}/crates/plugin/src"

if [ -d "${PLUGIN_SRC}" ]; then
    # 检查关键文件
    KEY_FILES=(
        "router.rs:路由模块"
        "skill_registry.rs:技能注册表"
        "lib.rs:库入口"
    )
    
    for file_info in "${KEY_FILES[@]}"; do
        filename=$(echo "${file_info}" | cut -d':' -f1)
        desc=$(echo "${file_info}" | cut -d':' -f2)
        
        if [ -f "${PLUGIN_SRC}/${filename}" ]; then
            log_ok "  ✓ ${desc} (${filename})"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            log_error "  ✗ ${desc} (${filename}) 缺失"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    done
else
    log_error "插件网关源代码目录不存在"
    FAILED_TESTS=$((FAILED_TESTS + 3))
    TOTAL_TESTS=$((TOTAL_TESTS + 3))
fi

echo ""

# ============================================================================
# 测试 5: 编译检查
# ============================================================================
log_test "检查编译状态..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "${PROJECT_ROOT}"

log_info "检查 openclaw-plugin-gateway 编译"
if cargo check -p openclaw-plugin-gateway 2>&1 | grep -q "Finished"; then
    log_ok "openclaw-plugin-gateway 编译通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "openclaw-plugin-gateway 编译失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 生成测试报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  插件网关测试结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "测试类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "插件网关 Rust 模块: 1 个测试"
echo "技能文档和示例:     3 个测试"
echo "技能分类检查:       7 个测试"
echo "代码质量检查:       3 个测试"
echo "编译检查:           1 个测试"

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
    log_ok "✅ 所有插件网关测试通过！"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    echo "⚠️  部分插件网关测试失败"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
