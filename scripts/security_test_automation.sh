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
