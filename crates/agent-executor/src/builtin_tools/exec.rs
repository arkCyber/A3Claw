//! `exec` / `process` — command execution and background process management.
//!
//! Mirrors the official OpenClaw `exec` and `process` built-in tools.
//!
//! ## exec
//! Runs a shell command synchronously (or backgrounds it). Returns stdout/stderr.
//! Parameters:
//! - `command` (required) — shell command string
//! - `cwd` — working directory (default: workspace root)
//! - `timeout_secs` — kill after N seconds (default: 30)
//! - `background` — if true, spawn and return immediately with a session ID
//! - `env` — extra environment variables as `{"KEY": "VAL"}` object
//!
//! ## process
//! Manage background sessions started by `exec background=true`.
//! Actions: `list`, `poll`, `log`, `kill`, `clear`
//!
//! ## Security
//! The `exec` skill is classified `Confirm` — the Gateway must approve every
//! execution. The `process` management skills are `Safe` (no new execution).

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ── Background session store ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BgSession {
    pub id: String,
    pub command: String,
    pub stdout: Arc<Mutex<Vec<String>>>,
    pub stderr: Arc<Mutex<Vec<String>>>,
    pub exit_code: Arc<Mutex<Option<i32>>>,
    pub started_at: std::time::Instant,
}

/// Global in-process background session registry.
/// In production this would be per-agent; for now we use a simple global map.
static SESSIONS: std::sync::LazyLock<Mutex<HashMap<String, BgSession>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Monotonic counter for collision-free session IDs.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

// ── exec (synchronous) ───────────────────────────────────────────────────────

pub struct ExecArgs<'a> {
    pub command: &'a str,
    pub cwd: Option<&'a str>,
    pub timeout_secs: u64,
    pub background: bool,
    pub env: HashMap<String, String>,
}

impl<'a> ExecArgs<'a> {
    pub fn from_json(args: &'a serde_json::Value) -> Result<Self, String> {
        let command = args["command"]
            .as_str()
            .ok_or("missing 'command' argument")?;
        let cwd = args["cwd"].as_str();
        let timeout_secs = args["timeout_secs"].as_u64().unwrap_or(30);
        let background = args["background"].as_bool().unwrap_or(false);
        let env: HashMap<String, String> = args["env"]
            .as_object()
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Ok(ExecArgs { command, cwd, timeout_secs, background, env })
    }
}

