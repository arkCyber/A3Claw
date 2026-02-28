//! OpenClaw workspace bootstrapper.
//!
//! Sets up the per-agent workspace directory, writes the plugin-config.json
//! that points OpenClaw at our Gateway, checks for the WasmEdge-QuickJS
//! engine, and verifies the OpenClaw JS bundle is present.

use crate::error::ExecutorError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

// ── Bootstrap config ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Agent ID (used as workspace subdirectory name).
    pub agent_id: String,
    /// Gateway HTTP port.
    pub gateway_port: u16,
    /// Path to openclaw/dist/index.js (None = auto-detect).
    pub openclaw_entry: Option<PathBuf>,
    /// Path to wasmedge_quickjs.wasm (None = auto-detect).
    pub quickjs_wasm: Option<PathBuf>,
}

impl BootstrapConfig {
    pub fn new(agent_id: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            agent_id: agent_id.into(),
            gateway_port,
            openclaw_entry: None,
            quickjs_wasm: None,
        }
    }
}

// ── Bootstrap result ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BootstrapResult {
    pub workspace_dir: PathBuf,
    pub plugin_config_path: PathBuf,
    pub openclaw_entry: PathBuf,
    pub quickjs_wasm: PathBuf,
    pub shim_path: PathBuf,
    /// True when OpenClaw JS bundle was found (false = demo/stub mode).
    pub openclaw_available: bool,
    /// True when WasmEdge-QuickJS WASM was found.
    pub quickjs_available: bool,
}

// ── plugin-config.json shape ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginConfig {
    plugin_id: String,
    plugin_name: String,
    version: String,
    hooks: HooksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HooksConfig {
    before_skill: String,
    after_skill: String,
    agent_start: String,
    agent_stop: String,
}

// ── Bootstrapper ──────────────────────────────────────────────────────────────

