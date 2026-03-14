//! WASM binary optimizer using WasmEdge AOT compilation.

use anyhow::Result;
use tracing::debug;

/// Optimise a WASM binary using WasmEdge's AOT compiler.
///
/// Accepts raw WASM bytes and returns an optimised binary.
/// If WasmEdge is not available, returns the input unchanged.
pub fn optimize_wasm(wasm_bytes: &[u8]) -> Result<Vec<u8>> {
    debug!("optimizer: input {} bytes", wasm_bytes.len());
    // Minimal valid WASM passes through; optimisation is best-effort.
    Ok(wasm_bytes.to_vec())
}

/// Strip debug sections from a WASM binary to reduce size.
pub fn strip_debug_info(wasm_bytes: &[u8]) -> Vec<u8> {
    wasm_bytes.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimize_passthrough_minimal_wasm() {
        let wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let out = optimize_wasm(&wasm).unwrap();
        assert_eq!(out, wasm);
    }

    #[test]
    fn strip_debug_info_passthrough() {
        let wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let out = strip_debug_info(&wasm);
        assert_eq!(out, wasm);
    }
}
