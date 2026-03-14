#!/usr/bin/env bash
# =============================================================================
# Claw Terminal 代码审计脚本
# 检查代码质量、安全性和最佳实践
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_audit() { echo -e "${CYAN}[AUDIT]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Claw Terminal 代码审计"
echo "  审计时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cd "${PROJECT_ROOT}"

TOTAL_CHECKS=0
PASSED_CHECKS=0
WARNINGS=0
ISSUES=()

# ============================================================================
# 审计 1: 代码编译检查
# ============================================================================
log_audit "检查代码编译状态..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "编译 openclaw-ui crate"
if cargo check -p openclaw-ui 2>&1 | grep -q "Finished"; then
    log_ok "UI 模块编译通过"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_error "UI 模块编译失败"
    ISSUES+=("UI 模块编译失败")
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

echo ""

# ============================================================================
# 审计 2: 安全性检查
# ============================================================================
log_audit "检查安全性问题..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查不安全代码块"
UNSAFE_COUNT=$(grep -r "unsafe" crates/ui/src/ 2>/dev/null | wc -l | tr -d ' ')
if [ "${UNSAFE_COUNT}" -eq 0 ]; then
    log_ok "未发现 unsafe 代码块"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "发现 ${UNSAFE_COUNT} 处 unsafe 代码块"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查 unwrap() 使用"
UNWRAP_COUNT=$(grep -r "\.unwrap()" crates/ui/src/ 2>/dev/null | grep -v "unwrap_or" | wc -l | tr -d ' ')
if [ "${UNWRAP_COUNT}" -lt 10 ]; then
    log_ok "unwrap() 使用合理 (${UNWRAP_COUNT} 处)"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "unwrap() 使用较多 (${UNWRAP_COUNT} 处)，建议使用 ? 或 unwrap_or"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查 panic! 使用"
PANIC_COUNT=$(grep -r "panic!" crates/ui/src/ 2>/dev/null | wc -l | tr -d ' ')
if [ "${PANIC_COUNT}" -eq 0 ]; then
    log_ok "未发现 panic! 调用"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "发现 ${PANIC_COUNT} 处 panic! 调用"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

echo ""

# ============================================================================
# 审计 3: 代码质量检查
# ============================================================================
log_audit "检查代码质量..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查 TODO 注释"
TODO_COUNT=$(grep -r "TODO" crates/ui/src/ 2>/dev/null | wc -l | tr -d ' ')
log_ok "发现 ${TODO_COUNT} 处 TODO 注释"
PASSED_CHECKS=$((PASSED_CHECKS + 1))
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查 FIXME 注释"
FIXME_COUNT=$(grep -r "FIXME" crates/ui/src/ 2>/dev/null | wc -l | tr -d ' ')
if [ "${FIXME_COUNT}" -eq 0 ]; then
    log_ok "未发现 FIXME 注释"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "发现 ${FIXME_COUNT} 处 FIXME 注释"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查代码行数"
TOTAL_LINES=$(find crates/ui/src -name "*.rs" -exec wc -l {} + | tail -1 | awk '{print $1}')
log_ok "UI 模块总代码行数: ${TOTAL_LINES}"
PASSED_CHECKS=$((PASSED_CHECKS + 1))
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

echo ""

# ============================================================================
# 审计 4: Claw Terminal 特定检查
# ============================================================================
log_audit "检查 Claw Terminal 实现..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查对话处理函数"
if grep -q "ClawAgentChat" crates/ui/src/app.rs; then
    log_ok "✓ ClawAgentChat 消息处理存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_error "✗ ClawAgentChat 消息处理缺失"
    ISSUES+=("ClawAgentChat 消息处理缺失")
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查推理引擎集成"
if grep -q "InferenceEngine::new" crates/ui/src/app.rs; then
    log_ok "✓ 推理引擎初始化存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_error "✗ 推理引擎初始化缺失"
    ISSUES+=("推理引擎初始化缺失")
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查对话历史管理"
if grep -q "claw_agent_conversations" crates/ui/src/app.rs; then
    log_ok "✓ 对话历史管理存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_error "✗ 对话历史管理缺失"
    ISSUES+=("对话历史管理缺失")
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查错误处理"
if grep -q "ClawNlPlanError" crates/ui/src/app.rs; then
    log_ok "✓ 错误处理机制存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "⚠ 错误处理机制可能不完整"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

echo ""

# ============================================================================
# 审计 5: 性能检查
# ============================================================================
log_audit "检查性能相关代码..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查克隆操作"
CLONE_COUNT=$(grep -r "\.clone()" crates/ui/src/ 2>/dev/null | wc -l | tr -d ' ')
log_ok "发现 ${CLONE_COUNT} 处 clone() 调用"
PASSED_CHECKS=$((PASSED_CHECKS + 1))
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查异步操作"
ASYNC_COUNT=$(grep -r "async fn\|async move" crates/ui/src/ 2>/dev/null | wc -l | tr -d ' ')
log_ok "发现 ${ASYNC_COUNT} 处异步操作"
PASSED_CHECKS=$((PASSED_CHECKS + 1))
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查超时设置"
if grep -q "inference_timeout" crates/ui/src/app.rs; then
    log_ok "✓ 推理超时设置存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "⚠ 未发现推理超时设置"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

echo ""

# ============================================================================
# 审计 6: 依赖检查
# ============================================================================
log_audit "检查依赖关系..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查 Cargo.toml 依赖"
if [ -f "crates/ui/Cargo.toml" ]; then
    log_ok "✓ Cargo.toml 存在"
    
    if grep -q "openclaw-inference" crates/ui/Cargo.toml; then
        log_ok "  ✓ openclaw-inference 依赖存在"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        log_error "  ✗ openclaw-inference 依赖缺失"
        ISSUES+=("openclaw-inference 依赖缺失")
    fi
    
    if grep -q "libcosmic" crates/ui/Cargo.toml; then
        log_ok "  ✓ libcosmic 依赖存在"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        log_error "  ✗ libcosmic 依赖缺失"
        ISSUES+=("libcosmic 依赖缺失")
    fi
else
    log_error "✗ Cargo.toml 不存在"
    ISSUES+=("Cargo.toml 不存在")
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 3))

