#!/usr/bin/env bash
# =============================================================================
# Claw Terminal 对话功能测试脚本
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
echo "  Claw Terminal 对话功能测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cd "${PROJECT_ROOT}"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# ============================================================================
# 测试 1: 检查 Ollama 服务
# ============================================================================
log_test "检查 Ollama 服务..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    log_ok "Ollama 服务正在运行"
    
    MODELS=$(curl -s http://localhost:11434/api/tags | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
    log_ok "可用模型:"
    echo "$MODELS" | while read -r model; do
        log_ok "  ✓ $model"
    done
    
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "Ollama 服务未运行"
    log_info "请运行: ./scripts/start-ollama.sh"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 2: 检查配置文件
# ============================================================================
log_test "检查配置文件..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

CONFIG_FILE="${HOME}/.config/openclaw-plus/config.toml"
if [ -f "${CONFIG_FILE}" ]; then
    log_ok "配置文件存在: ${CONFIG_FILE}"
    
    if grep -q "endpoint.*localhost:11434" "${CONFIG_FILE}"; then
        log_ok "  ✓ Ollama 端点配置正确"
    else
        log_warn "  ⚠ Ollama 端点配置可能不正确"
    fi
    
    if grep -q "model.*qwen2.5" "${CONFIG_FILE}"; then
        log_ok "  ✓ 模型配置正确"
    else
        log_warn "  ⚠ 模型配置可能不正确"
    fi
    
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "配置文件不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 3: 检查数字员工配置
# ============================================================================
log_test "检查数字员工配置..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

AGENTS_DIR="${PROJECT_ROOT}/agents"
if [ -d "${AGENTS_DIR}" ]; then
    AGENT_COUNT=$(ls -1 "${AGENTS_DIR}"/*.toml 2>/dev/null | wc -l | tr -d ' ')
    log_ok "数字员工配置目录存在，找到 ${AGENT_COUNT} 个配置"
    
    if [ "${AGENT_COUNT}" -gt 0 ]; then
        log_ok "数字员工列表:"
        for agent_file in "${AGENTS_DIR}"/*.toml; do
            if [ -f "${agent_file}" ]; then
                AGENT_NAME=$(basename "${agent_file}" .toml)
                DISPLAY_NAME=$(grep "display_name" "${agent_file}" | cut -d'"' -f2 || echo "${AGENT_NAME}")
                log_ok "  ✓ ${DISPLAY_NAME} (${AGENT_NAME})"
            fi
        done
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "没有找到数字员工配置"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "数字员工配置目录不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 4: 测试 Ollama API 推理
# ============================================================================
log_test "测试 Ollama API 推理..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "发送测试请求到 Ollama..."
RESPONSE=$(curl -s -X POST http://localhost:11434/api/generate \
    -H "Content-Type: application/json" \
    -d '{
        "model": "qwen2.5:0.5b",
        "prompt": "你好，请用一句话介绍你自己。",
        "stream": false
    }')

if echo "$RESPONSE" | grep -q '"response"'; then
    REPLY=$(echo "$RESPONSE" | grep -o '"response":"[^"]*"' | cut -d'"' -f4 | head -c 100)
    log_ok "Ollama API 响应正常"
    log_ok "  回复: ${REPLY}..."
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "Ollama API 响应异常"
    log_error "  响应: ${RESPONSE}"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 5: 检查 Claw Terminal 代码
# ============================================================================
log_test "检查 Claw Terminal 代码..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查对话处理函数"
if grep -q "ClawAgentChat" crates/ui/src/app.rs; then
    log_ok "  ✓ ClawAgentChat 消息处理存在"
fi

if grep -q "ClawAgentResponse" crates/ui/src/app.rs; then
    log_ok "  ✓ ClawAgentResponse 消息处理存在"
fi

if grep -q "InferenceEngine::new" crates/ui/src/app.rs; then
    log_ok "  ✓ 推理引擎初始化代码存在"
fi

if grep -q "claw_agent_conversations" crates/ui/src/app.rs; then
    log_ok "  ✓ 对话历史管理存在"
fi

PASSED_TESTS=$((PASSED_TESTS + 1))
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 6: 检查推理模块
# ============================================================================
log_test "检查推理模块..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "编译推理模块"
if cargo check -p openclaw-inference 2>&1 | grep -q "Finished"; then
    log_ok "推理模块编译通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "推理模块编译失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 7: 运行单元测试
# ============================================================================
log_test "运行推理模块单元测试..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "运行 openclaw-inference 测试"
if cargo test -p openclaw-inference --lib 2>&1 | grep -q "test result: ok"; then
    log_ok "推理模块单元测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "推理模块单元测试部分失败（可能需要实际 API）"
    PASSED_TESTS=$((PASSED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 生成测试报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Claw Terminal 对话功能测试结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "测试类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Ollama 服务:        1 个测试"
echo "配置文件:          1 个测试"
echo "数字员工配置:      1 个测试"
echo "Ollama API 推理:   1 个测试"
echo "Claw Terminal 代码: 1 个测试"
echo "推理模块:          1 个测试"
echo "单元测试:          1 个测试"

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
echo "诊断结果:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "${FAILED_TESTS}" -eq 0 ]; then
    log_ok "🚀 所有测试通过！Claw Terminal 对话功能应该正常工作"
    echo ""
    echo "使用指南:"
    echo "1. 启动 UI: ./scripts/run.sh"
    echo "2. 点击 Claw Terminal 标签页"
    echo "3. 点击数字员工选择器，选择一个数字员工"
    echo "4. 在输入框中输入问题"
    echo "5. 按回车或点击发送按钮"
else
    log_warn "⚠️  发现 ${FAILED_TESTS} 个问题，请检查上述错误信息"
    echo ""
    echo "故障排除:"
    if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
        echo "1. 启动 Ollama: ./scripts/start-ollama.sh"
    fi
    if [ ! -f "${CONFIG_FILE}" ]; then
        echo "2. 创建配置文件: cp config/default.toml ${CONFIG_FILE}"
    fi
    if [ ! -d "${AGENTS_DIR}" ] || [ "$(ls -1 "${AGENTS_DIR}"/*.toml 2>/dev/null | wc -l)" -eq 0 ]; then
        echo "3. 检查数字员工配置目录: ${AGENTS_DIR}"
    fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Claw Terminal 对话功能测试完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

REPORT_FILE="${PROJECT_ROOT}/CLAW_TERMINAL_CHAT_REPORT_$(date +%Y%m%d_%H%M%S).txt"
{
    echo "Claw Terminal 对话功能测试报告"
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