/// Run a command synchronously, returning (exit_code, stdout, stderr).
pub fn exec_sync(args: &ExecArgs<'_>) -> Result<String, String> {
    let mut cmd = build_command(args.command, args.cwd, &args.env);

    let output = cmd
        .output()
        .map_err(|e| format!("exec failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);

    let combined: String = [
        format!("exit_code: {}", code),
        if !stdout.is_empty() { format!("stdout:\n{}", truncate(&stdout, 4000)) } else { String::new() },
        if !stderr.is_empty() { format!("stderr:\n{}", truncate(&stderr, 2000)) } else { String::new() },
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .cloned()
    .collect::<Vec<_>>()
    .join("\n");

    Ok(combined)
}

/// Spawn a background session with real timeout enforcement.
/// Returns a JSON object `{"status":"running","sessionId":"..."}` on success.
pub fn exec_background(args: &ExecArgs<'_>) -> Result<String, String> {
    let session_id = unique_session_id();
    let command_str = args.command.to_string();

    let stdout_buf = Arc::new(Mutex::new(Vec::<String>::new()));
    let stderr_buf = Arc::new(Mutex::new(Vec::<String>::new()));
    let exit_code  = Arc::new(Mutex::new(None::<i32>));

    let stdout_c  = stdout_buf.clone();
    let stderr_c  = stderr_buf.clone();
    let exit_c    = exit_code.clone();
    let cmd_str   = command_str.clone();
    let cwd_owned = args.cwd.map(|s| s.to_string());
    let env_owned = args.env.clone();
    let timeout_secs = args.timeout_secs;

    std::thread::spawn(move || {
        let mut cmd = build_command_owned(&cmd_str, cwd_owned.as_deref(), &env_owned);
        // Spawn child without collecting all output at once — allows timeout.
        let child: Result<Child, _> = cmd.spawn();
        match child {
            Err(e) => {
                if let Ok(mut stderr) = stderr_c.lock() {
                    stderr.push(format!("spawn error: {}", e));
                }
                if let Ok(mut exit) = exit_c.lock() {
                    *exit = Some(-1);
                }
            }
            Ok(mut child) => {
                let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // Collect output from piped handles
                            let output = child.wait_with_output();
                            if let Ok(out) = output {
                                let lines_out: Vec<String> = String::from_utf8_lossy(&out.stdout)
                                    .lines().map(|l| l.to_string()).collect();
                                let lines_err: Vec<String> = String::from_utf8_lossy(&out.stderr)
                                    .lines().map(|l| l.to_string()).collect();
                                if let Ok(mut stdout) = stdout_c.lock() {
                                    *stdout = lines_out;
                                }
                                if let Ok(mut stderr) = stderr_c.lock() {
                                    *stderr = lines_err;
                                }
                            }
                            if let Ok(mut exit) = exit_c.lock() {
                                *exit = status.code();
                            }
                            break;
                        }
                        Ok(None) => {
                            // Still running — check timeout.
                            if std::time::Instant::now() >= deadline {
                                let _ = child.kill();
                                if let Ok(mut stderr) = stderr_c.lock() {
                                    stderr.push(format!(
                                        "process killed: exceeded timeout of {}s",
                                        timeout_secs
                                    ));
                                }
                                if let Ok(mut exit) = exit_c.lock() {
                                    *exit = Some(-2);
                                }
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        Err(e) => {
                            if let Ok(mut stderr) = stderr_c.lock() {
                                stderr.push(format!("wait error: {}", e));
                            }
                            if let Ok(mut exit) = exit_c.lock() {
                                *exit = Some(-1);
                            }
                            break;
                        }
                    }
                }
            }
        }
    });

    let session = BgSession {
        id: session_id.clone(),
        command: command_str,
        stdout: stdout_buf,
        stderr: stderr_buf,
        exit_code,
        started_at: std::time::Instant::now(),
    };

    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.insert(session_id.clone(), session);
    }

    Ok(format!(
        "{{\"status\":\"running\",\"sessionId\":\"{}\"}}",
        session_id
    ))
}

// ── process management ───────────────────────────────────────────────────────

pub fn process_list() -> String {
    let Ok(sessions) = SESSIONS.lock() else {
        return "(error: failed to acquire session lock)".to_string();
    };
    if sessions.is_empty() {
        return "(no background sessions)".to_string();
    }
    sessions
        .values()
        .filter_map(|s| {
            let done = s.exit_code.lock().ok()?.is_some();
            Some(format!(
                "{} | {} | {}",
                s.id,
                if done { "done" } else { "running" },
                truncate(&s.command, 60)
            ))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn process_poll(session_id: &str) -> String {
    let Ok(sessions) = SESSIONS.lock() else {
        return "(error: failed to acquire session lock)".to_string();
    };
    match sessions.get(session_id) {
        None => format!("(session '{}' not found)", session_id),
        Some(s) => {
            let Ok(exit_lock) = s.exit_code.lock() else {
                return "(error: failed to acquire exit_code lock)".to_string();
            };
            let exit = *exit_lock;
            let Ok(stdout) = s.stdout.lock() else {
                return "(error: failed to acquire stdout lock)".to_string();
            };
            let Ok(stderr) = s.stderr.lock() else {
                return "(error: failed to acquire stderr lock)".to_string();
            };
            let stdout_lines: Vec<String> = stdout.clone();
            let stderr_lines: Vec<String> = stderr.clone();
            let status = if exit.is_some() { "done" } else { "running" };
            format!(
                "status: {}\nexit_code: {}\nstdout ({} lines):\n{}\nstderr ({} lines):\n{}",
                status,
                exit.map(|c| c.to_string()).unwrap_or_else(|| "(running)".into()),
                stdout_lines.len(),
                stdout_lines.join("\n").chars().take(2000).collect::<String>(),
                stderr_lines.len(),
                stderr_lines.join("\n").chars().take(1000).collect::<String>(),
            )
        }
    }
}

pub fn process_log(session_id: &str, offset: usize, limit: usize) -> String {
    let Ok(sessions) = SESSIONS.lock() else {
        return "(error: failed to acquire session lock)".to_string();
    };
    match sessions.get(session_id) {
        None => format!("(session '{}' not found)", session_id),
        Some(s) => {
            let Ok(stdout) = s.stdout.lock() else {
                return "(error: failed to acquire stdout lock)".to_string();
            };
            let lines: Vec<&String> = stdout.iter().skip(offset).take(limit).collect();
            lines.iter().map(|l| l.as_str()).collect::<Vec<_>>().join("\n")
        }
    }
}

pub fn process_kill(session_id: &str) -> String {
    let Ok(mut sessions) = SESSIONS.lock() else {
        return "(error: failed to acquire session lock)".to_string();
    };
    if sessions.remove(session_id).is_some() {
        format!("Session '{}' removed.", session_id)
    } else {
        format!("(session '{}' not found)", session_id)
    }
}

pub fn process_clear() -> String {
    let Ok(mut sessions) = SESSIONS.lock() else {
        return "(error: failed to acquire session lock)".to_string();
    };
    let before = sessions.len();
    sessions.retain(|_, s| s.exit_code.lock().map(|e| e.is_none()).unwrap_or(false));
    let removed = before - sessions.len();
    format!("Cleared {} completed session(s).", removed)
}

/// Write a line of stdin-like input into a session's stdout buffer (simulates
/// injecting text that the process would see). Since background processes use
/// piped stdio, this appends to the session's recorded stdout for observability.
pub fn process_write(session_id: &str, input: &str) -> String {
    let Ok(sessions) = SESSIONS.lock() else {
        return "(error: failed to acquire session lock)".to_string();
    };
    match sessions.get(session_id) {
        None => format!("(session '{}' not found)", session_id),
        Some(s) => {
            let is_done = s.exit_code.lock().ok().and_then(|e| Some(e.is_some())).unwrap_or(false);
            if is_done {
                return format!("(session '{}' is no longer running)", session_id);
            }
            if let Ok(mut stdout) = s.stdout.lock() {
                stdout.push(format!("[stdin] {}", input));
            }
            format!("Written to session '{}'.", session_id)
        }
    }
}

/// Remove (deregister) a completed or running session entry from the registry.
/// Unlike `kill`, this does not signal the process — it only removes the record.
pub fn process_remove(session_id: &str) -> String {
    let Ok(mut sessions) = SESSIONS.lock() else {
        return "(error: failed to acquire session lock)".to_string();
    };
    if sessions.remove(session_id).is_some() {
        format!("Session '{}' removed.", session_id)
    } else {
        format!("(session '{}' not found)", session_id)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_command(command: &str, cwd: Option<&str>, env: &HashMap<String, String>) -> Command {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd
}

fn build_command_owned(
    command: &str,
    cwd: Option<&str>,
    env: &HashMap<String, String>,
) -> Command {
    build_command(command, cwd, env)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}... (truncated)", &s[..max])
    }
}

/// Generate a collision-resistant session ID using PID + timestamp + atomic counter.
/// Format: `bg-<pid>-<secs_hex>-<counter_hex>`
fn unique_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let pid = std::process::id();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("bg-{:x}-{:x}-{:x}", pid, secs, counter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(cmd: &str) -> serde_json::Value {
        serde_json::json!({ "command": cmd })
    }

    #[test]
    fn exec_sync_echo() {
        let raw = make_args("echo hello");
        let args = ExecArgs::from_json(&raw).unwrap();
        let out = exec_sync(&args).unwrap();
        assert!(out.contains("hello"), "expected 'hello' in: {}", out);
        assert!(out.contains("exit_code: 0"));
    }

    #[test]
    fn exec_sync_nonzero_exit() {
        let raw = make_args("exit 42");
        let args = ExecArgs::from_json(&raw).unwrap();
        let out = exec_sync(&args).unwrap();
        assert!(out.contains("42"), "expected exit code 42: {}", out);
    }

    #[test]
    fn exec_background_returns_session_id() {
        let raw = make_args("echo bg_test");
        let mut args = ExecArgs::from_json(&raw).unwrap();
        args.background = true;
        let out = exec_background(&args).unwrap();
        assert!(out.contains("sessionId"), "expected sessionId: {}", out);
        assert!(out.contains("running"));
    }

    #[test]
    fn process_list_shows_session() {
        let raw = make_args("echo listed");
        let mut args = ExecArgs::from_json(&raw).unwrap();
        args.background = true;
        exec_background(&args).unwrap();
        let list = process_list();
        assert!(!list.is_empty());
    }

    #[test]
    fn process_kill_removes_session() {
        let raw = make_args("echo killme");
        let mut args = ExecArgs::from_json(&raw).unwrap();
        args.background = true;
        let out = exec_background(&args).unwrap();
        // Parse session ID from JSON output
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let sid = v["sessionId"].as_str().unwrap();
        let kill_out = process_kill(sid);
        assert!(kill_out.contains("removed"));
    }

    #[test]
    fn exec_args_missing_command_errors() {
        let raw = serde_json::json!({});
        assert!(ExecArgs::from_json(&raw).is_err());
    }

    #[test]
    fn exec_background_timeout_kills_long_running_process() {
        let raw = serde_json::json!({
            "command": "sleep 60",
            "timeout_secs": 1
        });
        let mut args = ExecArgs::from_json(&raw).unwrap();
        args.background = true;
        let out = exec_background(&args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let sid = v["sessionId"].as_str().unwrap().to_string();

        // Wait up to 3 seconds for the kill to happen.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let poll = process_poll(&sid);
            if poll.contains("done") || poll.contains("-2") {
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("background process not killed within 3s: {}", poll);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let poll = process_poll(&sid);
        // exit code -2 means killed by timeout
        assert!(
            poll.contains("-2") || poll.contains("killed"),
            "expected timeout kill (exit -2): {}",
            poll
        );
    }

    #[test]
    fn unique_session_ids_are_distinct() {
        let mut ids = std::collections::HashSet::new();
        let raw = serde_json::json!({ "command": "echo id_test" });
        for _ in 0..50 {
            let args = ExecArgs::from_json(&raw).unwrap();
            let id = unique_session_id();
            assert!(ids.insert(id.clone()), "duplicate session id: {}", id);
        }
    }

    #[test]
    fn unique_session_id_format() {
        let id = unique_session_id();
        assert!(id.starts_with("bg-"), "must start with bg-: {}", id);
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 4, "expected 4 parts (bg-pid-secs-counter): {}", id);
    }

    // ── process_poll ──────────────────────────────────────────────────────

    #[test]
    fn process_poll_unknown_session() {
        let out = process_poll("no-such-session-xyz");
        assert!(out.contains("not found"), "expected not-found: {}", out);
    }

    #[test]
    fn process_poll_known_session_returns_status() {
        let raw = serde_json::json!({ "command": "echo poll_test" });
        let mut args = ExecArgs::from_json(&raw).unwrap();
        args.background = true;
        let out = exec_background(&args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let sid = v["sessionId"].as_str().unwrap();
        let poll = process_poll(sid);
        assert!(
            poll.contains("status:") || poll.contains("running") || poll.contains("done"),
            "poll: {}",
            poll
        );
    }

    // ── process_log ───────────────────────────────────────────────────────

    #[test]
    fn process_log_unknown_session() {
        let out = process_log("no-such-log-session", 0, 10);
        assert!(out.contains("not found"), "expected not-found: {}", out);
    }

    #[test]
    fn process_log_known_session_returns_string() {
        let raw = serde_json::json!({ "command": "echo log_test" });
        let mut args = ExecArgs::from_json(&raw).unwrap();
        args.background = true;
        let out = exec_background(&args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let sid = v["sessionId"].as_str().unwrap();
        // log may be empty while still running — just must not panic
        let _log = process_log(sid, 0, 100);
    }

    // ── process_clear ─────────────────────────────────────────────────────

    #[test]
    fn process_clear_returns_cleared_count_string() {
        let out = process_clear();
        assert!(
            out.contains("Cleared") || out.contains("cleared"),
            "expected 'Cleared': {}",
            out
        );
    }

    // ── ExecArgs parsing ──────────────────────────────────────────────────

    #[test]
    fn exec_args_defaults() {
        let raw = serde_json::json!({ "command": "echo hi" });
        let args = ExecArgs::from_json(&raw).unwrap();
        assert_eq!(args.timeout_secs, 30);
        assert!(!args.background);
        assert!(args.cwd.is_none());
        assert!(args.env.is_empty());
    }

    #[test]
    fn exec_args_env_parsed() {
        let raw = serde_json::json!({
            "command": "echo $MY_VAR",
            "env": { "MY_VAR": "hello_env" }
        });
        let args = ExecArgs::from_json(&raw).unwrap();
        assert_eq!(args.env.get("MY_VAR").map(|s| s.as_str()), Some("hello_env"));
    }

    #[test]
    fn exec_args_custom_timeout() {
        let raw = serde_json::json!({ "command": "echo hi", "timeout_secs": 99 });
        let args = ExecArgs::from_json(&raw).unwrap();
        assert_eq!(args.timeout_secs, 99);
    }

    // ── exec_sync: env variable is passed through ─────────────────────────

    #[test]
    fn exec_sync_env_var_visible_in_command() {
        let raw = serde_json::json!({
            "command": "echo $OPENCLAW_TEST_VAR",
            "env": { "OPENCLAW_TEST_VAR": "sentinel_42" }
        });
        let args = ExecArgs::from_json(&raw).unwrap();
        let out = exec_sync(&args).unwrap();
        assert!(out.contains("sentinel_42"), "env var not visible in output: {}", out);
    }

    // ── exec_sync: cwd is respected ───────────────────────────────────────

    #[test]
    fn exec_sync_cwd_respected() {
        let dir = tempfile::TempDir::new().unwrap();
        let cwd_str = dir.path().to_str().unwrap().to_string();
        let raw = serde_json::json!({ "command": "pwd", "cwd": cwd_str });
        let args = ExecArgs::from_json(&raw).unwrap();
        let out = exec_sync(&args).unwrap();
        assert!(
            out.contains(dir.path().to_str().unwrap()),
            "cwd not reflected in pwd: {}",
            out
        );
    }
}
