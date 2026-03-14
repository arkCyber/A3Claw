#!/usr/bin/env bash
# OpenClaw+ 安全监控脚本

echo "OpenClaw+ 安全监控报告"
echo "生成时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "=================================="

# 检查配置文件
USER_CONFIG="${HOME}/.config/openclaw-plus/config.toml"
if [ -f "${USER_CONFIG}" ]; then
    echo "✓ 配置文件存在"
    CONFIG_PERMS=$(stat -f "%Lp" "${USER_CONFIG}" 2>/dev/null || stat -c "%a" "${USER_CONFIG}" 2>/dev/null)
    echo "  权限: ${CONFIG_PERMS}"
else
    echo "✗ 配置文件不存在"
fi

# 检查工作目录
WORKSPACE_DIR="${HOME}/.openclaw-plus/workspace"
if [ -d "${WORKSPACE_DIR}" ]; then
    echo "✓ 工作目录存在"
    DIR_PERMS=$(stat -f "%Lp" "${WORKSPACE_DIR}" 2>/dev/null || stat -c "%a" "${WORKSPACE_DIR}" 2>/dev/null)
    echo "  权限: ${DIR_PERMS}"
else
    echo "✗ 工作目录不存在"
fi

# 检查数字员工配置
AGENTS_DIR="${PROJECT_ROOT}/agents"
if [ -d "${AGENTS_DIR}" ]; then
    AGENT_COUNT=$(ls -1 "${AGENTS_DIR}"/*.toml 2>/dev/null | wc -l | tr -d ' ')
    echo "✓ 数字员工配置: ${AGENT_COUNT} 个"
else
    echo "✗ 数字员工配置目录不存在"
fi

echo ""
echo "安全监控完成"
