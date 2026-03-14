//! WASM plugin registry — manages all loaded plugins and routes skill calls.

use crate::abi::SkillManifest;
use crate::error::PluginError;
use crate::executor::WasmExecutor;
use crate::loader::PluginLoader;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};

// ── Registry ──────────────────────────────────────────────────────────────────

/// Maps skill names to the plugin executor that provides them.
pub struct WasmPluginRegistry {
    /// plugin_id → executor
    executors: HashMap<String, WasmExecutor>,
    /// skill_name → plugin_id
    skill_index: HashMap<String, String>,
}

impl WasmPluginRegistry {
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
            skill_index: HashMap::new(),
        }
    }

    /// Load all plugins discovered by the given loader.
    pub fn load_from_loader(&mut self, loader: &PluginLoader) {
        for meta in loader.discover() {
            if let Err(e) = self.load_file(&meta.path) {
                warn!(path = %meta.path.display(), "Failed to load plugin: {}", e);
            }
        }
    }

    /// Load a single `.wasm` plugin file and register its skills.
    pub fn load_file(&mut self, path: &PathBuf) -> Result<(), PluginError> {
        let executor = WasmExecutor::load(path)?;
        let id = executor.id.clone();
        let manifest = executor.manifest.clone();

        info!(
            plugin = %id,
            skills = manifest.skills.len(),
            "Registered WASM plugin"
        );

        for skill in &manifest.skills {
            if self.skill_index.contains_key(&skill.name) {
                warn!(
                    skill = %skill.name,
                    new_plugin = %id,
                    "Skill already registered — overriding with new plugin"
                );
            }
            self.skill_index.insert(skill.name.clone(), id.clone());
        }

        self.executors.insert(id, executor);
        Ok(())
    }

    /// Returns true if any loaded plugin provides this skill.
    pub fn has_skill(&self, skill_name: &str) -> bool {
        self.skill_index.contains_key(skill_name)
    }

    /// Execute a skill via the appropriate plugin.
    pub fn execute(
        &mut self,
        skill_name: &str,
        args: &serde_json::Value,
        request_id: &str,
    ) -> Result<String, String> {
        let plugin_id = self
            .skill_index
            .get(skill_name)
            .cloned()
            .ok_or_else(|| format!("No WASM plugin provides skill '{}'", skill_name))?;

        let executor = self
            .executors
            .get_mut(&plugin_id)
            .ok_or_else(|| format!("Plugin '{}' not found in executor map", plugin_id))?;

        let resp = executor
            .execute(skill_name, args, request_id)
            .map_err(|e| e.to_string())?;

        if resp.ok {
            Ok(resp.output)
        } else {
            Err(resp.error)
        }
    }

    /// All manifests for loaded plugins.
    pub fn manifests(&self) -> Vec<&SkillManifest> {
        self.executors.values().map(|e| &e.manifest).collect()
    }

    /// All skill names provided by loaded plugins.
    pub fn all_skill_names(&self) -> Vec<&str> {
        self.skill_index.keys().map(|s| s.as_str()).collect()
    }

    /// Number of loaded plugins.
    pub fn plugin_count(&self) -> usize {
        self.executors.len()
    }
}

impl Default for WasmPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_has_no_skills() {
        let reg = WasmPluginRegistry::new();
        assert!(!reg.has_skill("weather.current"));
        assert_eq!(reg.plugin_count(), 0);
    }

    #[test]
    fn execute_unknown_skill_returns_error() {
        let mut reg = WasmPluginRegistry::new();
        let err = reg.execute("no.such", &serde_json::json!({}), "r1");
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("no.such"));
    }

    #[test]
    fn load_file_invalid_path_errors() {
        let mut reg = WasmPluginRegistry::new();
        let result = reg.load_file(&PathBuf::from("/nonexistent/path/plugin.wasm"));
        assert!(result.is_err());
    }

    // ── all_skill_names ───────────────────────────────────────────────────

    #[test]
    fn all_skill_names_empty_on_new_registry() {
        let reg = WasmPluginRegistry::new();
        let names = reg.all_skill_names();
        assert!(names.is_empty(), "new registry must have no skill names");
    }

    // ── plugin_count ──────────────────────────────────────────────────────

    #[test]
    fn plugin_count_zero_on_new_registry() {
        let reg = WasmPluginRegistry::new();
        assert_eq!(reg.plugin_count(), 0);
    }

    // ── manifests ─────────────────────────────────────────────────────────

    #[test]
    fn manifests_empty_on_new_registry() {
        let reg = WasmPluginRegistry::new();
        let manifests = reg.manifests();
        assert!(manifests.is_empty(), "new registry must have no manifests");
    }

    // ── Default trait ─────────────────────────────────────────────────────

    #[test]
    fn default_constructor_behaves_like_new() {
        let reg: WasmPluginRegistry = Default::default();
        assert_eq!(reg.plugin_count(), 0);
        assert!(!reg.has_skill("any.skill"));
        assert!(reg.all_skill_names().is_empty());
    }

    // ── has_skill edge cases ──────────────────────────────────────────────

    #[test]
    fn has_skill_empty_string_returns_false() {
        let reg = WasmPluginRegistry::new();
        assert!(!reg.has_skill(""));
    }

    #[test]
    fn has_skill_case_sensitive() {
        let reg = WasmPluginRegistry::new();
        assert!(!reg.has_skill("Weather.Current"));
        assert!(!reg.has_skill("weather.current"));
    }

    // ── execute on empty registry ─────────────────────────────────────────

    #[test]
    fn execute_returns_error_message_containing_skill_name() {
        let mut reg = WasmPluginRegistry::new();
        let err = reg.execute("my.custom.skill", &serde_json::json!({"key": "value"}), "req-1");
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(msg.contains("my.custom.skill"), "error must name the missing skill: {}", msg);
    }

    // ── load_from_loader ──────────────────────────────────────────────────

    #[test]
    fn load_from_loader_empty_discovers_nothing() {
        let mut reg = WasmPluginRegistry::new();
        let loader = PluginLoader::new_explicit_only();
        reg.load_from_loader(&loader);
        assert_eq!(reg.plugin_count(), 0);
        assert!(reg.all_skill_names().is_empty());
    }

    #[test]
    fn load_from_loader_skips_invalid_wasm_files() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bad.wasm"), b"not-wasm").unwrap();

        let mut loader = PluginLoader::new_explicit_only();
        loader.add_path(dir.path());

        let mut reg = WasmPluginRegistry::new();
        reg.load_from_loader(&loader);
        // Invalid WASM must be skipped, not panic
        assert_eq!(reg.plugin_count(), 0);
    }

    // ── load_file bad path ────────────────────────────────────────────────

    #[test]
    fn load_file_nonexistent_path_errors_with_io() {
        use std::path::PathBuf;
        let mut reg = WasmPluginRegistry::new();
        let r = reg.load_file(&PathBuf::from("/does/not/exist.wasm"));
        assert!(r.is_err(), "loading nonexistent file must fail");
    }

    // ── execute after load failure ────────────────────────────────────────

    #[test]
    fn has_skill_false_after_failed_load() {
        use std::path::PathBuf;
        let mut reg = WasmPluginRegistry::new();
        let _ = reg.load_file(&PathBuf::from("/no/such.wasm"));
        assert!(!reg.has_skill("anything"));
    }
}
