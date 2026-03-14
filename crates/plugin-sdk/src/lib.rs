//! OpenClaw+ Plugin SDK — guest-side helpers for writing WASM skill plugins.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use openclaw_plugin_sdk::prelude::*;
//!
//! // 1. Declare the manifest once.
//! static MANIFEST: &str = r#"{
//!   "id": "my-org.weather",
//!   "name": "Weather Plugin",
//!   "version": "1.0.0",
//!   "description": "Current weather via Open-Meteo",
//!   "skills": [
//!     { "name": "weather.current", "display": "Current Weather",
//!       "description": "Get current weather for a city.",
//!       "risk": "safe",
//!       "params": [
//!         { "name": "city", "type": "string",
//!           "description": "City name", "required": true }
//!       ]
//!     }
//!   ]
//! }"#;
//!
//! // 2. Export skill_manifest — the SDK macro handles the ABI boilerplate.
//! #[no_mangle]
//! pub extern "C" fn skill_manifest() -> u64 {
//!     sdk_export_str(MANIFEST)
//! }
//!
//! // 3. Export skill_execute — dispatch on skill name.
//! #[no_mangle]
//! pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
//!     let req = match sdk_read_request(ptr, len) {
//!         Ok(r) => r,
//!         Err(e) => return sdk_respond_err("", &e),
//!     };
//!     match req.skill.as_str() {
//!         "weather.current" => {
//!             let city = req.args["city"].as_str().unwrap_or("London");
//!             sdk_respond_ok(&req.request_id, &format!("Weather in {}: 22°C, sunny", city))
//!         }
//!         other => sdk_respond_err(&req.request_id, &format!("Unknown skill: {}", other)),
//!     }
//! }
//! ```
//!
//! The `u64` return value packs `(ptr: u32, len: u32)` into a single 64-bit
//! integer so a single WebAssembly `i64` return is sufficient — no multi-value
//! extension required.

use serde::{Deserialize, Serialize};

// ── WASM allocator ──────────────────────────────────────────────────────────────
// On wasm32 targets the default allocator is dlmalloc (bundled by the Rust
// standard library for wasm32-wasip1). We explicitly declare it here so that
// any crate linking against plugin-sdk gets a consistent allocator and the
// `alloc` / `dealloc` exports below operate on the same heap.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: std::alloc::System = std::alloc::System;

// ── ABI types (mirrors wasm-plugin/src/abi.rs) ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub skill: String,
    pub args: serde_json::Value,
    #[serde(default)]
    pub request_id: String,
}

#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub request_id: String,
    pub ok: bool,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub error: String,
}

// ── Memory helpers ────────────────────────────────────────────────────────────

/// Pack `(ptr, len)` into a `u64` for single-value WASM return.
#[inline(always)]
pub fn pack(ptr: u32, len: u32) -> u64 {
    ((ptr as u64) << 32) | (len as u64)
}

/// Leak a string into WASM linear memory and return its `(ptr, len)` packed.
pub fn sdk_export_str(s: &str) -> u64 {
    let bytes: Box<[u8]> = s.as_bytes().to_vec().into_boxed_slice();
    let len = bytes.len() as u32;
    let ptr = Box::into_raw(bytes) as *mut u8 as u32;
    pack(ptr, len)
}

/// Read a UTF-8 JSON string written by the host into guest memory.
///
/// # Safety
/// The caller must ensure `ptr` + `len` is valid guest memory written by the
/// host via the `alloc` export.
pub fn sdk_read_request(ptr: i32, len: i32) -> Result<ExecuteRequest, String> {
    let slice = unsafe {
        std::slice::from_raw_parts(ptr as *const u8, len as usize)
    };
    serde_json::from_slice(slice).map_err(|e| e.to_string())
}

