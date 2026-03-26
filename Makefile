# OpenClaw+ — cross-platform development convenience targets
#
# Supported platforms: macOS · Linux · Windows (via Git Bash / MSYS2 / WSL2)
#
# Usage:
#   make build          — release-build all crates for the current OS
#   make build-windows  — cross-compile for Windows (requires mingw toolchain)
#   make build-linux    — cross-compile for Linux   (requires musl toolchain)
#   make test           — run all unit + doc tests
#   make check          — cargo check (fast, no codegen)
#   make gateway        — start the plugin gateway on port 54321
#   make store          — launch the store UI (platform-appropriate method)
#   make run            — gateway + store UI together
#   make clean          — remove build artefacts
#   make fmt            — cargo fmt
#   make clippy         — cargo clippy -D warnings
#   make smoke          — HTTP smoke-test against a running gateway
#
# Platform detection:
#   The Makefile auto-detects the host OS via $(OS) (Windows) or uname.
#   Override with: make store OS_OVERRIDE=linux

CARGO   := cargo
PORT    := 54321
REGISTRY := $(shell pwd)/registry/index.json

# ── OS detection ──────────────────────────────────────────────────────────────
# $(OS) is set to "Windows_NT" by cmd.exe / PowerShell on Windows.
# On Unix we fall back to uname.
ifeq ($(OS),Windows_NT)
    DETECTED_OS := windows
else
    UNAME := $(shell uname -s)
    ifeq ($(UNAME),Darwin)
        DETECTED_OS := macos
    else ifeq ($(UNAME),Linux)
        DETECTED_OS := linux
    else
        DETECTED_OS := unknown
    endif
endif

# Allow manual override: make store DETECTED_OS=linux
ifdef OS_OVERRIDE
    DETECTED_OS := $(OS_OVERRIDE)
endif

# ── Binary paths (platform-aware) ─────────────────────────────────────────────
ifeq ($(DETECTED_OS),windows)
    GATEWAY := target/release/openclaw-plugin-gateway.exe
    STORE   := target/release/openclaw-store.exe
    EXE_EXT := .exe
else
    GATEWAY := target/release/openclaw-plugin-gateway
    STORE   := target/release/openclaw-store
    EXE_EXT :=
endif

APP_BUNDLE := /tmp/OpenClawStore.app

# ── Build ─────────────────────────────────────────────────────────────────────

.PHONY: build
build:
	$(CARGO) build --release \
		-p openclaw-security \
		-p openclaw-plugin-gateway \
		-p openclaw-store

# Windows: WasmEdge sandbox is disabled (no prebuilt Windows libs).
# Build with --no-default-features to skip wasmedge in openclaw-sandbox.
.PHONY: build-windows
build-windows:
	$(CARGO) build --release \
		-p openclaw-security \
		-p openclaw-plugin-gateway \
		-p openclaw-store \
		--target x86_64-pc-windows-gnu

# Linux musl static binary (useful for containers / CI).
.PHONY: build-linux
build-linux:
	$(CARGO) build --release \
		-p openclaw-security \
		-p openclaw-plugin-gateway \
		-p openclaw-store \
		--target x86_64-unknown-linux-musl

.PHONY: check
check:
	$(CARGO) check \
		-p openclaw-security \
		-p openclaw-plugin-gateway \
		-p openclaw-store

# ── Test ──────────────────────────────────────────────────────────────────────

.PHONY: test
test:
	$(CARGO) test \
		-p openclaw-security \
		-p openclaw-plugin-gateway \
		-p openclaw-store

# ── Gateway ───────────────────────────────────────────────────────────────────

.PHONY: gateway
gateway: build
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) \
	RUST_LOG=info \
	$(GATEWAY) --port $(PORT)

# ── Store UI launch (platform-specific) ───────────────────────────────────────
# ⚠️  DEPRECATED: openclaw-store is the OLD UI - DO NOT USE
# ✅  Use 'make ui' or 'make ui-app' for the NEW UI with CLI Terminal sidebar

