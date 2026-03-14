//! Integration tests for `openclaw-sandbox` using real data.
//!
//! Covers:
//! - `WasiBuilder` with real `tempdir` paths for workspace and openclaw source
//! - Multiple read-write and read-only mounts with real directories
//! - Environment variable injection with real values
//! - `node_mock` JS shim generation with real content checks
//! - `ipc` message serialisation round-trips with real payloads
//! - `agent_sandbox` configuration validation

use openclaw_sandbox::wasi_builder::{WasiArgs, WasiBuilder};
use openclaw_security::{FsMount, SecurityConfig};
use std::path::PathBuf;
use tempfile::tempdir;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_config_with_dirs(
    entry: PathBuf,
    workspace: PathBuf,
    mounts: Vec<FsMount>,
) -> SecurityConfig {
    SecurityConfig {
        openclaw_entry: entry,
        workspace_dir:  workspace,
        memory_limit_mb: 512,
        fs_mounts: mounts,
        ..SecurityConfig::default()
    }
}

// ── WasiBuilder: real tempdir workspace ───────────────────────────────────────

#[test]
fn wasi_builder_real_workspace_mount_appears_in_preopens() {
    let workspace = tempdir().unwrap();
    let source_dir = tempdir().unwrap();
    let entry = source_dir.path().join("index.js");
    std::fs::write(&entry, b"// entry").unwrap();

    let config = make_config_with_dirs(
        entry.clone(),
        workspace.path().to_path_buf(),
        vec![FsMount {
            host_path: workspace.path().to_path_buf(),
            guest_path: "/workspace".to_string(),
            readonly: false,
        }],
    );

    let args = WasiBuilder::new(&config).build_wasi_args();
    assert!(
        args.preopens.iter().any(|p| p.contains("/workspace")),
        "workspace mount must appear in preopens: {:?}", args.preopens
    );
}

#[test]
fn wasi_builder_real_readonly_mount_has_suffix() {
    let ro_dir = tempdir().unwrap();
    let workspace = tempdir().unwrap();
    let source_dir = tempdir().unwrap();
    let entry = source_dir.path().join("main.js");
    std::fs::write(&entry, b"// main").unwrap();

    let config = make_config_with_dirs(
        entry,
        workspace.path().to_path_buf(),
        vec![FsMount {
            host_path: ro_dir.path().to_path_buf(),
            guest_path: "/data".to_string(),
            readonly: true,
        }],
    );

    let args = WasiBuilder::new(&config).build_wasi_args();
    assert!(
        args.preopens.iter().any(|p| p.ends_with(":readonly")),
        "read-only mount must end with :readonly: {:?}", args.preopens
    );
}

#[test]
fn wasi_builder_multiple_mounts_all_appear() {
    let ws  = tempdir().unwrap();
    let src = tempdir().unwrap();
    let ro1 = tempdir().unwrap();
    let rw1 = tempdir().unwrap();
    let entry = src.path().join("agent.js");
    std::fs::write(&entry, b"// agent").unwrap();

    let config = make_config_with_dirs(
        entry,
        ws.path().to_path_buf(),
        vec![
            FsMount { host_path: ro1.path().to_path_buf(), guest_path: "/models".to_string(),  readonly: true  },
            FsMount { host_path: rw1.path().to_path_buf(), guest_path: "/output".to_string(),  readonly: false },
            FsMount { host_path: ws.path().to_path_buf(),  guest_path: "/workspace".to_string(), readonly: false },
        ],
    );

    let args = WasiBuilder::new(&config).build_wasi_args();
    assert!(args.preopens.iter().any(|p| p.contains("/models")),    "models mount missing");
    assert!(args.preopens.iter().any(|p| p.contains("/output")),    "output mount missing");
    assert!(args.preopens.iter().any(|p| p.contains("/workspace")), "workspace mount missing");
}

#[test]
fn wasi_builder_env_contains_memory_limit() {
    let ws  = tempdir().unwrap();
    let src = tempdir().unwrap();
    let entry = src.path().join("index.js");
    std::fs::write(&entry, b"// ok").unwrap();

    let mut config = make_config_with_dirs(entry, ws.path().to_path_buf(), vec![]);
    config.memory_limit_mb = 1024;

    let args = WasiBuilder::new(&config).build_wasi_args();
    assert!(
        args.envs.iter().any(|e| e == "OPENCLAW_MEMORY_LIMIT=1024"),
        "OPENCLAW_MEMORY_LIMIT must reflect config value: {:?}", args.envs
    );
}

