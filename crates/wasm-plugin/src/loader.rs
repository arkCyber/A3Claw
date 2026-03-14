//! WASM plugin loader — discovers, verifies and instantiates `.wasm` files.
//!
//! Search order:
//! 1. `<workspace>/.openclaw/skills/*.wasm`
//! 2. `~/.openclaw/skills/*.wasm`
//! 3. Paths explicitly passed to [`PluginLoader::add_path`].

use crate::abi::SkillManifest;
use crate::error::PluginError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// ── Plugin metadata ───────────────────────────────────────────────────────────

/// Metadata for a discovered WASM plugin file.
#[derive(Debug, Clone)]
pub struct WasmPluginMeta {
    /// Absolute path to the `.wasm` file.
    pub path: PathBuf,
    /// SHA-256 hex digest of the file contents.
    pub sha256: String,
    /// Manifest parsed from the guest's `skill_manifest()` export (lazy).
    pub manifest: Option<SkillManifest>,
}

impl WasmPluginMeta {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let path = path.as_ref().to_path_buf();
        let bytes = std::fs::read(&path)
            .map_err(|e| PluginError::Io { path: path.clone(), source: e })?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        debug!(path = %path.display(), sha256 = %sha256, "Loaded WASM plugin file");
        Ok(Self { path, sha256, manifest: None })
    }

    /// Verify that this plugin's SHA-256 is in the provided allowlist.
    ///
    /// Returns `Err(PluginError::Instantiation)` if `allowed_hashes` is non-empty
    /// and the plugin's hash is not present. When `allowed_hashes` is empty the
    /// check is skipped (open policy — suitable for development).
    pub fn verify_hash(&self, allowed_hashes: &[impl AsRef<str>]) -> Result<(), PluginError> {
        if allowed_hashes.is_empty() {
            return Ok(());
        }
        let matches = allowed_hashes.iter().any(|h| h.as_ref() == self.sha256);
        if matches {
            Ok(())
        } else {
            Err(PluginError::Instantiation {
                path: self.path.clone(),
                reason: format!(
                    "plugin SHA-256 '{}' is not in the trusted allowlist",
                    self.sha256
                ),
            })
        }
    }
}

// ── Loader ────────────────────────────────────────────────────────────────────

/// Discovers WASM plugin files from well-known directories plus any explicitly
/// added paths.
#[derive(Debug, Default)]
pub struct PluginLoader {
    extra_paths: Vec<PathBuf>,
    #[cfg(test)]
    skip_default_paths: bool,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a loader that **only** searches paths explicitly added via
    /// [`add_path`] — it does not probe `cwd/.openclaw/skills` or
    /// `~/.openclaw/skills`.  Use this in tests to avoid picking up real
    /// `.wasm` files installed on the developer's machine.
    #[cfg(test)]
    pub(crate) fn new_explicit_only() -> Self {
        Self { skip_default_paths: true, ..Self::default() }
    }