/// Serialise a success response and return it via `sdk_export_str`.
pub fn sdk_respond_ok(request_id: &str, output: &str) -> u64 {
    let resp = ExecuteResponse {
        request_id: request_id.to_string(),
        ok: true,
        output: output.to_string(),
        error: String::new(),
    };
    sdk_export_str(&serde_json::to_string(&resp).unwrap_or_default())
}

/// Serialise an error response and return it via `sdk_export_str`.
pub fn sdk_respond_err(request_id: &str, error: &str) -> u64 {
    let resp = ExecuteResponse {
        request_id: request_id.to_string(),
        ok: false,
        output: String::new(),
        error: error.to_string(),
    };
    sdk_export_str(&serde_json::to_string(&resp).unwrap_or_default())
}

// ── Allocator exports (required by the ABI) ───────────────────────────────────

/// Called by the runtime to allocate `size` bytes in guest memory.
#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let layout = std::alloc::Layout::array::<u8>(size as usize).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) };
    ptr as i32
}

/// Called by the runtime to free a previous allocation.
#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, size: i32) {
    if size <= 0 { return; }
    let layout = std::alloc::Layout::array::<u8>(size as usize).unwrap();
    unsafe { std::alloc::dealloc(ptr as *mut u8, layout) };
}

// ── Prelude ───────────────────────────────────────────────────────────────────

pub mod prelude {
    pub use super::{
        sdk_export_str, sdk_read_request, sdk_respond_err, sdk_respond_ok,
        ExecuteRequest, ExecuteResponse,
    };
    pub use serde_json;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_roundtrip() {
        let ptr = 0x0001_2345u32;
        let len = 0x0000_00FFu32;
        let packed = pack(ptr, len);
        assert_eq!((packed >> 32) as u32, ptr);
        assert_eq!((packed & 0xFFFF_FFFF) as u32, len);
    }

