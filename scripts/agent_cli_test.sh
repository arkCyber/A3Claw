#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ Agent 命令行测试工具
# 手动向数字员工发送消息并测试功能
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
USER_AGENTS_DIR="$HOME/.openclaw-plus/agents"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_success() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_agent() { echo -e "${CYAN}[AGENT]${NC} $*"; }

# 显示使用帮助
show_help() {
    cat << EOF
OpenClaw+ Agent 命令行测试工具

用法:
  $0 [选项] <agent_id> <message>

选项:
  -h, --help          显示此帮助信息
  -l, --list          列出所有可用的 Agent
  -s, --status        显示 Agent 状态
  -c, --config        显示 Agent 配置
  -t, --test          运行预定义测试任务
  -i, --interactive   进入交互模式

示例:
  $0 -l                           # 列出所有 Agent
  $0 -s code-reviewer-001         # 查看代码审查员状态
  $0 code-reviewer-001 "审查这段代码"  # 发送消息给代码审查员
  $0 -t data-analyst-001          # 运行数据分析师测试
  $0 -i                           # 交互模式

专业 Agent 列表:
  - code-reviewer-001      代码审查员 Alpha
  - data-analyst-001       数据分析师 Insight
  - knowledge-officer-001  知识库首席官 Librarian
  - report-generator-001   报告生成器 Scribe
  - security-auditor-001   安全审计员 Guardian
EOF
}

# 列出所有 Agent
list_agents() {
    log_info "可用的数字员工列表:"
    echo ""
    
    local professional_agents=(
        "code-reviewer-001:代码审查员 Alpha"
        "data-analyst-001:数据分析师 Insight"
        "knowledge-officer-001:知识库首席官 Librarian"
        "report-generator-001:报告生成器 Scribe"
        "security-auditor-001:安全审计员 Guardian"
    )
    
    for agent_info in "${professional_agents[@]}"; do
        local agent_id="${agent_info%%:*}"
        local agent_name="${agent_info##*:}"
        
        if [[ -d "$USER_AGENTS_DIR/$agent_id" ]]; then
            local profile_file="$USER_AGENTS_DIR/$agent_id/profile.toml"
            if [[ -f "$profile_file" ]]; then
                local role=$(grep '^role = ' "$profile_file" | head -1 | sed 's/role = "\(.*\)"/\1/')
                local memory=$(grep '^memory_limit_mb = ' "$profile_file" | head -1 | sed 's/memory_limit_mb = \(.*\)/\1/')
                local capabilities=$(grep -c '^\[\[agent.capabilities\]\]' "$profile_file" || echo "0")
                
                echo "  🤖 $agent_name"
                echo "     ID: $agent_id"
                echo "     角色: $role"
                echo "     内存: ${memory}MB"
                echo "     能力: $capabilities 个"
                echo ""
            fi
        fi
    done
    
    log_info "历史测试 Agent: $(find "$USER_AGENTS_DIR" -name "00000000-*" -type d | wc -l | tr -d ' ') 个"
    log_info "Agent 总数: $(find "$USER_AGENTS_DIR" -maxdepth 1 -type d | wc -l | tr -d ' ') 个"
}

# 显示 Agent 状态
show_status() {
    local agent_id="$1"
    local agent_dir="$USER_AGENTS_DIR/$agent_id"
    
    if [[ ! -d "$agent_dir" ]]; then
        log_error "Agent 不存在: $agent_id"
        return 1
    fi
    
    log_info "Agent 状态: $agent_id"
    echo ""
    
    local profile_file="$agent_dir/profile.toml"
    if [[ -f "$profile_file" ]]; then
        echo "📋 基本信息:"
        grep -E '^(display_name|role|status)' "$profile_file" | sed 's/^/  /'
        echo ""
        
        echo "💾 资源配置:"
        grep -E '^(memory_limit_mb|intercept_shell)' "$profile_file" | sed 's/^/  /'
        echo ""
        
        echo "🌐 网络白名单:"
        grep -A 10 '^network_allowlist' "$profile_file" | grep -v '^network_allowlist' | grep '"' | sed 's/^/  /'
        echo ""
        
        echo "🔧 能力列表:"
        grep -A 2 '^\[\[agent.capabilities\]\]' "$profile_file" | grep 'name = ' | sed 's/^/  - /'
        echo ""
        
        echo "📊 统计信息:"
        grep -E '^(total_runs|successful_runs|failed_runs)' "$profile_file" | sed 's/^/  /'
    fi
    
    # 检查工作空间
    local workspace="$agent_dir/workspace"
    if [[ -d "$workspace" ]]; then
        echo "📁 工作空间文件:"
        ls -la "$workspace" 2>/dev/null | tail -n +2 | sed 's/^/  /'
    else
        log_warn "工作空间目录不存在"
    fi
}

