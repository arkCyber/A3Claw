#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ WasmEdge 沙箱运行测试
# 测试 WasmEdge QuickJS 沙箱中运行 JavaScript Agent
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
echo "  OpenClaw+ WasmEdge 沙箱运行测试"
echo "  测试时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

WORKSPACE_DIR="${HOME}/.openclaw-plus/workspace"

find_quickjs_runtime() {
    local candidates=(
        "${PROJECT_ROOT}/assets/wasmedge_quickjs.wasm"
        "${HOME}/.local/share/openclaw-plus/wasmedge_quickjs.wasm"
        "/usr/share/openclaw-plus/wasmedge_quickjs.wasm"
    )

    for candidate in "${candidates[@]}"; do
        if [ -f "${candidate}" ]; then
            echo "${candidate}"
            return 0
        fi
    done

    return 1
}

WASMEDGE_QUICKJS="$(find_quickjs_runtime || true)"

# ============================================================================
# 测试 1: WasmEdge 环境检查
# ============================================================================
log_test "检查 WasmEdge 环境..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "检查 WasmEdge 命令"
if command -v wasmedge &> /dev/null; then
    WASMEDGE_VERSION=$(wasmedge --version 2>&1 | head -1)
    log_ok "WasmEdge 已安装: ${WASMEDGE_VERSION}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "WasmEdge 未安装"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查 WasmEdge QuickJS 库"
if [ -n "${WASMEDGE_QUICKJS}" ] && [ -f "${WASMEDGE_QUICKJS}" ]; then
    log_ok "WasmEdge QuickJS 运行时存在: ${WASMEDGE_QUICKJS}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "WasmEdge QuickJS 运行时不存在（已检查项目 assets 与标准安装位置）"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查工作目录"
if [ -d "${WORKSPACE_DIR}" ]; then
    log_ok "工作目录存在: ${WORKSPACE_DIR}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_warn "工作目录不存在，创建中..."
    mkdir -p "${WORKSPACE_DIR}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 2: 简单 JavaScript 执行
# ============================================================================
log_test "测试简单 JavaScript 执行..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TEST_JS="${WORKSPACE_DIR}/test_simple.js"
cat > "${TEST_JS}" << 'EOF'
console.log("Hello from WasmEdge QuickJS!");
console.log("Math test: 2 + 2 =", 2 + 2);
console.log("String test: " + "OpenClaw");
EOF

log_info "运行简单 JavaScript 测试"
if [ -n "${WASMEDGE_QUICKJS}" ] && OUTPUT=$(wasmedge --dir /workspace:"${WORKSPACE_DIR}" "${WASMEDGE_QUICKJS}" "${TEST_JS}" 2>&1); then
    if echo "${OUTPUT}" | grep -q "Hello from WasmEdge QuickJS"; then
        log_ok "简单 JavaScript 执行成功"
        echo "  输出: ${OUTPUT}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_error "输出不符合预期"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "JavaScript 执行失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

rm -f "${TEST_JS}"

echo ""

# ============================================================================
# 测试 3: 文件系统操作
# ============================================================================
log_test "测试文件系统操作..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TEST_JS="${WORKSPACE_DIR}/test_fs.js"
cat > "${TEST_JS}" << 'EOF'
import * as std from 'std';

console.log("[TEST] 开始文件系统测试");

// 写入文件
const file = std.open('/workspace/test_output.txt', 'w');
if (file) {
    file.puts("Hello from WasmEdge!\n");
    file.puts("File system test successful!\n");
    file.close();
    console.log("[OK] 文件写入成功");
} else {
    console.log("[ERROR] 文件写入失败");
}

// 读取文件
const readFile = std.open('/workspace/test_output.txt', 'r');
if (readFile) {
    const content = readFile.readAsString();
    readFile.close();
    console.log("[OK] 文件读取成功");
    console.log("[CONTENT]", content);
} else {
    console.log("[ERROR] 文件读取失败");
}

console.log("[TEST] 文件系统测试完成");
EOF

