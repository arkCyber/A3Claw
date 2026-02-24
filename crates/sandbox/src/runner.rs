#[cfg(feature = "wasm-sandbox")]
use crate::host_funcs::{build_import_object, HostContext};
#[allow(unused_imports)]
use crate::node_mock;
#[cfg(feature = "wasm-sandbox")]
use crate::wasi_builder::WasiBuilder;
use anyhow::Result;
use openclaw_security::{Interceptor, SecurityConfig};
use std::sync::Arc;
use tracing::{info, warn};
#[cfg(feature = "wasm-sandbox")]
use wasmedge_sdk::{
    config::{CommonConfigOptions, ConfigBuilder, HostRegistrationConfigOptions},
    params, Module, VmBuilder,
};

/// WasmEdge sandbox runner.
///
/// Responsible for loading the OpenClaw JavaScript bundle and executing it
/// inside a controlled WASI environment with capability-based security.
/// All sensitive operations are intercepted via registered host functions
/// before they reach the underlying OS.
#[allow(dead_code)]
pub struct SandboxRunner {
    config: SecurityConfig,
    interceptor: Arc<Interceptor>,
}

impl SandboxRunner {
    /// Creates a new `SandboxRunner`.
    ///
    /// # Parameters
    /// - `config`      — Active security configuration (mounts, allowlists, limits).
    /// - `interceptor` — Shared interceptor that evaluates every sandbox operation.
    pub fn new(config: SecurityConfig, interceptor: Arc<Interceptor>) -> Self {
        Self { config, interceptor }
    }

    /// Starts the sandbox: loads OpenClaw and runs it inside the WasmEdge VM.
    ///
    /// If the configured OpenClaw entry file does not exist, the runner falls
    /// back to **demo mode** which emits simulated events for UI testing.
    ///
    /// # Execution steps
    /// 1. Write the Node.js Security Shim to a temp file.
    /// 2. Start the IPC server in a background task.
    /// 3. Build the WasmEdge configuration.
    /// 4. Build WASI pre-opened directories (capability map).
    /// 5. Construct the VM instance.
    /// 6. Register the WASI module.
    /// 7. Register the security host-function ImportObject (`ocplus` module).
    /// 8. Load the WasmEdge-QuickJS engine WASM.
    /// 9. Emit a `SandboxStart` event.
    /// 10. Execute the QuickJS `_start` entry point.
    /// 11. Emit a `SandboxStop` event.
    pub async fn run(&self) -> Result<()> {
        // When WasmEdge is not available, always run in demo mode.
        #[cfg(not(feature = "wasm-sandbox"))]
        {
            warn!("WasmEdge not available — running in demo mode");
            return self.run_demo_mode().await;
        }

        #[cfg(feature = "wasm-sandbox")]
        self.run_wasm_sandbox().await
    }