# 显示 Agent 配置
show_config() {
    local agent_id="$1"
    local profile_file="$USER_AGENTS_DIR/$agent_id/profile.toml"
    
    if [[ ! -f "$profile_file" ]]; then
        log_error "Agent 配置文件不存在: $agent_id"
        return 1
    fi
    
    log_info "Agent 配置: $agent_id"
    echo ""
    cat "$profile_file"
}

# 发送消息给 Agent
send_message() {
    local agent_id="$1"
    local message="$2"
    
    log_info "发送消息给 Agent: $agent_id"
    log_agent "消息: $message"
    echo ""
    
    # 检查 Agent 是否存在
    local agent_dir="$USER_AGENTS_DIR/$agent_id"
    if [[ ! -d "$agent_dir" ]]; then
        log_error "Agent 不存在: $agent_id"
        return 1
    fi
    
    # 模拟 Agent 响应（实际实现需要调用 OpenClaw+ API）
    log_agent "正在处理消息..."
    
    # 根据角色生成模拟响应
    local profile_file="$agent_dir/profile.toml"
    local role=$(grep '^role = ' "$profile_file" | head -1 | sed 's/role = "\(.*\)"/\1/')
    
    case "$role" in
        "code_reviewer")
            log_agent "🔍 代码审查分析中..."
            sleep 1
            log_agent "✅ 审查完成！"
            echo ""
            echo "📝 审查报告:"
            echo "  - 代码结构: 良好"
            echo "  - 潜在问题: 无"
            echo "  - 改进建议: 建议添加更多注释"
            echo "  - 安全检查: 通过"
            ;;
        "data_analyst")
            log_agent "📊 数据分析中..."
            sleep 1
            log_agent "✅ 分析完成！"
            echo ""
            echo "📈 分析结果:"
            echo "  - 数据质量: 优秀"
            echo "  - 趋势分析: 上升"
            echo "  - 异常检测: 无异常"
            echo "  - 建议: 继续监控"
            ;;
        "knowledge_officer")
            log_agent "🔍 知识检索中..."
            sleep 1
            log_agent "✅ 检索完成！"
            echo ""
            echo "📚 知识库结果:"
            echo "  - 找到相关文档: 3 篇"
            echo "  - 匹配度: 95%"
            echo "  - 关键信息: 已提取"
            echo "  - 来源: 技术文档库"
            ;;
        "report_generator")
            log_agent "📝 报告生成中..."
            sleep 1
            log_agent "✅ 生成完成！"
            echo ""
            echo "📄 报告摘要:"
            echo "  - 报告类型: 状态报告"
            echo "  - 数据范围: 本周"
            echo "  - 关键指标: 正常"
            echo "  - 生成格式: PDF"
            ;;
        "security_auditor")
            log_agent "🛡️ 安全扫描中..."
            sleep 1
            log_agent "✅ 扫描完成！"
            echo ""
            echo "🔒 安全评估:"
            echo "  - 漏洞扫描: 无高危漏洞"
            echo "  - 权限检查: 正常"
            echo "  - 合规性: 符合标准"
            echo "  - 风险等级: 低"
            ;;
        *)
            log_agent "💬 通用响应..."
            sleep 1
            log_agent "✅ 处理完成！"
            echo ""
            echo "📤 响应:"
            echo "  收到消息: $message"
            echo "  处理状态: 成功"
            echo "  时间戳: $(date)"
            ;;
    esac
    
    echo ""
    log_success "消息处理完成"
}

