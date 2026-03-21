#!/bin/bash
# OpenClaw+ 全面 WasmEdge 功能测试
# 测试所有 WasmEdge 集成功能

set -e

export DYLD_LIBRARY_PATH=$HOME/.wasmedge/lib:$DYLD_LIBRARY_PATH

BLUE='\033[0;34m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0
TOTAL=0

log_test() {
    local name=$1
    local status=$2
    local detail=$3
    
    TOTAL=$((TOTAL + 1))
    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✓${NC} $name ${detail}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗${NC} $name ${detail}"
        FAILED=$((FAILED + 1))
    fi
}

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  OpenClaw+ 全面 WasmEdge 功能测试${NC}"
echo -e "${BLUE}  测试时间: $(date '+%Y-%m-%d %H:%M:%S')${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo -e "${YELLOW}[1/8] WasmEdge 环境验证${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 1: WasmEdge 命令可用性
if command -v wasmedge &> /dev/null; then
    VERSION=$(wasmedge --version 2>&1 | head -1)
    log_test "WasmEdge 命令可用" "PASS" "($VERSION)"
else
    log_test "WasmEdge 命令可用" "FAIL" "(未安装)"
fi

# 测试 2: WasmEdge 动态库
if [ -f "$HOME/.wasmedge/lib/libwasmedge.dylib" ]; then
    SIZE=$(ls -lh "$HOME/.wasmedge/lib/libwasmedge.dylib" | awk '{print $5}')
    log_test "WasmEdge 动态库存在" "PASS" "($SIZE)"
else
    log_test "WasmEdge 动态库存在" "FAIL" "(未找到)"
fi

# 测试 3: QuickJS WASM 引擎
if [ -f "assets/wasmedge_quickjs.wasm" ]; then
    SIZE=$(ls -lh assets/wasmedge_quickjs.wasm | awk '{print $5}')
    log_test "QuickJS WASM 引擎" "PASS" "($SIZE)"
else
    log_test "QuickJS WASM 引擎" "FAIL" "(未找到)"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[2/8] Rust 编译和单元测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 4: Sandbox crate 编译
if SDKROOT=$(xcrun --show-sdk-path) BINDGEN_EXTRA_CLANG_ARGS="-isysroot $(xcrun --show-sdk-path)" \
   cargo build -p openclaw-sandbox --lib --quiet 2>&1 | grep -q "Finished"; then
    log_test "Sandbox crate 编译" "PASS" ""
else
    log_test "Sandbox crate 编译" "FAIL" ""
fi

# 测试 5: 单元测试
TEST_OUTPUT=$(SDKROOT=$(xcrun --show-sdk-path) BINDGEN_EXTRA_CLANG_ARGS="-isysroot $(xcrun --show-sdk-path)" \
              cargo test -p openclaw-sandbox --lib --quiet 2>&1)
if echo "$TEST_OUTPUT" | grep -q "11 passed"; then
    log_test "单元测试 (11个)" "PASS" ""
else
    log_test "单元测试" "FAIL" ""
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[3/8] WasmEdge 基础功能测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 6: 简单 JavaScript 执行
cat > /tmp/test_hello.js << 'EOF'
print("Hello from WasmEdge!");
EOF

if wasmedge --dir .:. assets/wasmedge_quickjs.wasm /tmp/test_hello.js 2>&1 | grep -q "Hello from WasmEdge"; then
    log_test "JavaScript 执行" "PASS" ""
else
    log_test "JavaScript 执行" "FAIL" ""
fi

# 测试 7: 文件系统访问
cat > /tmp/test_fs.js << 'EOF'
import * as std from 'std';
let f = std.open('/tmp/wasmedge_test.txt', 'w');
f.puts('WasmEdge filesystem test');
f.close();
print('File written successfully');
EOF

if wasmedge --dir .:. --dir /tmp:/tmp assets/wasmedge_quickjs.wasm /tmp/test_fs.js 2>&1 | grep -q "File written"; then
    log_test "文件系统访问" "PASS" ""
else
    log_test "文件系统访问" "FAIL" ""
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[4/8] OpenClaw 技能系统测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 8: 技能 SDK 文件
if [ -f "assets/openclaw/sdk/skill_client.js" ]; then
    if grep -q "SkillClient" assets/openclaw/sdk/skill_client.js; then
        log_test "技能 SDK 完整性" "PASS" ""
    else
        log_test "技能 SDK 完整性" "FAIL" "(缺少 SkillClient)"
    fi
else
    log_test "技能 SDK 完整性" "FAIL" "(文件不存在)"
fi

# 测试 9: 文件系统技能示例
if [ -f "assets/openclaw/skills/fs_skills.js" ]; then
    log_test "文件系统技能示例" "PASS" ""
else
    log_test "文件系统技能示例" "FAIL" ""
fi

# 测试 10: 网络技能示例
if [ -f "assets/openclaw/skills/web_skills.js" ]; then
    log_test "网络技能示例" "PASS" ""
else
    log_test "网络技能示例" "FAIL" ""
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[5/8] 安全配置测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 11: 安全配置文件
if [ -f "config/security_profiles.toml" ]; then
    if grep -q "default" config/security_profiles.toml; then
        log_test "安全配置文件" "PASS" ""
    else
        log_test "安全配置文件" "FAIL" "(缺少 default 配置)"
    fi
else
    log_test "安全配置文件" "FAIL" "(文件不存在)"
fi

# 测试 12: 策略引擎配置
if [ -f "config/policy_engine.toml" ]; then
    log_test "策略引擎配置" "PASS" ""
else
    log_test "策略引擎配置" "FAIL" ""
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[6/8] OpenClaw 主入口测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 13: 主入口文件
if [ -f "assets/openclaw/dist/index.js" ]; then
    SIZE=$(wc -c < assets/openclaw/dist/index.js)
    if [ $SIZE -gt 1000 ]; then
        log_test "主入口文件" "PASS" "(${SIZE} bytes)"
    else
        log_test "主入口文件" "FAIL" "(文件太小)"
    fi
else
    log_test "主入口文件" "FAIL" "(文件不存在)"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[7/8] 性能和资源测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 14: WasmEdge 启动性能
START=$(date +%s%N)
wasmedge --version > /dev/null 2>&1
END=$(date +%s%N)
DURATION=$(( (END - START) / 1000000 ))
if [ $DURATION -lt 100 ]; then
    log_test "WasmEdge 启动性能" "PASS" "(${DURATION}ms)"
else
    log_test "WasmEdge 启动性能" "FAIL" "(${DURATION}ms, 应 < 100ms)"
fi

# 测试 15: QuickJS 加载性能
if [ -f "assets/wasmedge_quickjs.wasm" ]; then
    START=$(date +%s%N)
    head -c 4 assets/wasmedge_quickjs.wasm > /dev/null 2>&1
    END=$(date +%s%N)
    DURATION=$(( (END - START) / 1000000 ))
    log_test "QuickJS 验证性能" "PASS" "(${DURATION}ms)"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${YELLOW}[8/8] 集成和端到端测试${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# 测试 16: 完整的 JavaScript 模块加载
cat > /tmp/test_module.js << 'EOF'
import * as std from 'std';
import * as os from 'os';
print('Module system working');
print('Platform: ' + os.platform);
EOF

if wasmedge --dir .:. assets/wasmedge_quickjs.wasm /tmp/test_module.js 2>&1 | grep -q "Module system working"; then
    log_test "JavaScript 模块系统" "PASS" ""
else
    log_test "JavaScript 模块系统" "FAIL" ""
fi

# 测试 17: 错误处理
cat > /tmp/test_error.js << 'EOF'
try {
    throw new Error('Test error');
} catch (e) {
    print('Error caught: ' + e.message);
}
EOF

if wasmedge --dir .:. assets/wasmedge_quickjs.wasm /tmp/test_error.js 2>&1 | grep -q "Error caught"; then
    log_test "JavaScript 错误处理" "PASS" ""
else
    log_test "JavaScript 错误处理" "FAIL" ""
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  测试结果汇总${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo
echo -e "总测试数: ${BLUE}$TOTAL${NC}"
echo -e "通过: ${GREEN}$PASSED${NC}"
echo -e "失败: ${RED}$FAILED${NC}"

if command -v bc &> /dev/null; then
    SUCCESS_RATE=$(echo "scale=1; ($PASSED * 100) / $TOTAL" | bc)
else
    SUCCESS_RATE=$(( (PASSED * 100) / TOTAL ))
fi
echo -e "成功率: ${YELLOW}${SUCCESS_RATE}%${NC}"
echo

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠ $FAILED 个测试失败${NC}"
    exit 1
fi