    #[cfg(feature = "wasm-sandbox")]
    async fn run_wasm_sandbox(&self) -> Result<()> {
        // Check whether the OpenClaw entry script exists.
        let entry_path = &self.config.openclaw_entry;
        if !entry_path.exists() {
            warn!(
                "OpenClaw entry file not found: {:?} — falling back to demo mode",
                entry_path
            );
            return self.run_demo_mode().await;
        }

        info!("Loading OpenClaw entry: {:?}", entry_path);

        // ── Step 1: Write the Node.js Security Shim to a temp file ───
        let shim_path = node_mock::write_shim_to_temp()
            .context("Failed to write Node.js Security Shim")?;
        info!("Node.js Security Shim written to: {:?}", shim_path);

        // ── Step 2: Start the IPC server in a background task ─────────
        let (ipc_event_tx, ipc_event_rx) = flume::unbounded();
        let (ipc_control_tx, _ipc_control_rx) = flume::unbounded::<ControlCommand>();
        let ipc_server = IpcServer::new(ipc_event_rx, ipc_control_tx);
        tokio::spawn(async move {
            if let Err(e) = ipc_server.serve().await {
                info!("IPC server not started (UI not connected): {}", e);
            }
        });
        drop(ipc_event_tx);

        // ── Step 3: Build the WasmEdge configuration ──────────────────
        let wasm_config = ConfigBuilder::new(CommonConfigOptions::default())
            .with_host_registration_config(
                HostRegistrationConfigOptions::default().wasi(true),
            )
            .build()
            .context("Failed to build WasmEdge configuration")?;

        // ── Step 4: Build WASI pre-opened directories (capability map) ─
        let shim_dir = shim_path.parent().unwrap().to_path_buf();
        let mut wasi_args = WasiBuilder::new(&self.config)
            .with_shim(shim_path.clone())
            .build_wasi_args();
        wasi_args.preopens.push(format!(
            "{}:/shim:readonly",
            shim_dir.display()
        ));

        // ── Step 5: Construct the VM instance ─────────────────────────
        let mut vm = VmBuilder::new()
            .with_config(wasm_config)
            .build()
            .context("Failed to build WasmEdge VM")?;

        // ── Step 6: Register the WASI module ──────────────────────────
        let mut wasi_module = wasmedge_sdk::wasi::WasiModule::create(
            Some(wasi_args.args.iter().map(|s| s.as_str()).collect()),
            Some(wasi_args.envs.iter().map(|s| s.as_str()).collect()),
            Some(wasi_args.preopens.iter().map(|s| s.as_str()).collect()),
        )
        .context("Failed to initialise WASI module")?;
        vm.register_wasi_module(&mut wasi_module)
            .context("Failed to register WASI module")?;

        // ── Step 7: Register the security host-function ImportObject ───
        let host_ctx = HostContext::new(self.interceptor.clone());
        let import_obj = build_import_object(host_ctx)
            .context("Failed to build security host-function ImportObject")?;
        vm.register_import_module(import_obj)
            .context("Failed to register security host functions")?;
        info!("Security host functions registered (ocplus: check_file_read/write/delete/network/shell)");

        // ── Step 8: Load the WasmEdge-QuickJS engine ──────────────────
        let quickjs_wasm_path = self.find_quickjs_wasm()?;
        let module = Module::from_file(Some(&vm.config().unwrap()), &quickjs_wasm_path)
            .context("Failed to load QuickJS WASM")?;
        vm.register_module(Some("main"), module)
            .context("Failed to register QuickJS module")?;
        info!("WasmEdge-QuickJS engine loaded — starting OpenClaw…");

        // ── Step 9: Emit sandbox start event ──────────────────────────
        let start_event = openclaw_security::SandboxEvent::new(
            0,
            openclaw_security::EventKind::SandboxStart,
            openclaw_security::ResourceKind::System,
            None,
            "OpenClaw sandbox started (WasmEdge + Security Shim)",
        );
        let _ = self.interceptor.event_sender().send_async(start_event).await;

        // ── Step 10: Execute the QuickJS _start entry point ───────────
        match vm.run_func(Some("main"), "_start", params!()) {
            Ok(_) => info!("OpenClaw execution completed"),
            Err(e) => error!("OpenClaw execution error: {}", e),
        }

        // ── Step 11: Emit sandbox stop event ──────────────────────────
        let stop_event = openclaw_security::SandboxEvent::new(
            u64::MAX,
            openclaw_security::EventKind::SandboxStop,
            openclaw_security::ResourceKind::System,
            None,
            "OpenClaw sandbox stopped",
        );
        let _ = self.interceptor.event_sender().send_async(stop_event).await;

        Ok(())
    }

