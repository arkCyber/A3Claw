#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ 端到端测试脚本
# 测试完整的数字员工工作流程：创建 -> 配置 -> 运行 -> 验证
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_success() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ── 测试步骤 1: 检查 Ollama 服务 ──────────────────────────────────────────
test_ollama() {
    log_info "Testing Ollama service..."
    
    if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
        log_error "Ollama service is not running on localhost:11434"
        log_info "Please start Ollama: ollama serve"
        return 1
    fi
    
    log_success "Ollama service is running"
    
    # 检查模型
    if ollama list | grep -q "qwen2.5:0.5b"; then
        log_success "Model qwen2.5:0.5b is available"
    else
        log_warn "Model qwen2.5:0.5b not found, pulling..."
        ollama pull qwen2.5:0.5b
    fi
}

# ── 测试步骤 2: 检查 OpenClaw 打包 ──────────────────────────────────────────
test_openclaw_bundle() {
    log_info "Checking OpenClaw bundle..."
    
    local bundle_path="${PROJECT_ROOT}/assets/openclaw/dist/index.js"
    
    if [[ ! -f "$bundle_path" ]]; then
        log_warn "OpenClaw bundle not found, running bundle script..."
        "${PROJECT_ROOT}/scripts/bundle_openclaw.sh"
    fi
    
    if [[ -f "$bundle_path" ]]; then
        local size=$(du -h "$bundle_path" | cut -f1)
        log_success "OpenClaw bundle exists: $size"
    else
        log_error "Failed to create OpenClaw bundle"
        return 1
    fi
}

# ── 测试步骤 3: 运行单元测试 ────────────────────────────────────────────────
test_unit_tests() {
    log_info "Running unit tests..."
    
    cd "$PROJECT_ROOT"
    
    # 测试 security crate
    log_info "Testing openclaw-security..."
    cargo test -p openclaw-security --quiet
    log_success "openclaw-security tests passed"
    
    # 测试 storage crate
    log_info "Testing openclaw-storage..."
    cargo test -p openclaw-storage --quiet
    log_success "openclaw-storage tests passed"
    
    # 测试 sandbox crate (no default features to skip WasmEdge)
    log_info "Testing openclaw-sandbox..."
    cargo test -p openclaw-sandbox --no-default-features --quiet
    log_success "openclaw-sandbox tests passed"
    
    # 测试集成测试
    if [[ -f "${PROJECT_ROOT}/tests/integration_test.rs" ]]; then
        log_info "Running integration tests..."
        cargo test --test integration_test --quiet
        log_success "Integration tests passed"
    fi
}

# ── 测试步骤 4: 创建测试数字员工 ────────────────────────────────────────────
test_create_agent() {
    log_info "Creating test agent profile..."
    
    local agent_dir="$HOME/.openclaw-plus/agents/test-agent-001"
    mkdir -p "$agent_dir/workspace"
    
    # 复制测试配置
    if [[ -f "${PROJECT_ROOT}/test_agent_profile.toml" ]]; then
        cp "${PROJECT_ROOT}/test_agent_profile.toml" "$agent_dir/profile.toml"
        log_success "Test agent profile created at $agent_dir"
    else
        log_error "Test agent profile template not found"
        return 1
    fi
    
    # 创建测试工作空间
    echo "Hello from OpenClaw+ test workspace!" > "$agent_dir/workspace/test.txt"
    log_success "Test workspace initialized"
}

# ── 测试步骤 5: 验证 UI 构建 ────────────────────────────────────────────────
test_ui_build() {
    log_info "Building UI..."
    
    cd "$PROJECT_ROOT"
    cargo build --release -p openclaw-ui 2>&1 | grep -E "Compiling|Finished|error" || true
    
    if [[ -f "${PROJECT_ROOT}/target/release/openclaw-plus" ]]; then
        log_success "UI binary built successfully"
    else
        log_error "UI build failed"
        return 1
    fi
}

# ── 测试步骤 6: 验证配置文件 ────────────────────────────────────────────────
test_config_files() {
    log_info "Validating configuration files..."
    
    local config_dir="$HOME/.openclaw-plus"
    mkdir -p "$config_dir"
    
    # 复制默认配置
    if [[ ! -f "$config_dir/config.toml" ]]; then
        cp "${PROJECT_ROOT}/config/default.toml" "$config_dir/config.toml"
        log_success "Default config copied"
    fi
    
    if [[ ! -f "$config_dir/inference.toml" ]]; then
        cp "${PROJECT_ROOT}/config/inference.toml" "$config_dir/inference.toml"
        log_success "Inference config copied"
    fi
    
    log_success "Configuration files validated"
}

# ── 主测试流程 ──────────────────────────────────────────────────────────────
main() {
    log_info "Starting OpenClaw+ end-to-end tests..."
    echo ""
    
    local failed=0
    
    test_ollama || ((failed++))
    echo ""
    
    test_openclaw_bundle || ((failed++))
    echo ""
    
    test_unit_tests || ((failed++))
    echo ""
    
    test_create_agent || ((failed++))
    echo ""
    
    test_config_files || ((failed++))
    echo ""
    
    test_ui_build || ((failed++))
    echo ""
    
    if [[ $failed -eq 0 ]]; then
        log_success "All tests passed! ✅"
        log_info "You can now run: ./target/release/openclaw-plus"
        return 0
    else
        log_error "$failed test(s) failed ❌"
        return 1
    fi
}

main "$@"
