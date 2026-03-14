#!/usr/bin/env bash
# run-with-gateway.sh — Start Plugin Gateway and UI together
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── 1. Kill existing processes ──────────────────────────────────────────────
echo "▶ Stopping existing processes..."
pkill -f "openclaw-plugin-gateway" 2>/dev/null || true
pkill -f "OpenClawPlus.app" 2>/dev/null || true
sleep 1

# ── 2. Start Plugin Gateway ─────────────────────────────────────────────────
echo "▶ Starting Plugin Gateway on port 7878..."
cd "$PROJECT_ROOT"
GATEWAY_PORT=7878 ./target/release/openclaw-plugin-gateway > /tmp/gateway.log 2>&1 &
GATEWAY_PID=$!
echo "  Gateway PID: $GATEWAY_PID"

# Wait for Gateway to be ready
echo "▶ Waiting for Gateway to start..."
for i in {1..10}; do
    if curl -s http://localhost:7878/health > /dev/null 2>&1; then
        echo "  ✓ Gateway is ready"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "  ✗ Gateway failed to start. Check /tmp/gateway.log"
        cat /tmp/gateway.log
        exit 1
    fi
    sleep 1
done

# ── 3. Build and launch UI ──────────────────────────────────────────────────
echo "▶ Building openclaw-ui (release)…"
cargo build --release -p openclaw-ui --manifest-path "$PROJECT_ROOT/Cargo.toml"

APP=/tmp/OpenClawPlus.app
echo "▶ Updating $APP bundle…"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"
cp "$PROJECT_ROOT/target/release/openclaw-plus" "$APP/Contents/MacOS/"

cat > "$APP/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>   <string>com.openclaw.plus</string>
  <key>CFBundleName</key>         <string>OpenClaw+</string>
  <key>CFBundleDisplayName</key>  <string>OpenClaw+</string>
  <key>CFBundleExecutable</key>   <string>openclaw-plus</string>
  <key>CFBundleVersion</key>      <string>0.1.0</string>
  <key>CFBundlePackageType</key>  <string>APPL</string>
  <key>LSUIElement</key>          <false/>
  <key>NSHighResolutionCapable</key> <true/>
  <key>NSPrincipalClass</key>     <string>NSApplication</string>
</dict>
</plist>
PLIST

echo "▶ Launching ${APP}…"
open "${APP}"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✓ OpenClaw+ launched with Plugin Gateway"
echo ""
echo "  Plugin Gateway: http://localhost:7878"
echo "  Gateway PID:    $GATEWAY_PID"
echo "  Gateway Log:    /tmp/gateway.log"
echo ""
echo "  Chinese / Japanese / Korean IME input is enabled."
echo ""
echo "  To stop:"
echo "    pkill -f openclaw-plugin-gateway"
echo "    pkill -f OpenClawPlus.app"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
