# 🚀 快速发布指南 / Quick Publish Guide

[English](#english) | [中文](#中文)

---

## English

### Prerequisites

1. GitHub account
2. Git installed and configured
3. Project built and tested

### Step 1: Clean Up Project

```bash
# Run the cleanup script
./scripts/clean_for_release.sh

# Or manually clean
cargo clean
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --workspace --lib
```

### Step 2: Commit Changes

```bash
# Check status
git status

# Add all files
git add .

# Commit
git commit -m "chore: prepare for initial release

- Add comprehensive README in English and Chinese
- Add MIT LICENSE
- Update .gitignore with complete rules
- Update Cargo.toml with correct repository info
- Clean up temporary test files
"

# Create tag
git tag -a v0.1.0 -m "Initial release v0.1.0"
```

### Step 3: Create GitHub Repository

1. Go to https://github.com/new
2. Repository name: `openclaw-plus`
3. Description: `AI Agent Security Platform - WasmEdge sandbox with visual workflow editor`
4. Choose **Public**
5. **DO NOT** initialize with README, .gitignore, or LICENSE (we already have them)
6. Click "Create repository"

### Step 4: Push to GitHub

```bash
# Add remote
git remote add origin https://github.com/arksong2018/openclaw-plus.git

# Push main branch
git push -u origin main

# Push tags
git push origin v0.1.0
```

### Step 5: Create Release

1. Go to https://github.com/arksong2018/openclaw-plus/releases/new
2. Choose tag: `v0.1.0`
3. Release title: `OpenClaw+ v0.1.0 - Initial Release`
4. Add release notes (see RELEASE_CHECKLIST.md for template)
5. Click "Publish release"

### Step 6: Configure Repository

1. **About section**:
   - Add description
   - Add topics: `rust`, `wasmedge`, `ai-agent`, `security`, `sandbox`, `wasm`

2. **Settings**:
   - Enable Issues
   - Enable Discussions
   - Set branch protection (optional)

### Done! 🎉

Your project is now live at: https://github.com/arksong2018/openclaw-plus

---

## 中文

### 前置条件

1. GitHub 账号
2. 已安装并配置 Git
3. 项目已构建和测试

### 步骤 1: 清理项目

```bash
# 运行清理脚本
./scripts/clean_for_release.sh

# 或手动清理
cargo clean
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --workspace --lib
```

### 步骤 2: 提交更改

```bash
# 检查状态
git status

# 添加所有文件
git add .

# 提交
git commit -m "chore: prepare for initial release

- Add comprehensive README in English and Chinese
- Add MIT LICENSE
- Update .gitignore with complete rules
- Update Cargo.toml with correct repository info
- Clean up temporary test files
"

# 创建标签
git tag -a v0.1.0 -m "Initial release v0.1.0"
```

### 步骤 3: 创建 GitHub 仓库

1. 访问 https://github.com/new
2. 仓库名称：`openclaw-plus`
3. 描述：`AI Agent Security Platform - WasmEdge sandbox with visual workflow editor`
4. 选择 **Public**（公开）
5. **不要**初始化 README、.gitignore 或 LICENSE（我们已经有了）
6. 点击 "Create repository"

### 步骤 4: 推送到 GitHub

```bash
# 添加远程仓库
git remote add origin https://github.com/arksong2018/openclaw-plus.git

# 推送主分支
git push -u origin main

# 推送标签
git push origin v0.1.0
```

### 步骤 5: 创建发布版本

1. 访问 https://github.com/arksong2018/openclaw-plus/releases/new
2. 选择标签：`v0.1.0`
3. 发布标题：`OpenClaw+ v0.1.0 - Initial Release`
4. 添加发布说明（参见 RELEASE_CHECKLIST.md 中的模板）
5. 点击 "Publish release"

### 步骤 6: 配置仓库

1. **About 部分**：
   - 添加描述
   - 添加主题标签：`rust`, `wasmedge`, `ai-agent`, `security`, `sandbox`, `wasm`

2. **Settings 设置**：
   - 启用 Issues
   - 启用 Discussions
   - 设置分支保护（可选）

### 完成！🎉

你的项目现已上线：https://github.com/arksong2018/openclaw-plus

---

## 📞 需要帮助？

- 📧 Email: arksong2018@gmail.com
- 📖 详细清单: [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)
- 🐛 问题反馈: [GitHub Issues](https://github.com/arksong2018/openclaw-plus/issues)
