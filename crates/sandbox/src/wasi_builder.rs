use openclaw_security::SecurityConfig;
use std::path::PathBuf;
use tracing::info;

/// Arguments passed to the WasmEdge WASI module at VM startup.
pub struct WasiArgs {
    /// Command-line arguments (argv) for the WASM module.
    pub args: Vec<String>,
    /// Environment variables exposed inside the sandbox.
    pub envs: Vec<String>,
    /// Pre-opened directory mappings in the format
    /// `"host_path:guest_path"` or `"host_path:guest_path:readonly"`.
    pub preopens: Vec<String>,
}

/// Builds [`WasiArgs`] from a [`SecurityConfig`], mapping filesystem mounts
/// and injecting the Node.js Security Shim as a `--pre-script` argument.
pub struct WasiBuilder<'a> {
    config: &'a SecurityConfig,
    /// Optional path to the Node.js Security Shim.
    /// When set, it is injected as `--pre-script /shim/security_shim.js`.
    shim_path: Option<PathBuf>,
}

impl<'a> WasiBuilder<'a> {
    /// Creates a new `WasiBuilder` for the given configuration.
    pub fn new(config: &'a SecurityConfig) -> Self {
        Self { config, shim_path: None }
    }

    /// Sets the Security Shim path to inject as `--pre-script` into the
    /// WasmEdge-QuickJS execution environment.
    pub fn with_shim(mut self, shim_path: PathBuf) -> Self {
        self.shim_path = Some(shim_path);
        self
    }

    /// Builds the final [`WasiArgs`] from the configuration.
    ///
    /// WasmEdge-QuickJS argument format:
    /// ```text
    /// wasmedge_quickjs.wasm [--pre-script <shim>] <entry_script>
    /// ```
    pub fn build_wasi_args(&self) -> WasiArgs {
        let mut args = vec!["wasmedge_quickjs.wasm".to_string()];

        // Inject the Security Shim before the OpenClaw entry script.
        if let Some(shim) = &self.shim_path {
            args.push("--pre-script".to_string());
            args.push("/shim/security_shim.js".to_string());
            info!("Security Shim configured as pre-script: {:?} -> /shim/security_shim.js", shim);
        }

        args.push(self.config.openclaw_entry.to_string_lossy().to_string());

        let envs = vec![
            "OPENCLAW_SANDBOX=1".to_string(),
            format!("OPENCLAW_WORKSPACE={}", self.config.workspace_dir.display()),
            format!("OPENCLAW_MEMORY_LIMIT={}", self.config.memory_limit_mb),
        ];

        // Build pre-opened directory list (capability map).
        let mut preopens = Vec::new();
        for mount in &self.config.fs_mounts {
            let host = mount.host_path.to_string_lossy();
            let guest = &mount.guest_path;

            // Create the host directory if it does not exist yet.
            if !mount.host_path.exists() {
                let _ = std::fs::create_dir_all(&mount.host_path);
                info!("Created sandbox workspace directory: {:?}", mount.host_path);
            }

            let preopen = if mount.readonly {
                format!("{}:{}:readonly", host, guest)
            } else {
                format!("{}:{}", host, guest)
            };

            info!("Mount: {} -> {} (read-only: {})", host, guest, mount.readonly);
            preopens.push(preopen);
        }

        // Mount the OpenClaw source directory as read-only.
        if let Some(parent) = self.config.openclaw_entry.parent() {
            if parent.exists() {
                preopens.push(format!(
                    "{}:/openclaw:readonly",
                    parent.display()
                ));
                info!("Mounted OpenClaw source directory (read-only): {:?}", parent);
            }
        }

        WasiArgs { args, envs, preopens }
    }
}
