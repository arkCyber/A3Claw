#!/usr/bin/env bash
# run.sh — Build and launch OpenClaw+ as a proper macOS .app bundle
# This is required for correct IME (Chinese/Japanese/Korean input method) behaviour.
# Running the binary directly from a terminal causes macOS to keep IME focus on the
# terminal, so composed CJK characters never reach the app window.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP=/tmp/OpenClawPlus.app

# ── 1. Build ────────────────────────────────────────────────────────────────
echo "▶ Building openclaw-ui (release)…"
cargo build --release -p openclaw-ui --manifest-path "$PROJECT_ROOT/Cargo.toml"

# ── 2. Create / update .app bundle ──────────────────────────────────────────
echo "▶ Updating $APP bundle…"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

cp "$PROJECT_ROOT/target/release/openclaw-plus" "$APP/Contents/MacOS/"

# Write a minimal Info.plist so macOS treats this as a proper GUI app and
# assigns IME focus to our window instead of the launching terminal.
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

# ── 3. Launch ────────────────────────────────────────────────────────────────
echo "▶ Launching $APP…"
open "$APP"
echo "✓ OpenClaw+ launched.  Chinese / Japanese / Korean IME input is enabled."
