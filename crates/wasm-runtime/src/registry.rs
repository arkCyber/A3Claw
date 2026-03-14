//! WASM skill module registry.
//!
//! Maps skill names to their compiled WASM bytes and provides `get_bytes`
//! for the runtime to instantiate on demand.

use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info};

// ── WasmSkillRegistry ────────────────────────────────────────────────────────

/// In-memory registry that stores compiled WASM bytes for each skill.
///
/// Skill modules are registered by name at startup and retrieved by the
/// [`WasmSkillRuntime`](super::WasmSkillRuntime) when a skill is invoked.
pub struct WasmSkillRegistry {
    /// skill_name → WASM bytes
    modules: HashMap<String, Vec<u8>>,
}

impl WasmSkillRegistry {
    pub fn new() -> Self {
        Self { modules: HashMap::new() }
    }

    /// Register a WASM binary under the given skill name.
    pub async fn register_skill(&mut self, skill_name: &str, wasm_bytes: Vec<u8>) -> Result<()> {
        info!(skill = skill_name, bytes = wasm_bytes.len(), "Registering WASM skill");
        self.modules.insert(skill_name.to_string(), wasm_bytes);
        Ok(())
    }

    /// Retrieve a clone of the raw WASM bytes for a skill.
    ///
    /// Returns an error when the skill is not registered.
    pub async fn get_bytes(&self, skill_name: &str) -> Result<Vec<u8>> {
        let bytes = self.modules.get(skill_name).ok_or_else(|| {
            anyhow::anyhow!("WASM skill '{}' not found in registry", skill_name)
        })?;
        debug!(skill = skill_name, "Fetching WASM bytes from registry");
        Ok(bytes.clone())
    }

    /// Returns `true` if the named skill is registered.
    pub async fn has_skill(&self, skill_name: &str) -> bool {
        self.modules.contains_key(skill_name)
    }

    /// Names of all registered skills.
    pub async fn list_skills(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Returns `true` when no skills are registered.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Remove a skill from the registry.
    pub async fn unregister_skill(&mut self, skill_name: &str) -> bool {
        self.modules.remove(skill_name).is_some()
    }

    /// Remove all registered skills.
    pub fn cleanup(&mut self) {
        self.modules.clear();
    }
}

impl Default for WasmSkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_registry_is_empty() {
        let reg = WasmSkillRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[tokio::test]
    async fn register_and_has_skill() {
        let mut reg = WasmSkillRegistry::new();
        reg.register_skill("test.add", b"\x00asm\x01\x00\x00\x00".to_vec())
            .await
            .unwrap();
        assert!(reg.has_skill("test.add").await);
        assert!(!reg.has_skill("test.missing").await);
    }

    #[tokio::test]
    async fn list_skills_after_registration() {
        let mut reg = WasmSkillRegistry::new();
        reg.register_skill("skill.a", vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])
            .await
            .unwrap();
        reg.register_skill("skill.b", vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])
            .await
            .unwrap();
        let mut names = reg.list_skills().await;
        names.sort();
        assert_eq!(names, vec!["skill.a", "skill.b"]);
    }

    #[tokio::test]
    async fn get_bytes_unknown_skill_errors() {
        let reg = WasmSkillRegistry::new();
        let err = reg.get_bytes("no.such").await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("no.such"));
    }

    #[tokio::test]
    async fn get_bytes_returns_clone() {
        let mut reg = WasmSkillRegistry::new();
        let bytes = b"\x00asm\x01\x00\x00\x00".to_vec();
        reg.register_skill("test.skill", bytes.clone()).await.unwrap();
        let got = reg.get_bytes("test.skill").await.unwrap();
        assert_eq!(got, bytes);
    }

    #[tokio::test]
    async fn unregister_removes_skill() {
        let mut reg = WasmSkillRegistry::new();
        reg.register_skill("temp.skill", b"\x00asm\x01\x00\x00\x00".to_vec())
            .await
            .unwrap();
        let removed = reg.unregister_skill("temp.skill").await;
        assert!(removed);
        assert!(!reg.has_skill("temp.skill").await);
    }

    #[tokio::test]
    async fn unregister_unknown_returns_false() {
        let mut reg = WasmSkillRegistry::new();
        assert!(!reg.unregister_skill("ghost").await);
    }

    #[tokio::test]
    async fn cleanup_empties_registry() {
        let mut reg = WasmSkillRegistry::new();
        reg.register_skill("x", b"\x00asm\x01\x00\x00\x00".to_vec())
            .await
            .unwrap();
        reg.cleanup();
        assert!(reg.is_empty());
    }

    #[test]
    fn default_constructor_is_empty() {
        let reg: WasmSkillRegistry = Default::default();
        assert!(reg.is_empty());
    }
}
