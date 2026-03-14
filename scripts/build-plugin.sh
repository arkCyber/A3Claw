#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════
# build-plugin.sh — Multi-language WASM plugin builder for OpenClaw+
# ═══════════════════════════════════════════════════════════════════════════
#
# Usage:
#   ./scripts/build-plugin.sh <plugin-dir> [--release] [--sign]
#
# Supports:
#   - Rust       → cargo build --target wasm32-wasip1
#   - TypeScript → javy compile (QuickJS WASM runtime)
#   - Python     → py2wasm (CPython-in-WASM)
#   - Go         → tinygo build -target=wasi
#   - C/C++      → wasi-sdk clang
#
# Output:
#   - <plugin-dir>/target/<plugin-id>.wasm
#   - <plugin-dir>/target/<plugin-id>.wasm.sha256
#   - <plugin-dir>/target/<plugin-id>.wasm.sig (if --sign)
#
# ═══════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PLUGIN_DIR="${1:-}"
RELEASE_MODE="${2:-}"
SIGN_MODE="${3:-}"

if [[ -z "$PLUGIN_DIR" ]]; then
    echo "Usage: $0 <plugin-dir> [--release] [--sign]"
    echo ""
    echo "Examples:"
    echo "  $0 examples/hello-skill-plugin"
    echo "  $0 examples/typescript-plugin --release --sign"
    exit 1
fi

if [[ ! -d "$PLUGIN_DIR" ]]; then
    echo "Error: Plugin directory not found: $PLUGIN_DIR"
    exit 1
fi

cd "$PLUGIN_DIR"

# ── Detect plugin language ────────────────────────────────────────────────

LANG=""
if [[ -f "Cargo.toml" ]]; then
    LANG="rust"
elif [[ -f "package.json" ]] && grep -q '"type".*"module"' package.json 2>/dev/null; then
    LANG="typescript"
elif [[ -f "pyproject.toml" ]] || [[ -f "setup.py" ]]; then
    LANG="python"
elif [[ -f "go.mod" ]]; then
    LANG="go"
elif [[ -f "CMakeLists.txt" ]] || [[ -f "Makefile" ]]; then
    LANG="c"
else
    echo "Error: Could not detect plugin language in $PLUGIN_DIR"
    echo "Supported: Cargo.toml (Rust), package.json (TS/JS), pyproject.toml (Python), go.mod (Go), CMakeLists.txt (C/C++)"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════════════════════"
echo "Building plugin: $(basename "$PLUGIN_DIR")"
echo "Language: $LANG"
echo "Release mode: ${RELEASE_MODE:-debug}"
echo "═══════════════════════════════════════════════════════════════════════════"

# ── Build based on language ───────────────────────────────────────────────

WASM_OUTPUT=""

