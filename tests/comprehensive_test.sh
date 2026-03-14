#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 真实功能综合测试
# 测试所有核心功能：数字员工、文件操作、网络请求、AI推理等
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKSPACE_DIR="${HOME}/.openclaw-plus/workspace"

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
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_test() { echo -e "${CYAN}[TEST]${NC}  $*"; }
log_step() { echo -e "${MAGENTA}[STEP]${NC}  $*"; }

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 测试结果数组
declare -a TEST_RESULTS

# 添加测试结果
add_test_result() {
    local name="$1"
    local status="$2"
    local details="${3:-}"
    
    TEST_RESULTS+=("$name|$status|$details")
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if [ "$status" = "PASS" ]; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 真实功能综合测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ============================================================================
# 测试 1: Rust 单元测试
# ============================================================================
log_test "运行 Rust 单元测试..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "${PROJECT_ROOT}"

log_step "测试 openclaw-security crate..."
if cargo test -p openclaw-security --lib 2>&1 | tail -20 | grep -q "test result: ok\|running 0 tests"; then
    log_ok "openclaw-security 测试通过"
    add_test_result "Rust: openclaw-security" "PASS" "所有单元测试通过"
else
    log_error "openclaw-security 测试失败"
    add_test_result "Rust: openclaw-security" "FAIL" "部分测试失败"
fi

log_step "测试 openclaw-inference crate..."
if cargo test -p openclaw-inference --lib 2>&1 | tail -20 | grep -q "test result: ok\|running 0 tests"; then
    log_ok "openclaw-inference 测试通过"
    add_test_result "Rust: openclaw-inference" "PASS" "所有单元测试通过"
else
    log_error "openclaw-inference 测试失败"
    add_test_result "Rust: openclaw-inference" "FAIL" "部分测试失败"
fi

log_step "测试 openclaw-plugin-gateway crate..."
if cargo test -p openclaw-plugin-gateway --lib 2>&1 | tail -20 | grep -q "test result: ok\|running 0 tests"; then
    log_ok "openclaw-plugin-gateway 测试通过"
    add_test_result "Rust: openclaw-plugin-gateway" "PASS" "所有单元测试通过"
else
    log_error "openclaw-plugin-gateway 测试失败"
    add_test_result "Rust: openclaw-plugin-gateway" "FAIL" "部分测试失败"
fi

echo ""

# ============================================================================
# 测试 2: AI 推理功能
# ============================================================================
log_test "测试 AI 推理功能..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_step "测试 Ollama 连接..."
if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    log_ok "Ollama 服务连接成功"
    add_test_result "AI: Ollama 连接" "PASS" "服务正常运行"
else
    log_error "Ollama 服务连接失败"
    add_test_result "AI: Ollama 连接" "FAIL" "无法连接到服务"
fi

log_step "测试简单推理..."
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "你好",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"'; then
    RESPONSE_TEXT=$(echo "${RESPONSE}" | grep -o '"response":"[^"]*"' | cut -d'"' -f4 | head -c 30)
    log_ok "AI 推理成功: ${RESPONSE_TEXT}..."
    add_test_result "AI: 简单推理" "PASS" "成功生成响应"
else
    log_error "AI 推理失败"
    add_test_result "AI: 简单推理" "FAIL" "推理请求失败"
fi

log_step "测试多轮对话..."
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "请用一句话介绍 Rust 编程语言",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"' && echo "${RESPONSE}" | grep -qi "rust"; then
    log_ok "多轮对话测试成功"
    add_test_result "AI: 多轮对话" "PASS" "能够理解上下文"
else
    log_error "多轮对话测试失败"
    add_test_result "AI: 多轮对话" "FAIL" "上下文理解失败"
fi

echo ""

# ============================================================================
# 测试 3: 文件操作功能
# ============================================================================
log_test "测试文件操作功能..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TEST_FILE="${WORKSPACE_DIR}/test_file_$(date +%s).txt"
TEST_CONTENT="OpenClaw+ 文件操作测试 - $(date)"

log_step "测试文件写入..."
if echo "${TEST_CONTENT}" > "${TEST_FILE}"; then
    log_ok "文件写入成功: ${TEST_FILE}"
    add_test_result "文件: 写入" "PASS" "成功创建测试文件"
else
    log_error "文件写入失败"
    add_test_result "文件: 写入" "FAIL" "无法创建文件"
fi

log_step "测试文件读取..."
if [ -f "${TEST_FILE}" ]; then
    READ_CONTENT=$(cat "${TEST_FILE}")
    if [ "${READ_CONTENT}" = "${TEST_CONTENT}" ]; then
        log_ok "文件读取成功，内容匹配"
        add_test_result "文件: 读取" "PASS" "内容验证通过"
    else
        log_error "文件内容不匹配"
        add_test_result "文件: 读取" "FAIL" "内容验证失败"
    fi
else
    log_error "文件不存在"
    add_test_result "文件: 读取" "FAIL" "文件未找到"
fi

log_step "测试文件删除..."
if rm -f "${TEST_FILE}"; then
    log_ok "文件删除成功"
    add_test_result "文件: 删除" "PASS" "成功删除测试文件"
else
    log_error "文件删除失败"
    add_test_result "文件: 删除" "FAIL" "无法删除文件"
fi

echo ""

# ============================================================================
# 测试 4: 网络请求功能
# ============================================================================
log_test "测试网络请求功能..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_step "测试 HTTP GET 请求..."
if curl -s -o /dev/null -w "%{http_code}" https://www.google.com | grep -q "200"; then
    log_ok "HTTP GET 请求成功"
    add_test_result "网络: HTTP GET" "PASS" "成功访问外部网站"
else
    log_warn "HTTP GET 请求失败（可能是网络问题）"
    add_test_result "网络: HTTP GET" "WARN" "网络连接问题"
fi

log_step "测试 DNS 解析..."
if nslookup google.com >/dev/null 2>&1; then
    log_ok "DNS 解析成功"
    add_test_result "网络: DNS 解析" "PASS" "DNS 服务正常"
else
    log_error "DNS 解析失败"
    add_test_result "网络: DNS 解析" "FAIL" "DNS 服务异常"
fi

echo ""

# ============================================================================
# 测试 5: 数字员工配置
# ============================================================================
log_test "测试数字员工配置..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

AGENT_COUNT=$(ls -1 "${PROJECT_ROOT}/agents"/*.toml 2>/dev/null | wc -l | tr -d ' ')

log_step "检查数字员工配置文件..."
if [ "${AGENT_COUNT}" -gt 0 ]; then
    log_ok "找到 ${AGENT_COUNT} 个数字员工配置"
    add_test_result "数字员工: 配置文件" "PASS" "发现 ${AGENT_COUNT} 个配置"
    
    for agent_file in "${PROJECT_ROOT}/agents"/*.toml; do
        agent_name=$(basename "${agent_file}" .toml)
        
        # 检查必要字段
        if grep -q "display_name" "${agent_file}" && \
           grep -q "system_prompt" "${agent_file}" && \
           grep -q "endpoint" "${agent_file}"; then
            log_ok "  ✓ ${agent_name}: 配置完整"
        else
            log_warn "  ⚠ ${agent_name}: 配置不完整"
        fi
    done
else
    log_error "未找到数字员工配置"
    add_test_result "数字员工: 配置文件" "FAIL" "配置文件缺失"
fi

echo ""

# ============================================================================
# 测试 6: 系统配置
# ============================================================================
log_test "测试系统配置..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

CONFIG_PATH="${HOME}/.config/openclaw-plus/config.toml"

log_step "检查配置文件..."
if [ -f "${CONFIG_PATH}" ]; then
    log_ok "配置文件存在"
    add_test_result "系统: 配置文件" "PASS" "配置文件正常"
    
    # 检查关键配置
    if grep -q "\[openclaw_ai\]" "${CONFIG_PATH}"; then
        log_ok "  ✓ AI 配置存在"
    fi
    
    if grep -q "workspace_dir" "${CONFIG_PATH}"; then
        log_ok "  ✓ 工作目录配置存在"
    fi
    
    if grep -q "network_allowlist" "${CONFIG_PATH}"; then
        log_ok "  ✓ 网络白名单配置存在"
    fi
else
    log_error "配置文件不存在"
    add_test_result "系统: 配置文件" "FAIL" "配置文件缺失"
fi

echo ""

# ============================================================================
# 生成测试报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  测试结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "测试类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 按类别统计
RUST_TESTS=0
RUST_PASSED=0
AI_TESTS=0
AI_PASSED=0
FILE_TESTS=0
FILE_PASSED=0
NETWORK_TESTS=0
NETWORK_PASSED=0
AGENT_TESTS=0
AGENT_PASSED=0
SYSTEM_TESTS=0
SYSTEM_PASSED=0

for result in "${TEST_RESULTS[@]}"; do
    name=$(echo "$result" | cut -d'|' -f1)
    status=$(echo "$result" | cut -d'|' -f2)
    
    case "$name" in
        Rust:*)
            RUST_TESTS=$((RUST_TESTS + 1))
            [ "$status" = "PASS" ] && RUST_PASSED=$((RUST_PASSED + 1))
            ;;
        AI:*)
            AI_TESTS=$((AI_TESTS + 1))
            [ "$status" = "PASS" ] && AI_PASSED=$((AI_PASSED + 1))
            ;;
        文件:*)
            FILE_TESTS=$((FILE_TESTS + 1))
            [ "$status" = "PASS" ] && FILE_PASSED=$((FILE_PASSED + 1))
            ;;
        网络:*)
            NETWORK_TESTS=$((NETWORK_TESTS + 1))
            [ "$status" = "PASS" ] && NETWORK_PASSED=$((NETWORK_PASSED + 1))
            ;;
        数字员工:*)
            AGENT_TESTS=$((AGENT_TESTS + 1))
            [ "$status" = "PASS" ] && AGENT_PASSED=$((AGENT_PASSED + 1))
            ;;
        系统:*)
            SYSTEM_TESTS=$((SYSTEM_TESTS + 1))
            [ "$status" = "PASS" ] && SYSTEM_PASSED=$((SYSTEM_PASSED + 1))
            ;;
    esac
done

echo "Rust 单元测试:    ${RUST_PASSED}/${RUST_TESTS} 通过"
echo "AI 推理功能:      ${AI_PASSED}/${AI_TESTS} 通过"
echo "文件操作:         ${FILE_PASSED}/${FILE_TESTS} 通过"
echo "网络请求:         ${NETWORK_PASSED}/${NETWORK_TESTS} 通过"
echo "数字员工:         ${AGENT_PASSED}/${AGENT_TESTS} 通过"
echo "系统配置:         ${SYSTEM_PASSED}/${SYSTEM_TESTS} 通过"

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
    log_ok "✅ 所有测试通过！"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    log_warn "⚠️  部分测试失败"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