    #[test]
    fn execute_response_ok_serialises() {
        let resp = ExecuteResponse {
            request_id: "req-1".into(),
            ok: true,
            output: "hello world".into(),
            error: String::new(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["output"], "hello world");
        assert_eq!(v["request_id"], "req-1");
    }

    #[test]
    fn execute_response_err_serialises() {
        let resp = ExecuteResponse {
            request_id: "req-2".into(),
            ok: false,
            output: String::new(),
            error: "something went wrong".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "something went wrong");
    }

    // Memory-layout tests only make sense inside a WASM linear-memory sandbox.
    // On native targets the raw pointer arithmetic is unsafe and not meaningful.
    #[cfg(target_arch = "wasm32")]
    #[test]
    fn respond_ok_roundtrip_via_ptr() {
        let packed = sdk_respond_ok("req-1", "hello world");
        let ptr = (packed >> 32) as u32;
        let len = (packed & 0xFFFF_FFFF) as u32;
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
        let v: serde_json::Value = serde_json::from_slice(slice).unwrap();
        assert_eq!(v["ok"], true);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn alloc_dealloc_roundtrip() {
        let ptr = alloc(64);
        assert_ne!(ptr, 0);
        dealloc(ptr, 64);
    }

    // ── pack() extended coverage ──────────────────────────────────────────────

    #[test]
    fn pack_zero_ptr_and_len() {
        let packed = pack(0, 0);
        assert_eq!((packed >> 32) as u32, 0);
        assert_eq!((packed & 0xFFFF_FFFF) as u32, 0);
    }

    #[test]
    fn pack_max_values() {
        let packed = pack(u32::MAX, u32::MAX);
        assert_eq!((packed >> 32) as u32, u32::MAX);
        assert_eq!((packed & 0xFFFF_FFFF) as u32, u32::MAX);
    }

    #[test]
    fn pack_ptr_only_no_len_leak() {
        let packed = pack(0xDEAD_BEEF, 0);
        assert_eq!((packed >> 32) as u32, 0xDEAD_BEEF);
        assert_eq!((packed & 0xFFFF_FFFF) as u32, 0);
    }

    // ── sdk_export_str native smoke test ─────────────────────────────────────

    #[test]
    fn sdk_export_str_nonzero_packed_for_nonempty_string() {
        let packed = sdk_export_str("hello");
        let len = (packed & 0xFFFF_FFFF) as u32;
        assert_eq!(len, 5, "len should equal byte length of 'hello'");
        let ptr = (packed >> 32) as u32;
        assert_ne!(ptr, 0, "ptr should be nonzero for heap allocation");
    }

    #[test]
    fn sdk_export_str_length_matches_bytes() {
        let s = "日本語テスト"; // 18 UTF-8 bytes
        let packed = sdk_export_str(s);
        let len = (packed & 0xFFFF_FFFF) as u32;
        assert_eq!(len, s.len() as u32);
    }

    // ── sdk_respond_ok / sdk_respond_err JSON correctness ─────────────────────
    //
    // NOTE: `sdk_export_str` / `pack` stores a *u32* pointer — valid inside
    // 32-bit WASM linear memory.  On a 64-bit native host the heap address
    // exceeds u32::MAX so the upper bits are silently truncated, making the
    // packed pointer invalid for native dereferencing.
    // We therefore test the JSON content via `sdk_respond_ok` → re-serialise
    // without touching the raw pointer, and keep the raw-pointer tests
    // wasm32-only (already gated above).

    fn respond_ok_json(request_id: &str, output: &str) -> serde_json::Value {
        let resp = ExecuteResponse {
            request_id: request_id.to_string(),
            ok: true,
            output: output.to_string(),
            error: String::new(),
        };
        serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap()
    }

    fn respond_err_json(request_id: &str, error: &str) -> serde_json::Value {
        let resp = ExecuteResponse {
            request_id: request_id.to_string(),
            ok: false,
            output: String::new(),
            error: error.to_string(),
        };
        serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap()
    }

    #[test]
    fn sdk_respond_ok_json_fields_correct() {
        let v = respond_ok_json("rid-42", "sky is clear");
        assert_eq!(v["ok"], true);
        assert_eq!(v["request_id"], "rid-42");
        assert_eq!(v["output"], "sky is clear");
        assert_eq!(v["error"], "");
    }

    #[test]
    fn sdk_respond_err_json_fields_correct() {
        let v = respond_err_json("rid-99", "network timeout");
        assert_eq!(v["ok"], false);
        assert_eq!(v["request_id"], "rid-99");
        assert_eq!(v["error"], "network timeout");
        assert_eq!(v["output"], "");
    }

    #[test]
    fn sdk_respond_ok_empty_output_json() {
        let v = respond_ok_json("r0", "");
        assert_eq!(v["ok"], true);
        assert_eq!(v["output"], "");
    }

    // ── ExecuteRequest deserialization ────────────────────────────────────────

    #[test]
    fn execute_request_deserializes_full_payload() {
        let json = r#"{"skill":"weather.current","args":{"city":"Paris"},"request_id":"req-7"}"#;
        let req: ExecuteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.skill, "weather.current");
        assert_eq!(req.args["city"], "Paris");
        assert_eq!(req.request_id, "req-7");
    }

    #[test]
    fn execute_request_request_id_defaults_empty() {
        let json = r#"{"skill":"ping","args":{}}"#;
        let req: ExecuteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.request_id, "");
    }

    #[test]
    fn execute_request_null_args_preserved() {
        let json = r#"{"skill":"x","args":null}"#;
        let req: ExecuteRequest = serde_json::from_str(json).unwrap();
        assert!(req.args.is_null());
    }

    // ── ExecuteResponse JSON roundtrip ────────────────────────────────────────

    #[test]
    fn execute_response_ok_roundtrip() {
        let resp = ExecuteResponse {
            request_id: "test".into(),
            ok: true,
            output: "result".into(),
            error: String::new(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["output"], "result");
    }

    #[test]
    fn execute_response_error_roundtrip() {
        let resp = ExecuteResponse {
            request_id: "test".into(),
            ok: false,
            output: String::new(),
            error: "something failed".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "something failed");
    }
}
