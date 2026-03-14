//! `ssh.*` — SSH remote command execution and file transfer via system binaries.
//!
//! Delegates to the system `ssh` / `scp` binaries so the Rust code stays thin,
//! inherits the host `~/.ssh/config` and ssh-agent, and supports all key types.
//!
//! All skills are classified `Confirm` — the Gateway must approve each call.

use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 300;
const MAX_OUTPUT_BYTES: usize = 32_000;

// ── Argument types ────────────────────────────────────────────────────────────

pub struct SshExecArgs {
    pub host: String,
    pub user: Option<String>,
    pub port: u16,
    pub command: String,
    pub identity_file: Option<String>,
    pub timeout_secs: u64,
    pub strict_host_check: bool,
    pub env: HashMap<String, String>,
}

pub struct ScpArgs {
    pub host: String,
    pub user: Option<String>,
    pub port: u16,
    pub local_path: String,
    pub remote_path: String,
    pub identity_file: Option<String>,
    pub timeout_secs: u64,
    pub strict_host_check: bool,
    pub upload: bool,
}

impl SshExecArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        Ok(Self {
            host: v["host"].as_str().ok_or("ssh.exec: missing 'host'")?.to_string(),
            command: v["command"].as_str().ok_or("ssh.exec: missing 'command'")?.to_string(),
            user: v["user"].as_str().map(|s| s.to_string()),
            port: v["port"].as_u64().unwrap_or(22) as u16,
            identity_file: v["identity_file"].as_str().map(|s| s.to_string()),
            timeout_secs: v["timeout_secs"].as_u64().unwrap_or(DEFAULT_TIMEOUT_SECS).min(MAX_TIMEOUT_SECS),
            strict_host_check: v["strict_host_check"].as_bool().unwrap_or(true),
            env: parse_env(&v["env"]),
        })
    }
}

impl ScpArgs {
    pub fn from_json_upload(v: &serde_json::Value) -> Result<Self, String> {
        Self::from_json_dir(v, true)
    }
    pub fn from_json_download(v: &serde_json::Value) -> Result<Self, String> {
        Self::from_json_dir(v, false)
    }
    fn from_json_dir(v: &serde_json::Value, upload: bool) -> Result<Self, String> {
        let skill = if upload { "ssh.upload" } else { "ssh.download" };
        Ok(Self {
            host: v["host"].as_str().ok_or_else(|| format!("{skill}: missing 'host'"))?.to_string(),
            local_path: v["local_path"].as_str().ok_or_else(|| format!("{skill}: missing 'local_path'"))?.to_string(),
            remote_path: v["remote_path"].as_str().ok_or_else(|| format!("{skill}: missing 'remote_path'"))?.to_string(),
            user: v["user"].as_str().map(|s| s.to_string()),
            port: v["port"].as_u64().unwrap_or(22) as u16,
            identity_file: v["identity_file"].as_str().map(|s| s.to_string()),
            timeout_secs: v["timeout_secs"].as_u64().unwrap_or(DEFAULT_TIMEOUT_SECS).min(MAX_TIMEOUT_SECS),
            strict_host_check: v["strict_host_check"].as_bool().unwrap_or(true),
            upload,
        })
    }
}

// ── ssh.exec ──────────────────────────────────────────────────────────────────

pub fn ssh_exec(args: &SshExecArgs) -> Result<String, String> {
    which_binary("ssh")?;
    let mut cmd = std::process::Command::new("ssh");
    apply_ssh_flags(&mut cmd, &args.host, args.user.as_deref(), args.port,
        args.identity_file.as_deref(), args.strict_host_check, args.timeout_secs);

    // Inject extra env vars as a prefix to the remote command.
    let env_prefix: String = args.env.iter()
        .map(|(k, v)| format!("{}={} ", shell_escape(k), shell_escape(v)))
        .collect();
    cmd.arg("--").arg(format!("{}{}", env_prefix, args.command));

    run_subprocess(cmd, args.timeout_secs, "ssh.exec")
}

// ── ssh.upload / ssh.download ─────────────────────────────────────────────────