/// Prepare an agent's workspace for execution.
pub async fn bootstrap(cfg: &BootstrapConfig) -> Result<BootstrapResult, ExecutorError> {
    let workspace = agent_workspace_dir(&cfg.agent_id);
    tokio::fs::create_dir_all(&workspace).await.map_err(ExecutorError::IoError)?;

    // ── 1. Write plugin-config.json ───────────────────────────────────────────
    let plugin_config_path = workspace.join("plugin-config.json");
    let gateway_base = format!("http://localhost:{}", cfg.gateway_port);
    let plugin_cfg = PluginConfig {
        plugin_id: format!("openclaw-plus-{}", cfg.agent_id),
        plugin_name: "OpenClaw+ Security Gateway".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hooks: HooksConfig {
            before_skill: format!("{}/hooks/before-skill", gateway_base),
            after_skill:  format!("{}/hooks/after-skill",  gateway_base),
            agent_start:  format!("{}/hooks/agent-start",  gateway_base),
            agent_stop:   format!("{}/hooks/agent-stop",   gateway_base),
        },
    };
    let json = serde_json::to_string_pretty(&plugin_cfg)
        .map_err(ExecutorError::SerdeError)?;
    tokio::fs::write(&plugin_config_path, &json)
        .await
        .map_err(ExecutorError::IoError)?;
    info!(agent_id = %cfg.agent_id, path = ?plugin_config_path, "plugin-config.json written");

    // ── 2. Write Security Shim ────────────────────────────────────────────────
    let shim_dir = workspace.join("shim");
    tokio::fs::create_dir_all(&shim_dir).await.map_err(ExecutorError::IoError)?;
    let shim_path = shim_dir.join("security_shim.js");
    let shim_content = generate_security_shim(cfg.gateway_port);
    tokio::fs::write(&shim_path, &shim_content)
        .await
        .map_err(ExecutorError::IoError)?;
    info!(path = ?shim_path, "Security shim written");

    // ── 3. Locate OpenClaw JS bundle ──────────────────────────────────────────
    let openclaw_candidates: Vec<PathBuf> = vec![
        cfg.openclaw_entry.clone().unwrap_or_else(|| PathBuf::from(".")),
        workspace.join("openclaw").join("dist").join("index.js"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".openclaw-plus")
            .join("openclaw")
            .join("dist")
            .join("index.js"),
        PathBuf::from("/usr/local/lib/openclaw/dist/index.js"),
        PathBuf::from("/opt/openclaw/dist/index.js"),
    ];
    let (openclaw_entry, openclaw_available) = find_first_existing(&openclaw_candidates)
        .map(|p| (p, true))
        .unwrap_or_else(|| {
            warn!("OpenClaw JS bundle not found — will run in executor-native mode");
            (workspace.join("openclaw").join("dist").join("index.js"), false)
        });

    // ── 4. Locate WasmEdge-QuickJS WASM ──────────────────────────────────────
    let quickjs_candidates: Vec<PathBuf> = vec![
        cfg.quickjs_wasm.clone().unwrap_or_else(|| PathBuf::from(".")),
        PathBuf::from("assets/wasmedge_quickjs.wasm"),
        dirs::data_dir()
            .unwrap_or_default()
            .join("openclaw-plus")
            .join("wasmedge_quickjs.wasm"),
        PathBuf::from("/usr/share/openclaw-plus/wasmedge_quickjs.wasm"),
    ];
    let (quickjs_wasm, quickjs_available) = find_first_existing(&quickjs_candidates)
        .map(|p| (p, true))
        .unwrap_or_else(|| {
            warn!("wasmedge_quickjs.wasm not found — WasmEdge sandbox unavailable");
            (
                dirs::data_dir()
                    .unwrap_or_default()
                    .join("openclaw-plus")
                    .join("wasmedge_quickjs.wasm"),
                false,
            )
        });

    // ── 5. Write openclaw.env (env vars passed to the JS bundle) ─────────────
    let env_path = workspace.join("openclaw.env");
    let env_content = format!(
        "OPENCLAW_PLUGIN_CONFIG={}\nOPENCLAW_GATEWAY_URL=http://localhost:{}\nOPENCLAW_AGENT_ID={}\n",
        plugin_config_path.display(),
        cfg.gateway_port,
        cfg.agent_id,
    );
    tokio::fs::write(&env_path, &env_content)
        .await
        .map_err(ExecutorError::IoError)?;

    info!(
        agent_id = %cfg.agent_id,
        openclaw = openclaw_available,
        quickjs = quickjs_available,
        "Bootstrap complete"
    );

    Ok(BootstrapResult {
        workspace_dir: workspace,
        plugin_config_path,
        openclaw_entry,
        quickjs_wasm,
        shim_path,
        openclaw_available,
        quickjs_available,
    })
}

// ── Install check helpers ─────────────────────────────────────────────────────

/// Check if `npm` is available.
pub fn npm_available() -> bool {
    std::process::Command::new("npm")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if `node` is available and meets minimum version (18+).
pub fn node_available() -> Option<String> {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Check if `wasmedge` CLI is available.
pub fn wasmedge_cli_available() -> bool {
    std::process::Command::new("wasmedge")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// System readiness report used by the UI environment check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReadiness {
    pub node_version: Option<String>,
    pub npm_available: bool,
    pub wasmedge_cli: bool,
    pub openclaw_js_found: bool,
    pub quickjs_wasm_found: bool,
    /// Overall: can run in full WasmEdge mode.
    pub full_mode_ready: bool,
    /// Can run in executor-native mode (no WasmEdge).
    pub native_mode_ready: bool,
}

pub fn check_system_readiness(agent_id: &str) -> SystemReadiness {
    let node_version = node_available();
    let npm_ok = npm_available();
    let wasmedge_ok = wasmedge_cli_available();

    let workspace = agent_workspace_dir(agent_id);
    let openclaw_js = workspace.join("openclaw").join("dist").join("index.js").exists()
        || dirs::home_dir()
            .unwrap_or_default()
            .join(".openclaw-plus/openclaw/dist/index.js")
            .exists();
    let quickjs_wasm = PathBuf::from("assets/wasmedge_quickjs.wasm").exists()
        || dirs::data_dir()
            .unwrap_or_default()
            .join("openclaw-plus/wasmedge_quickjs.wasm")
            .exists();

    SystemReadiness {
        node_version: node_version.clone(),
        npm_available: npm_ok,
        wasmedge_cli: wasmedge_ok,
        openclaw_js_found: openclaw_js,
        quickjs_wasm_found: quickjs_wasm,
        full_mode_ready: openclaw_js && quickjs_wasm && wasmedge_ok,
        native_mode_ready: true, // always available — pure Rust ReAct loop
    }
}

// ── Path helpers ──────────────────────────────────────────────────────────────

pub fn agent_workspace_dir(agent_id: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw-plus")
        .join("agents")
        .join(agent_id)
        .join("workspace")
}

fn find_first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.exists()).cloned()
}