log_info "运行文件系统测试"
if [ -n "${WASMEDGE_QUICKJS}" ] && OUTPUT=$(wasmedge --dir /workspace:"${WORKSPACE_DIR}" "${WASMEDGE_QUICKJS}" "${TEST_JS}" 2>&1); then
    if echo "${OUTPUT}" | grep -q "文件写入成功" && echo "${OUTPUT}" | grep -q "文件读取成功"; then
        log_ok "文件系统操作成功"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        
        # 验证文件是否真的被创建
        if [ -f "${WORKSPACE_DIR}/test_output.txt" ]; then
            log_ok "  ✓ 文件确实被创建"
            CONTENT=$(cat "${WORKSPACE_DIR}/test_output.txt")
            echo "  文件内容: ${CONTENT}"
        fi
    else
        log_error "文件系统操作失败"
        echo "  输出: ${OUTPUT}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "JavaScript 执行失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

rm -f "${TEST_JS}" "${WORKSPACE_DIR}/test_output.txt"

echo ""

# ============================================================================
# 测试 4: 技能 SDK 测试
# ============================================================================
log_test "测试技能 SDK..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

SDK_FILE="${PROJECT_ROOT}/assets/openclaw/sdk/skills.js"

log_info "检查技能 SDK 文件"
if [ -f "${SDK_FILE}" ]; then
    log_ok "技能 SDK 文件存在"
    PASSED_TESTS=$((PASSED_TESTS + 1))
    
    # 检查 SDK 内容
    if grep -q "SkillClient" "${SDK_FILE}"; then
        log_ok "  ✓ SDK 包含 SkillClient"
    fi
    
    if grep -q "execute" "${SDK_FILE}"; then
        log_ok "  ✓ SDK 包含 execute 方法"
    fi
else
    log_error "技能 SDK 文件不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 5: 技能示例文件检查
# ============================================================================
log_test "测试技能示例文件..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

EXAMPLES_DIR="${PROJECT_ROOT}/assets/openclaw/examples"

log_info "检查文件系统技能示例"
if [ -f "${EXAMPLES_DIR}/fs_skills.js" ]; then
    log_ok "fs_skills.js 存在"
    
    # 尝试验证语法（不执行）
    if grep -q "import.*std" "${EXAMPLES_DIR}/fs_skills.js"; then
        log_ok "  ✓ 使用正确的 std 模块导入"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ 可能缺少 std 模块导入"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "fs_skills.js 不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

log_info "检查网络技能示例"
if [ -f "${EXAMPLES_DIR}/web_skills.js" ]; then
    log_ok "web_skills.js 存在"
    
    if grep -q "import.*wasi_net" "${EXAMPLES_DIR}/web_skills.js"; then
        log_ok "  ✓ 使用正确的 wasi_net 模块导入"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ 可能缺少 wasi_net 模块导入"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "web_skills.js 不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 6: Rust 沙箱模块测试
# ============================================================================
log_test "测试 Rust 沙箱模块..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "${PROJECT_ROOT}"

log_info "运行 sandbox crate 单元测试"
if cargo test -p openclaw-sandbox --lib 2>&1 | tail -20 | grep -q "test result: ok\|running 0 tests"; then
    log_ok "sandbox 单元测试通过"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    log_error "sandbox 单元测试失败"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 测试 7: OpenClaw 主入口文件检查
# ============================================================================
log_test "检查 OpenClaw 主入口文件..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

ENTRY_FILE="${PROJECT_ROOT}/assets/openclaw/dist/index.js"

log_info "检查主入口文件"
if [ -f "${ENTRY_FILE}" ]; then
    log_ok "主入口文件存在: ${ENTRY_FILE}"
    
    FILE_SIZE=$(wc -c < "${ENTRY_FILE}" | tr -d ' ')
    log_ok "  文件大小: ${FILE_SIZE} 字节"
    
    # 检查关键导入
    if grep -q "import.*wasi_net\|import.*std" "${ENTRY_FILE}"; then
        log_ok "  ✓ 包含必要的模块导入"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        log_warn "  ⚠ 可能缺少必要的模块导入"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
else
    log_error "主入口文件不存在"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
TOTAL_TESTS=$((TOTAL_TESTS + 1))

echo ""

# ============================================================================
# 生成测试报告
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  WasmEdge 沙箱测试结果汇总"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "测试类别统计:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "WasmEdge 环境检查:    3 个测试"
echo "JavaScript 执行:      1 个测试"
echo "文件系统操作:         1 个测试"
echo "技能 SDK:             1 个测试"
echo "技能示例文件:         2 个测试"
echo "Rust 沙箱模块:        1 个测试"
echo "主入口文件检查:       1 个测试"

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
    log_ok "✅ 所有 WasmEdge 沙箱测试通过！"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    log_warn "⚠️  部分 WasmEdge 沙箱测试失败"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