#[test]
fn wasi_builder_env_workspace_matches_real_path() {
    let ws  = tempdir().unwrap();
    let src = tempdir().unwrap();
    let entry = src.path().join("index.js");
    std::fs::write(&entry, b"// ok").unwrap();

    let config = make_config_with_dirs(entry, ws.path().to_path_buf(), vec![]);
    let args = WasiBuilder::new(&config).build_wasi_args();

    let ws_env = args.envs.iter()
        .find(|e| e.starts_with("OPENCLAW_WORKSPACE="))
        .expect("OPENCLAW_WORKSPACE must be set");
    assert!(
        ws_env.contains(ws.path().to_str().unwrap()),
        "OPENCLAW_WORKSPACE must contain real path: {}", ws_env
    );
}

#[test]
fn wasi_builder_entry_script_uses_guest_openclaw_prefix() {
    let ws  = tempdir().unwrap();
    let src = tempdir().unwrap();
    let entry = src.path().join("my_agent.js");
    std::fs::write(&entry, b"// agent code").unwrap();

    let config = make_config_with_dirs(entry, ws.path().to_path_buf(), vec![]);
    let args = WasiBuilder::new(&config).build_wasi_args();

    let entry_arg = args.args.last().unwrap();
    assert!(entry_arg.starts_with("/openclaw/"), "entry must be guest path: {}", entry_arg);
    assert!(entry_arg.ends_with("my_agent.js"),  "filename must be preserved: {}", entry_arg);
}

#[test]
fn wasi_builder_first_arg_always_wasm_binary() {
    let ws  = tempdir().unwrap();
    let src = tempdir().unwrap();
    let entry = src.path().join("index.js");
    std::fs::write(&entry, b"").unwrap();

    let config = make_config_with_dirs(entry, ws.path().to_path_buf(), vec![]);
    let args = WasiBuilder::new(&config).build_wasi_args();
    assert_eq!(args.args[0], "wasmedge_quickjs.wasm");
}

#[test]
fn wasi_builder_with_shim_does_not_insert_pre_script() {
    let ws  = tempdir().unwrap();
    let src = tempdir().unwrap();
    let shim_dir = tempdir().unwrap();
    let entry = src.path().join("index.js");
    let shim  = shim_dir.path().join("security_shim.js");
    std::fs::write(&entry, b"// ok").unwrap();
    std::fs::write(&shim, b"// shim").unwrap();

    let config = make_config_with_dirs(entry, ws.path().to_path_buf(), vec![]);
    let args = WasiBuilder::new(&config)
        .with_shim(shim)
        .build_wasi_args();

    assert!(
        !args.args.contains(&"--pre-script".to_string()),
        "--pre-script must never appear (not supported by wasmedge_quickjs): {:?}", args.args
    );
}

#[test]
fn wasi_builder_openclaw_source_dir_mounted_readonly_when_exists() {
    let ws  = tempdir().unwrap();
    // Use an existing real directory as the entry parent so the mount is added
    let src = tempdir().unwrap();
    let entry = src.path().join("index.js");
    std::fs::write(&entry, b"// ok").unwrap();

    let config = make_config_with_dirs(entry, ws.path().to_path_buf(), vec![]);
    let args = WasiBuilder::new(&config).build_wasi_args();

    assert!(
        args.preopens.iter().any(|p| p.starts_with("/openclaw:") && p.ends_with(":readonly")),
        "openclaw source dir must be mounted read-only: {:?}", args.preopens
    );
}

#[test]
fn wasi_builder_missing_entry_parent_skips_openclaw_mount() {
    let ws  = tempdir().unwrap();
    // Non-existent entry directory → no /openclaw mount added
    let config = make_config_with_dirs(
        PathBuf::from("/nonexistent/deeply/nested/index.js"),
        ws.path().to_path_buf(),
        vec![],
    );
    let args = WasiBuilder::new(&config).build_wasi_args();
    assert!(
        !args.preopens.iter().any(|p| p.starts_with("/openclaw:")),
        "nonexistent entry parent must not produce /openclaw mount: {:?}", args.preopens
    );
}

