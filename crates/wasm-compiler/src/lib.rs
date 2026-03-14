//! OpenClaw WASM技能编译器
//! 
//! 将Rust实现的技能编译为WASM字节码，在WasmEdge沙箱中运行

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};
#[allow(unused_imports)]
use wasmedge_sys::{Config, Loader, Validator};

pub mod adapter;
pub mod error;
pub mod optimizer;

pub use adapter::{generate_wasm_adapter, SkillAdapterConfig};
pub use error::{CompilerError, CompilationResult};

/// WASM技能编译器
pub struct SkillWasmCompiler {
    adapter_config: SkillAdapterConfig,
}

impl SkillWasmCompiler {
    /// 创建新的编译器实例
    pub fn new() -> Result<Self> {
        // Probe WasmEdge availability.
        Config::create().context("WasmEdge Config::create failed — is the native library installed?")?;
        let adapter_config = SkillAdapterConfig::default();
        Ok(Self { adapter_config })
    }

    /// 编译单个技能为WASM
    pub async fn compile_skill_to_wasm(
        &self,
        skill_name: &str,
        skill_code: &str,
    ) -> Result<Vec<u8>> {
        info!("编译技能: {}", skill_name);

        // 1. 生成WASM适配器
        let wasm_adapter = generate_wasm_adapter(skill_name, skill_code, &self.adapter_config)
            .context("Failed to generate WASM adapter")?;

        debug!("生成的WASM适配器代码长度: {} 字符", wasm_adapter.len());

        // 2. 创建临时文件
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let rust_file = temp_dir.path().join("skill.rs");
        fs::write(&rust_file, wasm_adapter).context("Failed to write skill file")?;

        // 3. 编译为WASM
        let wasm_binary = self.compile_rust_to_wasm(&rust_file)
            .await
            .context("Failed to compile Rust to WASM")?;

        info!("✅ 技能 {} 编译成功, WASM大小: {} 字节", skill_name, wasm_binary.len());

        Ok(wasm_binary)
    }

    /// 批量编译技能
    pub async fn compile_skills_batch(
        &self,
        skills: HashMap<String, String>,
    ) -> Result<HashMap<String, CompilationResult>> {
        let mut results = HashMap::new();
        let mut compiled_count = 0;
        let mut failed_count = 0;

        info!("开始批量编译 {} 个技能", skills.len());

        for (skill_name, skill_code) in skills {
            match self.compile_skill_to_wasm(&skill_name, &skill_code).await {
                Ok(wasm_binary) => {
                    let size_bytes = wasm_binary.len();
                    results.insert(skill_name.clone(), CompilationResult::Success {
                        wasm_binary,
                        size_bytes,
                    });
                    compiled_count += 1;
                    debug!("✅ 编译成功: {}", skill_name);
                }
                Err(e) => {
                    warn!("❌ 编译失败: {} - {}", skill_name, e);
                    results.insert(skill_name.clone(), CompilationResult::Error {
                        error: e.to_string(),
                    });
                    failed_count += 1;
                }
            }
        }

        info!(
            "批量编译完成: 成功 {}, 失败 {}",
            compiled_count, failed_count
        );

        Ok(results)
    }

    /// 将Rust代码编译为WASM字节码
    ///
    /// 步骤：
    /// 1. 将 .rs 文件写入临时 Cargo crate 目录
    /// 2. 调用 `cargo build --target wasm32-wasip1 --release`
    /// 3. 读取生成的 `.wasm` 文件并返回字节
    async fn compile_rust_to_wasm(&self, rust_file: &Path) -> Result<Vec<u8>> {
        use std::process::Command;

        // Build a minimal Cargo project around the provided .rs file.
        let crate_dir = rust_file
            .parent()
            .context("rust_file has no parent directory")?;

        // Write a minimal Cargo.toml if it doesn't already exist.
        let cargo_toml = crate_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            let skill_name = rust_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("skill");
            let toml = format!(
                r#"[package]
name = "{skill_name}"
version = "0.1.0"
edition = "2021"

[lib]
name = "{skill_name}"
crate-type = ["cdylib"]
path = "skill.rs"

[profile.release]
opt-level = "z"
lto = true
strip = true
"#
            );
            fs::write(&cargo_toml, toml).context("Failed to write Cargo.toml for skill crate")?;
        }

