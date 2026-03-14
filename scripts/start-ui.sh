#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ UI 启动脚本
# 
# ⚠️  警告：此脚本直接运行二进制文件，中文输入法可能不工作！
# ✅  推荐使用：./scripts/run.sh（创建 .app bundle，支持中文输入）
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

# 检查前置条件
log_info "检查前置条件..."

# 1. 检查 Ollama 服务
if ! curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    log_warn "Ollama 服务未运行，正在启动..."
    "${SCRIPT_DIR}/start-ollama.sh"
    sleep 3
fi

# 2. 检查配置文件
CONFIG_PATH="${HOME}/.config/openclaw-plus/config.toml"
if [ ! -f "${CONFIG_PATH}" ]; then
    log_warn "配置文件不存在，正在创建..."
    mkdir -p "$(dirname "${CONFIG_PATH}")"
    cp "${PROJECT_ROOT}/config/default.toml" "${CONFIG_PATH}"
    log_ok "配置文件已创建: ${CONFIG_PATH}"
fi

# 3. 检查 OpenClaw 入口文件
OPENCLAW_ENTRY=$(grep "openclaw_entry" "${CONFIG_PATH}" | cut -d'"' -f2 | sed 's|^//|/|')
if [ ! -f "${OPENCLAW_ENTRY}" ]; then
    log_error "OpenClaw 入口文件不存在: ${OPENCLAW_ENTRY}"
    log_info "请运行 ./scripts/bundle_openclaw.sh"
    exit 1
fi

log_ok "前置条件检查完成"

# 启动 UI
log_info "启动 OpenClaw+ UI..."
cd "${PROJECT_ROOT}"

# 设置环境变量
export RUST_LOG=info
export RUST_BACKTRACE=1

# 启动应用
cargo run --release -p openclaw-ui

log_ok "OpenClaw+ UI 已启动"