    /// Demo mode: emits simulated sandbox events when OpenClaw is not installed.
    ///
    /// Allows the UI to be tested and demonstrated without a real OpenClaw
    /// installation or WasmEdge runtime.
    async fn run_demo_mode(&self) -> Result<()> {
        use openclaw_security::{EventKind, ResourceKind, SandboxEvent};
        use tokio::time::{sleep, Duration};

        info!("Entering demo mode — emitting simulated sandbox events…");

        let sender = self.interceptor.event_sender();

        let demo_events = vec![
            SandboxEvent::new(1, EventKind::SandboxStart,        ResourceKind::System,  None,                                                      "OpenClaw sandbox started (demo mode)"),
            SandboxEvent::new(2, EventKind::FileAccess,          ResourceKind::File,    Some("/workspace/config.json".to_string()),                  "Read config file"),
            SandboxEvent::new(3, EventKind::NetworkRequest,      ResourceKind::Network, Some("api.openai.com".to_string()),                          "Request to OpenAI API"),
            SandboxEvent::new(4, EventKind::FileWrite,           ResourceKind::File,    Some("/workspace/output/result.json".to_string()),            "Write execution result"),
            SandboxEvent::new(5, EventKind::ShellExec,           ResourceKind::Process, None,                                                      "ls -la /workspace"),
            SandboxEvent::new(6, EventKind::UserConfirmRequired, ResourceKind::File,    Some("/workspace/important.txt".to_string()),                "Delete file: /workspace/important.txt"),
            SandboxEvent::new(7, EventKind::NetworkRequest,      ResourceKind::Network, Some("unknown-host.example.com".to_string()),                "Request to unknown host (blocked)"),
            SandboxEvent::new(8, EventKind::SandboxStop,         ResourceKind::System,  None,                                                      "OpenClaw sandbox stopped (demo mode)"),
        ];

        for mut event in demo_events {
            sleep(Duration::from_millis(800)).await;

            // Simulate policy decisions for demo events.
            event.allowed = match event.kind {
                EventKind::FileAccess | EventKind::FileWrite | EventKind::SandboxStart | EventKind::SandboxStop => Some(true),
                EventKind::NetworkRequest if event.path.as_deref() == Some("unknown-host.example.com") => Some(false),
                EventKind::NetworkRequest => Some(true),
                EventKind::ShellExec => Some(true),
                EventKind::UserConfirmRequired => None,
                _ => None,
            };

            let _ = sender.send_async(event).await;
        }

        // Demo mode: keep the sandbox "running" indefinitely by emitting a
        // heartbeat event every 30 seconds. This prevents the task from
        // completing and triggering a Stopped/Error status in the UI.
        // The loop exits only when the sender channel is closed (app shutdown).
        let mut heartbeat_seq: u64 = 100;
        loop {
            sleep(Duration::from_secs(30)).await;
            heartbeat_seq += 1;
            let hb = SandboxEvent::new(
                heartbeat_seq,
                EventKind::SandboxStart,  // reuse SandboxStart kind as a heartbeat marker
                openclaw_security::ResourceKind::System,
                None,
                "sandbox heartbeat (demo mode — idle)",
            );
            // If the receiver is gone (app shut down), exit cleanly.
            if sender.send_async(hb).await.is_err() {
                break;
            }
        }

        Ok(())
    }

    /// Searches well-known locations for the `wasmedge_quickjs.wasm` engine file.
    ///
    /// Search order:
    /// 1. `assets/wasmedge_quickjs.wasm` (relative to the working directory)
    /// 2. `{data_dir}/openclaw-plus/wasmedge_quickjs.wasm`
    /// 3. `/usr/share/openclaw-plus/wasmedge_quickjs.wasm`
    ///
    /// # Errors
    /// Returns an error listing all searched paths if the file is not found.
    #[cfg(feature = "wasm-sandbox")]
    fn find_quickjs_wasm(&self) -> Result<std::path::PathBuf> {
        let candidates = vec![
            std::path::PathBuf::from("assets/wasmedge_quickjs.wasm"),
            dirs::data_dir()
                .unwrap_or_default()
                .join("openclaw-plus")
                .join("wasmedge_quickjs.wasm"),
            std::path::PathBuf::from("/usr/share/openclaw-plus/wasmedge_quickjs.wasm"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        anyhow::bail!(
            "wasmedge_quickjs.wasm not found. Place it in one of:\n{}",
            candidates.iter().map(|p| format!("  - {:?}", p)).collect::<Vec<_>>().join("\n")
        )
    }
}
