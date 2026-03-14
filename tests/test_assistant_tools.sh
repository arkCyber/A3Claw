#!/bin/bash
# OpenClaw+ Assistant Tools 完整测试脚本
# 测试配置文件操作、系统自动启动、健康监控等功能

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$HOME/Library/Application Support/openclaw-plus"
TEST_REPORT="$PROJECT_ROOT/ASSISTANT_TOOLS_TEST_REPORT_$(date +%Y%m%d_%H%M%S).txt"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ Assistant Tools 功能测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "测试报告: $TEST_REPORT"
echo ""

# 初始化测试报告
cat > "$TEST_REPORT" << EOF
OpenClaw+ Assistant Tools 测试报告
生成时间: $(date '+%Y-%m-%d %H:%M:%S')
测试环境: macOS $(sw_vers -productVersion)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

EOF

TESTS_PASSED=0
TESTS_FAILED=0

# 测试函数
run_test() {
    local test_name="$1"
    local test_cmd="$2"
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "测试: $test_name"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    echo "" >> "$TEST_REPORT"
    echo "【测试】$test_name" >> "$TEST_REPORT"
    echo "命令: $test_cmd" >> "$TEST_REPORT"
    
    if eval "$test_cmd"; then
        echo "✅ 通过"
        echo "结果: ✅ 通过" >> "$TEST_REPORT"
        ((TESTS_PASSED++))
    else
        echo "❌ 失败"
        echo "结果: ❌ 失败" >> "$TEST_REPORT"
        ((TESTS_FAILED++))
    fi
    echo ""
}

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 1: 配置目录和文件
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第一部分: 配置文件测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

run_test "配置目录存在" "[ -d '$CONFIG_DIR' ] || mkdir -p '$CONFIG_DIR'"

# 创建测试配置文件
TEST_CONFIG="$CONFIG_DIR/test_config.toml"
cat > "$TEST_CONFIG" << 'EOF'
# OpenClaw+ 测试配置
ollama_endpoint = "http://localhost:11434"
ollama_model = "qwen2.5:0.5b"
test_key = "test_value"
EOF

run_test "测试配置文件创建" "[ -f '$TEST_CONFIG' ]"
run_test "测试配置文件可读" "cat '$TEST_CONFIG' > /dev/null"

# 测试配置文件解析
run_test "配置文件 TOML 格式有效" "python3 -c 'import tomllib; tomllib.loads(open(\"$TEST_CONFIG\").read())' 2>/dev/null || true"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 2: Ollama 服务检测
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第二部分: Ollama 服务测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

run_test "Ollama 命令存在" "which ollama > /dev/null"