# macOS: wrap in a .app bundle so Metal / window focus works correctly.
.PHONY: macos-app
macos-app: build
	@echo "⚠️  WARNING: This is the OLD UI (openclaw-store)"
	@echo "✅  Please use 'make ui-app' for the NEW UI with CLI Terminal"
	@echo ""
	@echo "Continuing in 3 seconds... Press Ctrl+C to cancel"
	@sleep 3
	mkdir -p $(APP_BUNDLE)/Contents/MacOS $(APP_BUNDLE)/Contents/Resources
	cp $(STORE) $(APP_BUNDLE)/Contents/MacOS/openclaw-store
	@printf '#!/bin/bash\nexport OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT)\nexport CLAWPLUS_REGISTRY_URL=file://$(REGISTRY)\nexport RUST_LOG=info\nDIR="$$(cd "$$(dirname "$$0")" && pwd)"\nexec "$$DIR/openclaw-store" "$$@"\n' \
		> $(APP_BUNDLE)/Contents/MacOS/openclaw-store-launcher
	chmod +x $(APP_BUNDLE)/Contents/MacOS/openclaw-store-launcher
	@printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict>\n<key>CFBundleExecutable</key><string>openclaw-store-launcher</string>\n<key>CFBundleIdentifier</key><string>dev.clawplus.store</string>\n<key>CFBundleName</key><string>OpenClaw+ Store (OLD)</string>\n<key>CFBundlePackageType</key><string>APPL</string>\n<key>NSHighResolutionCapable</key><true/>\n<key>NSPrincipalClass</key><string>NSApplication</string>\n</dict></plist>\n' \
		> $(APP_BUNDLE)/Contents/Info.plist
	@echo "⚠️  OLD UI App bundle ready: $(APP_BUNDLE)"

# Unified 'store' target — dispatches to the right launcher per OS.
# ⚠️  DEPRECATED - Use 'make ui' instead
.PHONY: store
store:
	@echo "⚠️  ERROR: 'make store' is DEPRECATED (old UI)"
	@echo "✅  Please use 'make ui' or 'make ui-app' for the NEW UI"
	@echo ""
	@echo "The new UI includes:"
	@echo "  • CLI Terminal sidebar"
	@echo "  • Improved interface"
	@echo "  • Better performance"
	@exit 1

# ── NEW UI (openclaw-plus) launch ─────────────────────────────────────────────
# ✅  This is the CORRECT UI with CLI Terminal sidebar

UI_BINARY := target/release/openclaw-plus
UI_APP_BUNDLE := /tmp/OpenClawPlus.app

# Build the new UI
.PHONY: build-ui
build-ui:
	$(CARGO) build --release -p openclaw-ui

# macOS: Create .app bundle for the NEW UI
.PHONY: ui-app
ui-app: build-ui
	@echo "✅  Creating NEW UI .app bundle with CLI Terminal..."
	mkdir -p $(UI_APP_BUNDLE)/Contents/MacOS $(UI_APP_BUNDLE)/Contents/Resources
	cp $(UI_BINARY) $(UI_APP_BUNDLE)/Contents/MacOS/openclaw-plus
	@printf '#!/bin/bash\nexport OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT)\nexport RUST_LOG=info\nDIR="$$(cd "$$(dirname "$$0")" && pwd)"\nexec "$$DIR/openclaw-plus" "$$@"\n' \
		> $(UI_APP_BUNDLE)/Contents/MacOS/openclaw-plus-launcher
	chmod +x $(UI_APP_BUNDLE)/Contents/MacOS/openclaw-plus-launcher
	@printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict>\n<key>CFBundleExecutable</key><string>openclaw-plus-launcher</string>\n<key>CFBundleIdentifier</key><string>dev.clawplus.ui</string>\n<key>CFBundleName</key><string>OpenClaw+ UI</string>\n<key>CFBundlePackageType</key><string>APPL</string>\n<key>NSHighResolutionCapable</key><true/>\n<key>NSPrincipalClass</key><string>NSApplication</string>\n</dict></plist>\n' \
		> $(UI_APP_BUNDLE)/Contents/Info.plist
	@echo "✅  NEW UI App bundle ready: $(UI_APP_BUNDLE)"
	@echo "    Features: CLI Terminal, AI Assistant, Security Dashboard"

# Launch the NEW UI directly (non-bundle)
.PHONY: ui
ui: build-ui
	@echo "✅  Launching NEW UI (openclaw-plus)..."
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	RUST_LOG=info \
	$(UI_BINARY)

