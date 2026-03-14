//! WASM plugin executor — instantiates a plugin and calls its exports.
//!
//! This module abstracts over two execution backends:
//! - **`runtime` feature** (default): Uses WasmEdge SDK for real sandboxed execution.
//! - **stub mode** (no `runtime` feature): Returns meaningful error strings so the
//!   rest of the codebase compiles and tests without the WasmEdge SDK installed.

use crate::abi::{ExecuteRequest, ExecuteResponse, SkillManifest};
use crate::error::PluginError;
use std::path::Path;
#[cfg(feature = "runtime")]
use tracing::debug;
#[cfg(not(feature = "runtime"))]
use tracing::warn;

// ── Executor ──────────────────────────────────────────────────────────────────

/// A loaded and instantiated WASM plugin ready to execute skills.
pub struct WasmExecutor {
    /// Plugin identifier from the manifest.
    pub id: String,
    /// Parsed skill manifest.
    pub manifest: SkillManifest,
    /// Inner backend — real or stub.
    inner: ExecutorInner,
}

enum ExecutorInner {
    #[cfg(feature = "runtime")]
    Real(RealExecutor),
    #[cfg_attr(feature = "runtime", allow(dead_code))]
    Stub { path: String },
}

impl WasmExecutor {
    /// Load a WASM file and instantiate the plugin.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let path = path.as_ref();

        #[cfg(feature = "runtime")]
        {
            let mut real = RealExecutor::load(path)?;
            let manifest = real.read_manifest()?;
            let id = manifest.id.clone();
            return Ok(Self {
                id,
                manifest,
                inner: ExecutorInner::Real(real),
            });
        }

        #[cfg(not(feature = "runtime"))]
        {
            warn!(
                path = %path.display(),
                "WASM runtime not compiled in — plugin will return stubs"
            );
            let path_str = path.display().to_string();
            let stub_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let manifest = SkillManifest {
                id: stub_id.clone(),
                name: stub_id.clone(),
                version: "0.0.0-stub".into(),
                description: "Stub — compiled without runtime feature".into(),
                skills: Vec::new(),
            };
            Ok(Self {
                id: stub_id,
                manifest,
                inner: ExecutorInner::Stub { path: path_str },
            })
        }
    }

    /// Call a skill on the plugin, returning a text observation.
    pub fn execute(
        &mut self,
        skill: &str,
        args: &serde_json::Value,
        request_id: &str,
    ) -> Result<ExecuteResponse, PluginError> {
        // Validate the skill is in the manifest.
        if !self.manifest.skills.iter().any(|s| s.name == skill) {
            return Err(PluginError::SkillNotFound {
                id: self.id.clone(),
                skill: skill.to_string(),
            });
        }

        #[cfg_attr(not(feature = "runtime"), allow(unused_variables))]
        let req = ExecuteRequest {
            skill: skill.to_string(),
            args: args.clone(),
            request_id: request_id.to_string(),
        };

        match &mut self.inner {
            #[cfg(feature = "runtime")]
            ExecutorInner::Real(real) => real.call_execute(&req),

            ExecutorInner::Stub { path } => {
                let msg = format!(
                    "(WASM plugin '{}' at {} — runtime not compiled in; \
                     rebuild with --features runtime to enable)",
                    skill, path
                );
                Ok(ExecuteResponse::ok(request_id, msg))
            }
        }
    }
}

// ── Real WasmEdge backend (wasmedge-sys 0.19) ────────────────────────────────

#[cfg(feature = "runtime")]
struct RealExecutor {
    path: std::path::PathBuf,
    executor: wasmedge_sys::Executor,
    #[allow(dead_code)]
    store: wasmedge_sys::Store,
    instance: wasmedge_sys::Instance,
}

