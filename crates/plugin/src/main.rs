//! OpenClaw+ Plugin Gateway — binary entry point.
//!
//! Starts an HTTP server that implements the OpenClaw plugin hook protocol.
//! OpenClaw calls this server before and after every Skill execution, allowing
//! the security layer to intercept, audit, and block operations in real time.
//!
//! Usage (normally launched by OpenClaw via the plugin manifest):
//! ```text
//! openclaw-plugin-gateway [--config <path>] [--port <port>]
//! ```

use anyhow::{Context, Result};
use openclaw_security::SecurityConfig;
use std::net::SocketAddr;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod router;
mod skill_registry;
mod state;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("openclaw_plugin_gateway=debug,info")),
        )
        .init();

    info!("OpenClaw+ Plugin Gateway starting");

    // ── CLI args ──────────────────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();
    let config_path = parse_flag(&args, "--config")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            dirs::config_dir().map(|d| d.join("openclaw-plus").join("config.toml"))
        });

    let port: u16 = parse_flag(&args, "--port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0); // 0 = OS assigns a free port

    // ── Load config ───────────────────────────────────────────────────────────
    let config = if let Some(ref path) = config_path {
        if path.exists() {
            SecurityConfig::load_from_file(path)
                .with_context(|| format!("Failed to load config from {:?}", path))?
        } else {
            warn!("Config file not found at {:?}, using defaults", path);
            SecurityConfig::default()
        }
    } else {
        warn!("No config path resolved, using defaults");
        SecurityConfig::default()
    };

    info!("Loaded security config: memory_limit={}MB, intercept_shell={}, network_allowlist={} entries",
        config.memory_limit_mb,
        config.intercept_shell,
        config.network_allowlist.len()
    );

    // ── Build shared state ────────────────────────────────────────────────────
    let state = state::GatewayState::new(config);

    // ── Build router ──────────────────────────────────────────────────────────
    let app = router::build_router(state.clone());

    // ── Bind listener ─────────────────────────────────────────────────────────
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind HTTP listener")?;

    let bound_addr = listener.local_addr()?;
    info!("Plugin gateway listening on http://{}", bound_addr);

    // Print the bound port to stdout so OpenClaw can read it and register
    // the gateway URL in the plugin manifest at runtime.
    println!("GATEWAY_PORT={}", bound_addr.port());

    // Mark as ready only after the listener is bound.
    state.set_ready();

    // ── Serve ─────────────────────────────────────────────────────────────────
    axum::serve(listener, app)
        .await
        .context("HTTP server error")?;

    Ok(())
}

/// Parses `--flag value` from `args`, returning `Some(value)` if found.
fn parse_flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].as_str())
}
