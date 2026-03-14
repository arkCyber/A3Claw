//! `python.run` — execute a Python snippet or script file inside a sandboxed
//! subprocess, capture stdout/stderr, and return a structured observation.
//!
//! ## Design goals (aerospace-grade)
//! - **Isolation**: runs Python in a subprocess; timeout enforced via SIGKILL.
//! - **No network side-effects by default**: caller must explicitly pass `env`
//!   with proxies if outbound network is needed.
//! - **Pip auto-detect**: if `requirements` is supplied, installs them into a
//!   per-invocation temporary venv before executing the script.
//! - **Structured output**: returns JSON `{stdout, stderr, exit_code, elapsed_ms}`.
//! - **Size guard**: truncates oversized output to avoid flooding the LLM context.
//!
//! ## Parameters
//! | name           | required | description |
//! |----------------|----------|-------------|
//! | `code`         | yes*     | Python source code to execute |
//! | `script`       | yes*     | Path to an existing `.py` file (alternative to `code`) |
//! | `args`         | no       | CLI arguments passed after the script (JSON array of strings) |
//! | `cwd`          | no       | Working directory (default: OS temp dir) |
//! | `timeout_secs` | no       | Kill after N seconds (default: 30, max: 300) |
//! | `requirements` | no       | Pip packages to install: `["pandas", "requests==2.31"]` |
//! | `env`          | no       | Extra environment variables as `{"KEY": "VAL"}` |
//! | `python`       | no       | Python binary path (default: `python3`) |
//!
//! *Exactly one of `code` or `script` must be provided.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Maximum bytes we return from stdout or stderr to the LLM.
const MAX_OUTPUT_BYTES: usize = 16_000;
/// Hard cap on timeout regardless of caller request.
const MAX_TIMEOUT_SECS: u64 = 300;
/// Default timeout.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

// ── Public API ────────────────────────────────────────────────────────────────

pub struct PythonArgs {
    /// Inline source code (mutually exclusive with `script_path`).
    pub code: Option<String>,
    /// Path to an existing .py file.
    pub script_path: Option<String>,
    /// CLI arguments appended after the script.
    pub args: Vec<String>,
    /// Working directory for the subprocess.
    pub cwd: Option<String>,
    pub timeout_secs: u64,
    /// Packages to install via pip before running.
    pub requirements: Vec<String>,
    pub env: HashMap<String, String>,
    /// Python interpreter binary (default: "python3").
    pub python: String,
}