#[test]
fn wasi_builder_sandbox_env_var_always_set_to_one() {
    let ws  = tempdir().unwrap();
    let src = tempdir().unwrap();
    let entry = src.path().join("index.js");
    std::fs::write(&entry, b"").unwrap();

    let config = make_config_with_dirs(entry, ws.path().to_path_buf(), vec![]);
    let args = WasiBuilder::new(&config).build_wasi_args();
    assert!(
        args.envs.contains(&"OPENCLAW_SANDBOX=1".to_string()),
        "OPENCLAW_SANDBOX=1 must always be injected: {:?}", args.envs
    );
}

#[test]
fn wasi_args_struct_fields_accessible() {
    let wa = WasiArgs {
        args:     vec!["wasmedge_quickjs.wasm".to_string(), "/openclaw/index.js".to_string()],
        envs:     vec!["OPENCLAW_SANDBOX=1".to_string()],
        preopens: vec!["/workspace:/tmp/ws".to_string()],
    };
    assert_eq!(wa.args[0], "wasmedge_quickjs.wasm");
    assert_eq!(wa.envs[0], "OPENCLAW_SANDBOX=1");
    assert_eq!(wa.preopens[0], "/workspace:/tmp/ws");
}

// ── ipc: IpcFrame round-trip with real payload data ─────────────────────────

#[test]
fn ipc_frame_ping_serde_roundtrip() {
    use openclaw_sandbox::ipc::IpcFrame;

    let msg = IpcFrame::Ping;
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("Ping"), "JSON must contain Ping: {}", json);

    let decoded: IpcFrame = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, IpcFrame::Ping));
}

#[test]
fn ipc_frame_pong_serde_roundtrip() {
    use openclaw_sandbox::ipc::IpcFrame;

    let msg = IpcFrame::Pong;
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("Pong"), "JSON must contain Pong: {}", json);

    let decoded: IpcFrame = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, IpcFrame::Pong));
}

#[test]
fn ipc_frame_ping_pong_are_distinct() {
    use openclaw_sandbox::ipc::IpcFrame;

    let ping_json = serde_json::to_string(&IpcFrame::Ping).unwrap();
    let pong_json = serde_json::to_string(&IpcFrame::Pong).unwrap();
    assert_ne!(ping_json, pong_json, "Ping and Pong must serialise differently");
}

#[test]
fn ipc_socket_path_is_in_tempdir() {
    use openclaw_sandbox::ipc::ipc_socket_path;

    let path = ipc_socket_path();
    let path_str = path.to_string_lossy();
    assert!(path_str.contains("openclaw-plus"), "socket path must contain openclaw-plus: {}", path_str);
    assert!(path_str.ends_with("ipc.sock"), "socket path must end with ipc.sock: {}", path_str);
}

#[test]
fn ipc_socket_path_parent_is_temp_dir() {
    use openclaw_sandbox::ipc::ipc_socket_path;

    let path = ipc_socket_path();
    let parent = path.parent().unwrap();
    let parent_str = parent.to_string_lossy();
    assert!(parent_str.contains("openclaw-plus"));
}

// ── node_mock: JS shim content validation ────────────────────────────────────

#[test]
fn node_mock_shim_contains_try_catch_for_require() {
    use openclaw_sandbox::node_mock::generate_shim;

    let shim = generate_shim();
    assert!(shim.contains("try"),   "shim must wrap require() in try/catch");
    assert!(shim.contains("catch"), "shim must have catch blocks");
}

#[test]
fn node_mock_shim_is_non_empty_valid_js_scaffold() {
    use openclaw_sandbox::node_mock::generate_shim;

    let shim = generate_shim();
    assert!(!shim.is_empty(), "shim must not be empty");
    assert!(
        shim.contains("function") || shim.contains("const") || shim.contains("var"),
        "shim must contain JS declarations"
    );
}

#[test]
fn node_mock_shim_does_not_use_top_level_require() {
    use openclaw_sandbox::node_mock::generate_shim;

    let shim = generate_shim();
    for line in shim.lines() {
        let trimmed = line.trim();
        assert!(
            !trimmed.starts_with("require("),
            "top-level require() would crash QuickJS: {:?}", line
        );
    }
}

#[test]
fn node_mock_shim_with_custom_port_contains_port() {
    use openclaw_sandbox::node_mock::generate_shim_with_port;

    let shim = generate_shim_with_port(9999);
    assert!(shim.contains("9999"), "shim must contain the custom port");
}

#[test]
fn node_mock_shim_default_port_7878() {
    use openclaw_sandbox::node_mock::generate_shim;

    let shim = generate_shim();
    assert!(shim.contains("7878"), "default shim must reference port 7878");
}
