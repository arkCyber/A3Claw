#!/bin/bash
# 清理项目，准备发布到 GitHub

set -e

echo "🧹 清理 OpenClaw+ 项目..."

# 删除所有测试报告和临时文档
echo "📝 删除测试报告和临时文档..."
rm -f *_REPORT*.txt *_REPORT*.md
rm -f *_TEST*.txt *_TEST*.md
rm -f *_SUMMARY*.md
rm -f *_GUIDE*.md
rm -f *_AUDIT*.md
rm -f *_FIX*.md
rm -f *_COMPLETE*.md
rm -f *.patch

# 删除临时测试文件
echo "🗑️  删除临时测试文件..."
rm -f test_*.js
rm -f test_*.toml
rm -f test_*.md
rm -f test_*.wat

# 删除构建产物
echo "🔨 清理构建产物..."
cargo clean

# 删除日志文件
echo "📋 删除日志文件..."
rm -rf logs/*.log

# 删除临时文件
echo "🗂️  删除临时文件..."
rm -f pont pu llama-server
rm -f *.tmp

# 格式化代码
echo "✨ 格式化代码..."
cargo fmt --all

# 运行 clippy
echo "🔍 运行 clippy 检查..."
cargo clippy --all-targets --all-features -- -D warnings || echo "⚠️  警告: clippy 发现了一些问题，请修复后再发布"

# 运行测试
echo "🧪 运行测试..."
cargo test --workspace --lib || echo "⚠️  警告: 部分测试失败，请检查后再发布"

echo ""
echo "✅ 清理完成！"
echo ""
echo "📋 下一步："
echo "1. 检查 git status 确认要提交的文件"
echo "2. 运行: git add ."
echo "3. 运行: git commit -m 'chore: prepare for initial release'"
echo "4. 运行: git tag -a v0.1.0 -m 'Initial release v0.1.0'"
echo "5. 创建 GitHub 仓库: https://github.com/new"
echo "6. 运行: git remote add origin https://github.com/arksong2018/openclaw-plus.git"
echo "7. 运行: git push -u origin main"
echo "8. 运行: git push origin v0.1.0"
echo ""
echo "📖 详细步骤请查看 RELEASE_CHECKLIST.md"