pub fn scp_transfer(args: &ScpArgs) -> Result<String, String> {
    which_binary("scp")?;
    let remote = format!(
        "{}{}:{}",
        args.user.as_deref().map(|u| format!("{u}@")).unwrap_or_default(),
        args.host,
        args.remote_path
    );
    let mut cmd = std::process::Command::new("scp");
    cmd.arg("-P").arg(args.port.to_string());
    cmd.arg("-o").arg(format!("StrictHostKeyChecking={}", if args.strict_host_check { "yes" } else { "no" }));
    cmd.arg("-o").arg(format!("ConnectTimeout={}", args.timeout_secs));
    cmd.arg("-o").arg("BatchMode=yes");
    if let Some(id) = &args.identity_file { cmd.arg("-i").arg(id); }
    if args.upload {
        cmd.arg(&args.local_path).arg(&remote);
    } else {
        cmd.arg(&remote).arg(&args.local_path);
    }
    run_subprocess(cmd, args.timeout_secs, if args.upload { "ssh.upload" } else { "ssh.download" })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn apply_ssh_flags(
    cmd: &mut std::process::Command,
    host: &str, user: Option<&str>, port: u16,
    identity_file: Option<&str>, strict: bool, timeout: u64,
) {
    cmd.arg("-p").arg(port.to_string());
    cmd.arg("-o").arg(format!("StrictHostKeyChecking={}", if strict { "yes" } else { "no" }));
    cmd.arg("-o").arg(format!("ConnectTimeout={timeout}"));
    cmd.arg("-o").arg("BatchMode=yes");
    cmd.arg("-o").arg("LogLevel=ERROR");
    if let Some(id) = identity_file { cmd.arg("-i").arg(id); }
    if let Some(u) = user { cmd.arg(format!("{u}@{host}")); } else { cmd.arg(host); }
}

fn run_subprocess(mut cmd: std::process::Command, timeout_secs: u64, skill: &str) -> Result<String, String> {
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let start = std::time::Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("{skill}: failed to spawn: {e}"))?;
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = drain(child.stdout.take());
                let stderr = drain(child.stderr.take());
                let code = status.code().unwrap_or(-1);
                let out_str = truncate_utf8(&stdout, MAX_OUTPUT_BYTES);
                let err_str = truncate_utf8(&stderr, MAX_OUTPUT_BYTES / 4);
                let elapsed = start.elapsed().as_millis() as u64;
                if code != 0 {
                    return Err(format!(
                        "{skill}: exited with code {code}\nstderr: {err_str}\nstdout: {out_str}"
                    ));
                }
                return Ok(serde_json::json!({
                    "exit_code": code,
                    "stdout": out_str,
                    "stderr": err_str,
                    "elapsed_ms": elapsed,
                }).to_string());
            }
            Ok(None) => {
                if start.elapsed() >= timeout { let _ = child.kill(); return Err(format!("{skill}: timed out after {timeout_secs}s")); }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("{skill}: wait error: {e}")),
        }
    }
}

fn drain(stream: Option<impl std::io::Read>) -> Vec<u8> {
    stream.map(|mut r| { let mut b = Vec::new(); let _ = std::io::Read::read_to_end(&mut r, &mut b); b }).unwrap_or_default()
}

fn which_binary(name: &str) -> Result<(), String> {
    let ok = std::process::Command::new("which").arg(name).output()
        .map(|o| o.status.success()).unwrap_or(false);
    if ok { Ok(()) } else { Err(format!("ssh.*: '{name}' not found in PATH — install OpenSSH client")) }
}

