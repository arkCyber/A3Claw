#!/bin/bash
# IME 输入测试脚本 - 独立运行模式
# 用于验证独立运行是否能解决 IME 焦点问题

set -e

echo "=========================================="
echo "OpenClaw+ IME 输入测试 - 独立运行模式"
echo "=========================================="
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 检查二进制文件
BINARY="./target/release/openclaw-plus"
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}错误: 找不到编译后的二进制文件${NC}"
    echo "请先运行: cargo build --release -p openclaw-ui"
    exit 1
fi

echo -e "${GREEN}✓ 找到二进制文件: $BINARY${NC}"
echo ""

# 停止所有运行中的实例
echo -e "${YELLOW}停止所有运行中的 openclaw-plus 实例...${NC}"
pkill -f openclaw-plus 2>/dev/null || true
sleep 1
echo -e "${GREEN}✓ 已停止${NC}"
echo ""

# 清空日志文件
LOG_FILE="/tmp/openclaw_ime_standalone.log"
> "$LOG_FILE"
echo -e "${GREEN}✓ 日志文件已清空: $LOG_FILE${NC}"
echo ""

# 启动应用
echo -e "${BLUE}=========================================="
echo "启动 OpenClaw+ (独立模式)"
echo "==========================================${NC}"
echo ""
echo -e "${YELLOW}注意: 应用将在独立窗口中运行${NC}"
echo -e "${YELLOW}      不要在 Windsurf 中运行此脚本${NC}"
echo ""

# 启动应用并记录日志
echo -e "${GREEN}启动中...${NC}"
"$BINARY" 2>&1 | tee -a "$LOG_FILE" &
APP_PID=$!

sleep 2

# 检查应用是否成功启动
if ! ps -p $APP_PID > /dev/null 2>&1; then
    echo -e "${RED}错误: 应用启动失败${NC}"
    echo "请查看日志: $LOG_FILE"
    exit 1
fi

echo -e "${GREEN}✓ 应用已启动 (PID: $APP_PID)${NC}"
echo ""

# 显示测试说明
echo -e "${BLUE}=========================================="
echo "IME 输入测试步骤"
echo "==========================================${NC}"
echo ""
echo -e "${YELLOW}1. 进入 Claw Terminal 页面${NC}"
echo "   - 点击左侧导航栏的 'Claw Terminal' 图标"
echo ""
echo -e "${YELLOW}2. 点击输入框${NC}"
echo "   - 确认输入框边框变为浅蓝色（表示获得焦点）"
echo ""
echo -e "${YELLOW}3. 切换中文输入法${NC}"
echo "   - macOS: Ctrl+Space 或点击输入法图标"
echo "   - 切换到搜狗拼音或系统自带中文输入法"
echo ""
echo -e "${YELLOW}4. 输入拼音${NC}"
echo "   - 例如输入: nihao"
echo "   - 观察 IME 候选框是否显示"
echo ""
echo -e "${YELLOW}5. 选择汉字${NC}"
echo "   - 从候选框中选择 '你好'"
echo "   - 观察汉字出现在哪里"
echo ""
echo -e "${BLUE}=========================================="
echo "预期结果"
echo "==========================================${NC}"
echo ""
echo -e "${GREEN}✓ IME 候选框正常显示${NC}"
echo -e "${GREEN}✓ 汉字出现在 OpenClaw+ 的 Terminal 输入框内${NC}"
echo -e "${GREEN}✓ 汉字不会出现在 Windsurf 输入框（因为独立运行）${NC}"
echo ""

# 监控日志
echo -e "${BLUE}=========================================="
echo "实时日志监控"
echo "==========================================${NC}"
echo ""
echo -e "${YELLOW}按 Ctrl+C 停止监控日志${NC}"
echo ""

# 等待用户按键
read -p "按 Enter 开始监控日志..." -r

# 监控日志中的 IME 相关消息
echo ""
echo -e "${GREEN}监控 IME 相关日志...${NC}"
echo ""
tail -f "$LOG_FILE" | grep --line-buffered "IME" &
TAIL_PID=$!

# 等待用户中断
trap "kill $TAIL_PID 2>/dev/null; exit 0" INT TERM

wait $TAIL_PID
