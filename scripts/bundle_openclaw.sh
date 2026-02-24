#!/usr/bin/env bash
# =============================================================================
# OpenClaw+ Bundle Script
# Bundles the OpenClaw source into a single JS file for WasmEdge-QuickJS.
#
# Usage:
#   ./scripts/bundle_openclaw.sh [openclaw_repo_path] [output_dir]
#
# Requirements:
#   - Node.js >= 18
#   - esbuild (npm install -g esbuild)
#   - git
#
# Output:
#   assets/openclaw/dist/index.js       (single-file bundle)
#   assets/openclaw/dist/index.js.map   (source map for debugging)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

OPENCLAW_REPO="${1:-${PROJECT_ROOT}/vendor/openclaw}"
OUTPUT_DIR="${2:-${PROJECT_ROOT}/assets/openclaw/dist}"
OPENCLAW_GIT_URL="https://github.com/isontheline/OpenClaw.git"

# ── Colour output helpers ────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ── Step 1: Check build dependencies ────────────────────────
log_info "Checking build dependencies..."

if ! command -v node &>/dev/null; then
    log_error "Node.js not found. Install it from: https://nodejs.org"
    exit 1
fi

if ! command -v esbuild &>/dev/null; then
    log_warn "esbuild not found, installing..."
    npm install -g esbuild
fi

NODE_VERSION=$(node --version)
ESBUILD_VERSION=$(esbuild --version)
log_ok "Node.js: ${NODE_VERSION}"
log_ok "esbuild: ${ESBUILD_VERSION}"

# ── Step 2: Fetch OpenClaw source ───────────────────────────
if [ ! -d "${OPENCLAW_REPO}" ]; then
    log_info "Cloning OpenClaw source to: ${OPENCLAW_REPO}"
    git clone --depth=1 "${OPENCLAW_GIT_URL}" "${OPENCLAW_REPO}"
    log_ok "OpenClaw source cloned"
else
    log_info "OpenClaw source already exists: ${OPENCLAW_REPO}"
    log_info "Pulling latest changes..."
    git -C "${OPENCLAW_REPO}" pull --ff-only || log_warn "Pull failed, using existing version"
fi

# ── Step 3: Install OpenClaw dependencies ───────────────────
log_info "Installing OpenClaw dependencies..."
cd "${OPENCLAW_REPO}"

if [ -f "package-lock.json" ]; then
    npm ci --prefer-offline
elif [ -f "yarn.lock" ]; then
    yarn install --frozen-lockfile
else
    npm install
fi
log_ok "Dependencies installed"

# ── Step 4: Locate OpenClaw entry file ──────────────────────
ENTRY_FILE=""
for candidate in "src/index.ts" "src/main.ts" "src/index.js" "src/main.js" "index.ts" "index.js"; do
    if [ -f "${OPENCLAW_REPO}/${candidate}" ]; then
        ENTRY_FILE="${OPENCLAW_REPO}/${candidate}"
        log_ok "Entry file found: ${candidate}"
        break
    fi
done

if [ -z "${ENTRY_FILE}" ]; then
    # Fall back to the 'main' field in package.json.
    if [ -f "${OPENCLAW_REPO}/package.json" ]; then
        MAIN_FIELD=$(node -e "console.log(require('./package.json').main || '')" 2>/dev/null || echo "")
        if [ -n "${MAIN_FIELD}" ] && [ -f "${OPENCLAW_REPO}/${MAIN_FIELD}" ]; then
            ENTRY_FILE="${OPENCLAW_REPO}/${MAIN_FIELD}"
            log_ok "Entry file from package.json: ${MAIN_FIELD}"
        fi
    fi
fi

if [ -z "${ENTRY_FILE}" ]; then
    log_error "Cannot find OpenClaw entry file. Specify it manually."
    log_error "Supported candidates: src/index.ts, src/main.ts, src/index.js, src/main.js"
    exit 1
fi

# ── Step 5: Create output directory ────────────────────────
mkdir -p "${OUTPUT_DIR}"
log_info "Output directory: ${OUTPUT_DIR}"

# ── Step 6: Bundle with esbuild ────────────────────────────
log_info "Bundling OpenClaw with esbuild..."

OUTPUT_FILE="${OUTPUT_DIR}/index.js"

esbuild "${ENTRY_FILE}" \
    --bundle \
    --platform=node \
    --target=node18 \
    --format=cjs \
    --outfile="${OUTPUT_FILE}" \
    --sourcemap \
    --metafile="${OUTPUT_DIR}/meta.json" \
    --log-level=info \
    --external:electron \
    --external:@electron \
    --define:process.env.NODE_ENV='"production"' \
    --define:OPENCLAW_PLUS_SANDBOX='"true"'

log_ok "Bundle written: ${OUTPUT_FILE}"

# ── Step 7: Print bundle info ───────────────────────────────
BUNDLE_SIZE=$(du -sh "${OUTPUT_FILE}" | cut -f1)
log_info "Bundle size: ${BUNDLE_SIZE}"

# Compute SHA-256 hash for integrity verification.
if command -v sha256sum &>/dev/null; then
    HASH=$(sha256sum "${OUTPUT_FILE}" | cut -d' ' -f1)
elif command -v shasum &>/dev/null; then
    HASH=$(shasum -a 256 "${OUTPUT_FILE}" | cut -d' ' -f1)
else
    HASH="unavailable"
fi

log_ok "SHA-256: ${HASH}"

# Write bundle metadata for the Rust side to verify on load.
cat > "${OUTPUT_DIR}/bundle_info.json" <<EOF
{
  "version": "$(git -C "${OPENCLAW_REPO}" describe --tags --always 2>/dev/null || echo 'unknown')",
  "commit": "$(git -C "${OPENCLAW_REPO}" rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "bundled_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "entry": "${ENTRY_FILE}",
  "output": "${OUTPUT_FILE}",
  "size_bytes": $(wc -c < "${OUTPUT_FILE}"),
  "sha256": "${HASH}"
}
EOF

log_ok "Bundle metadata written: ${OUTPUT_DIR}/bundle_info.json"

# ── Step 8: Update config.toml ──────────────────────────────
CONFIG_PATH="${HOME}/.config/openclaw-plus/config.toml"
if [ -f "${CONFIG_PATH}" ]; then
    # Patch the openclaw_entry path in-place.
    sed -i.bak "s|^openclaw_entry = .*|openclaw_entry = \"${OUTPUT_FILE}\"|" "${CONFIG_PATH}"
    log_ok "Config updated: ${CONFIG_PATH}"
    log_info "openclaw_entry = \"${OUTPUT_FILE}\""
fi

echo ""
log_ok "=========================================="
log_ok "OpenClaw bundle complete!"
log_ok "Bundle: ${OUTPUT_FILE}"
log_ok "Size:   ${BUNDLE_SIZE}"
log_ok "Hash:   ${HASH}"
log_ok "=========================================="
echo ""
log_info "Next step: cargo run -p openclaw-ui"