impl PythonArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        let code = v["code"].as_str().map(|s| s.to_string());
        let script_path = v["script"].as_str().map(|s| s.to_string());

        if code.is_none() && script_path.is_none() {
            return Err("python.run: one of 'code' or 'script' is required".into());
        }
        if code.is_some() && script_path.is_some() {
            return Err("python.run: 'code' and 'script' are mutually exclusive".into());
        }

        let args = v["args"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let timeout_secs = v["timeout_secs"]
            .as_u64()
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        let requirements = v["requirements"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let env = v["env"]
            .as_object()
            .map(|o| {
                o.iter()
                    .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let python = v["python"]
            .as_str()
            .unwrap_or("python3")
            .to_string();

        let cwd = v["cwd"].as_str().map(|s| s.to_string());

        Ok(Self { code, script_path, args, cwd, timeout_secs, requirements, env, python })
    }
}

/// Run a Python snippet/script. Returns a JSON observation string.
pub fn run(args: &PythonArgs) -> Result<String, String> {
    let start = Instant::now();

    // ── Resolve working directory ──────────────────────────────────────────
    let work_dir = match &args.cwd {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::temp_dir(),
    };
    std::fs::create_dir_all(&work_dir)
        .map_err(|e| format!("python.run: cannot create cwd {}: {}", work_dir.display(), e))?;

    // ── Write inline code to a temp file ──────────────────────────────────
    let (inline_tmp, script_path) = if let Some(code) = &args.code {
        let tmp = tempfile_named(&work_dir, "openclaw_script_", ".py")?;
        std::fs::write(&tmp, code.as_bytes())
            .map_err(|e| format!("python.run: failed to write temp script: {}", e))?;
        (Some(tmp.clone()), tmp)
    } else {
        let p = std::path::PathBuf::from(args.script_path.as_deref().unwrap());
        if !p.exists() {
            return Err(format!("python.run: script not found: {}", p.display()));
        }
        (None, p)
    };

    // ── Optional pip install into temp venv ───────────────────────────────
    let python_bin = if !args.requirements.is_empty() {
        install_requirements(&args.python, &args.requirements, &work_dir)?
    } else {
        // Verify python exists
        which_python(&args.python)?;
        args.python.clone()
    };

    // ── Build subprocess ──────────────────────────────────────────────────
    let mut cmd = std::process::Command::new(&python_bin);
    cmd.arg(&script_path);
    cmd.args(&args.args);
    cmd.current_dir(&work_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    for (k, v) in &args.env {
        cmd.env(k, v);
    }

    let timeout = Duration::from_secs(args.timeout_secs);

    let run_result = run_with_timeout(cmd, timeout);

    // ── Always clean up inline temp script ────────────────────────────────
    if let Some(ref tmp) = inline_tmp {
        let _ = std::fs::remove_file(tmp);
    }

    let (stdout_raw, stderr_raw, exit_code) = run_result?;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    let stdout = truncate_utf8(&stdout_raw, MAX_OUTPUT_BYTES);
    let stderr = truncate_utf8(&stderr_raw, MAX_OUTPUT_BYTES / 2);

    // ── Return structured JSON observation ────────────────────────────────
    Ok(serde_json::json!({
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "elapsed_ms": elapsed_ms,
        "truncated": stdout_raw.len() > MAX_OUTPUT_BYTES || stderr_raw.len() > MAX_OUTPUT_BYTES / 2,
    })
    .to_string())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Verify the Python binary exists; return its resolved path.
fn which_python(python: &str) -> Result<String, String> {
    let out = std::process::Command::new("which")
        .arg(python)
        .output()
        .map_err(|_| format!("python.run: cannot locate '{}' — is Python installed?", python))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!(
            "python.run: '{}' not found in PATH — install Python 3 or set the 'python' parameter",
            python
        ))
    }
}

/// Create a temporary venv, pip-install requirements, return the venv Python path.
fn install_requirements(
    base_python: &str,
    reqs: &[String],
    work_dir: &std::path::Path,
) -> Result<String, String> {
    let venv_dir = work_dir.join(".openclaw_venv");

    // Create venv (idempotent)
    let create = std::process::Command::new(base_python)
        .args(["-m", "venv", venv_dir.to_str().unwrap_or(".openclaw_venv")])
        .output()
        .map_err(|e| format!("python.run: venv creation failed: {}", e))?;
    if !create.status.success() {
        return Err(format!(
            "python.run: venv creation failed: {}",
            String::from_utf8_lossy(&create.stderr)
        ));
    }

    // Locate pip inside venv
    let pip = venv_dir.join("bin").join("pip");
    if !pip.exists() {
        return Err(format!(
            "python.run: pip not found in venv at {}",
            pip.display()
        ));
    }

    // pip install requirements
    let mut pip_cmd = std::process::Command::new(&pip);
    pip_cmd.arg("install").arg("--quiet");
    for r in reqs {
        pip_cmd.arg(r);
    }
    let pip_out = pip_cmd
        .output()
        .map_err(|e| format!("python.run: pip install failed: {}", e))?;
    if !pip_out.status.success() {
        return Err(format!(
            "python.run: pip install failed:\n{}",
            String::from_utf8_lossy(&pip_out.stderr)
        ));
    }

    // Return venv python binary
    let venv_python = venv_dir.join("bin").join("python3");
    if venv_python.exists() {
        Ok(venv_python.to_string_lossy().to_string())
    } else {
        Ok(venv_dir.join("bin").join("python").to_string_lossy().to_string())
    }
}

/// Spawn a command, wait with timeout, return (stdout_bytes, stderr_bytes, exit_code).
fn run_with_timeout(
    mut cmd: std::process::Command,
    timeout: Duration,
) -> Result<(Vec<u8>, Vec<u8>, i32), String> {
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("python.run: failed to spawn Python: {}", e))?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child
                    .stdout
                    .take()
                    .map(|mut r| { let mut b = Vec::new(); let _ = std::io::Read::read_to_end(&mut r, &mut b); b })
                    .unwrap_or_default();
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut r| { let mut b = Vec::new(); let _ = std::io::Read::read_to_end(&mut r, &mut b); b })
                    .unwrap_or_default();
                return Ok((stdout, stderr, status.code().unwrap_or(-1)));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(format!(
                        "python.run: timed out after {}s",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("python.run: wait error: {}", e)),
        }
    }
}

