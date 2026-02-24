#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ Agent 验证脚本
# 验证所有已初始化的 Agent 配置是否正确
# =============================================================================

set -euo pipefail

USER_AGENTS_DIR="$HOME/.openclaw-plus/agents"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_success() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }

# 验证单个 Agent
verify_agent() {
    local agent_dir="$1"
    local agent_id=$(basename "$agent_dir")
    
    log_info "Verifying agent: $agent_id"
    
    # 检查必需文件
    if [[ ! -f "$agent_dir/profile.toml" ]]; then
        log_error "Missing profile.toml"
        return 1
    fi
    
    # 检查必需目录
    for dir in workspace logs cache; do
        if [[ ! -d "$agent_dir/$dir" ]]; then
            log_warn "Missing directory: $dir"
        fi
    done
    
    # 提取配置信息
    local display_name=$(grep '^display_name = ' "$agent_dir/profile.toml" | head -1 | sed 's/display_name = "\(.*\)"/\1/')
    local role=$(grep '^role = ' "$agent_dir/profile.toml" | head -1 | sed 's/role = "\(.*\)"/\1/')
    local memory_limit=$(grep '^memory_limit_mb = ' "$agent_dir/profile.toml" | head -1 | sed 's/memory_limit_mb = \(.*\)/\1/')
    
    echo "  Display Name: $display_name"
    echo "  Role: $role"
    echo "  Memory Limit: ${memory_limit}MB"
    
    # 统计能力数量
    local capabilities_count=$(grep -c '^\[\[agent.capabilities\]\]' "$agent_dir/profile.toml" || echo "0")
    echo "  Capabilities: $capabilities_count"
    
    # 统计通信渠道
    local channels_count=$(grep -c '^\[\[agent.channels\]\]' "$agent_dir/profile.toml" || echo "0")
    echo "  Channels: $channels_count"
    
    log_success "Agent verified: $display_name"
    echo ""
}

# 主函数
main() {
    log_info "Verifying OpenClaw+ agents..."
    echo ""
    
    if [[ ! -d "$USER_AGENTS_DIR" ]]; then
        log_error "Agents directory not found: $USER_AGENTS_DIR"
        return 1
    fi
    
    local count=0
    local verified=0
    
    for agent_dir in "$USER_AGENTS_DIR"/*; do
        if [[ -d "$agent_dir" ]]; then
            ((count++))
            if verify_agent "$agent_dir"; then
                ((verified++))
            fi
        fi
    done
    
    echo ""
    log_success "Verified $verified/$count agents"
    
    if [[ $verified -eq $count ]]; then
        log_success "All agents are properly configured! ✅"
    else
        log_warn "Some agents have issues, please check the output above"
    fi
}

main "$@"
