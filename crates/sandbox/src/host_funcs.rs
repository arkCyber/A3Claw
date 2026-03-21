#![cfg(feature = "wasm-sandbox")]
//! WasmEdge host-function registration for the `ocplus` security module.
//!
//! # Architecture
//!
//! When WasmEdge-QuickJS runs OpenClaw's JavaScript bundle, the injected
//! Node.js Security Shim calls `globalThis.__ocplus_*` functions before every
//! sensitive operation. These functions cross the WASM boundary and are
//! dispatched here, where the Rust [`Interceptor`] evaluates them against the
//! active [`PolicyEngine`].
//!
//! Return convention: `1` = allowed, `0` = denied.
//!
//! ## Example shim (injected before OpenClaw entry script)
//!
//! ```js
//! const _orig_readFile = fs.readFileSync;
//! fs.readFileSync = (path, ...args) => {
//!   if (!globalThis.__ocplus_check_file_read(path)) {
//!     throw new Error(`[OpenClaw+] Security policy denied read: ${path}`);
//!   }
//!   return _orig_readFile(path, ...args);
//! };
//! ```

use anyhow::{Context, Result};
use openclaw_security::{InterceptResult, Interceptor};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, warn, error};
use wasmedge_sdk::{
    CallingFrame,
    ImportObjectBuilder,
    WasmValue,
};

/// Shared context passed to every WasmEdge host function closure.
///
/// Holds a reference to the [`Interceptor`] and a handle to the Tokio runtime
/// so that async interception calls can be driven synchronously from the
/// WASM host-function boundary (which is inherently synchronous).
///
/// Must be `Clone + Send + Sync` to satisfy the WasmEdge SDK constraints.
#[derive(Clone)]
pub struct HostContext {
    pub interceptor: Arc<Interceptor>,
    pub runtime: tokio::runtime::Handle,
    /// Rate limiting: tracks the number of security checks per second
    check_count: Arc<AtomicU64>,
    /// Last reset time for rate limiting
    last_reset: Arc<Mutex<Instant>>,
}

