use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;
use wasmedge_sdk::{
    config::{CommonConfigOptions, ConfigBuilder, HostRegistrationConfigOptions},
    params, Module, Store, Vm,
};

// ── WasmLoader ────────────────────────────────────────────────────────────────

/// Loads a compiled `.wasm` plugin into a fresh WasmEdge VM and invokes its
/// `_start` (or a named export) so the plugin can register itself.
///
/// Each call creates an isolated VM — plugins cannot share memory or state
/// across invocations, which preserves the sandbox boundary.
pub struct WasmLoader;

impl WasmLoader {
    /// Loads the `.wasm` file at `path` and calls `_start`.
    /// Returns `Ok(())` when the plugin initialises without trapping.
    pub fn load_and_start(path: &Path) -> Result<()> {
        let vm = Self::build_vm()?;
        let module = Module::from_file(None, path)
            .with_context(|| format!("loading WASM from {}", path.display()))?;

        let vm = vm
            .register_module(Some("plugin"), module)
            .context("registering plugin module")?;

        info!(path = %path.display(), "running plugin _start");
        vm.run_func(Some("plugin"), "_start", params!())
            .context("executing plugin _start")?;

        Ok(())
    }

    /// Loads the `.wasm` file and calls a named export function with no args.
    pub fn call_export(path: &Path, func: &str) -> Result<()> {
        let vm = Self::build_vm()?;
        let module = Module::from_file(None, path)
            .with_context(|| format!("loading WASM from {}", path.display()))?;

        let vm = vm
            .register_module(Some("plugin"), module)
            .context("registering plugin module")?;

        info!(path = %path.display(), func, "calling plugin export");
        vm.run_func(Some("plugin"), func, params!())
            .with_context(|| format!("calling export `{func}`"))?;

        Ok(())
    }

    /// Builds a WasmEdge VM with WASI enabled and a restricted capability set.
    fn build_vm() -> Result<Vm> {
        let config = ConfigBuilder::new(CommonConfigOptions::default())
            .with_host_registration_config(
                HostRegistrationConfigOptions::default().wasi(true),
            )
            .build()
            .context("building WasmEdge config")?;

        let store = Store::new(Some(&config), [])
            .context("creating WasmEdge store")?;

        let vm = Vm::new(Some(config), Some(store))
            .context("creating WasmEdge VM")?;

        Ok(vm)
    }
}