# Launch NEW UI as .app bundle (macOS)
.PHONY: ui-open
ui-open: ui-app
	@echo "✅  Opening NEW UI .app bundle..."
	open -n $(UI_APP_BUNDLE)

# ── Run (gateway + NEW UI together) ───────────────────────────────────────────

.PHONY: run
ifeq ($(DETECTED_OS),macos)
run: ui-app
	@echo "Starting plugin gateway on port $(PORT)…"
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) \
	RUST_LOG=info \
	$(GATEWAY) --port $(PORT) &
	@sleep 1
	@echo "✅  Launching NEW UI with CLI Terminal..."
	open -n $(UI_APP_BUNDLE)
else ifeq ($(DETECTED_OS),linux)
run: build
	@echo "Starting plugin gateway on port $(PORT)…"
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) \
	RUST_LOG=info \
	$(GATEWAY) --port $(PORT) &
	@sleep 1
	@echo "Launching plugin store UI…"
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) \
	RUST_LOG=info \
	$(STORE) &
else ifeq ($(DETECTED_OS),windows)
run: build
	@echo "Starting gateway + store on Windows…"
	@cmd /C "start /B $(GATEWAY) --port $(PORT)"
	@ping -n 2 127.0.0.1 > nul
	@cmd /C "set OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) && \
	         set CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) && \
	         set RUST_LOG=info && \
	         start $(STORE)"
else
run: build
	$(GATEWAY) --port $(PORT) &
	sleep 1
	$(STORE)
endif

# ── Utility ───────────────────────────────────────────────────────────────────

.PHONY: clean
clean:
	$(CARGO) clean

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: clippy
clippy:
	$(CARGO) clippy \
		-p openclaw-security \
		-p openclaw-plugin-gateway \
		-p openclaw-store \
		-- -D warnings

# ── Gateway smoke-test (requires gateway to be running) ───────────────────────

.PHONY: smoke
smoke:
	@echo "=== /health ===" && curl -sf http://127.0.0.1:$(PORT)/health
	@echo "\n=== /ready ===" && curl -sf http://127.0.0.1:$(PORT)/ready
	@echo "\n=== /skills/status ===" && curl -sf http://127.0.0.1:$(PORT)/skills/status
	@echo "\n=== before-skill (file.read) ===" && \
		curl -sf http://127.0.0.1:$(PORT)/hooks/before-skill \
			-X POST -H "Content-Type: application/json" \
			-d '{"invocationId":"smoke-1","skillName":"file.read","sessionId":"smoke","args":{"path":"/tmp/x"},"timestamp":"2026-01-01T00:00:00Z"}'
	@echo "\n=== before-skill (shell.exec — denied) ===" && \
		curl -sf http://127.0.0.1:$(PORT)/hooks/before-skill \
			-X POST -H "Content-Type: application/json" \
			-d '{"invocationId":"smoke-2","skillName":"shell.exec","sessionId":"smoke","args":{"cmd":"ls"},"timestamp":"2026-01-01T00:00:00Z"}'
	@echo "\nSmoke test passed."

# ── Help ──────────────────────────────────────────────────────────────────────

.PHONY: help
help:
	@echo "OpenClaw+ build system  (detected OS: $(DETECTED_OS))"
	@echo ""
	@echo "Targets:"
	@echo "  build          Release build for current OS"
	@echo "  build-windows  Cross-compile for Windows (x86_64-pc-windows-gnu)"
	@echo "  build-linux    Cross-compile for Linux musl (x86_64-unknown-linux-musl)"
	@echo "  check          Fast cargo check"
	@echo "  test           Run all tests"
	@echo "  gateway        Start plugin gateway (port $(PORT))"
	@echo "  store          Launch store UI"
	@echo "  run            Start gateway + store UI"
	@echo "  fmt            cargo fmt"
	@echo "  clippy         cargo clippy -D warnings"
	@echo "  smoke          HTTP smoke-test (gateway must be running)"
	@echo "  clean          Remove build artefacts"
	@echo ""
	@echo "Override OS detection: make store OS_OVERRIDE=linux|macos|windows"