impl HostContext {
    /// Creates a new `HostContext` capturing the current Tokio runtime handle.
    pub fn new(interceptor: Arc<Interceptor>) -> Self {
        Self {
            interceptor,
            runtime: tokio::runtime::Handle::current(),
            check_count: Arc::new(AtomicU64::new(100)), // <--- updated initialization
            last_reset: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Checks rate limit before processing security check.
    /// Returns true if within rate limit, false if exceeded.
    fn check_rate_limit(&self) -> bool {
        const MAX_CHECKS_PER_SECOND: u64 = 1000;
        
        let count = self.check_count.fetch_add(1, Ordering::Relaxed);
        
        if count > MAX_CHECKS_PER_SECOND {
            let mut last_reset = self.last_reset.lock().unwrap();
            if last_reset.elapsed() >= Duration::from_secs(1) {
                // Reset counter
                *last_reset = Instant::now();
                self.check_count.store(0, Ordering::Relaxed);
                true
            } else {
                warn!("Rate limit exceeded: {} checks/sec", count);
                false
            }
        } else {
            true
        }
    }

    /// Drives an async interception call to completion on the current Tokio
    /// runtime, blocking the calling thread until the policy decision is made.
    ///
    /// Includes timeout protection and rate limiting.
    /// Returns `1` for [`InterceptResult::Allow`] and `0` for [`InterceptResult::Deny`].
    fn sync_check<F, Fut>(&self, f: F) -> i32
    where
        F: FnOnce(Arc<Interceptor>) -> Fut,
        Fut: std::future::Future<Output = InterceptResult>,
    {
        if !self.check_rate_limit() {
            return 0;
        }
        let interceptor = self.interceptor.clone();
        let timeout_duration = Duration::from_secs(5);
        let result = self.runtime.block_on(async {
            match tokio::time::timeout(timeout_duration, f(interceptor)).await {
                Ok(r) => r,
                Err(_) => {
                    error!("Security check timed out after 5 seconds");
                    InterceptResult::Deny("Security check timeout".to_string())
                }
            }
        });
        
        match result {
            InterceptResult::Allow => 1,
            InterceptResult::Deny(_) => 0,
        }
    }

    /// Checks whether a file read operation is permitted.
    pub fn check_file_read(&self, path: &str) -> i32 {
        debug!("host fn: check_file_read({})", path);
        let p = path.to_string();
        self.sync_check(move |i| async move { i.intercept_file_access(&p).await })
    }

    /// Checks whether a file write operation is permitted.
    pub fn check_file_write(&self, path: &str) -> i32 {
        debug!("host fn: check_file_write({})", path);
        let p = path.to_string();
        self.sync_check(move |i| async move { i.intercept_file_write(&p).await })
    }

    /// Checks whether a file deletion is permitted.
    pub fn check_file_delete(&self, path: &str) -> i32 {
        debug!("host fn: check_file_delete({})", path);
        let p = path.to_string();
        self.sync_check(move |i| async move { i.intercept_file_delete(&p).await })
    }

    /// Checks whether an outbound network request is permitted.
    pub fn check_network(&self, host: &str, url: &str) -> i32 {
        debug!("host fn: check_network({}, {})", host, url);
        let h = host.to_string();
        let u = url.to_string();
        self.sync_check(move |i| async move { i.intercept_network(&h, &u).await })
    }

    /// Checks whether a shell command execution is permitted.
    pub fn check_shell_exec(&self, command: &str) -> i32 {
        warn!("host fn: check_shell_exec({})", command);
        let c = command.to_string();
        self.sync_check(move |i| async move { i.intercept_shell_exec(&c).await })
    }
}

/// Reads a UTF-8 string from the WASM linear memory.
///
/// # Parameters
/// - `ptr` — byte offset of the string in WASM memory (linear memory index 0).
/// - `len` — byte length of the string.
///
/// # Security
/// - Validates pointer and length are non-negative
/// - Enforces maximum string length (1MB)
/// - Validates UTF-8 encoding
fn read_wasm_string(frame: &CallingFrame, ptr: i32, len: i32) -> Result<String> {
    // Boundary checks
    if ptr < 0 {
        anyhow::bail!("Invalid WASM pointer: {} (must be non-negative)", ptr);
    }
    if len < 0 {
        anyhow::bail!("Invalid WASM length: {} (must be non-negative)", len);
    }
    if len > 1_000_000 {
        anyhow::bail!("WASM string too large: {} bytes (max 1MB)", len);
    }
    
    let mem = frame
        .memory_ref(0)
        .context("Failed to access WASM linear memory")?;
    let bytes = mem
        .get_data(ptr as u32, len as u32)
        .context("Failed to read WASM memory")?;
    String::from_utf8(bytes.to_vec()).context("WASM string is not valid UTF-8")
}

/// Builds and returns the `ocplus` [`ImportObject`] containing all security host functions.
///
/// Registered WASM-visible function signatures:
/// - `ocplus.check_file_read(ptr: i32, len: i32) -> i32`
/// - `ocplus.check_file_write(ptr: i32, len: i32) -> i32`
/// - `ocplus.check_file_delete(ptr: i32, len: i32) -> i32`
/// - `ocplus.check_network(host_ptr: i32, host_len: i32, url_ptr: i32, url_len: i32) -> i32`
/// - `ocplus.check_shell(cmd_ptr: i32, cmd_len: i32) -> i32`
///
/// Return value: `1` = allowed, `0` = denied.
pub fn build_import_object(ctx: HostContext) -> Result<wasmedge_sdk::ImportObject<HostContext>> {
    let mut import = ImportObjectBuilder::new("ocplus", ctx)
        .context("Failed to create ImportObjectBuilder")?;

    // ── File read check ───────────────────────────────────────────
    import
        .with_func::<(i32, i32), i32>(
            "check_file_read",
            |data: &mut HostContext, _inst, frame: &mut CallingFrame, args: Vec<WasmValue>| {
                let ptr = args[0].to_i32();
                let len = args[1].to_i32();
                let path = read_wasm_string(&frame, ptr, len)
                    .unwrap_or_else(|_| "<invalid utf8>".to_string());
                let result = data.check_file_read(&path);
                Ok(vec![WasmValue::from_i32(result)])
            },
        )
        .context("Failed to register check_file_read")?;

    // ── File write check ──────────────────────────────────────────
    import
        .with_func::<(i32, i32), i32>(
            "check_file_write",
            |data: &mut HostContext, _inst, frame: &mut CallingFrame, args: Vec<WasmValue>| {
                let ptr = args[0].to_i32();
                let len = args[1].to_i32();
                let path = read_wasm_string(&frame, ptr, len)
                    .unwrap_or_else(|_| "<invalid utf8>".to_string());
                let result = data.check_file_write(&path);
                Ok(vec![WasmValue::from_i32(result)])
            },
        )
        .context("Failed to register check_file_write")?;

    // ── File delete check ─────────────────────────────────────────
    import
        .with_func::<(i32, i32), i32>(
            "check_file_delete",
            |data: &mut HostContext, _inst, frame: &mut CallingFrame, args: Vec<WasmValue>| {
                let ptr = args[0].to_i32();
                let len = args[1].to_i32();
                let path = read_wasm_string(&frame, ptr, len)
                    .unwrap_or_else(|_| "<invalid utf8>".to_string());
                let result = data.check_file_delete(&path);
                Ok(vec![WasmValue::from_i32(result)])
            },
        )
        .context("Failed to register check_file_delete")?;

    // ── Network request check ─────────────────────────────────────
    import
        .with_func::<(i32, i32, i32, i32), i32>(
            "check_network",
            |data: &mut HostContext, _inst, frame: &mut CallingFrame, args: Vec<WasmValue>| {
                let host_ptr = args[0].to_i32();
                let host_len = args[1].to_i32();
                let url_ptr  = args[2].to_i32();
                let url_len  = args[3].to_i32();
                let host = read_wasm_string(&frame, host_ptr, host_len)
                    .unwrap_or_default();
                let url = read_wasm_string(&frame, url_ptr, url_len)
                    .unwrap_or_default();
                let result = data.check_network(&host, &url);
                Ok(vec![WasmValue::from_i32(result)])
            },
        )
        .context("Failed to register check_network")?;

    // ── Shell execution check ─────────────────────────────────────
    import
        .with_func::<(i32, i32), i32>(
            "check_shell",
            |data: &mut HostContext, _inst, frame: &mut CallingFrame, args: Vec<WasmValue>| {
                let ptr = args[0].to_i32();
                let len = args[1].to_i32();
                let cmd = read_wasm_string(&frame, ptr, len)
                    .unwrap_or_else(|_| "<invalid utf8>".to_string());
                let result = data.check_shell_exec(&cmd);
                Ok(vec![WasmValue::from_i32(result)])
            },
        )
        .context("Failed to register check_shell")?;

    Ok(import.build())
}
