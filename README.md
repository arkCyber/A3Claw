# OpenClaw+ 🛡️

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![WasmEdge](https://img.shields.io/badge/WasmEdge-0.14%2B-blue.svg)](https://wasmedge.org/)

> **AI Agent Security Platform** — A comprehensive AI agent execution platform with WasmEdge sandbox, visual workflow editor, and enterprise-grade security controls.

[English](README.md) | [中文](README_ZH.md)

OpenClaw+ wraps OpenClaw in a WasmEdge WASI sandbox **without modifying a single line of OpenClaw's source code**, providing:

- 🔒 **Filesystem isolation** — only the configured workspace directory is accessible; sensitive paths such as `.ssh/` and `/etc/passwd` are blocked automatically.
- 🌐 **Network access control** — outbound connections are restricted to an explicit allowlist of LLM API hostnames.
- 💻 **Shell command interception** — every shell execution attempt requires explicit user approval (human-in-the-loop).
- 🗑️ **File deletion protection** — a confirmation dialog appears before any deletion, preventing accidental data loss.
- 📊 **Real-time monitoring dashboard** — a native libcosmic UI visualises all sandbox events as they happen.
- 🔴 **Circuit breaker** — automatically trips and terminates the sandbox when anomaly thresholds are exceeded (too many denials, dangerous commands, or memory overuse).
- 📝 **Audit log** — every operation is persisted as NDJSON for post-hoc review.

## Architecture

```
┌─────────────────────────────────────────────┐
│         libcosmic Monitoring UI             │
│  Dashboard | Event Log | Settings | Confirm │
└──────────────┬──────────────────────────────┘
               │ event stream / control commands
┌──────────────▼──────────────────────────────┐
│      Rust Security Layer (openclaw-security) │
│  PolicyEngine | Interceptor | AuditLog      │
│  CircuitBreaker                             │
└──────────────┬──────────────────────────────┘
               │ WASI syscall interception
┌──────────────▼──────────────────────────────┐
│     WasmEdge Sandbox (openclaw-sandbox)     │
│  WasmEdge-QuickJS | WASI capability map     │
│  Node.js Security Shim (pre-script)         │
└──────────────┬──────────────────────────────┘
               │ controlled filesystem view
┌──────────────▼──────────────────────────────┐
│        OpenClaw source (unmodified)         │
│  Runs as if in a standard Node.js env       │
└─────────────────────────────────────────────┘
```

## Project Structure

```
OpenClaw+/
├── Cargo.toml                    # Workspace root
├── openclaw.plugin.json          # OpenClaw plugin manifest
├── config/
│   └── default.toml              # Default security configuration template
├── crates/
│   ├── security/                 # Security policy engine (core library)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs          # SandboxEvent, EventKind, ControlCommand
│   │       ├── config.rs         # SecurityConfig (TOML-backed)
│   │       ├── policy.rs         # PolicyEngine — allow / deny / confirm
│   │       ├── interceptor.rs    # Interceptor — central interception hub
│   │       ├── audit.rs          # AuditLog — NDJSON append-only log
│   │       └── circuit_breaker.rs# CircuitBreaker — anomaly auto-termination
│   ├── sandbox/                  # WasmEdge host process (embedded mode)
│   │   └── src/
│   │       ├── main.rs           # Binary entry point
│   │       ├── runner.rs         # SandboxRunner — VM lifecycle
│   │       ├── wasi_builder.rs   # WASI capability map builder
│   │       ├── host_funcs.rs     # Security host-function registration
│   │       ├── node_mock.rs      # Node.js API shim (pre-script)
│   │       └── ipc.rs            # IPC server (sandbox ↔ UI)
│   ├── plugin/                   # OpenClaw plugin gateway (plugin mode)
│   │   └── src/
│   │       ├── main.rs           # HTTP server entry point
│   │       ├── router.rs         # Axum routes (hooks + skills + admin)
│   │       ├── state.rs          # Shared GatewayState (Arc)
│   │       ├── skill_registry.rs # Skill name → RiskLevel mapping
│   │       └── types.rs          # Hook request / response JSON types
│   └── ui/                       # libcosmic monitoring UI
│       └── src/
│           ├── main.rs
│           ├── app.rs            # OpenClawApp (embedded + plugin modes)
│           ├── ipc_client.rs     # IPC client for external sandbox process
│           ├── pages/
│           │   ├── dashboard.rs  # Overview, stats, confirmations
│           │   ├── events.rs     # Full event log table
│           │   ├── settings.rs   # Security configuration viewer
│           │   └── confirm.rs    # Inline confirmation dialog
│           └── widgets/
│               ├── status_badge.rs
│               └── event_chip.rs
└── .github/
    └── workflows/
        ├── ci.yml                # CI checks (fmt, clippy, test)
        └── release.yml           # Multi-platform release builds
```

## Deployment Modes

OpenClaw+ supports two deployment modes:

| Mode         | How it works                                                                        | When to use                            |
| ------------ | ----------------------------------------------------------------------------------- | -------------------------------------- |
| **Embedded** | UI spawns WasmEdge in-process; events flow over `flume` channels                    | Development, standalone use            |
| **Plugin**   | Registered as an OpenClaw plugin; gateway intercepts all Skill calls via HTTP hooks | Production, OpenClaw already installed |

## Quick Start

### Prerequisites

```bash
# Install WasmEdge (required for embedded mode)
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash

# macOS
brew install pkg-config

# Ubuntu / Debian
sudo apt-get install -y libwayland-dev libxkbcommon-dev pkg-config cmake \
    libfontconfig1-dev libfreetype6-dev
```

### Build

```bash
git clone https://github.com/arkCyber/A3Claw
cd openclaw-plus

# Build all crates
cargo build --release
```

### Option A — Embedded Mode (standalone)

```bash
# Bundle OpenClaw into a single JS file
./scripts/bundle_openclaw.sh

# Run the monitoring UI (launches WasmEdge sandbox in-process)
cargo run --release -p openclaw-ui
```

### Option B — Plugin Mode (recommended for OpenClaw users)

Install the plugin so OpenClaw loads it automatically:

```bash
# Register the plugin with OpenClaw
openclaw plugins install ./openclaw.plugin.json

# Verify it is loaded
openclaw plugins list
```

OpenClaw will launch `openclaw-plugin-gateway` on startup and set
`OPENCLAW_GATEWAY_URL` before starting the monitoring UI. The UI
automatically detects this environment variable and switches to plugin mode.

To open the monitoring dashboard manually:

```bash
# The gateway URL is printed to stdout as GATEWAY_PORT=<n> on startup
export OPENCLAW_GATEWAY_URL=http://127.0.0.1:<port>
cargo run --release -p openclaw-ui
```

### Configure Security Policy

Edit `~/.config/openclaw-plus/config.toml`:

```toml
# Path to your OpenClaw build output (embedded mode only)
openclaw_entry = "/path/to/openclaw/dist/index.js"

# Sandbox workspace directory (mapped to /workspace inside the sandbox)
workspace_dir = "/path/to/your/workspace"

# Network allowlist
network_allowlist = [
    "api.openai.com",
    "api.anthropic.com",
    "api.deepseek.com",
    "openrouter.ai",
]

# Circuit-breaker thresholds
[circuit_breaker]
denial_window_secs      = 10
max_denials_per_window  = 20
max_dangerous_commands  = 3
```

### Plugin Skill API

When running in plugin mode, the gateway exposes these HTTP endpoints:

| Method  | Path                     | Description                        |
| ------- | ------------------------ | ---------------------------------- |
| `GET`   | `/health`                | Liveness probe                     |
| `GET`   | `/ready`                 | Readiness probe                    |
| `POST`  | `/hooks/before-skill`    | Intercept a Skill before execution |
| `POST`  | `/hooks/after-skill`     | Record a Skill after execution     |
| `POST`  | `/hooks/confirm`         | Resolve a pending confirmation     |
| `GET`   | `/skills/status`         | Security status snapshot           |
| `GET`   | `/skills/events?limit=N` | Recent audit events                |
| `PATCH` | `/skills/policy`         | Update security policy at runtime  |
| `POST`  | `/skills/allow/:id`      | Allow a pending confirmation       |
| `POST`  | `/skills/deny/:id`       | Deny a pending confirmation        |
| `POST`  | `/admin/emergency-stop`  | Trip the circuit breaker           |

### Skill Risk Levels

Every OpenClaw Skill is classified before execution:

| Risk Level  | Examples                                            | Action                     |
| ----------- | --------------------------------------------------- | -------------------------- |
| **Safe**    | `fs.readFile`, `search.query`, `knowledge.retrieve` | Allowed without prompting  |
| **Confirm** | `fs.writeFile`, `email.send`, `web.navigate`        | Paused until user approves |
| **Deny**    | `shell.exec`, `process.spawn`, `system.reboot`      | Blocked unconditionally    |

## Default Security Policy

| Operation                       | Default                     | Notes                                 |
| ------------------------------- | --------------------------- | ------------------------------------- |
| File read                       | ✅ Allow (within workspace)  | Paths outside workspace are denied    |
| File write                      | ✅ Allow (within workspace)  | Paths outside workspace are denied    |
| File delete                     | ⏳ Confirm                   | Dialog shown; auto-denied after 30 s  |
| Network request                 | ✅ Allow (allowlisted hosts) | All other hosts are denied            |
| Shell execution                 | ⏳ Confirm                   | High-risk commands denied immediately |
| Sensitive paths (`.ssh/`, etc.) | 🚫 Deny                      | Hard-coded security rules             |

## AI Inference Backend

OpenClaw+ includes an integrated AI inference engine with multiple backend options:

- **WASI-NN** (WasmEdge + llama.cpp): In-process inference with GGUF models
- **LlamaCppHttp**: HTTP API for llama.cpp server
- **Ollama**: Local Ollama server
- **OpenAI-compatible**: OpenAI, Anthropic, DeepSeek, Gemini, etc.

### WASI-NN Quick Start

```bash
# Install WasmEdge with wasi_nn plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh \
  | bash -s -- --plugins wasi_nn-ggml

# Download a GGUF model
mkdir -p models/gguf
curl -L -o models/gguf/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf"

# Build with wasi-nn feature
cargo build --release --features wasi-nn

# Run tests
cargo test --features wasi-nn --test wasi_nn_integration
```

📖 **Full guide**: [WASI-NN Integration Guide](docs/WASI_NN_GUIDE.md)

## Tech Stack

- **Runtime**: [WasmEdge](https://wasmedge.org/) 0.16+ (WASI + QuickJS + wasi_nn)
- **UI framework**: [libcosmic](https://github.com/pop-os/libcosmic) (iced-based)
- **Language**: Rust 2021 Edition
- **Async**: Tokio
- **AI Inference**: wasmedge-sdk 0.14 + llama.cpp (via WASI-NN)
- **Configuration**: TOML
- **Distribution**: GitHub Actions + cargo-dist

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## 📧 Contact

- **Author**: arksong2018@gmail.com
- **Issues**: [GitHub Issues](https://github.com/arkCyber/A3Claw/issues)

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [WasmEdge](https://wasmedge.org/) - High-performance WebAssembly runtime
- [libcosmic](https://github.com/pop-os/libcosmic) - Modern UI framework
- [Rust](https://www.rust-lang.org/) - Systems programming language

---

**Built with ❤️ by the OpenClaw+ Team**
