#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ AI Chat 自动化测试脚本
# 测试 AI 对话功能、模型检测、中文推理、消息滚动等
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
echo "  OpenClaw+ AI Chat 自动化测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cd "${PROJECT_ROOT}"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

OLLAMA_ENDPOINT="http://localhost:11434"
MODEL_NAME="qwen2.5:0.5b"

# ============================================================================
# 测试 1: 检查 Ollama 服务
# ============================================================================
log_test "检查 Ollama 服务..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if curl -s -m 5 "${OLLAMA_ENDPOINT}/api/tags" > /dev/null 2>&1; then
    log_ok "Ollama 服务运行中 (${OLLAMA_ENDPOINT})"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "Ollama 服务未运行"
    log_info "启动方法: ollama serve"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    echo ""
    echo "⚠️  Ollama 服务未运行，后续测试将跳过"
    exit 1
fi

echo ""

# ============================================================================
# 测试 2: 检查模型是否已下载
# ============================================================================
log_test "检查模型 ${MODEL_NAME}..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
MODELS_JSON=$(curl -s "${OLLAMA_ENDPOINT}/api/tags")
if echo "$MODELS_JSON" | grep -q "\"${MODEL_NAME}\""; then
    log_ok "模型 ${MODEL_NAME} 已安装"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "模型 ${MODEL_NAME} 未安装"
    log_info "安装方法: ollama pull ${MODEL_NAME}"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    echo ""
    echo "⚠️  模型未安装，后续测试将跳过"
    exit 1
fi

echo ""

# ============================================================================
# 测试 3: 测试模型列表 API
# ============================================================================
log_test "测试模型列表 API..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
MODEL_COUNT=$(echo "$MODELS_JSON" | grep -o '"name"' | wc -l | tr -d ' ')
if [ "$MODEL_COUNT" -gt 0 ]; then
    log_ok "检测到 ${MODEL_COUNT} 个已安装模型"
    echo "$MODELS_JSON" | grep '"name"' | head -5 | sed 's/^/  /'
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "未检测到任何模型"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

echo ""

# ============================================================================
# 测试 4: 测试中文推理（简单问题）
# ============================================================================
log_test "测试中文推理（简单问题）..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
log_info "发送请求: '你好，简单回答'"

RESPONSE=$(curl -s -X POST "${OLLAMA_ENDPOINT}/api/chat" \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"${MODEL_NAME}\",
        \"messages\": [{\"role\": \"user\", \"content\": \"你好，简单回答\"}],
        \"stream\": false
    }" 2>/dev/null)