fn parse_env(v: &serde_json::Value) -> HashMap<String, String> {
    v.as_object().map(|o| o.iter()
        .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
        .collect()).unwrap_or_default()
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn truncate_utf8(raw: &[u8], max: usize) -> String {
    if raw.len() <= max { return String::from_utf8_lossy(raw).to_string(); }
    let mut end = max;
    while end > 0 && (raw[end] & 0xC0) == 0x80 { end -= 1; }
    let mut s = String::from_utf8_lossy(&raw[..end]).to_string();
    s.push_str("\n...(output truncated)");
    s
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_args_missing_host_errors() {
        assert!(SshExecArgs::from_json(&serde_json::json!({"command": "ls"})).is_err());
    }

    #[test]
    fn exec_args_missing_command_errors() {
        assert!(SshExecArgs::from_json(&serde_json::json!({"host": "h"})).is_err());
    }

    #[test]
    fn exec_args_defaults() {
        let a = SshExecArgs::from_json(&serde_json::json!({"host": "h", "command": "uptime"})).unwrap();
        assert_eq!(a.port, 22);
        assert_eq!(a.timeout_secs, DEFAULT_TIMEOUT_SECS);
        assert!(a.strict_host_check);
        assert!(a.user.is_none());
        assert!(a.identity_file.is_none());
    }

    #[test]
    fn exec_args_timeout_capped() {
        let a = SshExecArgs::from_json(&serde_json::json!({"host": "h", "command": "c", "timeout_secs": 9999})).unwrap();
        assert_eq!(a.timeout_secs, MAX_TIMEOUT_SECS);
    }

    #[test]
    fn exec_args_custom_port_user() {
        let a = SshExecArgs::from_json(&serde_json::json!({"host": "h", "command": "c", "port": 2222, "user": "admin"})).unwrap();
        assert_eq!(a.port, 2222);
        assert_eq!(a.user.as_deref(), Some("admin"));
    }

    #[test]
    fn exec_args_strict_host_check_false() {
        let a = SshExecArgs::from_json(&serde_json::json!({"host": "h", "command": "c", "strict_host_check": false})).unwrap();
        assert!(!a.strict_host_check);
    }

    #[test]
    fn exec_args_env_parsed() {
        let a = SshExecArgs::from_json(&serde_json::json!({"host": "h", "command": "c", "env": {"FOO": "bar"}})).unwrap();
        assert_eq!(a.env.get("FOO").map(|s| s.as_str()), Some("bar"));
    }

    #[test]
    fn upload_args_missing_host_errors() {
        assert!(ScpArgs::from_json_upload(&serde_json::json!({"local_path": "/a", "remote_path": "/b"})).is_err());
    }

    #[test]
    fn upload_args_missing_local_path_errors() {
        assert!(ScpArgs::from_json_upload(&serde_json::json!({"host": "h", "remote_path": "/b"})).is_err());
    }

    #[test]
    fn upload_args_missing_remote_path_errors() {
        assert!(ScpArgs::from_json_upload(&serde_json::json!({"host": "h", "local_path": "/a"})).is_err());
    }

    #[test]
    fn download_args_ok() {
        let a = ScpArgs::from_json_download(&serde_json::json!({"host": "h", "local_path": "/a", "remote_path": "/b"})).unwrap();
        assert!(!a.upload);
        assert_eq!(a.host, "h");
    }

    #[test]
    fn upload_args_ok() {
        let a = ScpArgs::from_json_upload(&serde_json::json!({"host": "h", "local_path": "/a", "remote_path": "/b"})).unwrap();
        assert!(a.upload);
    }

    #[test]
    fn shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn shell_escape_with_single_quote() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn truncate_utf8_short_unchanged() {
        assert_eq!(truncate_utf8(b"hello", 100), "hello");
    }

    #[test]
    fn truncate_utf8_long_has_marker() {
        let long = b"x".repeat(200);
        let result = truncate_utf8(&long, 100);
        assert!(result.contains("truncated"));
    }

    #[test]
    fn run_subprocess_nonzero_exit_returns_err() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("exit 42");
        let result = run_subprocess(cmd, 5, "test.skill");
        assert!(result.is_err(), "non-zero exit must return Err");
        let msg = result.unwrap_err();
        assert!(msg.contains("42"), "error must include exit code: {msg}");
    }

    #[test]
    fn run_subprocess_zero_exit_returns_ok() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("echo hello");
        let result = run_subprocess(cmd, 5, "test.skill");
        assert!(result.is_ok(), "zero exit must return Ok");
        let v: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["exit_code"], 0);
        assert!(v["stdout"].as_str().unwrap().contains("hello"));
    }

    #[test]
    fn run_subprocess_stderr_in_error_message() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("echo 'some error' >&2; exit 1");
        let result = run_subprocess(cmd, 5, "test.skill");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("some error"), "stderr must appear in error: {msg}");
    }
}
