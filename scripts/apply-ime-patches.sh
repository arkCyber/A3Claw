#!/usr/bin/env bash
# =============================================================================
# 自动应用 libcosmic/iced IME 补丁以支持中文输入
# =============================================================================

set -euo pipefail

GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC}    $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC}  $*"; }

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OpenClaw+ IME 补丁应用工具"
echo "  恢复中文输入法支持"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 查找 libcosmic checkout 路径
LIBCOSMIC_BASE="${HOME}/.cargo/git/checkouts/libcosmic-41009aea1d72760b"
if [ ! -d "${LIBCOSMIC_BASE}" ]; then
    log_error "未找到 libcosmic checkout 目录: ${LIBCOSMIC_BASE}"
    exit 1
fi

LIBCOSMIC_DIR=$(find "${LIBCOSMIC_BASE}" -maxdepth 1 -type d -name "*" | grep -v "^${LIBCOSMIC_BASE}$" | head -1)
if [ -z "${LIBCOSMIC_DIR}" ]; then
    log_error "未找到 libcosmic 版本目录"
    exit 1
fi

log_info "找到 libcosmic 目录: ${LIBCOSMIC_DIR}"

# ============================================================================
# 补丁 1: IME 多字符提交修复
# ============================================================================
log_info "应用补丁 1: IME 多字符提交修复..."

TEXT_INPUT_FILE="${LIBCOSMIC_DIR}/src/widget/text_input/input.rs"
if [ ! -f "${TEXT_INPUT_FILE}" ]; then
    log_error "未找到文件: ${TEXT_INPUT_FILE}"
    exit 1
fi

# 检查是否已经应用过补丁
if grep -q "printable_text: Option<String>" "${TEXT_INPUT_FILE}"; then
    log_ok "补丁 1 已经应用过，跳过"
else
    log_info "应用补丁到 text_input/input.rs..."
    
    # 创建备份
    cp "${TEXT_INPUT_FILE}" "${TEXT_INPUT_FILE}.bak"
    
    # 应用补丁：将单字符提取改为多字符提取
    # 查找并替换 text.chars().next() 逻辑
    perl -i -pe '
        if (/if let Some\(c\) = text\.and_then\(\|t\| t\.chars\(\)\.next\(\)\.filter\(\|c\| !c\.is_control\(\)\)\)/) {
            $_ = "                let printable_text: Option<String> = text.map(|t| {\n" .
                 "                    t.chars().filter(|c| !c.is_control()).collect()\n" .
                 "                }).filter(|s: &String| !s.is_empty());\n" .
                 "                if let Some(printable) = printable_text {\n";
        } elsif (/^\s+editor\.insert\(c\);/) {
            $_ = "                    for c in printable.chars() {\n" .
                 "                        editor.insert(c);\n" .
                 "                    }\n";
        }
    ' "${TEXT_INPUT_FILE}"
    
    log_ok "补丁 1 应用成功"
fi

# ============================================================================
# 补丁 2 & 3 & 4: IME 启用、事件转发、候选窗口位置
# ============================================================================
log_info "应用补丁 2-4: IME 启用和事件处理..."

# 查找 iced winit 目录
ICED_WINIT_DIR=$(find "${LIBCOSMIC_DIR}" -type d -path "*/iced/winit" | head -1)
if [ -z "${ICED_WINIT_DIR}" ]; then
    log_error "未找到 iced/winit 目录"
    exit 1
fi

PROGRAM_FILE="${ICED_WINIT_DIR}/src/program.rs"
CONVERSION_FILE="${ICED_WINIT_DIR}/src/conversion.rs"

# 补丁 2 & 4: program.rs - IME 启用和候选窗口位置
if [ -f "${PROGRAM_FILE}" ]; then
    if grep -q "set_ime_allowed(true)" "${PROGRAM_FILE}"; then
        log_ok "补丁 2 & 4 已经应用过，跳过"
    else
        log_info "应用补丁到 program.rs..."
        cp "${PROGRAM_FILE}" "${PROGRAM_FILE}.bak"
        
        # 在 set_visible(true) 后添加 set_ime_allowed(true)
        # 在 Focused(true) 时设置 IME 光标位置
        perl -i -pe '
            if (/window\.set_visible\(true\);/ && !$ime_added) {
                $_ .= "                    window.set_ime_allowed(true);\n";
                $ime_added = 1;
            } elsif (/WindowEvent::Focused\(true\)/) {
                $in_focused = 1;
            } elsif ($in_focused && /=>/) {
                $_ = "                    window.set_ime_allowed(true);\n" .
                     "                    if let Some(logical_size) = window.inner_size().to_logical(window.scale_factor()) {\n" .
                     "                        let ime_y = (logical_size.height as f64 - 113.0).max(0.0);\n" .
                     "                        window.set_ime_cursor_area(\n" .
                     "                            winit::dpi::Position::Logical(\n" .
                     "                                winit::dpi::LogicalPosition::new(80.0, ime_y),\n" .
                     "                            ),\n" .
                     "                            winit::dpi::Size::Logical(\n" .
                     "                                winit::dpi::LogicalSize::new(400.0, 28.0),\n" .
                     "                            ),\n" .
                     "                        );\n" .
                     "                    }\n" . $_;
                $in_focused = 0;
            }
        ' "${PROGRAM_FILE}"
        
        log_ok "补丁 2 & 4 应用成功"
    fi
else
    log_warn "未找到 program.rs，跳过补丁 2 & 4"
fi

# 补丁 3: conversion.rs - IME Commit 事件转发
if [ -f "${CONVERSION_FILE}" ]; then
    if grep -q "WindowEvent::Ime.*Commit" "${CONVERSION_FILE}"; then
        log_ok "补丁 3 已经应用过，跳过"
    else
        log_info "应用补丁到 conversion.rs..."
        cp "${CONVERSION_FILE}" "${CONVERSION_FILE}.bak"
        
        # 在 window_event 函数中添加 IME Commit 处理
        # 查找 _ => None 之前插入
        perl -i -0777 -pe '
            s/(pub fn window_event.*?)(^\s+_ => None,)/
                $1        WindowEvent::Ime(winit::event::Ime::Commit(string)) => {\n            if string.is_empty() { return None; }\n            use crate::core::SmolStr;\n            Some(Event::Keyboard(keyboard::Event::KeyPressed {\n                key: keyboard::Key::Unidentified,\n                location: keyboard::Location::Standard,\n                modifiers: keyboard::Modifiers::default(),\n                text: Some(SmolStr::new(&string)),\n            }))\n        }\n$2/ms
        ' "${CONVERSION_FILE}"
        
        log_ok "补丁 3 应用成功"
    fi
else
    log_warn "未找到 conversion.rs，跳过补丁 3"
fi

# ============================================================================
# 清理并重新编译
# ============================================================================
log_info "清理旧的编译缓存..."

cd "$(dirname "$0")/.."
rm -f target/release/deps/libiced_winit-*.rlib \
      target/release/deps/libiced_winit-*.rmeta \
      target/release/deps/libcosmic-*.rlib \
      target/release/deps/libcosmic-*.rmeta 2>/dev/null || true

log_ok "缓存清理完成"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  IME 补丁应用完成！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
log_ok "所有 IME 补丁已成功应用"
log_info "现在请运行以下命令重新编译 UI："
echo ""
echo "  cargo build --release -p openclaw-ui"
echo ""
log_info "编译完成后，中文输入法将正常工作"
echo ""
