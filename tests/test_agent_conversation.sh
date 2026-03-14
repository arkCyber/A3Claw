#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 数字员工对话功能深度测试
# 测试 AI 对话、上下文理解、多轮交互等
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

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 数字员工对话功能深度测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# ============================================================================
# 测试 1: 基础对话能力
# ============================================================================
log_test "测试基础对话能力..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 测试简单问候
log_info "测试 1.1: 简单问候"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "你好，请介绍一下你自己",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"'; then
    RESPONSE_TEXT=$(echo "${RESPONSE}" | grep -o '"response":"[^"]*"' | cut -d'"' -f4)
    log_ok "简单问候测试通过"
    echo "  响应: ${RESPONSE_TEXT:0:100}..."
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "简单问候测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

# 测试专业问题
log_info "测试 1.2: 专业技术问题"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "请解释什么是 WASM 沙箱，以及它的安全优势",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"' && echo "${RESPONSE}" | grep -qi "wasm\|webassembly\|沙箱\|安全"; then
    log_ok "专业技术问题测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "专业技术问题测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

# 测试代码理解
log_info "测试 1.3: 代码理解能力"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "请解释这段 Rust 代码的作用：fn main() { println!(\"Hello, world!\"); }",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"' && echo "${RESPONSE}" | grep -qi "hello\|打印\|输出\|main"; then
    log_ok "代码理解能力测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "代码理解能力测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 2: 多轮对话能力
# ============================================================================
log_test "测试多轮对话能力..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "测试 2.1: 上下文记忆（第一轮）"
RESPONSE1=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "我的名字是张三，我是一名软件工程师",
  "stream": false
}' 2>&1)

if echo "${RESPONSE1}" | grep -q '"response"'; then
    log_ok "第一轮对话成功"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "第一轮对话失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "测试 2.2: 任务分解能力"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "如何学习 Rust 编程语言？请给出具体步骤",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"' && echo "${RESPONSE}" | grep -qi "步骤\|学习\|rust"; then
    log_ok "任务分解能力测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "任务分解能力测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 3: 不同角色的数字员工
# ============================================================================
log_test "测试不同角色的数字员工..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 读取知识库首席官的系统提示词
KNOWLEDGE_OFFICER_PROMPT=$(grep -A 10 "system_prompt" "${PROJECT_ROOT}/agents/knowledge_officer.toml" | grep -v "system_prompt" | sed 's/^"//' | sed 's/"$//' | tr '\n' ' ')

log_info "测试 3.1: 知识库首席官角色"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d "{
  \"model\": \"qwen2.5:0.5b\",
  \"prompt\": \"作为知识库管理员，请介绍你的主要职责\",
  \"stream\": false
}" 2>&1)

if echo "${RESPONSE}" | grep -q '"response"'; then
    log_ok "知识库首席官角色测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "知识库首席官角色测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "测试 3.2: 代码审查员角色"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "作为代码审查员，请审查这段代码：let x = 5; let y = x + 10;",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"'; then
    log_ok "代码审查员角色测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "代码审查员角色测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 4: 中文处理能力
# ============================================================================
log_test "测试中文处理能力..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "测试 4.1: 中文理解"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "请用中文解释什么是人工智能",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"' && echo "${RESPONSE}" | grep -q '人工智能\|AI\|智能'; then
    log_ok "中文理解测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "中文理解测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "测试 4.2: 中英文混合"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "请用中文解释 REST API 是什么",
  "stream": false
}' 2>&1)

if echo "${RESPONSE}" | grep -q '"response"'; then
    log_ok "中英文混合测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "中英文混合测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 5: 响应质量评估
# ============================================================================
log_test "测试响应质量..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "测试 5.1: 响应长度合理性"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "请简要介绍 Rust 编程语言的三个主要特点",
  "stream": false
}' 2>&1)

RESPONSE_TEXT=$(echo "${RESPONSE}" | grep -o '"response":"[^"]*"' | cut -d'"' -f4)
RESPONSE_LENGTH=${#RESPONSE_TEXT}

if [ "${RESPONSE_LENGTH}" -gt 50 ] && [ "${RESPONSE_LENGTH}" -lt 1000 ]; then
    log_ok "响应长度合理 (${RESPONSE_LENGTH} 字符)"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "响应长度异常 (${RESPONSE_LENGTH} 字符)"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "测试 5.2: 响应时间"
START_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:0.5b",
  "prompt": "你好",
  "stream": false
}' 2>&1)
END_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
DURATION=$((END_TIME - START_TIME))

if [ "${DURATION}" -lt 5000 ]; then
    log_ok "响应时间合理 (${DURATION}ms)"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "响应时间过长 (${DURATION}ms)"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

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
echo "基础对话能力:     3 个测试"
echo "多轮对话能力:     2 个测试"
echo "角色扮演能力:     2 个测试"
echo "中文处理能力:     2 个测试"
echo "响应质量评估:     2 个测试"

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
    log_ok "✅ 所有对话测试通过！"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    echo "⚠️  部分对话测试失败"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