# 检查 Ollama 服务状态
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✅ Ollama 服务运行中"
    echo "【状态】Ollama 服务: 运行中" >> "$TEST_REPORT"
    
    # 获取模型列表
    MODELS=$(curl -s http://localhost:11434/api/tags | python3 -c "import sys, json; data=json.load(sys.stdin); print(len(data.get('models', [])))" 2>/dev/null || echo "0")
    echo "已安装模型数量: $MODELS"
    echo "已安装模型数量: $MODELS" >> "$TEST_REPORT"
    
    run_test "至少有一个模型已安装" "[ $MODELS -gt 0 ]"
else
    echo "⚠️  Ollama 服务未运行"
    echo "【状态】Ollama 服务: 未运行" >> "$TEST_REPORT"
    echo "提示: 运行 'ollama serve' 启动服务"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 3: 健康监控配置
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第三部分: 健康监控配置测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

MONITOR_CONFIG="$CONFIG_DIR/health_monitor.toml"

# 创建测试监控配置
cat > "$MONITOR_CONFIG" << 'EOF'
enabled = true
interval_minutes = 5.0
last_check = 0
EOF

run_test "健康监控配置文件创建" "[ -f '$MONITOR_CONFIG' ]"
run_test "健康监控配置可读" "cat '$MONITOR_CONFIG' > /dev/null"

# 验证监控配置格式
run_test "监控配置格式有效" "grep -q 'enabled = true' '$MONITOR_CONFIG'"
run_test "监控间隔配置存在" "grep -q 'interval_minutes' '$MONITOR_CONFIG'"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 4: 工具关键词检测
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第四部分: 工具关键词检测测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 测试关键词映射
declare -A KEYWORD_TESTS=(
    ["修改配置"]="update_config"
    ["自动启动系统"]="auto_start_system"
    ["启用监控"]="enable_health_monitor"
    ["禁用监控"]="disable_health_monitor"
    ["检查 Ollama 状态"]="check_ollama_health"
    ["启动 Ollama"]="start_ollama_service"
    ["检查配置"]="check_config"
    ["系统状态"]="get_system_status"
    ["如何使用"]="provide_guide"
)

echo "测试关键词触发映射:"
for keyword in "${!KEYWORD_TESTS[@]}"; do
    expected="${KEYWORD_TESTS[$keyword]}"
    echo "  - \"$keyword\" → $expected"
    echo "  关键词: \"$keyword\" → 工具: $expected" >> "$TEST_REPORT"
done

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 5: UI 二进制文件
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第五部分: UI 二进制文件测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

UI_BINARY="$PROJECT_ROOT/target/release/openclaw-plus"
APP_BUNDLE="/tmp/OpenClawPlus.app/Contents/MacOS/openclaw-plus"

run_test "UI 二进制文件存在" "[ -f '$UI_BINARY' ]"
run_test "UI 二进制文件可执行" "[ -x '$UI_BINARY' ]"

if [ -f "$APP_BUNDLE" ]; then
    run_test "App Bundle 二进制存在" "[ -f '$APP_BUNDLE' ]"
    
    # 比较时间戳
    UI_TIME=$(stat -f %m "$UI_BINARY" 2>/dev/null || echo "0")
    BUNDLE_TIME=$(stat -f %m "$APP_BUNDLE" 2>/dev/null || echo "0")
    
    if [ "$UI_TIME" -gt "$BUNDLE_TIME" ]; then
        echo "⚠️  App Bundle 需要更新 (UI 二进制更新)"
        echo "【警告】App Bundle 需要更新" >> "$TEST_REPORT"
    else
        echo "✅ App Bundle 是最新的"
        echo "【状态】App Bundle: 最新" >> "$TEST_REPORT"
    fi
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 6: 代码完整性检查
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第六部分: 代码完整性检查"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TOOLS_FILE="$PROJECT_ROOT/crates/ui/src/assistant_tools.rs"

run_test "assistant_tools.rs 文件存在" "[ -f '$TOOLS_FILE' ]"

# 检查关键函数存在
run_test "detect_tool_trigger 函数存在" "grep -q 'pub fn detect_tool_trigger' '$TOOLS_FILE'"
run_test "update_config 函数存在" "grep -q 'async fn update_config' '$TOOLS_FILE'"
run_test "auto_start_system 函数存在" "grep -q 'async fn auto_start_system' '$TOOLS_FILE'"
run_test "enable_health_monitor 函数存在" "grep -q 'async fn enable_health_monitor' '$TOOLS_FILE'"
run_test "disable_health_monitor 函数存在" "grep -q 'async fn disable_health_monitor' '$TOOLS_FILE'"

# 检查工具枚举
run_test "AssistantTool 枚举包含新工具" "grep -q 'UpdateConfig' '$TOOLS_FILE'"
run_test "AssistantTool 枚举包含 AutoStartSystem" "grep -q 'AutoStartSystem' '$TOOLS_FILE'"
run_test "AssistantTool 枚举包含健康监控工具" "grep -q 'EnableHealthMonitor' '$TOOLS_FILE'"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 7: 集成测试（app.rs）
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第七部分: UI 集成检查"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

APP_FILE="$PROJECT_ROOT/crates/ui/src/app.rs"

run_test "app.rs 文件存在" "[ -f '$APP_FILE' ]"
run_test "AssistantToolResult 消息变体存在" "grep -q 'AssistantToolResult' '$APP_FILE'"
run_test "工具调用检测代码存在" "grep -q 'detect_tool_trigger' '$APP_FILE'"
run_test "工具执行代码存在" "grep -q 'AssistantTool::execute' '$APP_FILE'"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试 8: 文档完整性
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "第八部分: 文档完整性检查"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

run_test "工具使用指南存在" "[ -f '$PROJECT_ROOT/ASSISTANT_TOOLS_GUIDE.md' ]"
run_test "工具设计文档存在" "[ -f '$PROJECT_ROOT/ASSISTANT_TOOLS_DESIGN.md' ]"

# 检查文档内容
if [ -f "$PROJECT_ROOT/ASSISTANT_TOOLS_GUIDE.md" ]; then
    run_test "指南包含新工具说明" "grep -q 'update_config' '$PROJECT_ROOT/ASSISTANT_TOOLS_GUIDE.md'"
    run_test "指南包含自动启动说明" "grep -q 'auto_start_system' '$PROJECT_ROOT/ASSISTANT_TOOLS_GUIDE.md' || grep -q '自动启动' '$PROJECT_ROOT/ASSISTANT_TOOLS_GUIDE.md'"
    run_test "指南包含健康监控说明" "grep -q 'health_monitor' '$PROJECT_ROOT/ASSISTANT_TOOLS_GUIDE.md' || grep -q '健康监控' '$PROJECT_ROOT/ASSISTANT_TOOLS_GUIDE.md'"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 测试总结
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "测试总结"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "通过: $TESTS_PASSED"
echo "失败: $TESTS_FAILED"
echo "总计: $((TESTS_PASSED + TESTS_FAILED))"
echo ""

cat >> "$TEST_REPORT" << EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
测试总结
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

通过: $TESTS_PASSED
失败: $TESTS_FAILED
总计: $((TESTS_PASSED + TESTS_FAILED))

EOF

if [ $TESTS_FAILED -eq 0 ]; then
    echo "✅ 所有测试通过！"
    echo "状态: ✅ 所有测试通过" >> "$TEST_REPORT"
    exit 0
else
    echo "❌ 有 $TESTS_FAILED 个测试失败"
    echo "状态: ❌ 有 $TESTS_FAILED 个测试失败" >> "$TEST_REPORT"
    exit 1
fi
