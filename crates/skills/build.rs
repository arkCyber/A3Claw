//! build.rs — compile all skill crates to wasm32-wasip1 and install to ~/.openclaw/skills/
//!
//! This script is invoked by cargo when building the top-level `skills` workspace member.
//! It shells out to `cargo build --target wasm32-wasip1 --release` for each skill crate,
//! then copies the resulting `.wasm` file to `~/.openclaw/skills/<name>.wasm`.
//!
//! To trigger a rebuild whenever any skill source changes, each source directory is
//! registered as a rerun-if-changed directive.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Re-run this build script if any skill source file changes.
    let skill_crates = &[
        ("hash",     "skill_hash"),
        ("encode",   "skill_encode"),
        ("math",     "skill_math"),
        ("text",     "skill_text"),
        ("datetime", "skill_datetime"),
        ("crypto",   "skill_crypto"),
        ("uuid",     "skill_uuid"),
        ("compress", "skill_compress"),
        ("json",     "skill_json"),
        ("regex",    "skill_regex"),
        ("network",  "skill_network"),
        // Community skills (100 skills across 10 crates)
        ("string",   "skill_string"),
        ("number",   "skill_number"),
        ("array",    "skill_array"),
        ("color",    "skill_color"),
        ("data",     "skill_data"),
        ("geo",      "skill_geo"),
        ("stat",     "skill_stat"),
        ("fmt",      "skill_fmt"),
        ("bits",     "skill_bits"),
        ("token",    "skill_token"),
        // New skills (batch 2)
        ("logic",          "skill_logic"),
        ("sort",           "skill_sort"),
        ("convert",        "skill_convert"),
        ("validate",       "skill_validate"),
        ("random",         "skill_random"),
        ("diff",           "skill_diff"),
        ("template-skill", "skill_template_skill"),
        ("semver",         "skill_semver"),
        ("path",           "skill_path"),
        ("money",          "skill_money"),
        ("duration",       "skill_duration"),
        ("matrix",         "skill_matrix"),
        ("csv",            "skill_csv"),
        ("xml",            "skill_xml"),
        ("yaml",           "skill_yaml"),
    ];

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir is crates/skills/<name> when used per-crate, or crates/skills when standalone.
    // We navigate up to the workspace root, then into crates/skills.
    let workspace_root = manifest_dir
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("crates").exists())
        .expect("could not find workspace root")
        .to_path_buf();
    let skills_root = workspace_root.join("crates").join("skills");

    for (crate_name, _) in skill_crates {
        let src_dir = skills_root.join(crate_name).join("src");
        println!("cargo:rerun-if-changed={}", src_dir.display());
        println!("cargo:rerun-if-changed={}", skills_root.join(crate_name).join("Cargo.toml").display());
    }

    // Determine the output directory.
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))
        .expect("HOME / USERPROFILE must be set");
    let skills_out = PathBuf::from(&home).join(".openclaw").join("skills");
    std::fs::create_dir_all(&skills_out)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", skills_out.display()));

    // Check whether the WASM target is installed; skip compilation if not, but warn loudly.
    let target_installed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32-wasip1"))
        .unwrap_or(false);

    if !target_installed {
        println!("cargo:warning=wasm32-wasip1 target not installed; skipping skill WASM compilation.");
        println!("cargo:warning=Run: rustup target add wasm32-wasip1");
        return;
    }

    for (crate_name, lib_name) in skill_crates {
        let crate_dir = skills_root.join(crate_name);
        compile_skill(&workspace_root, &crate_dir, lib_name, &skills_out);
    }
}

fn compile_skill(workspace_root: &Path, crate_dir: &Path, lib_name: &str, out_dir: &Path) {
    println!("cargo:warning=Compiling skill: {} → {}.wasm", lib_name, lib_name);

    let status = Command::new("cargo")
        .current_dir(crate_dir)
        .args([
            "build",
            "--target", "wasm32-wasip1",
            "--release",
            "--no-default-features",
        ])
        .env("CARGO_TARGET_DIR", workspace_root.join("target"))
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn cargo for {lib_name}: {e}"));

    if !status.success() {
        panic!("cargo build failed for skill crate: {lib_name}");
    }

    let wasm_src = workspace_root
        .join("target")
        .join("wasm32-wasip1")
        .join("release")
        .join(format!("{lib_name}.wasm"));

    if !wasm_src.exists() {
        panic!(
            "expected WASM output not found: {}\nBuild succeeded but artifact is missing.",
            wasm_src.display()
        );
    }

    // Strip lib_ prefix that cargo adds for cdylib crates.
    let dest_name = lib_name.strip_prefix("skill_").unwrap_or(lib_name);
    let wasm_dst = out_dir.join(format!("{dest_name}.wasm"));

    std::fs::copy(&wasm_src, &wasm_dst).unwrap_or_else(|e| {
        panic!("copy {} → {} failed: {e}", wasm_src.display(), wasm_dst.display())
    });

    println!("cargo:warning=Installed: {}", wasm_dst.display());
}
