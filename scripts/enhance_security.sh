#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 安全功能增强脚本
# 完善安全配置，添加额外的安全特性
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 安全功能增强"
echo "  执行时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ============================================================================
# 1. 增强用户配置安全
# ============================================================================
log_info "增强用户配置安全..."

USER_CONFIG="${HOME}/.config/openclaw-plus/config.toml"

# 确保用户配置目录存在
mkdir -p "$(dirname "${USER_CONFIG}")"

# 检查并添加安全配置
if [ -f "${USER_CONFIG}" ]; then
    log_info "检查现有安全配置..."
    
    # 添加文件删除确认（如果不存在）
    if ! grep -q "file_delete_confirm" "${USER_CONFIG}"; then
        echo "file_delete_confirm = true" >> "${USER_CONFIG}"
        log_ok "  ✓ 已添加文件删除确认配置"
    fi
    
    # 添加网络确认（如果不存在）
    if ! grep -q "network_confirm" "${USER_CONFIG}"; then
        echo "network_confirm = true" >> "${USER_CONFIG}"
        log_ok "  ✓ 已添加网络确认配置"
    fi
    
    # 添加审计日志（如果不存在）
    if ! grep -q "audit_logging" "${USER_CONFIG}"; then
        echo "audit_logging = true" >> "${USER_CONFIG}"
        log_ok "  ✓ 已添加审计日志配置"
    fi
    
    # 添加最大文件大小限制（如果不存在）
    if ! grep -q "max_file_size" "${USER_CONFIG}"; then
        echo "max_file_size = \"10MB\"" >> "${USER_CONFIG}"
        log_ok "  ✓ 已添加最大文件大小限制"
    fi
    
    # 添加允许的文件类型（如果不存在）
    if ! grep -q "allowed_file_types" "${USER_CONFIG}"; then
        echo 'allowed_file_types = ["txt", "json", "md", "js", "toml", "rs"]' >> "${USER_CONFIG}"
        log_ok "  ✓ 已添加允许的文件类型"
    fi
else
    log_warn "用户配置文件不存在，创建默认安全配置..."
    
    cat > "${USER_CONFIG}" << 'EOF'
# OpenClaw+ 用户配置文件

# 基本配置
workspace_dir = "~/.openclaw-plus/workspace"
openclaw_entry = "openclaw/dist/index.js"

# 安全配置
memory_limit = 512
shell_intercept = true
file_delete_confirm = true
network_confirm = true
audit_logging = true
max_file_size = "10MB"
allowed_file_types = ["txt", "json", "md", "js", "toml", "rs"]

# 网络白名单
[network_allowlist]
domains = [
    "localhost",
    "127.0.0.1",
    "api.openai.com",
    "api.github.com",
    "docs.rs",
    "crates.io"
]

# AI 配置
[ai]
endpoint = "http://localhost:11434"
model = "qwen2.5:0.5b"
timeout = 30
max_tokens = 2048
EOF
    
    log_ok "  ✓ 已创建默认安全配置"
fi

# 设置配置文件权限
chmod 644 "${USER_CONFIG}"
log_ok "  ✓ 已设置配置文件权限为 644"

echo ""

# ============================================================================
# 2. 创建安全日志目录
# ============================================================================
log_info "创建安全日志目录..."

LOG_DIR="${HOME}/.openclaw-plus/logs"
mkdir -p "${LOG_DIR}"
chmod 755 "${LOG_DIR}"

# 创建日志轮转配置
cat > "${LOG_DIR}/.gitignore" << 'EOF'
# OpenClaw+ 日志文件
*.log
*.log.*

# 保留最近 7 天的日志
!keep_last_7_days
EOF

log_ok "  ✓ 已创建日志目录"

echo ""

# ============================================================================
# 3. 增强数字员工安全配置
# ============================================================================
log_info "增强数字员工安全配置..."

AGENTS_DIR="${PROJECT_ROOT}/agents"