#[cfg(feature = "runtime")]
impl RealExecutor {
    fn load(path: &Path) -> Result<Self, PluginError> {
        use wasmedge_sys::{Config, Executor, Loader, Store, Validator};

        let bytes = std::fs::read(path)
            .map_err(|e| PluginError::Io { path: path.to_path_buf(), source: e })?;

        let config = Config::create().map_err(|e| PluginError::Instantiation {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

        let loader = Loader::create(Some(&config)).map_err(|e| PluginError::Instantiation {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

        let module = loader
            .from_bytes(&bytes)
            .map_err(|e| PluginError::Instantiation {
                path: path.to_path_buf(),
                reason: format!("parse: {}", e),
            })?;

        let validator = Validator::create(Some(&config)).map_err(|e| PluginError::Instantiation {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

        validator.validate(&module).map_err(|e| PluginError::Instantiation {
            path: path.to_path_buf(),
            reason: format!("validate: {}", e),
        })?;

        let mut executor = Executor::create(Some(&config), None).map_err(|e| {
            PluginError::Instantiation {
                path: path.to_path_buf(),
                reason: e.to_string(),
            }
        })?;

        let mut store = Store::create().map_err(|e| PluginError::Instantiation {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

        // Register the `ocplus` import module so plugins that import
        // `host_log` / `host_http_fetch` can be instantiated without crashing.
        // These stubs are safe no-ops; the real implementations live in
        // `openclaw-sandbox/src/host_funcs.rs` for the JS-agent path.
        let ocplus_import = build_ocplus_import_object().map_err(|e| {
            PluginError::Instantiation {
                path: path.to_path_buf(),
                reason: format!("ocplus import: {}", e),
            }
        })?;
        executor
            .register_import_module(&mut store, &ocplus_import)
            .map_err(|e| PluginError::Instantiation {
                path: path.to_path_buf(),
                reason: format!("register ocplus: {}", e),
            })?;

        let instance = executor
            .register_named_module(&mut store, &module, "skill_plugin")
            .map_err(|e| PluginError::Instantiation {
                path: path.to_path_buf(),
                reason: format!("register: {}", e),
            })?;

        debug!(path = %path.display(), "WasmEdge plugin instantiated");
        Ok(Self { path: path.to_path_buf(), executor, store, instance })
    }

    fn read_manifest(&mut self) -> Result<SkillManifest, PluginError> {
        use wasmedge_sys::AsInstance;

        let id = self.path.display().to_string();
        let mut func = self.instance.get_func_mut("skill_manifest").map_err(|_| {
            PluginError::MissingExport { id: id.clone(), export: "skill_manifest".into() }
        })?;

        let results = self.executor.call_func(&mut func, vec![]).map_err(|e| {
            PluginError::MissingExport { id: id.clone(), export: format!("skill_manifest call: {}", e) }
        })?;

        // skill_manifest returns a packed u64: (ptr<<32 | len)
        let packed = self.extract_packed(&results, "skill_manifest")?;
        let json = self.read_guest_string(packed)?;
        serde_json::from_str(&json).map_err(|e| PluginError::ManifestParse {
            id,
            reason: e.to_string(),
        })
    }

    fn call_execute(&mut self, req: &ExecuteRequest) -> Result<ExecuteResponse, PluginError> {
        use wasmedge_sys::{AsInstance, WasmValue};

        let req_json = serde_json::to_string(req)?;
        let req_bytes = req_json.as_bytes();

        // 1. alloc(size) -> ptr
        let mut alloc_fn = self.instance.get_func_mut("alloc").map_err(|_| PluginError::Memory {
            id: req.skill.clone(),
            reason: "missing 'alloc' export".into(),
        })?;
        let alloc_result = self
            .executor
            .call_func(&mut alloc_fn, vec![WasmValue::from_i32(req_bytes.len() as i32)])
            .map_err(|e| PluginError::Memory {
                id: req.skill.clone(),
                reason: format!("alloc: {}", e),
            })?;
        let ptr = match alloc_result.first() {
            Some(v) => v.to_i32() as u32,
            None => return Err(PluginError::Memory {
                id: req.skill.clone(),
                reason: "alloc returned no value".into(),
            }),
        };

        // 2. Write bytes into guest memory
        let mut mem = self.instance.get_memory_mut("memory").map_err(|_| PluginError::Memory {
            id: req.skill.clone(),
            reason: "no exported 'memory'".into(),
        })?;
        mem.set_data(req_bytes, ptr).map_err(|e| PluginError::Memory {
            id: req.skill.clone(),
            reason: format!("set_data: {}", e),
        })?;

        // 3. skill_execute(ptr, len) -> u64
        let mut exec_fn = self.instance.get_func_mut("skill_execute").map_err(|_| {
            PluginError::MissingExport { id: req.skill.clone(), export: "skill_execute".into() }
        })?;
        let results = self
            .executor
            .call_func(
                &mut exec_fn,
                vec![
                    WasmValue::from_i32(ptr as i32),
                    WasmValue::from_i32(req_bytes.len() as i32),
                ],
            )
            .map_err(|e| PluginError::Execution {
                id: req.skill.clone(),
                skill: req.skill.clone(),
                reason: e.to_string(),
            })?;

        let packed = self.extract_packed(&results, "skill_execute")?;
        let json = self.read_guest_string(packed)?;
        serde_json::from_str(&json).map_err(|e| PluginError::Serde(e))
    }

    /// Extract a packed u64 `(ptr<<32 | len)` from a 1-element result vec.
    fn extract_packed(
        &self,
        vals: &[wasmedge_sys::WasmValue],
        ctx: &str,
    ) -> Result<u64, PluginError> {
        match vals.first() {
            Some(v) => Ok(v.to_i64() as u64),
            None => Err(PluginError::Memory {
                id: ctx.to_string(),
                reason: "expected u64 packed return".into(),
            }),
        }
    }

    /// Decode a packed `(ptr<<32 | len)` and read the guest string.
    fn read_guest_string(&self, packed: u64) -> Result<String, PluginError> {
        use wasmedge_sys::AsInstance;
        let ptr = (packed >> 32) as u32;
        let len = (packed & 0xFFFF_FFFF) as u32;

        let mem = self.instance.get_memory_ref("memory").map_err(|_| PluginError::Memory {
            id: "read_guest_string".into(),
            reason: "no exported 'memory'".into(),
        })?;

        let data = mem.get_data(ptr, len).map_err(|e| PluginError::Memory {
            id: "read_guest_string".into(),
            reason: e.to_string(),
        })?;

        String::from_utf8(data).map_err(|e| PluginError::Memory {
            id: "read_guest_string".into(),
            reason: format!("bad UTF-8: {}", e),
        })
    }
}

// ── ocplus stub import object ─────────────────────────────────────────────────

/// Builds a `wasmedge_sys::ImportModule` named `"ocplus"` that provides
/// no-op stubs for every host function a WASM plugin may import.
///
/// Plugins built with `openclaw-plugin-sdk` may optionally import:
/// - `ocplus.host_log(level_ptr, level_len, msg_ptr, msg_len)`  → void
/// - `ocplus.host_http_fetch(req_ptr, req_len)` → i64
/// - `ocplus.check_file_read(ptr, len)` → i32
/// - `ocplus.check_file_write(ptr, len)` → i32
/// - `ocplus.check_file_delete(ptr, len)` → i32
/// - `ocplus.check_network(hp, hl, up, ul)` → i32
/// - `ocplus.check_shell(ptr, len)` → i32
///
/// Stubs: security checks always return 1 (allow); log is a no-op;
/// `host_http_fetch` returns 0 (null packed pointer).
#[cfg(feature = "runtime")]
fn build_ocplus_import_object() -> anyhow::Result<wasmedge_sys::ImportModule<()>> {
    use wasmedge_sys::{CallingFrame, Function, ImportModule, WasmValue};
    use wasmedge_types::{FuncType, ValType};

    use wasmedge_types::error::CoreError;

    fn allow_i32i32(
        _data: &mut (), _inst: &mut wasmedge_sys::Instance,
        _frame: &mut CallingFrame, _args: Vec<WasmValue>,
    ) -> std::result::Result<Vec<WasmValue>, CoreError> {
        Ok(vec![WasmValue::from_i32(1)])
    }
    fn allow_4xi32(
        _data: &mut (), _inst: &mut wasmedge_sys::Instance,
        _frame: &mut CallingFrame, _args: Vec<WasmValue>,
    ) -> std::result::Result<Vec<WasmValue>, CoreError> {
        Ok(vec![WasmValue::from_i32(1)])
    }
    fn noop_log(
        _data: &mut (), _inst: &mut wasmedge_sys::Instance,
        _frame: &mut CallingFrame, _args: Vec<WasmValue>,
    ) -> std::result::Result<Vec<WasmValue>, CoreError> {
        Ok(vec![])
    }
    fn null_fetch(
        _data: &mut (), _inst: &mut wasmedge_sys::Instance,
        _frame: &mut CallingFrame, _args: Vec<WasmValue>,
    ) -> std::result::Result<Vec<WasmValue>, CoreError> {
        Ok(vec![WasmValue::from_i64(0)])
    }

    let mut module = ImportModule::<()>::create("ocplus", Box::new(()))
        .map_err(|e| anyhow::anyhow!("ImportModule::create: {:?}", e))?;

    let ty_i32i32_i32 = FuncType::new(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let ty_4xi32_i32  = FuncType::new(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32], vec![ValType::I32]);
    let ty_4xi32_void = FuncType::new(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32], vec![]);
    let ty_i32i32_i64 = FuncType::new(vec![ValType::I32, ValType::I32], vec![ValType::I64]);

    let data_ptr = module.get_host_data_mut() as *mut ();

    macro_rules! make_fn {
        ($ty:expr, $f:expr) => {{
            unsafe { Function::create_sync_func(&$ty, $f, data_ptr, 0) }
                .map_err(|e| anyhow::anyhow!("Function::create: {:?}", e))?
        }}
    }

    module.add_func("check_file_read",   make_fn!(ty_i32i32_i32, allow_i32i32));
    module.add_func("check_file_write",  make_fn!(ty_i32i32_i32, allow_i32i32));
    module.add_func("check_file_delete", make_fn!(ty_i32i32_i32, allow_i32i32));
    module.add_func("check_network",     make_fn!(ty_4xi32_i32,  allow_4xi32));
    module.add_func("check_shell",       make_fn!(ty_i32i32_i32, allow_i32i32));
    module.add_func("host_log",          make_fn!(ty_4xi32_void, noop_log));
    module.add_func("host_http_fetch",   make_fn!(ty_i32i32_i64, null_fetch));

    Ok(module)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(feature = "runtime")]
    #[test]
    fn build_ocplus_import_object_succeeds() {
        let result = build_ocplus_import_object();
        assert!(result.is_ok(), "build_ocplus_import_object must succeed: {:?}", result.err());
    }

    #[test]
    fn load_non_wasm_returns_error_or_stub() {
        let dir = TempDir::new().unwrap();
        let fake = dir.path().join("fake.wasm");
        std::fs::write(&fake, b"not-wasm-bytes").unwrap();

        let result = WasmExecutor::load(&fake);
        // With runtime feature: should error (invalid WASM).
        // Without runtime: loads as stub.
        #[cfg(not(feature = "runtime"))]
        {
            let exec = result.unwrap();
            assert!(exec.manifest.skills.is_empty());
        }
        #[cfg(feature = "runtime")]
        {
            assert!(result.is_err());
        }
    }

    #[test]
    fn execute_skill_not_in_manifest_errors() {
        let dir = TempDir::new().unwrap();
        let fake = dir.path().join("fake.wasm");
        std::fs::write(&fake, b"not-wasm-bytes").unwrap();

        #[cfg(not(feature = "runtime"))]
        {
            let mut exec = WasmExecutor::load(&fake).unwrap();
            let err = exec.execute("no.such", &serde_json::json!({}), "r1");
            assert!(err.is_err());
        }
    }
}
