//! Build script for openclaw-inference.
//!
//! When the `wasi-nn` feature is enabled, this script compiles the
//! `crates/wasi-nn-infer` crate to `wasm32-wasip1` and writes the resulting
//! binary to `$OUT_DIR/wasi_nn_infer.wasm`, which is then included via
//! `include_bytes!` in `backend.rs`.

fn main() {
    // Only run when wasi-nn feature is active.
    #[cfg(feature = "wasi-nn")]
    compile_wasi_nn_wasm();

    // Always re-run if the wasi-nn-infer source changes.
    println!("cargo:rerun-if-changed=../wasi-nn-infer/src/main.rs");
    println!("cargo:rerun-if-changed=../wasi-nn-infer/Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(feature = "wasi-nn")]
fn compile_wasi_nn_wasm() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let wasm_out = out_dir.join("wasi_nn_infer.wasm");

    // Locate the workspace root (two levels up from crates/inference/).
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("cannot determine workspace root");

    let wasi_nn_infer_dir = workspace_root.join("crates").join("wasi-nn-infer");

    // cargo build --release --target wasm32-wasip1 -p openclaw-wasi-nn-infer
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip1",
            "--manifest-path",
        ])
        .arg(wasi_nn_infer_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(out_dir.join("wasi_nn_build"))
        .status()
        .expect("failed to invoke cargo for wasi-nn-infer");

    if !status.success() {
        panic!("Failed to compile wasi-nn-infer to wasm32-wasip1");
    }

    // Copy the resulting .wasm to a predictable path.
    let wasm_src = out_dir
        .join("wasi_nn_build")
        .join("wasm32-wasip1")
        .join("release")
        .join("openclaw_wasi_nn_infer.wasm");

    if !wasm_src.exists() {
        panic!(
            "Expected WASM output not found at {}",
            wasm_src.display()
        );
    }

    std::fs::copy(&wasm_src, &wasm_out)
        .expect("failed to copy wasm output");

    println!("cargo:rustc-env=WASI_NN_INFER_WASM={}", wasm_out.display());
}