# 运行预定义测试
run_test() {
    local agent_id="$1"
    
    log_info "运行 Agent 测试: $agent_id"
    echo ""
    
    local profile_file="$USER_AGENTS_DIR/$agent_id/profile.toml"
    local role=$(grep '^role = ' "$profile_file" | head -1 | sed 's/role = "\(.*\)"/\1/')
    
    case "$role" in
        "code_reviewer")
            send_message "$agent_id" "请审查这段 Python 代码：def hello(): print('Hello, World!')"
            ;;
        "data_analyst")
            send_message "$agent_id" "分析这组数据的趋势：[1, 3, 5, 7, 9, 11]"
            ;;
        "knowledge_officer")
            send_message "$agent_id" "查找关于 Rust 编程的最佳实践文档"
            ;;
        "report_generator")
            send_message "$agent_id" "生成本周项目进度报告"
            ;;
        "security_auditor")
            send_message "$agent_id" "扫描当前目录的安全配置"
            ;;
        *)
            send_message "$agent_id" "这是一个测试消息，请确认收到"
            ;;
    esac
}

# 交互模式
interactive_mode() {
    log_info "进入交互模式"
    log_info "输入 'help' 查看命令，输入 'quit' 退出"
    echo ""
    
    while true; do
        echo -n "${CYAN}Agent CLI>${NC} "
        read -r input
        
        case "$input" in
            "quit"|"exit")
                log_info "退出交互模式"
                break
                ;;
            "help")
                echo "可用命令:"
                echo "  list              - 列出所有 Agent"
                echo "  status <agent_id> - 查看 Agent 状态"
                echo "  config <agent_id> - 查看 Agent 配置"
                echo "  test <agent_id>  - 运行测试"
                echo "  <agent_id> <msg> - 发送消息给 Agent"
                echo "  quit              - 退出"
                ;;
            "list")
                list_agents
                ;;
            status*)
                local agent_id=$(echo "$input" | cut -d' ' -f2)
                if [[ -n "$agent_id" ]]; then
                    show_status "$agent_id"
                else
                    log_error "请指定 Agent ID"
                fi
                ;;
            config*)
                local agent_id=$(echo "$input" | cut -d' ' -f2)
                if [[ -n "$agent_id" ]]; then
                    show_config "$agent_id"
                else
                    log_error "请指定 Agent ID"
                fi
                ;;
            test*)
                local agent_id=$(echo "$input" | cut -d' ' -f2)
                if [[ -n "$agent_id" ]]; then
                    run_test "$agent_id"
                else
                    log_error "请指定 Agent ID"
                fi
                ;;
            "")
                continue
                ;;
            *)
                local agent_id=$(echo "$input" | cut -d' ' -f1)
                local message=$(echo "$input" | cut -d' ' -f2-)
                if [[ -n "$agent_id" && -n "$message" ]]; then
                    send_message "$agent_id" "$message"
                else
                    log_error "格式: <agent_id> <message>"
                fi
                ;;
        esac
        echo ""
    done
}

# 主函数
main() {
    case "${1:-}" in
        -h|--help)
            show_help
            ;;
        -l|--list)
            list_agents
            ;;
        -s|--status)
            if [[ -z "${2:-}" ]]; then
                log_error "请指定 Agent ID"
                exit 1
            fi
            show_status "$2"
            ;;
        -c|--config)
            if [[ -z "${2:-}" ]]; then
                log_error "请指定 Agent ID"
                exit 1
            fi
            show_config "$2"
            ;;
        -t|--test)
            if [[ -z "${2:-}" ]]; then
                log_error "请指定 Agent ID"
                exit 1
            fi
            run_test "$2"
            ;;
        -i|--interactive)
            interactive_mode
            ;;
        "")
            show_help
            ;;
        *)
            if [[ -z "${2:-}" ]]; then
                log_error "请提供消息内容"
                show_help
                exit 1
            fi
            send_message "$1" "$2"
            ;;
    esac
}

main "$@"
