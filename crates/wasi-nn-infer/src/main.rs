//! WASI-NN inference module compiled to wasm32-wasip1.
//!
//! File-based I/O protocol (all files live inside the WasmEdge preopened
//! sandbox directory injected by the host):
//!
//!   OPENCLAW_REQ  env-var → filename of the JSON input  (request)
//!   OPENCLAW_RESP env-var → filename of the JSON output (response)
//!
//! Input  JSON: {"model":"default","prompt":"…","n_predict":512,
//!               "temperature":0.7,"top_p":0.9,"ctx_size":4096,"n_gpu_layers":0}
//! Output JSON: {"ok":true,"text":"…"}   on success
//!              {"ok":false,"error":"…"} on failure
//!
//! The host must call `PluginManager::nn_preload` with alias "default"
//! pointing at the GGUF model file before launching this module in the VM.

#![no_main]

use std::io::Write;

// ── WASI-NN raw host-function bindings ────────────────────────────────────────

#[link(wasm_import_module = "wasi_nn")]
extern "C" {
    fn load_by_name(
        name_ptr: *const u8,
        name_len: u32,
        graph_out: *mut u32,
    ) -> u32;

    fn init_execution_context(graph: u32, ctx_out: *mut u32) -> u32;

    fn set_input(
        ctx: u32,
        index: u32,
        tensor_ptr: *const TensorRaw,
    ) -> u32;

    fn compute(ctx: u32) -> u32;

    fn get_output(
        ctx: u32,
        index: u32,
        out_buf: *mut u8,
        out_buf_len: u32,
        bytes_written: *mut u32,
    ) -> u32;
}

#[repr(C)]
struct TensorDimensions {
    ptr: *const u32,
    len: u32,
}

#[repr(C)]
struct TensorRaw {
    dimensions: TensorDimensions,
    type_: u32,
    data_ptr: *const u8,
    data_len: u32,
}

const TENSOR_TYPE_U8: u32 = 0;
const WASI_NN_OK: u32 = 0;
const OUTPUT_BUF_SIZE: usize = 8 * 1024 * 1024;

// ── Inference request ─────────────────────────────────────────────────────────

struct InferReq {
    model_alias: String,
    prompt: String,
    n_predict: u32,
    temperature: f32,
    top_p: f32,
    ctx_size: u32,
    n_gpu_layers: i32,
}

fn parse_json_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let pos = json.find(needle.as_str())?;
    let after = &json[pos + needle.len()..];
    let colon = after.find(':')? + 1;
    let v = after[colon..].trim_start();
    if v.starts_with('"') {
        let inner = &v[1..];
        let end = inner.find('"')?;
        Some(&inner[..end])
    } else {
        let end = v
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(v.len());
        Some(&v[..end])
    }
}

fn parse_req(json: &str) -> Result<InferReq, String> {
    Ok(InferReq {
        model_alias:  parse_json_str(json, "model").unwrap_or("default").to_string(),
        prompt:       parse_json_str(json, "prompt").ok_or("missing prompt")?.to_string(),
        n_predict:    parse_json_str(json, "n_predict").and_then(|v| v.parse().ok()).unwrap_or(512),
        temperature:  parse_json_str(json, "temperature").and_then(|v| v.parse().ok()).unwrap_or(0.7),
        top_p:        parse_json_str(json, "top_p").and_then(|v| v.parse().ok()).unwrap_or(0.9),
        ctx_size:     parse_json_str(json, "ctx_size").and_then(|v| v.parse().ok()).unwrap_or(4096),
        n_gpu_layers: parse_json_str(json, "n_gpu_layers").and_then(|v| v.parse().ok()).unwrap_or(0),
    })
}

// ── WASI-NN inference pipeline ────────────────────────────────────────────────

