#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ Agent 初始化脚本
# 将预定义的 Agent 配置文件复制到用户目录并创建工作空间
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
AGENTS_CONFIG_DIR="${PROJECT_ROOT}/agents"
USER_AGENTS_DIR="$HOME/.openclaw-plus/agents"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_success() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }

# 初始化单个 Agent
init_agent() {
    local config_file="$1"
    local agent_name=$(basename "$config_file" .toml)
    
    # 从配置文件中提取 agent_id
    local agent_id=$(grep '^id = ' "$config_file" | head -1 | sed 's/id = "\(.*\)"/\1/')
    
    if [[ -z "$agent_id" ]]; then
        log_warn "Could not extract agent_id from $config_file"
        return 1
    fi
    
    local agent_dir="${USER_AGENTS_DIR}/${agent_id}"
    
    # 创建 Agent 目录结构
    mkdir -p "$agent_dir/workspace"
    mkdir -p "$agent_dir/logs"
    mkdir -p "$agent_dir/cache"
    
    # 复制配置文件
    cp "$config_file" "$agent_dir/profile.toml"
    
    # 创建 README
    cat > "$agent_dir/README.md" << EOF
# ${agent_name}

Agent ID: \`${agent_id}\`

## 目录结构

- \`profile.toml\` - Agent 配置文件
- \`workspace/\` - Agent 工作空间
- \`logs/\` - 运行日志
- \`cache/\` - 缓存数据

## 使用方法

在 OpenClaw+ UI 中选择此 Agent 并启动，或使用命令行：

\`\`\`bash
openclaw-plus run ${agent_id}
\`\`\`

## 配置修改

编辑 \`profile.toml\` 文件来修改 Agent 配置。
EOF
    
    log_success "Initialized agent: ${agent_name} (${agent_id})"
}

# 主函数
main() {
    log_info "Initializing OpenClaw+ agents..."
    echo ""
    
    # 确保目标目录存在
    mkdir -p "$USER_AGENTS_DIR"
    
    # 检查配置目录
    if [[ ! -d "$AGENTS_CONFIG_DIR" ]]; then
        log_warn "Agents config directory not found: $AGENTS_CONFIG_DIR"
        return 1
    fi
    
    # 初始化所有 Agent
    local count=0
    for config_file in "$AGENTS_CONFIG_DIR"/*.toml; do
        if [[ -f "$config_file" ]]; then
            init_agent "$config_file"
            ((count++))
        fi
    done
    
    echo ""
    log_success "Initialized $count agents in $USER_AGENTS_DIR"
    log_info "You can now manage these agents in the OpenClaw+ UI"
}

main "$@"
