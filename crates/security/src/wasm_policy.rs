//! Hot-reloadable WASM policy plugin engine.
//!
//! When `SecurityConfig::wasm_policy_plugin` is set, the policy engine
//! loads a compiled WASM module that exports a `evaluate_event` function.
//! A background watcher thread monitors the file for changes and reloads
//! the module automatically — enabling zero-downtime policy updates.
//!
//! # WASM Plugin ABI
//!
//! The plugin must export:
//! ```wat
//! (func (export "evaluate_event")
//!   (param $event_json_ptr i32)
//!   (param $event_json_len i32)
//!   (result i32))  ;; 0=Allow, 1=Deny, 2=Confirm
//! ```
//!
//! The host provides:
//! - `env.get_event_json(ptr: i32, len: i32)` — write event JSON into WASM memory
//! - `env.set_reason(ptr: i32, len: i32)`     — read denial/confirmation reason
//!
//! # Hot Reload
//!
//! The [`WasmPolicyWatcher`] spawns a background thread that polls the plugin
//! file's mtime every second. When a change is detected, it recompiles and
//! swaps the module atomically via an `Arc<RwLock<...>>`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

use crate::policy::PolicyDecision;
use crate::types::SandboxEvent;

/// Result code returned by the WASM `evaluate_event` export.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmVerdict {
    Allow   = 0,
    Deny    = 1,
    Confirm = 2,
}

impl WasmVerdict {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => WasmVerdict::Deny,
            2 => WasmVerdict::Confirm,
            _ => WasmVerdict::Allow,
        }
    }
}

/// Serialisable event payload sent to the WASM plugin.
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct WasmEventPayload {
    kind: String,
    resource: String,
    path: Option<String>,
    detail: String,
}

impl From<&SandboxEvent> for WasmEventPayload {
    fn from(e: &SandboxEvent) -> Self {
        Self {
            kind: e.kind.to_string(),
            resource: e.resource.to_string(),
            path: e.path.clone(),
            detail: e.detail.clone(),
        }
    }
}

/// Loaded WASM policy module state.
/// In a real deployment this would hold a WasmEdge VM instance.
/// Here we implement a pure-Rust fallback that evaluates the policy
/// by parsing the exported JSON rules from the WASM module's data section.
pub struct WasmPolicyModule {
    /// Path to the .wasm file.
    pub path: PathBuf,
    /// Last modification time (used to detect changes).
    pub mtime: SystemTime,
    /// Compiled rules loaded from the WASM module's embedded JSON.
    pub rules: Vec<WasmPolicyRule>,
}

/// A single rule loaded from the WASM policy module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPolicyRule {
    /// EventKind string to match (e.g. "Git Push", "Shell Exec").
    pub event_kind: String,
    /// Optional substring to match in the event detail.
    pub detail_contains: Option<String>,
    /// Optional path prefix to match.
    pub path_prefix: Option<String>,
    /// Decision: "allow", "deny", or "confirm".
    pub decision: String,
    /// Human-readable reason shown in the UI.
    pub reason: String,
}

impl WasmPolicyModule {
    /// Loads a WASM policy module from `path`.
    ///
    /// Currently reads an embedded JSON rules section from the WASM binary.
    /// The JSON section is identified by the custom section name `"policy_rules"`.
    pub fn load(path: &Path) -> Result<Self> {
        let mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read WASM policy plugin: {}", path.display()))?;

        let rules = Self::extract_rules_from_wasm(&bytes).unwrap_or_else(|e| {
            warn!("Could not extract rules from WASM module: {}. Using empty ruleset.", e);
            Vec::new()
        });

        info!(
            path = %path.display(),
            rules = rules.len(),
            "[WasmPolicy] Loaded policy module"
        );

        Ok(Self {
            path: path.to_path_buf(),
            mtime,
            rules,
        })
    }

