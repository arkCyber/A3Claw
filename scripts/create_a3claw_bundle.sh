#!/bin/bash
# 创建 A3Claw.app macOS 应用程序包

set -e

echo "🎨 创建 A3Claw.app bundle..."

# 定义路径
APP_NAME="A3Claw"
BUNDLE_DIR="$HOME/Applications/${APP_NAME}.app"
CONTENTS_DIR="${BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
BINARY_PATH="target/release/openclaw-plus"

# 1. 清理旧的 bundle
if [ -d "$BUNDLE_DIR" ]; then
    echo "🗑️  删除旧的 ${APP_NAME}.app..."
    rm -rf "$BUNDLE_DIR"
fi

# 2. 创建目录结构
echo "📁 创建 bundle 目录结构..."
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# 3. 复制二进制文件
echo "📦 复制二进制文件..."
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ 错误: 找不到 $BINARY_PATH"
    echo "   正在构建二进制文件..."
    cargo build -p openclaw-ui --release
fi
cp "$BINARY_PATH" "$MACOS_DIR/${APP_NAME}"
chmod +x "$MACOS_DIR/${APP_NAME}"

# 4. 创建启动脚本（处理环境变量）
echo "📝 创建启动脚本..."
cat > "$MACOS_DIR/${APP_NAME}-launcher" << 'EOF'
#!/bin/bash
# A3Claw 启动器 - 处理环境变量和服务检查

# 加载 Rust 环境
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
elif [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# 检查并启动 Ollama（如果需要）
if ! pgrep -x "ollama" > /dev/null; then
    if command -v /opt/homebrew/bin/ollama &> /dev/null; then
        /opt/homebrew/bin/ollama serve > /tmp/ollama.log 2>&1 &
        sleep 2
    elif command -v ollama &> /dev/null; then
        ollama serve > /tmp/ollama.log 2>&1 &
        sleep 2
    fi
fi

# 设置日志
export RUST_LOG=openclaw_ui=info,openclaw_inference=debug

# 获取 bundle 路径
BUNDLE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
MACOS_DIR="$BUNDLE_DIR/MacOS"

# 启动应用
cd "$MACOS_DIR"
exec "./A3Claw" "$@"
EOF

chmod +x "$MACOS_DIR/${APP_NAME}-launcher"

# 5. 创建 Info.plist
echo "📄 创建 Info.plist..."
cat > "$CONTENTS_DIR/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}-launcher</string>
    <key>CFBundleIdentifier</key>
    <string>com.a3claw.ui</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>LSUIElement</key>
    <false/>
    <key>CFBundleDisplayName</key>
    <string>A3Claw</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon.icns</string>
</dict>
</plist>
EOF

# 6. 创建简单的图标（使用系统默认）
echo "🎨 设置应用图标..."
# 创建一个简单的图标文件（如果需要可以替换为自定义图标）
if [ ! -f "$RESOURCES_DIR/AppIcon.icns" ]; then
    echo "   使用系统默认图标"
    # 这里可以添加自定义图标的生成代码
fi

# 7. 设置权限
echo "🔐 设置权限..."
chmod -R 755 "$BUNDLE_DIR"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ ${APP_NAME}.app 创建成功！"
echo ""
echo "📍 位置: $BUNDLE_DIR"
echo ""
echo "🚀 启动方式："
echo "   1. 在 Finder 中打开 ~/Applications/"
echo "   2. 双击 ${APP_NAME}.app"
echo ""
echo "   或者命令行："
echo "   open ~/Applications/${APP_NAME}.app"
echo ""
echo "   或者使用脚本："
echo "   ./scripts/launch_a3claw.sh"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