// ── Security shim (inlined for bootstrap, independent of sandbox crate) ───────

fn generate_security_shim(gateway_port: u16) -> String {
    format!(
        r#"'use strict';
// OpenClaw+ Security Shim (bootstrap-generated)
const GATEWAY = 'http://localhost:{port}';

async function _before_skill(skillName, args) {{
  try {{
    const r = await fetch(GATEWAY + '/hooks/before-skill', {{
      method: 'POST',
      headers: {{'Content-Type': 'application/json'}},
      body: JSON.stringify({{
        invocationId: Math.random().toString(36).slice(2),
        skillName, sessionId: process.env.OPENCLAW_SESSION_ID || 'default',
        args, timestamp: new Date().toISOString()
      }})
    }});
    const j = await r.json();
    return j.verdict === 'allow';
  }} catch(e) {{
    console.error('[OpenClaw+] Gateway unreachable:', e.message);
    return true; // fail-open in dev mode
  }}
}}

// Intercept fetch for network skill calls
const _origFetch = globalThis.fetch;
globalThis.fetch = async function(url, opts) {{
  const allowed = await _before_skill('web.fetch', {{url: String(url)}});
  if (!allowed) throw new Error('[OpenClaw+] web.fetch blocked by policy: ' + url);
  return _origFetch(url, opts);
}};

console.log('[OpenClaw+] Security Shim active — Gateway: http://localhost:{port}');
"#,
        port = gateway_port,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_workspace_path_contains_agent_id() {
        let p = agent_workspace_dir("agent-xyz");
        assert!(p.to_string_lossy().contains("agent-xyz"));
        assert!(p.to_string_lossy().contains("workspace"));
    }

    #[test]
    fn plugin_config_json_structure() {
        let cfg = BootstrapConfig::new("test-agent", 7878);
        let gateway_base = format!("http://localhost:{}", cfg.gateway_port);
        let pc = PluginConfig {
            plugin_id: "test".into(),
            plugin_name: "Test".into(),
            version: "0.1.0".into(),
            hooks: HooksConfig {
                before_skill: format!("{}/hooks/before-skill", gateway_base),
                after_skill:  format!("{}/hooks/after-skill",  gateway_base),
                agent_start:  format!("{}/hooks/agent-start",  gateway_base),
                agent_stop:   format!("{}/hooks/agent-stop",   gateway_base),
            },
        };
        let json = serde_json::to_string(&pc).unwrap();
        assert!(json.contains("before-skill"));
        assert!(json.contains("7878"));
    }

    #[test]
    fn security_shim_contains_gateway_port() {
        let shim = generate_security_shim(9999);
        assert!(shim.contains("9999"));
        assert!(shim.contains("web.fetch"));
    }

    #[test]
    fn find_first_existing_returns_none_for_missing() {
        let paths = vec![
            PathBuf::from("/nonexistent/path/a"),
            PathBuf::from("/nonexistent/path/b"),
        ];
        assert!(find_first_existing(&paths).is_none());
    }
}