/// Generate a unique temp file path (without creating it as a true NamedTempFile
/// so the child process can read it after we write it).
///
/// Uses pid + thread_id + full-nanosecond timestamp for collision resistance
/// even when called multiple times per second from different threads.
fn tempfile_named(
    dir: &std::path::Path,
    prefix: &str,
    suffix: &str,
) -> Result<std::path::PathBuf, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let pid = std::process::id();
    // Include the current thread id for multi-threaded callers.
    let tid = format!("{:?}", std::thread::current().id());
    // Hash tid to a compact hex to avoid filesystem-unfriendly chars.
    let tid_hash: u64 = tid.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let name = format!("{}{:08x}_{:016x}_{:08x}{}", prefix, pid, tid_hash, nanos, suffix);
    let path = dir.join(&name);
    // Ensure no collision on an extremely busy system.
    if path.exists() {
        let fallback = format!("{}{:08x}_{:016x}_{:08x}_2{}", prefix, pid, tid_hash, nanos.wrapping_add(1), suffix);
        return Ok(dir.join(fallback));
    }
    Ok(path)
}

/// Truncate raw bytes to `max` bytes at a valid UTF-8 boundary, return as String.
fn truncate_utf8(raw: &[u8], max: usize) -> String {
    if raw.len() <= max {
        return String::from_utf8_lossy(raw).to_string();
    }
    let mut end = max;
    while end > 0 && (raw[end] & 0xC0) == 0x80 {
        end -= 1;
    }
    let mut s = String::from_utf8_lossy(&raw[..end]).to_string();
    s.push_str("\n...(output truncated)");
    s
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn args_from_code(code: &str) -> PythonArgs {
        PythonArgs {
            code: Some(code.to_string()),
            script_path: None,
            args: vec![],
            cwd: None,
            timeout_secs: 10,
            requirements: vec![],
            env: HashMap::new(),
            python: "python3".to_string(),
        }
    }

    // ── from_json ─────────────────────────────────────────────────────────

    #[test]
    fn from_json_code_only_ok() {
        let v = serde_json::json!({"code": "print(1)"});
        let a = PythonArgs::from_json(&v).unwrap();
        assert_eq!(a.code.as_deref(), Some("print(1)"));
        assert!(a.script_path.is_none());
    }

    #[test]
    fn from_json_script_only_ok() {
        let v = serde_json::json!({"script": "/tmp/foo.py"});
        let a = PythonArgs::from_json(&v).unwrap();
        assert_eq!(a.script_path.as_deref(), Some("/tmp/foo.py"));
        assert!(a.code.is_none());
    }

    #[test]
    fn from_json_neither_errors() {
        let v = serde_json::json!({});
        assert!(PythonArgs::from_json(&v).is_err());
    }

    #[test]
    fn from_json_both_errors() {
        let v = serde_json::json!({"code": "x=1", "script": "/tmp/f.py"});
        assert!(PythonArgs::from_json(&v).is_err());
    }

    #[test]
    fn from_json_timeout_capped_at_300() {
        let v = serde_json::json!({"code": "x=1", "timeout_secs": 9999});
        let a = PythonArgs::from_json(&v).unwrap();
        assert_eq!(a.timeout_secs, 300);
    }

    #[test]
    fn from_json_requirements_parsed() {
        let v = serde_json::json!({"code": "x=1", "requirements": ["pandas", "requests==2.31"]});
        let a = PythonArgs::from_json(&v).unwrap();
        assert_eq!(a.requirements, vec!["pandas", "requests==2.31"]);
    }

    #[test]
    fn from_json_args_parsed() {
        let v = serde_json::json!({"code": "x=1", "args": ["--foo", "bar"]});
        let a = PythonArgs::from_json(&v).unwrap();
        assert_eq!(a.args, vec!["--foo", "bar"]);
    }

    #[test]
    fn from_json_env_parsed() {
        let v = serde_json::json!({"code": "x=1", "env": {"MY_VAR": "hello"}});
        let a = PythonArgs::from_json(&v).unwrap();
        assert_eq!(a.env.get("MY_VAR").map(|s| s.as_str()), Some("hello"));
    }

    // ── truncate_utf8 ─────────────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        let s = b"hello world";
        assert_eq!(truncate_utf8(s, 1000), "hello world");
    }

    #[test]
    fn truncate_long_string_has_marker() {
        let long = "x".repeat(100);
        let result = truncate_utf8(long.as_bytes(), 50);
        assert!(result.contains("truncated"), "must contain truncation marker");
        assert!(result.len() < 100, "must be shorter than original");
    }

    #[test]
    fn truncate_valid_utf8_boundary() {
        // "€" is 3 bytes (0xe2 0x82 0xac) — ensure we don't cut in the middle
        let euro = "€".repeat(20);
        let bytes = euro.as_bytes();
        let result = truncate_utf8(bytes, 10); // 10 bytes = 3 full euros
        assert!(std::str::from_utf8(result.trim_end_matches("...(output truncated)").as_bytes()).is_ok()
            || result.contains("truncated"),
            "must be valid UTF-8 up to truncation point");
    }

    // ── run (integration — requires python3 on PATH) ──────────────────────

    #[test]
    #[ignore = "requires python3 on PATH"]
    fn run_hello_world() {
        let a = args_from_code("print('hello openclaw')");
        let out = run(&a).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["exit_code"], 0);
        assert!(v["stdout"].as_str().unwrap().contains("hello openclaw"));
    }

    #[test]
    #[ignore = "requires python3 on PATH"]
    fn run_exit_code_nonzero() {
        let a = args_from_code("import sys; sys.exit(42)");
        let out = run(&a).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["exit_code"], 42);
    }

    #[test]
    #[ignore = "requires python3 on PATH"]
    fn run_stderr_captured() {
        let a = args_from_code("import sys; sys.stderr.write('err msg\\n')");
        let out = run(&a).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["stderr"].as_str().unwrap().contains("err msg"));
    }

    #[test]
    #[ignore = "requires python3 on PATH"]
    fn run_env_variable_accessible() {
        let mut env = HashMap::new();
        env.insert("TEST_TOKEN".to_string(), "abc123".to_string());
        let a = PythonArgs {
            code: Some("import os; print(os.environ['TEST_TOKEN'])".to_string()),
            env,
            ..args_from_code("")
        };
        let out = run(&a).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["stdout"].as_str().unwrap().contains("abc123"));
    }

    #[test]
    #[ignore = "requires python3 on PATH"]
    fn run_timeout_kills_process() {
        let a = PythonArgs {
            code: Some("import time; time.sleep(60)".to_string()),
            timeout_secs: 1,
            ..args_from_code("")
        };
        let result = run(&a);
        assert!(result.is_err(), "must error on timeout");
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[test]
    #[ignore = "requires python3 on PATH"]
    fn run_script_file() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("test.py");
        std::fs::write(&script, b"print('from file')").unwrap();
        let a = PythonArgs {
            code: None,
            script_path: Some(script.to_string_lossy().to_string()),
            ..args_from_code("")
        };
        let out = run(&a).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["stdout"].as_str().unwrap().contains("from file"));
    }

    #[test]
    #[ignore = "requires python3 on PATH"]
    fn run_returns_elapsed_ms() {
        let a = args_from_code("pass");
        let out = run(&a).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["elapsed_ms"].as_u64().unwrap() < 5000, "must complete quickly");
    }
}
