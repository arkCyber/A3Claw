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

# macOS: wrap in a .app bundle so Metal / window focus works correctly.
.PHONY: macos-app
macos-app: build
	mkdir -p $(APP_BUNDLE)/Contents/MacOS $(APP_BUNDLE)/Contents/Resources
	cp $(STORE) $(APP_BUNDLE)/Contents/MacOS/openclaw-store
	@printf '#!/bin/bash\nexport OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT)\nexport CLAWPLUS_REGISTRY_URL=file://$(REGISTRY)\nexport RUST_LOG=info\nDIR="$$(cd "$$(dirname "$$0")" && pwd)"\nexec "$$DIR/openclaw-store" "$$@"\n' \
		> $(APP_BUNDLE)/Contents/MacOS/openclaw-store-launcher
	chmod +x $(APP_BUNDLE)/Contents/MacOS/openclaw-store-launcher
	@printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict>\n<key>CFBundleExecutable</key><string>openclaw-store-launcher</string>\n<key>CFBundleIdentifier</key><string>dev.clawplus.store</string>\n<key>CFBundleName</key><string>OpenClaw+ Store</string>\n<key>CFBundlePackageType</key><string>APPL</string>\n<key>NSHighResolutionCapable</key><true/>\n<key>NSPrincipalClass</key><string>NSApplication</string>\n</dict></plist>\n' \
		> $(APP_BUNDLE)/Contents/Info.plist
	@echo "App bundle ready: $(APP_BUNDLE)"

# Unified 'store' target — dispatches to the right launcher per OS.
.PHONY: store
ifeq ($(DETECTED_OS),macos)
store: macos-app
	open -n $(APP_BUNDLE)
else ifeq ($(DETECTED_OS),linux)
store: build
	@echo "Launching store UI on Linux…"
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) \
	RUST_LOG=info \
	$(STORE) &
else ifeq ($(DETECTED_OS),windows)
store: build
	@echo "Launching store UI on Windows…"
	@cmd /C "set OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) && \
	         set CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) && \
	         set RUST_LOG=info && \
	         start $(STORE)"
else
store: build
	@echo "Unknown OS — running store binary directly."
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) \
	RUST_LOG=info \
	$(STORE)
endif

# ── Run (gateway + store together) ────────────────────────────────────────────

.PHONY: run
ifeq ($(DETECTED_OS),macos)
run: macos-app
	@echo "Starting plugin gateway on port $(PORT)…"
	OPENCLAW_GATEWAY_URL=http://127.0.0.1:$(PORT) \
	CLAWPLUS_REGISTRY_URL=file://$(REGISTRY) \
	RUST_LOG=info \
	$(GATEWAY) --port $(PORT) &
	@sleep 1
	@echo "Launching plugin store UI…"
	open -n $(APP_BUNDLE)
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
