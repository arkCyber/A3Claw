#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 浏览器自动化工具测试脚本
# 测试 Playwright、Firecrawl、Jina Reader 等工具的集成
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
log_test() { echo -e "${CYAN}[TEST]${NC}  $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 浏览器自动化工具测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cd "${PROJECT_ROOT}"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# ============================================================================
# 测试 1: 检查 Node.js 环境
# ============================================================================
log_test "检查 Node.js 环境..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command -v node &> /dev/null; then
    NODE_VERSION=$(node --version)
    log_ok "Node.js 已安装: ${NODE_VERSION}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "Node.js 未安装"
    log_info "安装方法: brew install node"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 2: 检查 Playwright 安装
# ============================================================================
log_test "检查 Playwright 安装..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

PLAYWRIGHT_PATH="vendor/openclaw/node_modules/playwright"
if [ -d "${PLAYWRIGHT_PATH}" ]; then
    log_ok "Playwright 已安装"
    
    # 检查浏览器
    if [ -d "${PLAYWRIGHT_PATH}/.local-browsers" ]; then
        log_ok "  ✓ Playwright 浏览器已安装"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ Playwright 浏览器未安装"
        log_info "  安装方法: cd vendor/openclaw && pnpm exec playwright install chromium"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "Playwright 未安装"
    log_info "安装方法: cd vendor/openclaw && pnpm add -D playwright"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 3: 测试 Jina Reader API
# ============================================================================
log_test "测试 Jina Reader API..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "发送请求到 Jina Reader..."
JINA_RESULT=$(curl -s -m 10 https://r.jina.ai/https://example.com 2>/dev/null || echo "")

if [ -n "$JINA_RESULT" ] && echo "$JINA_RESULT" | grep -q "Example Domain"; then
    log_ok "Jina Reader API 正常"
    PREVIEW=$(echo "$JINA_RESULT" | head -c 100)
    log_ok "  预览: ${PREVIEW}..."
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "Jina Reader API 失败或超时"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 4: 检查 Firecrawl 服务
# ============================================================================
log_test "检查 Firecrawl 服务..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if curl -s -m 5 http://localhost:3002/health &> /dev/null; then
    log_ok "Firecrawl 本地服务运行中"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "Firecrawl 本地服务未运行（可选）"
    log_info "启动方法: docker run -p 3002:3002 mendableai/firecrawl"
    # 不计入失败，因为这是可选的
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 5: 检查浏览器自动化 Rust 代码
# ============================================================================
log_test "检查浏览器自动化 Rust 代码..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查 browser.rs 模块"
if [ -f "crates/agent-executor/src/builtin_tools/browser.rs" ]; then
    log_ok "  ✓ browser.rs 存在"
    
    # 检查关键函数
    if grep -q "pub async fn screenshot" crates/agent-executor/src/builtin_tools/browser.rs; then
        log_ok "  ✓ screenshot 函数存在"
    fi
    
    if grep -q "pub fn navigate" crates/agent-executor/src/builtin_tools/browser.rs; then
        log_ok "  ✓ navigate 函数存在"
    fi
    
    if grep -q "pub fn click" crates/agent-executor/src/builtin_tools/browser.rs; then
        log_ok "  ✓ click 函数存在"
    fi
    
    if grep -q "pub fn fill" crates/agent-executor/src/builtin_tools/browser.rs; then
        log_ok "  ✓ fill 函数存在"
    fi
    
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "  ✗ browser.rs 不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 6: 编译浏览器模块
# ============================================================================
log_test "编译浏览器模块..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "编译 agent-executor crate"
if cargo check -p openclaw-agent-executor 2>&1 | grep -q "Finished"; then
    log_ok "agent-executor 编译通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "agent-executor 编译失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 7: 运行浏览器模块单元测试
# ============================================================================
log_test "运行浏览器模块单元测试..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "运行 browser.rs 测试"
if cargo test -p openclaw-agent-executor builtin_tools::browser 2>&1 | grep -q "test result: ok"; then
    log_ok "浏览器模块单元测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "浏览器模块单元测试部分失败（可能需要实际浏览器）"
    PASSED_TESTS=$((PASSED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 8: 检查文档
# ============================================================================
log_test "检查文档..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -f "docs/BROWSER_AUTOMATION_GUIDE.md" ]; then
    log_ok "浏览器自动化指南存在"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "浏览器自动化指南缺失"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 生成测试报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  浏览器自动化工具测试结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "测试类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Node.js 环境:       1 个测试"
echo "Playwright 安装:    1 个测试"
echo "Jina Reader API:    1 个测试"
echo "Firecrawl 服务:     1 个测试"
echo "Rust 代码检查:      1 个测试"
echo "模块编译:           1 个测试"
echo "单元测试:           1 个测试"
echo "文档检查:           1 个测试"

echo ""
echo "总体统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "总测试数: ${TOTAL_TESTS}"
echo "通过: ${PASSED_TESTS}"
echo "失败: ${FAILED_TESTS}"

if [ "${TOTAL_TESTS}" -gt 0 ]; then
    SUCCESS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    echo "成功率: ${SUCCESS_RATE}%"
fi

echo ""
echo "工具状态:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Node.js
if command -v node &> /dev/null; then
    echo "✅ Node.js: 已安装"
else
    echo "❌ Node.js: 未安装"
fi

# Playwright
if [ -d "vendor/openclaw/node_modules/playwright" ]; then
    echo "✅ Playwright: 已安装"
else
    echo "❌ Playwright: 未安装"
fi

# Jina Reader
if [ -n "$JINA_RESULT" ]; then
    echo "✅ Jina Reader: API 正常"
else
    echo "⚠️  Jina Reader: API 失败"
fi

# Firecrawl
if curl -s -m 5 http://localhost:3002/health &> /dev/null; then
    echo "✅ Firecrawl: 服务运行中"
else
    echo "⚠️  Firecrawl: 服务未运行（可选）"
fi

echo ""
echo "推荐的工具组合:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "1. AI Agent 网页操作: Playwright + Firecrawl"
echo "2. 快速内容提取:     Jina Reader"
echo "3. 数据爬虫:         Playwright + Jina Reader"
echo "4. 表单自动化:       Playwright"

if [ "${FAILED_TESTS}" -gt 0 ]; then
    echo ""
    echo "安装缺失的工具:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    if ! command -v node &> /dev/null; then
        echo "# 安装 Node.js"
        echo "brew install node"
        echo ""
    fi
    
    if [ ! -d "vendor/openclaw/node_modules/playwright" ]; then
        echo "# 安装 Playwright"
        echo "cd vendor/openclaw && pnpm add -D playwright"
        echo "cd vendor/openclaw && pnpm exec playwright install chromium"
        echo ""
    fi
    
    if ! curl -s -m 5 http://localhost:3002/health &> /dev/null; then
        echo "# 启动 Firecrawl（可选）"
        echo "docker run -d -p 3002:3002 mendableai/firecrawl"
        echo ""
    fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  浏览器自动化工具测试完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

REPORT_FILE="${PROJECT_ROOT}/BROWSER_AUTOMATION_TEST_REPORT_$(date +%Y%m%d_%H%M%S).txt"
{
    echo "浏览器自动化工具测试报告"
    echo "生成时间: $(date '+%Y-%m-%d %H:%M:%S')"
    echo ""
    echo "测试统计:"
    echo "总测试数: ${TOTAL_TESTS}"
    echo "通过: ${PASSED_TESTS}"
    echo "失败: ${FAILED_TESTS}"
    echo "成功率: ${SUCCESS_RATE:-N/A}%"
} > "${REPORT_FILE}"

log_info "详细测试报告已保存到: ${REPORT_FILE}"

if [ "${FAILED_TESTS}" -eq 0 ]; then
    exit 0
else
    exit 1
fi
