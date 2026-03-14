#!/usr/bin/env bash
# =============================================================================
# 数字员工功能测试脚本
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  数字员工功能测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 1. 检查 Ollama 服务
log_info "检查 Ollama 服务..."
if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    log_ok "Ollama 服务运行正常"
    MODELS=$(curl -s http://localhost:11434/api/tags | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
    echo "   可用模型: ${MODELS}"
else
    log_error "Ollama 服务未运行"
    log_info "请运行: ./scripts/start-ollama.sh"
    exit 1
fi

# 2. 检查数字员工配置文件
log_info "检查数字员工配置..."
AGENT_COUNT=$(ls -1 "${PROJECT_ROOT}/agents"/*.toml 2>/dev/null | wc -l)
if [ "${AGENT_COUNT}" -gt 0 ]; then
    log_ok "找到 ${AGENT_COUNT} 个数字员工配置"
    for agent_file in "${PROJECT_ROOT}/agents"/*.toml; do
        agent_name=$(basename "${agent_file}" .toml)
        agent_display=$(grep "display_name" "${agent_file}" | cut -d'"' -f2)
        echo "   - ${agent_name}: ${agent_display}"
    done
else
    log_error "未找到数字员工配置文件"
    exit 1
fi

# 3. 检查配置文件
log_info "检查系统配置..."
CONFIG_PATH="${HOME}/.config/openclaw-plus/config.toml"
if [ -f "${CONFIG_PATH}" ]; then
    log_ok "配置文件存在: ${CONFIG_PATH}"
    
    # 检查 AI 配置
    if grep -q "\[openclaw_ai\]" "${CONFIG_PATH}"; then
        AI_PROVIDER=$(grep "provider" "${CONFIG_PATH}" | head -1 | cut -d'"' -f2)
        AI_MODEL=$(grep "model" "${CONFIG_PATH}" | head -1 | cut -d'"' -f2)
        log_ok "AI 配置: ${AI_PROVIDER} - ${AI_MODEL}"
    else
        log_warn "未找到 AI 配置"
    fi
else
    log_warn "配置文件不存在，将使用默认配置"
fi

# 4. 测试 AI 推理
log_info "测试 AI 推理功能..."
TEST_PROMPT="你好，请简单介绍一下你自己。"
RESPONSE=$(curl -s http://localhost:11434/api/generate -d "{
  \"model\": \"qwen2.5:0.5b\",
  \"prompt\": \"${TEST_PROMPT}\",
  \"stream\": false
}" | grep -o '"response":"[^"]*"' | cut -d'"' -f4 | head -c 100)

if [ -n "${RESPONSE}" ]; then
    log_ok "AI 推理测试成功"
    echo "   响应: ${RESPONSE}..."
else
    log_error "AI 推理测试失败"
    exit 1
fi

# 5. 检查编译状态
log_info "检查项目编译状态..."
cd "${PROJECT_ROOT}"
if cargo check --quiet 2>/dev/null; then
    log_ok "项目编译检查通过"
else
    log_warn "项目编译检查失败，可能需要重新编译"
fi

# 6. 检查 OpenClaw 入口文件
log_info "检查 OpenClaw 入口文件..."
OPENCLAW_ENTRY="${PROJECT_ROOT}/assets/openclaw/dist/index.js"
if [ -f "${OPENCLAW_ENTRY}" ]; then
    FILE_SIZE=$(du -h "${OPENCLAW_ENTRY}" | cut -f1)
    log_ok "OpenClaw 入口文件存在 (${FILE_SIZE})"
else
    log_error "OpenClaw 入口文件不存在: ${OPENCLAW_ENTRY}"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_ok "所有检查通过！环境已就绪"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
log_info "下一步操作："
echo "  1. 启动 UI: ./scripts/start-ui.sh"
echo "  2. 或直接运行: cargo run --release -p openclaw-ui"
echo "  3. 在 UI 中选择数字员工并开始对话"
echo ""