case "$LANG" in
    rust)
        echo "→ Building Rust plugin with cargo..."
        if [[ "$RELEASE_MODE" == "--release" ]]; then
            cargo build --target wasm32-wasip1 --release
            WASM_OUTPUT="target/wasm32-wasip1/release/$(basename "$PLUGIN_DIR" | tr '-' '_').wasm"
        else
            cargo build --target wasm32-wasip1
            WASM_OUTPUT="target/wasm32-wasip1/debug/$(basename "$PLUGIN_DIR" | tr '-' '_').wasm"
        fi
        ;;

    typescript)
        echo "→ Building TypeScript plugin with javy..."
        if ! command -v javy &> /dev/null; then
            echo "Error: javy not found. Install via: cargo install javy-cli"
            exit 1
        fi
        
        # Transpile TS → JS if needed
        if [[ -f "tsconfig.json" ]]; then
            echo "  → Transpiling TypeScript..."
            npx tsc --outDir dist
            ENTRY_JS="dist/index.js"
        else
            ENTRY_JS="index.js"
        fi
        
        mkdir -p target
        WASM_OUTPUT="target/$(basename "$PLUGIN_DIR").wasm"
        javy compile "$ENTRY_JS" -o "$WASM_OUTPUT"
        ;;

    python)
        echo "→ Building Python plugin with py2wasm..."
        if ! command -v py2wasm &> /dev/null; then
            echo "Error: py2wasm not found. Install from: https://github.com/wasmerio/py2wasm"
            exit 1
        fi
        
        ENTRY_PY="${ENTRY_PY:-main.py}"
        mkdir -p target
        WASM_OUTPUT="target/$(basename "$PLUGIN_DIR").wasm"
        py2wasm "$ENTRY_PY" -o "$WASM_OUTPUT"
        ;;

    go)
        echo "→ Building Go plugin with tinygo..."
        if ! command -v tinygo &> /dev/null; then
            echo "Error: tinygo not found. Install from: https://tinygo.org/getting-started/install/"
            exit 1
        fi
        
        mkdir -p target
        WASM_OUTPUT="target/$(basename "$PLUGIN_DIR").wasm"
        tinygo build -target=wasi -o "$WASM_OUTPUT" .
        ;;

    c)
        echo "→ Building C/C++ plugin with wasi-sdk..."
        if ! command -v clang &> /dev/null || ! clang --version | grep -q wasi; then
            echo "Error: wasi-sdk clang not found. Install from: https://github.com/WebAssembly/wasi-sdk"
            exit 1
        fi
        
        mkdir -p target
        WASM_OUTPUT="target/$(basename "$PLUGIN_DIR").wasm"
        make wasm || clang --target=wasm32-wasi -o "$WASM_OUTPUT" src/*.c
        ;;

    *)
        echo "Error: Unsupported language: $LANG"
        exit 1
        ;;
esac

if [[ ! -f "$WASM_OUTPUT" ]]; then
    echo "Error: Build failed — WASM output not found: $WASM_OUTPUT"
    exit 1
fi

echo "✓ Build succeeded: $WASM_OUTPUT"
echo "  Size: $(du -h "$WASM_OUTPUT" | cut -f1)"

# ── Generate SHA-256 checksum ─────────────────────────────────────────────

echo ""
echo "→ Generating SHA-256 checksum..."
SHA256_FILE="${WASM_OUTPUT}.sha256"
if command -v sha256sum &> /dev/null; then
    sha256sum "$WASM_OUTPUT" | cut -d' ' -f1 > "$SHA256_FILE"
elif command -v shasum &> /dev/null; then
    shasum -a 256 "$WASM_OUTPUT" | cut -d' ' -f1 > "$SHA256_FILE"
else
    echo "Warning: sha256sum/shasum not found — skipping checksum"
fi

if [[ -f "$SHA256_FILE" ]]; then
    echo "✓ SHA-256: $(cat "$SHA256_FILE")"
fi

# ── Sign with Ed25519 (optional) ──────────────────────────────────────────

if [[ "$SIGN_MODE" == "--sign" ]]; then
    echo ""
    echo "→ Signing WASM with Ed25519..."
    
    SIGNING_KEY="${OPENCLAW_SIGNING_KEY:-$HOME/.openclaw/signing.key}"
    if [[ ! -f "$SIGNING_KEY" ]]; then
        echo "Error: Signing key not found: $SIGNING_KEY"
        echo "Generate one with: ssh-keygen -t ed25519 -f ~/.openclaw/signing.key"
        exit 1
    fi
    
    SIG_FILE="${WASM_OUTPUT}.sig"
    
    # Sign the SHA-256 hash (not the full WASM for performance)
    if command -v openssl &> /dev/null; then
        openssl dgst -sha256 -sign "$SIGNING_KEY" -out "$SIG_FILE" "$WASM_OUTPUT"
        echo "✓ Signature: $SIG_FILE"
    else
        echo "Warning: openssl not found — skipping signature"
    fi
fi

# ── Summary ───────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════════════════════════════════════════"
echo "Build complete!"
echo "  WASM:    $WASM_OUTPUT"
if [[ -f "$SHA256_FILE" ]]; then
    echo "  SHA-256: $SHA256_FILE"
fi
if [[ -f "${WASM_OUTPUT}.sig" ]]; then
    echo "  Signature: ${WASM_OUTPUT}.sig"
fi
echo "═══════════════════════════════════════════════════════════════════════════"
