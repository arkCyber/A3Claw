//! `openclaw-sandbox` binary entry point.
//!
//! Loads the security configuration, initialises the policy engine and audit log,
//! then hands control to [`SandboxRunner`] which starts the WasmEdge VM and runs
//! the OpenClaw JavaScript bundle inside the WASI sandbox.
//!
//! When run without a valid OpenClaw entry file the runner falls back to
//! **demo mode**, which emits simulated events so the UI can be tested
//! independently.

use anyhow::Result;
use openclaw_sandbox::runner::SandboxRunner;
use openclaw_security::{AuditLog, Interceptor, PolicyEngine, SecurityConfig};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise structured logging; respect RUST_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("openclaw_sandbox=info".parse()?),
        )
        .init();

    info!("OpenClaw+ sandbox starting…");

    // Load or create the default security configuration.
    let config = SecurityConfig::load_or_default();

    // Create the event and control channels.
    // In standalone mode the event receiver is dropped; the UI connects via IPC.
    let (event_tx, _event_rx) = flume::unbounded();
    let (_control_tx, control_rx) = flume::unbounded();

    // Build the security stack.
    let audit = AuditLog::new(config.audit_log_path.clone());
    let policy = PolicyEngine::new(config.clone());
    let interceptor =
        std::sync::Arc::new(Interceptor::new(policy, audit, event_tx, control_rx));

    info!("Security policy engine initialised");
    info!("Memory limit   : {} MB", config.memory_limit_mb);
    info!("Shell intercept: {}", config.intercept_shell);
    info!("Delete confirm : {}", config.confirm_file_delete);

    let runner = SandboxRunner::new(config, interceptor.clone());

    match runner.run().await {
        Ok(_) => info!("Sandbox exited normally"),
        Err(e) => error!("Sandbox exited with error: {}", e),
    }

    Ok(())
}
