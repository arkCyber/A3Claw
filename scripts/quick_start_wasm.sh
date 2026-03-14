#!/bin/bash

# OpenClaw WASM平台快速启动脚本
# 使用方法: ./scripts/quick_start_wasm.sh

set -e

echo "🚀 OpenClaw WASM平台快速启动"
echo "================================"

# 检查依赖
echo "📋 检查依赖..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo未找到，请安装Rust"
    exit 1
fi

if ! command -v wasmedge &> /dev/null; then
    echo "⚠️  WasmEdge未找到，某些功能可能不可用"
fi

# 创建必要目录
echo "📁 创建目录结构..."
mkdir -p assets/wasm-skills
mkdir -p logs

# 编译WASM组件
echo "🔧 编译WASM组件..."
echo "   - 编译WASM编译器..."
cargo build --release -p a3office-wasm-compiler

echo "   - 编译WASM运行时..."
cargo build --release -p a3office-wasm-runtime

# 运行演示
echo "🎪 运行WASM平台演示..."
echo "   这将展示技能编译、注册和执行的完整流程"
echo ""

# 设置环境变量
export RUST_LOG=info
export RUST_BACKTRACE=1

# 运行演示程序
cargo run --release --bin demo_wasm_platform

echo ""
echo "🎉 快速启动完成!"
echo ""
echo "📚 更多信息:"
echo "   - 愿景文档: WASM_EDGE_PLATFORM_VISION.md"
echo "   - 实施路线图: IMPLEMENTATION_ROADMAP.md"
echo "   - 技能编译: cargo run --release --bin compile_skills_to_wasm"
echo "   - UI界面: cargo run --release -p a3office-ui"
echo ""
echo "🔧 开发命令:"
echo "   - 编译所有组件: cargo build --release"
echo "   - 运行测试: cargo test --workspace"
echo "   - 查看日志: tail -f/logs/wasm-platform.log"
echo ""
echo "🚀 开始构建你的WASM AI应用平台吧!"