fn run_infer(req: &InferReq) -> Result<String, String> {
    // 1. Load pre-registered model graph by alias.
    let mut graph: u32 = 0;
    let alias = req.model_alias.as_bytes();
    let rc = unsafe { load_by_name(alias.as_ptr(), alias.len() as u32, &mut graph) };
    if rc != WASI_NN_OK {
        return Err(format!("load_by_name errno={}", rc));
    }

    // 2. Create execution context.
    let mut ctx: u32 = 0;
    let rc = unsafe { init_execution_context(graph, &mut ctx) };
    if rc != WASI_NN_OK {
        return Err(format!("init_execution_context errno={}", rc));
    }

    // 3. Set sampling config as tensor index 1 (metadata).
    let meta = format!(
        r#"{{"enable-log":false,"stream-stdout":false,"n-predict":{np},"temperature":{t:.4},"top-p":{tp:.4},"ctx-size":{cs},"n-gpu-layers":{gl}}}"#,
        np = req.n_predict,
        t  = req.temperature,
        tp = req.top_p,
        cs = req.ctx_size,
        gl = req.n_gpu_layers,
    );
    let meta_b = meta.as_bytes();
    let meta_dim: u32 = meta_b.len() as u32;
    let meta_tensor = TensorRaw {
        dimensions: TensorDimensions { ptr: &meta_dim, len: 1 },
        type_: TENSOR_TYPE_U8,
        data_ptr: meta_b.as_ptr(),
        data_len: meta_b.len() as u32,
    };
    let rc = unsafe { set_input(ctx, 1, &meta_tensor) };
    if rc != WASI_NN_OK {
        return Err(format!("set_input(meta) errno={}", rc));
    }

    // 4. Set prompt as tensor index 0.
    let pb = req.prompt.as_bytes();
    let pd: u32 = pb.len() as u32;
    let prompt_tensor = TensorRaw {
        dimensions: TensorDimensions { ptr: &pd, len: 1 },
        type_: TENSOR_TYPE_U8,
        data_ptr: pb.as_ptr(),
        data_len: pb.len() as u32,
    };
    let rc = unsafe { set_input(ctx, 0, &prompt_tensor) };
    if rc != WASI_NN_OK {
        return Err(format!("set_input(prompt) errno={}", rc));
    }

    // 5. Run inference.
    let rc = unsafe { compute(ctx) };
    if rc != WASI_NN_OK {
        return Err(format!("compute errno={}", rc));
    }

    // 6. Read output.
    let mut buf = vec![0u8; OUTPUT_BUF_SIZE];
    let mut written: u32 = 0;
    let rc = unsafe {
        get_output(ctx, 0, buf.as_mut_ptr(), buf.len() as u32, &mut written)
    };
    if rc != WASI_NN_OK {
        return Err(format!("get_output errno={}", rc));
    }

    Ok(String::from_utf8_lossy(&buf[..written as usize]).into_owned())
}

// ── JSON output helpers ───────────────────────────────────────────────────────

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"',  "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

fn write_response(path: &str, payload: &str) {
    if let Ok(mut f) = std::fs::File::create(path) {
        let _ = f.write_all(payload.as_bytes());
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[unsafe(export_name = "main")]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let req_file  = std::env::var("OPENCLAW_REQ").unwrap_or_else(|_| "request.json".into());
    let resp_file = std::env::var("OPENCLAW_RESP").unwrap_or_else(|_| "response.json".into());

    // Read request JSON.
    let input = match std::fs::read_to_string(&req_file) {
        Ok(s) => s,
        Err(e) => {
            write_response(&resp_file, &format!("{{\"ok\":false,\"error\":{:?}}}", e.to_string()));
            return 1;
        }
    };

    let response = match parse_req(input.trim()) {
        Err(e) => format!("{{\"ok\":false,\"error\":{:?}}}", e),
        Ok(req) => match run_infer(&req) {
            Ok(text) => format!("{{\"ok\":true,\"text\":\"{}\"}}", json_escape(&text)),
            Err(e)   => format!("{{\"ok\":false,\"error\":{:?}}}", e),
        },
    };

    write_response(&resp_file, &response);
    0
}