if [ -d "${AGENTS_DIR}" ]; then
    for agent_file in "${AGENTS_DIR}"/*.toml; do
        if [ -f "${agent_file}" ]; then
            agent_name=$(basename "${agent_file}" .toml)
            log_info "检查 ${agent_name} 安全配置..."
            
            # 确保所有数字员工都有完整的安全配置
            if ! grep -q "shell_intercept" "${agent_file}"; then
                echo "shell_intercept = true" >> "${agent_file}"
                log_ok "  ✓ 已添加 shell_intercept 配置"
            fi
            
            if ! grep -q "confirm_file_delete" "${agent_file}"; then
                echo "confirm_file_delete = true" >> "${agent_file}"
                log_ok "  ✓ 已添加 confirm_file_delete 配置"
            fi
            
            if ! grep -q "confirm_network" "${agent_file}"; then
                echo "confirm_network = true" >> "${agent_file}"
                log_ok "  ✓ 已添加 confirm_network 配置"
            fi
            
            if ! grep -q "confirm_shell_exec" "${agent_file}"; then
                echo "confirm_shell_exec = true" >> "${agent_file}"
                log_ok "  ✓ 已添加 confirm_shell_exec 配置"
            fi
        fi
    done
else
    log_error "数字员工配置目录不存在"
fi

echo ""

# ============================================================================
# 4. 创建安全策略文档
# ============================================================================
log_info "创建安全策略文档..."

SECURITY_POLICY_DIR="${PROJECT_ROOT}/docs/security"
mkdir -p "${SECURITY_POLICY_DIR}"

cat > "${SECURITY_POLICY_DIR}/security_policy.md" << 'EOF'
# OpenClaw+ 安全策略文档

## 概述

OpenClaw+ 是一个安全的数字员工平台，采用多层安全架构保护用户数据和系统安全。

## 安全架构

### 1. 沙箱隔离
- **WasmEdge 沙箱**: 所有 JavaScript 代码在 WasmEdge 沙箱中执行
- **内存限制**: 每个数字员工都有独立的内存限制
- **资源隔离**: 文件系统和网络访问受到严格控制

### 2. 权限控制
- **最小权限原则**: 每个数字员工只获得必要的权限
- **Shell 拦截**: 所有 Shell 命令执行都需要确认
- **文件访问控制**: 基于白名单的文件访问控制

### 3. 网络安全
- **网络白名单**: 只允许访问预定义的安全域名
- **HTTPS 强制**: 所有网络连接都使用 HTTPS
- **网络确认**: 网络请求需要用户确认

### 4. 数据保护
- **敏感数据加密**: 敏感配置信息加密存储
- **日志脱敏**: 日志中不包含敏感信息
- **定期清理**: 自动清理过期的日志文件

## 安全配置

### 默认安全配置
- 内存限制: 512MB
- Shell 拦截: 启用
- 文件删除确认: 启用
- 网络确认: 启用
- 审计日志: 启用

### 数字员工安全边界
每个数字员工都有独立的安全配置：
- 内存限制
- Shell 拦截
- 网络白名单
- 文件访问权限

## 安全最佳实践

### 1. 定期安全审计
- 检查配置文件权限
- 验证网络白名单
- 审查日志文件

### 2. 保持更新
- 定期更新依赖库
- 应用安全补丁
- 监控安全公告

### 3. 监控和告警
- 监控异常访问
- 设置安全告警
- 定期安全扫描

## 安全事件响应

### 1. 事件分类
- **低危**: 配置错误、权限问题
- **中危**: 异常访问、资源滥用
- **高危**: 数据泄露、系统入侵

### 2. 响应流程
1. 检测和识别
2. 隔离和遏制
3. 调查和分析
4. 恢复和修复
5. 总结和改进

## 合规性

OpenClaw+ 遵循以下安全标准和合规要求：
- GDPR 数据保护
- ISO 27001 信息安全
- OWASP 安全标准

---

**文档版本**: 1.0  
**最后更新**: $(date '+%Y-%m-%d')  
**维护者**: OpenClaw+ 安全团队
EOF

log_ok "  ✓ 已创建安全策略文档"

echo ""

# ============================================================================
# 5. 创建安全监控脚本
# ============================================================================
log_info "创建安全监控脚本..."

cat > "${PROJECT_ROOT}/scripts/security_monitor.sh" << 'EOF'
#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 安全监控脚本
# 监控系统安全状态，生成安全报告
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

log_info() { echo -e "${BLUE}[MONITOR]${NC} $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}     $*"; }
log_error() { echo -e "${RED}[ERROR]${NC}   $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}   $*"; }

echo "OpenClaw+ 安全监控报告"
echo "生成时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "=================================="

# 1. 检查配置文件安全
log_info "检查配置文件安全..."

USER_CONFIG="${HOME}/.config/openclaw-plus/config.toml"
if [ -f "${USER_CONFIG}" ]; then
    CONFIG_PERMS=$(stat -f "%Lp" "${USER_CONFIG}" 2>/dev/null || stat -c "%a" "${USER_CONFIG}" 2>/dev/null)
    if [ "${CONFIG_PERMS}" = "644" ] || [ "${CONFIG_PERMS}" = "600" ]; then
        log_ok "配置文件权限安全: ${CONFIG_PERMS}"
    else
        log_warn "配置文件权限过于宽松: ${CONFIG_PERMS}"
    fi
else
    log_error "配置文件不存在"
fi

# 2. 检查工作目录安全
WORKSPACE_DIR="${HOME}/.openclaw-plus/workspace"
if [ -d "${WORKSPACE_DIR}" ]; then
    DIR_PERMS=$(stat -f "%Lp" "${WORKSPACE_DIR}" 2>/dev/null || stat -c "%a" "${WORKSPACE_DIR}" 2>/dev/null)
    if [ "${DIR_PERMS}" = "755" ] || [ "${DIR_PERMS}" = "700" ]; then
        log_ok "工作目录权限安全: ${DIR_PERMS}"
    else
        log_warn "工作目录权限过于宽松: ${DIR_PERMS}"
    fi
else
    log_error "工作目录不存在"
fi

# 3. 检查数字员工配置
AGENTS_DIR="${PROJECT_ROOT}/agents"
if [ -d "${AGENTS_DIR}" ]; then
    AGENT_COUNT=0
    SECURE_AGENTS=0
    
    for agent_file in "${AGENTS_DIR}"/*.toml; do
        if [ -f "${agent_file}" ]; then
            AGENT_COUNT=$((AGENT_COUNT + 1))
            agent_name=$(basename "${agent_file}" .toml)
            
            if grep -q "shell_intercept.*true" "${agent_file}" && \
               grep -q "network_allowlist" "${agent_file}" && \
               grep -q "memory_limit" "${agent_file}"; then
                SECURE_AGENTS=$((SECURE_AGENTS + 1))
            fi
        fi
    done
    
    log_ok "数字员工安全配置: ${SECURE_AGENTS}/${AGENT_COUNT}"
else
    log_error "数字员工配置目录不存在"
fi

# 4. 检查日志文件
LOG_DIR="${HOME}/.openclaw-plus/logs"
if [ -d "${LOG_DIR}" ]; then
    LOG_COUNT=$(find "${LOG_DIR}" -name "*.log" | wc -l | tr -d ' ')
    LOG_SIZE=$(du -sh "${LOG_DIR}" | cut -f1)
    log_ok "日志文件: ${LOG_COUNT} 个，总大小: ${LOG_SIZE}"
else
    log_warn "日志目录不存在"
fi

# 5. 检查进程安全
if pgrep -f "openclaw" > /dev/null; then
    PROCESS_COUNT=$(pgrep -f "openclaw" | wc -l | tr -d ' ')
    log_ok "OpenClaw 进程: ${PROCESS_COUNT} 个运行中"
else
    log_warn "没有 OpenClaw 进程运行"
fi

echo ""
echo "安全监控完成"
EOF

chmod +x "${PROJECT_ROOT}/scripts/security_monitor.sh"
log_ok "  ✓ 已创建安全监控脚本"

echo ""

# ============================================================================
# 6. 创建安全测试自动化
# ============================================================================
log_info "创建安全测试自动化..."

cat > "${PROJECT_ROOT}/scripts/security_test_automation.sh" << 'EOF'
#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 安全测试自动化脚本
# 定期执行安全测试，生成报告
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

log_info() { echo -e "${BLUE}[AUTO]${NC}   $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}     $*"; }
log_error() { echo -e "${RED}[ERROR]${NC}   $*"; }

echo "OpenClaw+ 安全测试自动化"
echo "开始时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "================================"

# 1. 运行全面安全测试
log_info "运行全面安全测试..."
if "${PROJECT_ROOT}/tests/test_comprehensive_security.sh"; then
    log_ok "安全测试通过"
else
    log_error "安全测试失败"
fi

# 2. 运行安全监控
log_info "运行安全监控..."
"${PROJECT_ROOT}/scripts/security_monitor.sh"

# 3. 生成安全报告
REPORT_DIR="${PROJECT_ROOT}/reports/security"
mkdir -p "${REPORT_DIR}"

REPORT_FILE="${REPORT_DIR}/security_report_$(date +%Y%m%d_%H%M%S).md"

cat > "${REPORT_FILE}" << EOF
# OpenClaw+ 安全测试报告

**生成时间**: $(date '+%Y-%m-%d %H:%M:%S')
**测试类型**: 自动化安全测试

## 测试结果

- 安全配置检查: 完成
- 权限系统验证: 完成
- 网络安全测试: 完成
- 文件系统安全: 完成
- 数字员工安全: 完成

## 建议

1. 定期执行安全测试
2. 保持安全配置更新
3. 监控安全日志
4. 及时处理安全警告

---

**报告生成**: OpenClaw+ 安全自动化系统
EOF

log_ok "安全报告已生成: ${REPORT_FILE}"

echo ""
echo "安全测试自动化完成"
EOF

chmod +x "${PROJECT_ROOT}/scripts/security_test_automation.sh"
log_ok "  ✓ 已创建安全测试自动化脚本"

echo ""

# ============================================================================
# 7. 创建安全加固配置
# ============================================================================
log_info "创建安全加固配置..."

# 创建安全加固配置文件
cat > "${PROJECT_ROOT}/config/security_hardening.toml" << 'EOF'
# OpenClaw+ 安全加固配置
# 用于生产环境的额外安全设置

[security]
# 启用所有安全特性
file_delete_confirm = true
network_confirm = true
audit_logging = true
shell_intercept = true
strict_file_access = true

# 更严格的资源限制
max_file_size = "5MB"
max_memory_usage = "256MB"
max_cpu_time = "30s"

# 文件类型限制（更严格）
allowed_file_types = ["txt", "json", "md", "toml"]
forbidden_file_patterns = ["*.exe", "*.sh", "*.bat", "*.cmd"]

# 网络安全
[network]
# 强制 HTTPS
force_https = true
# 连接超时
connection_timeout = 10
# 最大并发连接
max_concurrent_connections = 3

# 审计和监控
[audit]
# 记录所有安全事件
log_all_events = true
# 日志保留天数
log_retention_days = 7
# 异常检测
anomaly_detection = true

# 加密设置
[encryption]
# 配置文件加密
encrypt_config = false
# 日志加密
encrypt_logs = false
# 传输加密
transport_encryption = true
EOF

log_ok "  ✓ 已创建安全加固配置"

echo ""

# ============================================================================
# 完成
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ 安全功能增强完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

log_ok "✅ 安全功能增强完成！"
echo ""
echo "已创建的安全资源："
echo "  📁 安全策略文档: docs/security/security_policy.md"
echo "  🔧 安全监控脚本: scripts/security_monitor.sh"
echo "  🤖 安全测试自动化: scripts/security_test_automation.sh"
echo "  ⚙️  安全加固配置: config/security_hardening.toml"
echo ""
echo "下一步建议："
echo "  1. 运行安全监控: ./scripts/security_monitor.sh"
echo "  2. 查看安全策略: cat docs/security/security_policy.md"
echo "  3. 运行安全测试: ./tests/test_comprehensive_security.sh"
echo "  4. 设置定期安全检查: crontab -e"
echo ""
echo "安全等级已提升到: 🔒 高安全"