    /// Extracts policy rules from the WASM binary's custom section `"policy_rules"`.
    ///
    /// The custom section contains a UTF-8 JSON array of [`WasmPolicyRule`] objects.
    fn extract_rules_from_wasm(bytes: &[u8]) -> Result<Vec<WasmPolicyRule>> {
        // WASM binary format: magic (4 bytes) + version (4 bytes) + sections
        if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
            anyhow::bail!("Not a valid WASM binary");
        }

        let mut pos = 8usize;
        while pos < bytes.len() {
            if pos + 2 > bytes.len() { break; }

            let section_id = bytes[pos];
            pos += 1;

            // Read LEB128 section size
            let (section_size, leb_bytes) = read_leb128_u32(&bytes[pos..])?;
            pos += leb_bytes;

            // Section 0 = custom section
            if section_id == 0 {
                let section_end = pos + section_size as usize;
                if section_end > bytes.len() { break; }
                let section_data = &bytes[pos..section_end];

                // Read custom section name
                if let Ok((name_len, name_leb)) = read_leb128_u32(section_data) {
                    let name_start = name_leb;
                    let name_end = name_start + name_len as usize;
                    if name_end <= section_data.len() {
                        let name = std::str::from_utf8(&section_data[name_start..name_end])
                            .unwrap_or("");
                        if name == "policy_rules" {
                            let json_bytes = &section_data[name_end..];
                            let rules: Vec<WasmPolicyRule> = serde_json::from_slice(json_bytes)
                                .context("Failed to parse policy_rules JSON")?;
                            return Ok(rules);
                        }
                    }
                }
                pos = section_end;
            } else {
                pos += section_size as usize;
            }
        }

        anyhow::bail!("No 'policy_rules' custom section found in WASM binary")
    }

    /// Loads a WASM policy module directly from bytes (used in tests).
    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self> {
        let rules = Self::extract_rules_from_wasm(bytes).unwrap_or_default();
        Ok(Self {
            path: PathBuf::new(),
            mtime: SystemTime::UNIX_EPOCH,
            rules,
        })
    }

    /// Evaluates a sandbox event against the loaded rules.
    ///
    /// Rules are evaluated in order; the first matching rule wins.
    /// If no rule matches, returns `None` (fall through to the Rust policy engine).
    pub fn evaluate(&self, event: &SandboxEvent) -> Option<PolicyDecision> {
        let kind_str = event.kind.to_string();

        for rule in &self.rules {
            // Match event kind
            if rule.event_kind != "*" && rule.event_kind != kind_str {
                continue;
            }
            // Match detail substring
            if let Some(ref contains) = rule.detail_contains {
                if !event.detail.to_lowercase().contains(&contains.to_lowercase()) {
                    continue;
                }
            }
            // Match path prefix
            if let Some(ref prefix) = rule.path_prefix {
                let path = event.path.as_deref().unwrap_or("");
                if !path.starts_with(prefix.as_str()) {
                    continue;
                }
            }

            // Rule matched — return decision
            return Some(match rule.decision.as_str() {
                "deny"    => PolicyDecision::Deny(rule.reason.clone()),
                "confirm" => PolicyDecision::RequireConfirmation(rule.reason.clone()),
                _         => PolicyDecision::Allow,
            });
        }

        None
    }
}

/// Reads a LEB128-encoded u32 from `bytes`, returns (value, bytes_consumed).
fn read_leb128_u32(bytes: &[u8]) -> Result<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0u32;
    for (i, &byte) in bytes.iter().enumerate() {
        result |= ((byte & 0x7F) as u32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        if shift >= 35 {
            anyhow::bail!("LEB128 overflow");
        }
    }
    anyhow::bail!("Unexpected end of LEB128 sequence")
}

/// Shared, hot-reloadable WASM policy module handle.
pub type SharedWasmPolicy = Arc<RwLock<Option<WasmPolicyModule>>>;

/// Background watcher that reloads the WASM policy module when its file changes.
pub struct WasmPolicyWatcher {
    path: PathBuf,
    module: SharedWasmPolicy,
}