    /// Add an explicit file or directory to the search path.
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.extra_paths.push(path.into());
    }

    /// Discover all `.wasm` plugin files from all search directories.
    pub fn discover(&self) -> Vec<WasmPluginMeta> {
        let mut paths: Vec<PathBuf> = Vec::new();

        #[cfg(test)]
        let skip = self.skip_default_paths;
        #[cfg(not(test))]
        let skip = false;

        if !skip {
            // 1. Workspace-local skills directory
            if let Ok(cwd) = std::env::current_dir() {
                paths.push(cwd.join(".openclaw").join("skills"));
            }

            // 2. User-global skills directory
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(".openclaw").join("skills"));
            }
        }

        // 3. Explicitly added paths
        for p in &self.extra_paths {
            paths.push(p.clone());
        }

        let mut metas = Vec::new();
        for dir in &paths {
            if !dir.exists() {
                continue;
            }
            if dir.is_file() && dir.extension().map_or(false, |e| e == "wasm") {
                match WasmPluginMeta::from_path(dir) {
                    Ok(m) => metas.push(m),
                    Err(e) => warn!(path = %dir.display(), "Failed to load WASM plugin: {}", e),
                }
                continue;
            }
            match std::fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |e| e == "wasm") {
                            match WasmPluginMeta::from_path(&p) {
                                Ok(m) => {
                                    info!(path = %p.display(), "Discovered WASM plugin");
                                    metas.push(m);
                                }
                                Err(e) => warn!(path = %p.display(), "Skip invalid plugin: {}", e),
                            }
                        }
                    }
                }
                Err(e) => debug!(dir = %dir.display(), "Cannot read plugin dir: {}", e),
            }
        }

        metas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── BUG-R6-06: SHA-256 allowlist verification ─────────────────────────────

    #[test]
    fn verify_hash_empty_allowlist_passes_always() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("ok.wasm");
        std::fs::write(&p, b"\x00asm\x01\x00\x00\x00").unwrap();
        let meta = WasmPluginMeta::from_path(&p).unwrap();
        // Empty allowlist = development mode, always passes
        let empty: &[String] = &[];
        let result = meta.verify_hash(empty);
        assert!(result.is_ok(), "empty allowlist must always pass");
    }

    #[test]
    fn verify_hash_matching_hash_passes() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("ok.wasm");
        std::fs::write(&p, b"\x00asm\x01\x00\x00\x00").unwrap();
        let meta = WasmPluginMeta::from_path(&p).unwrap();
        // Allow exactly this hash
        let result = meta.verify_hash(&[meta.sha256.clone()]);
        assert!(result.is_ok(), "matching hash must pass verification");
    }

    #[test]
    fn verify_hash_wrong_hash_is_rejected() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("ok.wasm");
        std::fs::write(&p, b"\x00asm\x01\x00\x00\x00").unwrap();
        let meta = WasmPluginMeta::from_path(&p).unwrap();
        let result = meta.verify_hash(&["deadbeef".to_string()]);
        assert!(result.is_err(), "wrong hash must be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("allowlist"), "error must mention allowlist: {}", msg);
    }

    #[test]
    fn verify_hash_one_matching_in_multi_hash_list_passes() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("ok.wasm");
        std::fs::write(&p, b"\x00asm\x01\x00\x00\x00").unwrap();
        let meta = WasmPluginMeta::from_path(&p).unwrap();
        let result = meta.verify_hash(&[
            "aaaa".to_string(),
            meta.sha256.clone(),
            "bbbb".to_string(),
        ]);
        assert!(result.is_ok(), "one matching hash in list must pass");
    }

    #[test]
    fn verify_hash_error_contains_actual_hash() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("evil.wasm");
        std::fs::write(&p, b"evil payload").unwrap();
        let meta = WasmPluginMeta::from_path(&p).unwrap();
        let result = meta.verify_hash(&["trusted_hash_only".to_string()]);
        let err = result.unwrap_err().to_string();
        assert!(err.contains(&meta.sha256), "error must include actual SHA-256: {}", err);
    }

    #[test]
    fn discover_empty_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        let mut loader = PluginLoader::new_explicit_only();
        loader.add_path(dir.path());
        let metas = loader.discover();
        assert!(metas.is_empty());
    }

    #[test]
    fn discover_finds_wasm_files() {
        let dir = TempDir::new().unwrap();
        let wasm = dir.path().join("test.wasm");
        std::fs::write(&wasm, b"\x00asm\x01\x00\x00\x00").unwrap(); // minimal WASM magic
        let mut loader = PluginLoader::new_explicit_only();
        loader.add_path(dir.path());
        let metas = loader.discover();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].path, wasm);
    }

    #[test]
    fn discover_ignores_non_wasm_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"hello").unwrap();
        let mut loader = PluginLoader::new_explicit_only();
        loader.add_path(dir.path());
        assert!(loader.discover().is_empty());
    }

    #[test]
    fn meta_sha256_is_deterministic() {
        let dir = TempDir::new().unwrap();
        let wasm = dir.path().join("a.wasm");
        std::fs::write(&wasm, b"\x00asm\x01\x00\x00\x00").unwrap();
        let m1 = WasmPluginMeta::from_path(&wasm).unwrap();
        let m2 = WasmPluginMeta::from_path(&wasm).unwrap();
        assert_eq!(m1.sha256, m2.sha256);
    }
}