        // Check whether the wasm32-wasip1 target is installed.
        let target = "wasm32-wasip1";
        let target_ok = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(target))
            .unwrap_or(false);

        if !target_ok {
            anyhow::bail!(
                "WASM target '{target}' is not installed. \
                 Run: rustup target add {target}"
            );
        }

        debug!("Running cargo build --target {target} in {}", crate_dir.display());

        let output = Command::new("cargo")
            .current_dir(crate_dir)
            .args(["build", "--target", target, "--release", "--no-default-features"])
            .output()
            .context("Failed to spawn cargo build")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo build failed for skill:\n{stderr}");
        }

        // Locate the compiled .wasm artifact.
        let lib_name = rust_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("skill");

        // cargo places the artifact under target/<target>/release/
        // from the workspace root if CARGO_TARGET_DIR is set, otherwise from the crate.
        let wasm_path = crate_dir
            .join("target")
            .join(target)
            .join("release")
            .join(format!("{lib_name}.wasm"));

        if !wasm_path.exists() {
            // Also check with lib_ prefix (cdylib on some platforms).
            let alt = crate_dir
                .join("target")
                .join(target)
                .join("release")
                .join(format!("lib{lib_name}.wasm"));
            if alt.exists() {
                let bytes = fs::read(&alt)
                    .with_context(|| format!("Failed to read WASM at {}", alt.display()))?;
                return Ok(bytes);
            }
            anyhow::bail!(
                "Expected WASM artifact not found at {} after successful build",
                wasm_path.display()
            );
        }

        let bytes = fs::read(&wasm_path)
            .with_context(|| format!("Failed to read WASM at {}", wasm_path.display()))?;

        info!("✅ cargo build succeeded: {} bytes from {}", bytes.len(), wasm_path.display());
        Ok(bytes)
    }

    /// 验证WASM字节码
    pub fn validate_wasm_binary(&self, wasm_binary: &[u8]) -> Result<()> {
        let config = Config::create().context("Config::create")?;
        let loader = Loader::create(Some(&config)).context("Loader::create")?;
        let module = loader.from_bytes(wasm_binary).context("Invalid WASM binary")?;
        let validator = Validator::create(Some(&config)).context("Validator::create")?;
        validator.validate(&module).context("WASM validation failed")?;
        Ok(())
    }

    /// 优化WASM字节码 (pass-through — AOT optimisation requires a file path)
    pub fn optimize_wasm_binary(&self, wasm_binary: &[u8]) -> Result<Vec<u8>> {
        Ok(wasm_binary.to_vec())
    }
}

impl Default for SkillWasmCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create default WASM compiler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires wasm32-wasip1 target and invokes cargo build"]
    async fn test_compile_simple_skill() {
        let compiler = SkillWasmCompiler::new().unwrap();
        
        let skill_name = "test.add";
        let skill_code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
        "#.trim();

        let result = compiler.compile_skill_to_wasm(skill_name, skill_code).await;
        assert!(result.is_ok());
        
        let wasm_binary = result.unwrap();
        assert!(!wasm_binary.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires wasm32-wasip1 target and invokes cargo build"]
    async fn test_batch_compilation() {
        let compiler = SkillWasmCompiler::new().unwrap();
        
        let mut skills = HashMap::new();
        skills.insert("test.add".to_string(), "pub fn add(a: i32, b: i32) -> i32 { a + b }".to_string());
        skills.insert("test.multiply".to_string(), "pub fn multiply(a: i32, b: i32) -> i32 { a * b }".to_string());

        let results = compiler.compile_skills_batch(skills).await.unwrap();
        assert_eq!(results.len(), 2);
        
        for (skill_name, result) in results {
            match result {
                CompilationResult::Success { wasm_binary, .. } => {
                    assert!(!wasm_binary.is_empty(), "Skill {} should have non-empty WASM", skill_name);
                }
                CompilationResult::Error { error } => {
                    panic!("Skill {} compilation failed: {}", skill_name, error);
                }
            }
        }
    }

    #[test]
    fn test_validate_wasm_binary() {
        let compiler = SkillWasmCompiler::new().unwrap();
        
        // 有效的WASM字节码 (最小有效WASM模块)
        let valid_wasm = vec![
            0x00, 0x61, 0x73, 0x6d, // Magic number
            0x01, 0x00, 0x00, 0x00, // Version
        ];
        
        assert!(compiler.validate_wasm_binary(&valid_wasm).is_ok());
        
        // 无效的字节码
        let invalid_wasm = vec![0x00, 0x01, 0x02, 0x03];
        assert!(compiler.validate_wasm_binary(&invalid_wasm).is_err());
    }
}
