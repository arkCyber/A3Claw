#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 性能和压力测试
# 测试并发处理、内存使用、响应时间等
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
echo "  OpenClaw+ 性能和压力测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ============================================================================
# 测试 1: 响应时间基准测试
# ============================================================================
log_test "测试响应时间基准..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "运行 10 次简单查询，测量平均响应时间"

TOTAL_TIME=0
SUCCESS_COUNT=0

for i in {1..10}; do
    START_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
    RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
      "model": "qwen2.5:0.5b",
      "prompt": "你好",
      "stream": false
    }' 2>&1)
    END_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
    
    if echo "${RESPONSE}" | grep -q '"response"'; then
        DURATION=$((END_TIME - START_TIME))
        TOTAL_TIME=$((TOTAL_TIME + DURATION))
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        echo "  请求 ${i}: ${DURATION}ms"
    else
        echo "  请求 ${i}: 失败"
    fi
done

if [ "${SUCCESS_COUNT}" -gt 0 ]; then
    AVG_TIME=$((TOTAL_TIME / SUCCESS_COUNT))
    log_ok "平均响应时间: ${AVG_TIME}ms (${SUCCESS_COUNT}/10 成功)"
else
    log_error "所有请求失败"
fi

echo ""

# ============================================================================
# 测试 2: 并发请求测试
# ============================================================================
log_test "测试并发请求处理..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "发送 5 个并发请求"

START_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')

# 启动 5 个后台请求
for i in {1..5}; do
    (
        curl -s http://localhost:11434/api/generate -d '{
          "model": "qwen2.5:0.5b",
          "prompt": "测试并发请求 '${i}'",
          "stream": false
        }' > /dev/null 2>&1
    ) &
done

# 等待所有后台任务完成
wait

END_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
CONCURRENT_TIME=$((END_TIME - START_TIME))

log_ok "5 个并发请求完成时间: ${CONCURRENT_TIME}ms"

echo ""

# ============================================================================
# 测试 3: 长文本处理
# ============================================================================
log_test "测试长文本处理能力..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "发送包含长文本的请求"

LONG_PROMPT="请总结以下内容：Rust 是一种系统编程语言，专注于安全、并发和性能。它由 Mozilla 研究院开发，旨在提供内存安全保证而不需要垃圾回收。Rust 的所有权系统是其核心特性之一，它在编译时检查内存安全，防止数据竞争和空指针解引用。Rust 还提供了强大的类型系统、模式匹配、trait 系统等现代编程语言特性。"

START_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
RESPONSE=$(curl -s http://localhost:11434/api/generate -d "{
  \"model\": \"qwen2.5:0.5b\",
  \"prompt\": \"${LONG_PROMPT}\",
  \"stream\": false
}" 2>&1)
END_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')

if echo "${RESPONSE}" | grep -q '"response"'; then
    DURATION=$((END_TIME - START_TIME))
    RESPONSE_TEXT=$(echo "${RESPONSE}" | grep -o '"response":"[^"]*"' | cut -d'"' -f4)
    RESPONSE_LENGTH=${#RESPONSE_TEXT}
    log_ok "长文本处理成功"
    echo "  处理时间: ${DURATION}ms"
    echo "  响应长度: ${RESPONSE_LENGTH} 字符"
else
    log_error "长文本处理失败"
fi

echo ""

# ============================================================================
# 测试 4: 文件系统性能
# ============================================================================
log_test "测试文件系统性能..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

WORKSPACE_DIR="${HOME}/.openclaw-plus/workspace"
TEST_FILE="${WORKSPACE_DIR}/perf_test_$(date +%s).txt"

log_info "测试文件写入性能（100 次）"

START_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
for i in {1..100}; do
    echo "Test line ${i}" >> "${TEST_FILE}"
done
END_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')

WRITE_TIME=$((END_TIME - START_TIME))
log_ok "100 次文件写入完成: ${WRITE_TIME}ms"

log_info "测试文件读取性能（100 次）"

START_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')
for i in {1..100}; do
    cat "${TEST_FILE}" > /dev/null
done
END_TIME=$(python3 -c 'import time; print(int(time.time() * 1000))')

READ_TIME=$((END_TIME - START_TIME))
log_ok "100 次文件读取完成: ${READ_TIME}ms"

# 清理测试文件
rm -f "${TEST_FILE}"

echo ""

# ============================================================================
# 测试 5: 内存使用监控
# ============================================================================
log_test "测试内存使用..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查 Ollama 进程内存使用"

if pgrep -x ollama > /dev/null; then
    OLLAMA_PID=$(pgrep -x ollama)
    OLLAMA_MEM=$(ps -o rss= -p "${OLLAMA_PID}" | awk '{print $1/1024}')
    log_ok "Ollama 内存使用: ${OLLAMA_MEM} MB"
else
    log_error "Ollama 进程未运行"
fi

echo ""

# ============================================================================
# 生成性能报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  性能测试总结"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "响应时间性能:"
echo "  平均响应时间: ${AVG_TIME}ms"
echo "  并发处理时间: ${CONCURRENT_TIME}ms"

echo ""
echo "文件系统性能:"
echo "  100 次写入: ${WRITE_TIME}ms (平均 $((WRITE_TIME / 100))ms/次)"
echo "  100 次读取: ${READ_TIME}ms (平均 $((READ_TIME / 100))ms/次)"

echo ""
echo "性能评估:"
if [ "${AVG_TIME}" -lt 500 ]; then
    echo "  ✅ 响应速度: 优秀 (< 500ms)"
elif [ "${AVG_TIME}" -lt 1000 ]; then
    echo "  ✅ 响应速度: 良好 (< 1s)"
else
    echo "  ⚠️  响应速度: 需要优化 (> 1s)"
fi

if [ "$((WRITE_TIME / 100))" -lt 10 ]; then
    echo "  ✅ 文件写入: 优秀 (< 10ms/次)"
else
    echo "  ⚠️  文件写入: 可接受"
fi

if [ "$((READ_TIME / 100))" -lt 10 ]; then
    echo "  ✅ 文件读取: 优秀 (< 10ms/次)"
else
    echo "  ⚠️  文件读取: 可接受"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_ok "性能测试完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
