#!/bin/bash

# OpenClaw+ 工作环境检查脚本
# 
# 检查所有必需的工具、依赖和配置
#
# Version: 1.0.0

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# 检查结果统计
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNING_CHECKS=0

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}OpenClaw+ 工作环境检查${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# 检查函数
check_command() {
    local cmd=$1
    local name=$2
    local required=$3
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if command -v "$cmd" &> /dev/null; then
        local version=$($cmd --version 2>&1 | head -n 1)
        echo -e "${GREEN}✓${NC} $name: $version"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        if [ "$required" = "true" ]; then
            echo -e "${RED}✗${NC} $name: 未安装 (必需)"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
            return 1
        else
            echo -e "${YELLOW}⚠${NC} $name: 未安装 (可选)"
            WARNING_CHECKS=$((WARNING_CHECKS + 1))
            return 2
        fi
    fi
}

check_file() {
    local file=$1
    local name=$2
    local required=$3
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if [ -f "$file" ]; then
        local size=$(ls -lh "$file" | awk '{print $5}')
        echo -e "${GREEN}✓${NC} $name: $file ($size)"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        if [ "$required" = "true" ]; then
            echo -e "${RED}✗${NC} $name: $file (未找到，必需)"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
            return 1
        else
            echo -e "${YELLOW}⚠${NC} $name: $file (未找到，可选)"
            WARNING_CHECKS=$((WARNING_CHECKS + 1))
            return 2
        fi
    fi
}

check_dir() {
    local dir=$1
    local name=$2
    local required=$3
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if [ -d "$dir" ]; then
        echo -e "${GREEN}✓${NC} $name: $dir"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        if [ "$required" = "true" ]; then
            echo -e "${RED}✗${NC} $name: $dir (未找到，必需)"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
            return 1
        else
            echo -e "${YELLOW}⚠${NC} $name: $dir (未找到，可选)"
            WARNING_CHECKS=$((WARNING_CHECKS + 1))
            return 2
        fi
    fi
}

# ============================================================================
# 1. Rust 工具链检查
# ============================================================================

echo -e "${BLUE}[1/8] Rust 工具链${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_command "rustc" "Rust 编译器" "true"
check_command "cargo" "Cargo 包管理器" "true"
check_command "rustup" "Rustup 工具链管理" "true"

# 检查 Rust 版本
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    # 检查是否 >= 1.70
    if [ "$(printf '%s\n' "1.70" "$RUST_VERSION" | sort -V | head -n1)" = "1.70" ]; then
        echo -e "${GREEN}✓${NC} Rust 版本兼容: $RUST_VERSION (>= 1.70)"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${YELLOW}⚠${NC} Rust 版本较旧: $RUST_VERSION (建议 >= 1.70)"
        WARNING_CHECKS=$((WARNING_CHECKS + 1))
    fi
fi

echo ""

# ============================================================================
# 2. WasmEdge 运行时检查
# ============================================================================

echo -e "${BLUE}[2/8] WasmEdge 运行时${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_command "wasmedge" "WasmEdge CLI" "true"

# 检查 WasmEdge 动态库
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
if [ -f "$HOME/.wasmedge/lib/libwasmedge.dylib" ] || [ -f "$HOME/.wasmedge/lib/libwasmedge.so" ]; then
    LIB_PATH="$HOME/.wasmedge/lib/libwasmedge.dylib"
    [ ! -f "$LIB_PATH" ] && LIB_PATH="$HOME/.wasmedge/lib/libwasmedge.so"
    SIZE=$(ls -lh "$LIB_PATH" | awk '{print $5}')
    echo -e "${GREEN}✓${NC} WasmEdge 动态库: $LIB_PATH ($SIZE)"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    echo -e "${RED}✗${NC} WasmEdge 动态库: 未找到"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

# 检查 QuickJS WASM
check_file "assets/wasmedge_quickjs.wasm" "QuickJS WASM 引擎" "true"

echo ""

# ============================================================================
# 3. Node.js 环境检查
# ============================================================================

echo -e "${BLUE}[3/8] Node.js 环境${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_command "node" "Node.js" "true"
check_command "npm" "npm 包管理器" "false"

echo ""

# ============================================================================
# 4. 系统工具检查
# ============================================================================

echo -e "${BLUE}[4/8] 系统工具${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_command "git" "Git 版本控制" "true"
check_command "bc" "bc 计算器" "true"
check_command "jq" "jq JSON 处理器" "false"
check_command "curl" "curl 下载工具" "true"

echo ""

# ============================================================================
# 5. 项目文件检查
# ============================================================================

echo -e "${BLUE}[5/8] 项目文件${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_file "Cargo.toml" "工作区配置" "true"
check_file ".cargo/config.toml" "Cargo 配置" "true"
check_file "crates/sandbox/Cargo.toml" "Sandbox crate" "true"
check_file "crates/security/Cargo.toml" "Security crate" "true"
check_file "crates/inference/Cargo.toml" "Inference crate" "true"

echo ""

# ============================================================================
# 6. 配置文件检查
# ============================================================================

echo -e "${BLUE}[6/8] 配置文件${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_file "config/security_profiles.toml" "安全配置文件" "true"
check_file "config/policy_engine.toml" "策略引擎配置" "true"

echo ""

# ============================================================================
# 7. JavaScript 技能文件检查
# ============================================================================

echo -e "${BLUE}[7/8] JavaScript 技能文件${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_file "assets/openclaw/sdk/skill_client.js" "技能客户端 SDK" "true"
check_file "assets/openclaw/skills/fs_skills.js" "文件系统技能" "true"
check_file "assets/openclaw/skills/web_skills.js" "网络技能" "true"

echo ""

# ============================================================================
# 8. 测试文件检查
# ============================================================================

echo -e "${BLUE}[8/8] 测试文件${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

check_file "tests/natural_language_test_suite.sh" "自然语言测试" "true"
check_file "tests/skill_system_integration_test.js" "JavaScript 集成测试" "true"
check_file "tests/mock_javascript_tests.sh" "JavaScript 模拟测试" "true"
check_file "tests/comprehensive_wasmedge_test.sh" "WasmEdge 功能测试" "true"
check_file "tests/run_all_tests.sh" "综合测试运行器" "true"

echo ""

# ============================================================================
# 环境变量检查
# ============================================================================

echo -e "${BLUE}环境变量${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
if [ -n "$DYLD_LIBRARY_PATH" ]; then
    echo -e "${GREEN}✓${NC} DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    echo -e "${YELLOW}⚠${NC} DYLD_LIBRARY_PATH: 未设置 (可能需要设置)"
    WARNING_CHECKS=$((WARNING_CHECKS + 1))
fi

TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
if [ -n "$PATH" ] && echo "$PATH" | grep -q ".cargo/bin"; then
    echo -e "${GREEN}✓${NC} PATH 包含 .cargo/bin"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    echo -e "${YELLOW}⚠${NC} PATH 可能不包含 .cargo/bin"
    WARNING_CHECKS=$((WARNING_CHECKS + 1))
fi

echo ""

# ============================================================================
# 磁盘空间检查
# ============================================================================

echo -e "${BLUE}系统资源${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
DISK_AVAIL=$(df -h . | tail -1 | awk '{print $4}')
echo -e "${GREEN}✓${NC} 可用磁盘空间: $DISK_AVAIL"
PASSED_CHECKS=$((PASSED_CHECKS + 1))

TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
if command -v free &> /dev/null; then
    MEM_AVAIL=$(free -h | grep Mem | awk '{print $7}')
    echo -e "${GREEN}✓${NC} 可用内存: $MEM_AVAIL"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
elif command -v vm_stat &> /dev/null; then
    # macOS
    FREE_PAGES=$(vm_stat | grep "Pages free" | awk '{print $3}' | sed 's/\.//')
    FREE_MB=$((FREE_PAGES * 4096 / 1024 / 1024))
    echo -e "${GREEN}✓${NC} 可用内存: ~${FREE_MB}MB"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
else
    echo -e "${YELLOW}⚠${NC} 无法检测内存"
    WARNING_CHECKS=$((WARNING_CHECKS + 1))
fi

echo ""

# ============================================================================
# 最终报告
# ============================================================================

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}检查结果汇总${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "总检查项: $TOTAL_CHECKS"
echo -e "${GREEN}通过: $PASSED_CHECKS${NC}"
echo -e "${YELLOW}警告: $WARNING_CHECKS${NC}"
echo -e "${RED}失败: $FAILED_CHECKS${NC}"

if [ $TOTAL_CHECKS -gt 0 ]; then
    SUCCESS_RATE=$(echo "scale=1; $PASSED_CHECKS * 100 / $TOTAL_CHECKS" | bc)
    echo "成功率: ${SUCCESS_RATE}%"
fi

echo ""

# 评估结果
if [ $FAILED_CHECKS -eq 0 ]; then
    if [ $WARNING_CHECKS -eq 0 ]; then
        echo -e "${GREEN}${BOLD}✅ 环境检查完美通过！${NC}"
        exit 0
    else
        echo -e "${GREEN}✅ 环境检查通过（有 $WARNING_CHECKS 个警告）${NC}"
        exit 0
    fi
else
    echo -e "${RED}❌ 环境检查失败（$FAILED_CHECKS 个必需项缺失）${NC}"
    echo ""
    echo -e "${YELLOW}请安装缺失的必需工具后重试${NC}"
    exit 1
fi