if [ -n "$RESPONSE" ]; then
    CONTENT=$(echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',{}).get('content',''))" 2>/dev/null || echo "")
    
    if [ -n "$CONTENT" ]; then
        log_ok "收到中文回复"
        log_info "回复内容: ${CONTENT:0:100}..."
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "回复内容为空"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "请求失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

echo ""

# ============================================================================
# 测试 5: 测试系统提示词 + 中文问题
# ============================================================================
log_test "测试系统提示词 + 中文问题..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
log_info "发送请求: '你能做什么？'"

RESPONSE=$(curl -s -X POST "${OLLAMA_ENDPOINT}/api/chat" \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"${MODEL_NAME}\",
        \"messages\": [
            {\"role\": \"system\", \"content\": \"You are OpenClaw+, an intelligent AI assistant. Always respond in the same language the user writes in.\"},
            {\"role\": \"user\", \"content\": \"你能做什么？\"}
        ],
        \"stream\": false,
        \"options\": {
            \"num_predict\": 128,
            \"temperature\": 0.7
        }
    }" 2>/dev/null)

if [ -n "$RESPONSE" ]; then
    CONTENT=$(echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',{}).get('content',''))" 2>/dev/null || echo "")
    
    if [ -n "$CONTENT" ]; then
        log_ok "收到系统提示词引导的回复"
        log_info "回复内容: ${CONTENT:0:150}..."
        
        # 检查是否包含 fallback 文字
        if echo "$CONTENT" | grep -qi "I'm not sure what you're asking\|Could you rephrase"; then
            log_error "⚠️  检测到 fallback 回复文字"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        else
            log_ok "✓ 无 fallback 回复"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        fi
    else
        log_error "回复内容为空"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "请求失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

echo ""

# ============================================================================
# 测试 6: 运行 Rust inference crate 的 live 测试
# ============================================================================
log_test "运行 Rust inference crate 的 live 测试..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
log_info "执行: cargo test -p openclaw-inference ollama_live -- --ignored --nocapture"

if PATH="/opt/homebrew/bin:$PATH" cargo test -p openclaw-inference "ollama_live" -- --ignored --nocapture 2>&1 | grep -q "test result: ok"; then
    log_ok "Rust live 测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "Rust live 测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

echo ""

# ============================================================================
# 测试 7: 检查 UI 代码中的关键修复
# ============================================================================
log_test "检查 UI 代码中的关键修复..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
FIXES_OK=true

# 检查 on_nav_select 是否有 AiListModels
if grep -q "chain(self.update(AppMessage::AiListModels))" crates/ui/src/app.rs; then
    log_ok "✓ on_nav_select 包含 AiListModels"
else
    log_error "✗ on_nav_select 缺少 AiListModels"
    FIXES_OK=false
fi

# 检查 NavSelect 是否有 AiListModels
if grep -A 5 "NavPage::AiChat =>" crates/ui/src/app.rs | grep -q "AiListModels"; then
    log_ok "✓ NavSelect(AiChat) 包含 AiListModels"
else
    log_error "✗ NavSelect(AiChat) 缺少 AiListModels"
    FIXES_OK=false
fi

# 检查 AiModelsListed 是否有自动选择逻辑
if grep -A 10 "AppMessage::AiModelsListed" crates/ui/src/app.rs | grep -q "Auto-selected model"; then
    log_ok "✓ AiModelsListed 包含自动选择逻辑"
else
    log_error "✗ AiModelsListed 缺少自动选择逻辑"
    FIXES_OK=false
fi

# 检查 anchor_bottom
if grep -q "anchor_bottom()" crates/ui/src/pages/ai_chat.rs; then
    log_ok "✓ 消息列表包含 anchor_bottom()"
else
    log_error "✗ 消息列表缺少 anchor_bottom()"
    FIXES_OK=false
fi

# 检查 system prompt
if grep -q "You are OpenClaw+" crates/ui/src/app.rs; then
    log_ok "✓ 包含 system prompt"
else
    log_error "✗ 缺少 system prompt"
    FIXES_OK=false
fi

if $FIXES_OK; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

echo ""

# ============================================================================
# 测试 8: 编译 UI（确保修复已应用）
# ============================================================================
log_test "编译 UI（确保修复已应用）..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
log_info "执行: cargo check -p openclaw-ui"

if PATH="/opt/homebrew/bin:$PATH" cargo check -p openclaw-ui 2>&1 | grep -q "Finished"; then
    log_ok "UI 编译通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "UI 编译失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

echo ""

# ============================================================================
# 测试 9: 检查 .app bundle 是否存在
# ============================================================================
log_test "检查 .app bundle..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if [ -f "/tmp/OpenClawPlus.app/Contents/MacOS/openclaw-plus" ]; then
    BUNDLE_TIME=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M:%S" /tmp/OpenClawPlus.app/Contents/MacOS/openclaw-plus)
    log_ok ".app bundle 存在"
    log_info "最后更新: ${BUNDLE_TIME}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn ".app bundle 不存在"
    log_info "创建方法: ./scripts/run.sh"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

echo ""

# ============================================================================
# 生成测试报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  AI Chat 测试结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "测试类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Ollama 服务:        1 个测试"
echo "模型检测:           1 个测试"
echo "模型列表 API:       1 个测试"
echo "中文推理（简单）:   1 个测试"
echo "系统提示词:         1 个测试"
echo "Rust live 测试:     1 个测试"
echo "代码修复检查:       1 个测试"
echo "UI 编译:            1 个测试"
echo ".app bundle:        1 个测试"

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
echo "关键功能状态:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Ollama
if curl -s -m 5 "${OLLAMA_ENDPOINT}/api/tags" > /dev/null 2>&1; then
    echo "✅ Ollama 服务: 运行中"
else
    echo "❌ Ollama 服务: 未运行"
fi

# 模型
if echo "$MODELS_JSON" | grep -q "\"${MODEL_NAME}\""; then
    echo "✅ 模型 ${MODEL_NAME}: 已安装"
else
    echo "❌ 模型 ${MODEL_NAME}: 未安装"
fi

# 代码修复
if $FIXES_OK; then
    echo "✅ 代码修复: 已应用"
else
    echo "⚠️  代码修复: 部分缺失"
fi

echo ""
echo "下一步操作:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "${FAILED_TESTS}" -gt 0 ]; then
    echo "1. 修复失败的测试项"
    
    if ! curl -s -m 5 "${OLLAMA_ENDPOINT}/api/tags" > /dev/null 2>&1; then
        echo "   - 启动 Ollama: ollama serve"
    fi
    
    if ! echo "$MODELS_JSON" | grep -q "\"${MODEL_NAME}\""; then
        echo "   - 安装模型: ollama pull ${MODEL_NAME}"
    fi
    
    if ! $FIXES_OK; then
        echo "   - 重新应用代码修复"
    fi
fi

echo "2. 重新构建 UI: PATH=\"/opt/homebrew/bin:\$PATH\" cargo build --release -p openclaw-ui"
echo "3. 更新 .app bundle: cp target/release/openclaw-plus /tmp/OpenClawPlus.app/Contents/MacOS/"
echo "4. 启动 UI 测试: open /tmp/OpenClawPlus.app"
echo "5. 在 AI Chat 页面发送: '你能做什么？'"
echo "6. 验证收到中文回复（非 fallback）"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  AI Chat 测试完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

REPORT_FILE="${PROJECT_ROOT}/AI_CHAT_TEST_REPORT_$(date +%Y%m%d_%H%M%S).txt"
{
    echo "AI Chat 自动化测试报告"
    echo "生成时间: $(date '+%Y-%m-%d %H:%M:%S')"
    echo ""
    echo "测试统计:"
    echo "总测试数: ${TOTAL_TESTS}"
    echo "通过: ${PASSED_TESTS}"
    echo "失败: ${FAILED_TESTS}"
    echo "成功率: ${SUCCESS_RATE:-N/A}%"
    echo ""
    echo "Ollama 端点: ${OLLAMA_ENDPOINT}"
    echo "测试模型: ${MODEL_NAME}"
} > "${REPORT_FILE}"

log_info "详细测试报告已保存到: ${REPORT_FILE}"

if [ "${FAILED_TESTS}" -eq 0 ]; then
    echo ""
    log_ok "🎉 所有测试通过！AI Chat 功能正常"
    exit 0
else
    echo ""
    log_error "⚠️  部分测试失败，请查看上述输出"
    exit 1
fi