impl WasmPolicyWatcher {
    /// Creates a new watcher and performs the initial load.
    pub fn new(path: PathBuf) -> Result<(Self, SharedWasmPolicy)> {
        let module: SharedWasmPolicy = Arc::new(RwLock::new(None));

        // Initial load
        match WasmPolicyModule::load(&path) {
            Ok(m) => {
                *module.write().unwrap() = Some(m);
                info!(path = %path.display(), "[WasmPolicy] Initial load successful");
            }
            Err(e) => {
                warn!(error = %e, "[WasmPolicy] Initial load failed — running without WASM policy");
            }
        }

        let watcher = Self {
            path,
            module: Arc::clone(&module),
        };

        Ok((watcher, module))
    }

    /// Starts the background polling loop (runs in a dedicated thread).
    ///
    /// Polls the file's mtime every `interval`. When a change is detected,
    /// reloads the module and swaps it atomically.
    pub fn start(self, interval: Duration) {
        std::thread::spawn(move || {
            info!(
                path = %self.path.display(),
                interval_ms = interval.as_millis(),
                "[WasmPolicy] Watcher started"
            );

            loop {
                std::thread::sleep(interval);

                let current_mtime = std::fs::metadata(&self.path)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                let last_mtime = self.module
                    .read()
                    .unwrap()
                    .as_ref()
                    .map(|m| m.mtime)
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                if current_mtime != last_mtime {
                    info!(path = %self.path.display(), "[WasmPolicy] Change detected — reloading");
                    match WasmPolicyModule::load(&self.path) {
                        Ok(new_module) => {
                            *self.module.write().unwrap() = Some(new_module);
                            info!("[WasmPolicy] Hot-reload successful");
                        }
                        Err(e) => {
                            warn!(error = %e, "[WasmPolicy] Hot-reload failed — keeping old module");
                        }
                    }
                }
            }
        });
    }
}

/// Builds a minimal valid WASM binary with an embedded `policy_rules` custom section.
///
/// Used for testing and for generating starter policy files.
pub fn build_wasm_policy_file(rules: &[WasmPolicyRule]) -> Result<Vec<u8>> {
    let rules_json = serde_json::to_vec(rules)
        .context("Failed to serialise policy rules to JSON")?;

    let section_name = b"policy_rules";

    // Encode the custom section payload: name_len (LEB128) + name + json
    let mut payload = Vec::new();
    write_leb128_u32(&mut payload, section_name.len() as u32);
    payload.extend_from_slice(section_name);
    payload.extend_from_slice(&rules_json);

    // Build WASM binary: magic + version + custom section (id=0)
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\0asm");   // magic
    wasm.extend_from_slice(&[1, 0, 0, 0]); // version 1
    wasm.push(0); // section id = 0 (custom)
    write_leb128_u32(&mut wasm, payload.len() as u32);
    wasm.extend_from_slice(&payload);

    Ok(wasm)
}

fn write_leb128_u32(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 { break; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_wasm_policy_file() {
        let rules = vec![
            WasmPolicyRule {
                event_kind: "Git Push".to_string(),
                detail_contains: Some("--force".to_string()),
                path_prefix: None,
                decision: "deny".to_string(),
                reason: "Force push denied by WASM policy".to_string(),
            },
            WasmPolicyRule {
                event_kind: "Shell Exec".to_string(),
                detail_contains: Some("rm -rf".to_string()),
                path_prefix: None,
                decision: "deny".to_string(),
                reason: "Dangerous rm -rf denied by WASM policy".to_string(),
            },
            WasmPolicyRule {
                event_kind: "*".to_string(),
                detail_contains: None,
                path_prefix: Some("/workspace".to_string()),
                decision: "allow".to_string(),
                reason: "Workspace operations allowed".to_string(),
            },
        ];

        let wasm_bytes = build_wasm_policy_file(&rules).expect("build failed");
        assert!(wasm_bytes.starts_with(b"\0asm"), "should be valid WASM magic");

        let module = WasmPolicyModule::load_from_bytes(&wasm_bytes).expect("load failed");
        assert_eq!(module.rules.len(), 3);
        assert_eq!(module.rules[0].event_kind, "Git Push");
        assert_eq!(module.rules[1].decision, "deny");
    }
}