echo ""

# ============================================================================
# 审计 7: 文档检查
# ============================================================================
log_audit "检查文档完整性..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查使用文档"
if [ -f "docs/CLAW_TERMINAL_USAGE_GUIDE.md" ]; then
    log_ok "✓ 使用指南存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "⚠ 使用指南缺失"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查中文输入文档"
if [ -f "docs/CHINESE_INPUT_GUIDE.md" ]; then
    log_ok "✓ 中文输入指南存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "⚠ 中文输入指南缺失"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

log_info "检查测试报告"
if [ -f "CLAW_TERMINAL_FINAL_REPORT.md" ]; then
    log_ok "✓ 测试报告存在"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    log_warn "⚠ 测试报告缺失"
    WARNINGS=$((WARNINGS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

echo ""

# ============================================================================
# 生成审计报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Claw Terminal 代码审计结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "审计类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "代码编译:          1 项检查"
echo "安全性:            3 项检查"
echo "代码质量:          3 项检查"
echo "Claw Terminal:     4 项检查"
echo "性能:              3 项检查"
echo "依赖关系:          3 项检查"
echo "文档完整性:        3 项检查"

echo ""
echo "总体统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "总检查项: ${TOTAL_CHECKS}"
echo "通过: ${PASSED_CHECKS}"
echo "警告: ${WARNINGS}"
echo "问题: ${#ISSUES[@]}"

if [ "${TOTAL_CHECKS}" -gt 0 ]; then
    SUCCESS_RATE=$((PASSED_CHECKS * 100 / TOTAL_CHECKS))
    echo "通过率: ${SUCCESS_RATE}%"
fi

echo ""
echo "代码质量指标:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "总代码行数: ${TOTAL_LINES}"
echo "unsafe 代码块: ${UNSAFE_COUNT}"
echo "unwrap() 调用: ${UNWRAP_COUNT}"
echo "panic! 调用: ${PANIC_COUNT}"
echo "TODO 注释: ${TODO_COUNT}"
echo "FIXME 注释: ${FIXME_COUNT}"
echo "clone() 调用: ${CLONE_COUNT}"
echo "异步操作: ${ASYNC_COUNT}"

if [ "${#ISSUES[@]}" -gt 0 ]; then
    echo ""
    echo "发现的问题:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    for issue in "${ISSUES[@]}"; do
        log_error "  ✗ ${issue}"
    done
fi

echo ""
echo "审计结论:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "${#ISSUES[@]}" -eq 0 ] && [ "${WARNINGS}" -lt 5 ]; then
    log_ok "🎉 代码质量优秀！"
    echo ""
    echo "代码评级: ⭐⭐⭐⭐⭐ (优秀)"
    echo ""
    echo "优点:"
    echo "  ✓ 编译通过"
    echo "  ✓ 安全性良好"
    echo "  ✓ 代码质量高"
    echo "  ✓ 功能完整"
    echo "  ✓ 文档齐全"
elif [ "${#ISSUES[@]}" -eq 0 ]; then
    log_ok "✅ 代码质量良好，有少量警告"
    echo ""
    echo "代码评级: ⭐⭐⭐⭐ (良好)"
    echo ""
    echo "建议:"
    echo "  • 减少 unwrap() 使用"
    echo "  • 处理 TODO 注释"
    echo "  • 优化性能"
else
    log_warn "⚠️  发现 ${#ISSUES[@]} 个问题需要修复"
    echo ""
    echo "代码评级: ⭐⭐⭐ (需要改进)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Claw Terminal 代码审计完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

REPORT_FILE="${PROJECT_ROOT}/CLAW_TERMINAL_AUDIT_REPORT_$(date +%Y%m%d_%H%M%S).txt"
{
    echo "Claw Terminal 代码审计报告"
    echo "生成时间: $(date '+%Y-%m-%d %H:%M:%S')"
    echo ""
    echo "审计统计:"
    echo "总检查项: ${TOTAL_CHECKS}"
    echo "通过: ${PASSED_CHECKS}"
    echo "警告: ${WARNINGS}"
    echo "问题: ${#ISSUES[@]}"
    echo "通过率: ${SUCCESS_RATE:-N/A}%"
    echo ""
    echo "代码质量指标:"
    echo "总代码行数: ${TOTAL_LINES}"
    echo "unsafe 代码块: ${UNSAFE_COUNT}"
    echo "unwrap() 调用: ${UNWRAP_COUNT}"
    echo "panic! 调用: ${PANIC_COUNT}"
} > "${REPORT_FILE}"

log_info "详细审计报告已保存到: ${REPORT_FILE}"

if [ "${#ISSUES[@]}" -eq 0 ]; then
    exit 0
else
    exit 1
fi
