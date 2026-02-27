#!/bin/bash
# 智能体聊天测试脚本
# 用于验证 Claw Terminal 向智能体发消息的完整流程

set -e

echo "=========================================="
echo "OpenClaw+ 智能体聊天测试"
echo "=========================================="
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 检查应用是否运行
if ! pgrep -f "openclaw-plus" > /dev/null; then
    echo -e "${RED}错误: OpenClaw+ 应用未运行${NC}"
    echo "请先启动应用: open /tmp/OpenClawPlus.app"
    exit 1
fi

echo -e "${GREEN}✓ OpenClaw+ 应用正在运行${NC}"
echo ""

# 清空日志文件
LOG_FILE="/tmp/openclaw.log"
> "$LOG_FILE"
echo -e "${GREEN}✓ 日志文件已清空: $LOG_FILE${NC}"
echo ""

# 显示测试步骤
echo -e "${BLUE}=========================================="
echo "测试步骤"
echo "==========================================${NC}"
echo ""
echo -e "${YELLOW}1. 进入 Claw Terminal 页面${NC}"
echo "   - 点击左侧导航栏的 'Claw Terminal' 图标"
echo ""
echo -e "${YELLOW}2. 选择智能体${NC}"
echo "   - 点击智能体选择器（默认显示 '选择数字员工'）"
echo "   - 选择一个智能体，例如 '知识库首席官 Librarian'"
echo ""
echo -e "${YELLOW}3. 发送测试消息${NC}"
echo "   - 在输入框中输入: '你好，请介绍一下你的功能'"
echo "   - 点击发送按钮或按 Enter"
echo ""
echo -e "${YELLOW}4. 观察响应${NC}"
echo "   - 智能体应该回复中文消息"
echo "   - 消息应该显示在聊天历史中"
echo ""
echo -e "${BLUE}=========================================="
echo "预期结果"
echo "==========================================${NC}"
echo ""
echo -e "${GREEN}✓ 智能体选择器显示选中的智能体名称${NC}"
echo -e "${GREEN}✓ 用户消息显示为 '[智能体名称] 你好，请介绍一下你的功能'${NC}"
echo -e "${GREEN}✓ 智能体回复显示为 '🤖 智能体名称' 开头的消息${NC}"
echo -e "${GREEN}✓ 消息状态从 'Running' 变为 'Success'${NC}"
echo ""

# 监控日志
echo -e "${BLUE}=========================================="
echo "实时日志监控"
echo "==========================================${NC}"
echo ""
echo -e "${YELLOW}按 Ctrl+C 停止监控日志${NC}"
echo ""
echo -e "${YELLOW}关键日志消息:${NC}"
echo "  - [CLAW] Agent selected: ..."
echo "  - [CLAW] Routing to agent chat"
echo "  - [CLAW-AGENT] init fresh engine: ..."
echo "  - [CLAW-AGENT] sending X messages to agent ..."
echo "  - [CLAW-AGENT] response (...) ms: ..."
echo ""

# 等待用户按键
read -p "按 Enter 开始监控日志..." -r

# 监控日志中的关键消息
echo ""
echo -e "${GREEN}开始监控日志...${NC}"
echo ""
tail -f "$LOG_FILE" | grep --line-buffered -E "\[CLAW\]|\[CLAW-AGENT\]" &
TAIL_PID=$!

# 等待用户中断
trap "kill $TAIL_PID 2>/dev/null; exit 0" INT TERM

wait $TAIL_PID
