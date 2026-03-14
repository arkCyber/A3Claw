//! `cron` — scheduled job management tool.
//!
//! Mirrors the official OpenClaw `cron` built-in tool.
//! Delegates to the Gateway `/cron` endpoint for persistent scheduling.
//! When the gateway is unavailable, maintains an in-process job registry
//! for testing and development.
//!
//! Actions: `status`, `list`, `add`, `update`, `remove`, `run`, `runs`

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Monotonic counter for collision-free cron job IDs.
static CRON_COUNTER: AtomicU64 = AtomicU64::new(1);

// ── In-process fallback registry ─────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub goal: String,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub run_count: u64,
}

static JOBS: std::sync::LazyLock<Mutex<HashMap<String, CronJob>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

// ── Gateway delegation ────────────────────────────────────────────────────────

pub async fn dispatch_cron(
    client: &reqwest::Client,
    gateway_url: &str,
    action: &str,
    args: &serde_json::Value,
) -> Result<String, String> {
    // Try gateway first
    let url = format!("{}/cron/{}", gateway_url, action);
    match client.post(&url).json(args).send().await {
        Ok(resp) if resp.status().is_success() => {
            return resp.text().await.map_err(|e| e.to_string());
        }
        Ok(resp) if resp.status().as_u16() == 404 => {
            // endpoint not found — gateway may not support cron
        }
        _ => {
            // Gateway unreachable — fall through to local fallback
        }
    }

    // Local fallback
    local_dispatch(action, args)
}

fn local_dispatch(action: &str, args: &serde_json::Value) -> Result<String, String> {
    match action {
        "status" => {
            let jobs = JOBS.lock().unwrap();
            Ok(format!(
                "{{\"status\":\"ok\",\"jobCount\":{}}}",
                jobs.len()
            ))
        }
        "list" => {
            let jobs = JOBS.lock().unwrap();
            let list: Vec<&CronJob> = jobs.values().collect();
            serde_json::to_string(&list).map_err(|e| e.to_string())
        }
        "add" => {
            let id = short_id();
            let job = CronJob {
                id: id.clone(),
                name: args["name"].as_str().unwrap_or("unnamed").to_string(),
                schedule: args["schedule"].as_str().unwrap_or("").to_string(),
                goal: args["goal"].as_str().unwrap_or("").to_string(),
                enabled: args["enabled"].as_bool().unwrap_or(true),
                last_run: None,
                run_count: 0,
            };
            if job.schedule.is_empty() {
                return Err("missing 'schedule' (cron expression)".to_string());
            }
            if job.goal.is_empty() {
                return Err("missing 'goal'".to_string());
            }
            JOBS.lock().unwrap().insert(id.clone(), job);
            Ok(format!("{{\"id\":\"{}\",\"status\":\"created\"}}", id))
        }
        "update" => {
            let job_id = args["jobId"]
                .as_str()
                .or_else(|| args["id"].as_str())
                .ok_or("missing 'jobId'")?;
            let mut jobs = JOBS.lock().unwrap();
            match jobs.get_mut(job_id) {
                None => Err(format!("job '{}' not found", job_id)),
                Some(job) => {
                    if let Some(name) = args["patch"]["name"].as_str() { job.name = name.to_string(); }
                    if let Some(sched) = args["patch"]["schedule"].as_str() { job.schedule = sched.to_string(); }
                    if let Some(goal) = args["patch"]["goal"].as_str() { job.goal = goal.to_string(); }
                    if let Some(en) = args["patch"]["enabled"].as_bool() { job.enabled = en; }
                    Ok(format!("{{\"id\":\"{}\",\"status\":\"updated\"}}", job_id))
                }
            }
        }
        "remove" => {
            let job_id = args["jobId"]
                .as_str()
                .or_else(|| args["id"].as_str())
                .ok_or("missing 'jobId'")?;
            let removed = JOBS.lock().unwrap().remove(job_id).is_some();
            if removed {
                Ok(format!("Job '{}' removed.", job_id))
            } else {
                Err(format!("job '{}' not found", job_id))
            }
        }
        "run" => {
            let job_id = args["jobId"]
                .as_str()
                .or_else(|| args["id"].as_str())
                .ok_or("missing 'jobId'")?;
            let mut jobs = JOBS.lock().unwrap();
            match jobs.get_mut(job_id) {
                None => Err(format!("job '{}' not found", job_id)),
                Some(job) => {
                    job.run_count += 1;
                    job.last_run = Some(now_iso());
                    Ok(format!(
                        "Job '{}' triggered manually (run #{}).",
                        job_id, job.run_count
                    ))
                }
            }
        }
        "runs" => {
            let job_id = args["jobId"]
                .as_str()
                .or_else(|| args["id"].as_str())
                .ok_or("missing 'jobId'")?;
            let jobs = JOBS.lock().unwrap();
            match jobs.get(job_id) {
                None => Err(format!("job '{}' not found", job_id)),
                Some(job) => Ok(format!(
                    "{{\"jobId\":\"{}\",\"runCount\":{},\"lastRun\":{}}}",
                    job_id,
                    job.run_count,
                    job.last_run
                        .as_deref()
                        .map(|s| format!("\"{}\"", s))
                        .unwrap_or_else(|| "null".to_string())
                )),
            }
        }
        "enable" => {
            let job_id = args["jobId"]
                .as_str()
                .or_else(|| args["id"].as_str())
                .ok_or("missing 'jobId'")?;
            let mut jobs = JOBS.lock().unwrap();
            match jobs.get_mut(job_id) {
                None => Err(format!("job '{}' not found", job_id)),
                Some(job) => {
                    job.enabled = true;
                    Ok(format!("Job '{}' enabled.", job_id))
                }
            }
        }
        "disable" => {
            let job_id = args["jobId"]
                .as_str()
                .or_else(|| args["id"].as_str())
                .ok_or("missing 'jobId'")?;
            let mut jobs = JOBS.lock().unwrap();
            match jobs.get_mut(job_id) {
                None => Err(format!("job '{}' not found", job_id)),
                Some(job) => {
                    job.enabled = false;
                    Ok(format!("Job '{}' disabled.", job_id))
                }
            }
        }
        _ => Err(format!(
            "Unknown cron action '{}'. Valid: status, list, add, update, remove, run, runs, enable, disable",
            action
        )),
    }
}

/// Generate a collision-resistant cron job ID.
/// Format: `cron-<pid_hex>-<secs_hex>-<counter_hex>`
fn short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let pid = std::process::id();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let counter = CRON_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("cron-{:x}-{:x}-{:x}", pid, secs, counter)
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}Z", secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    // Serialise all cron tests to prevent global JOBS state races.
    static TEST_LOCK: std::sync::LazyLock<StdMutex<()>> =
        std::sync::LazyLock::new(|| StdMutex::new(()));

    fn reset() {
        JOBS.lock().unwrap().clear();
    }

    #[test]
    fn cron_add_and_list() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let add_args = serde_json::json!({
            "name": "daily-report",
            "schedule": "0 9 * * *",
            "goal": "Generate daily sales report"
        });
        let result = local_dispatch("add", &add_args).unwrap();
        assert!(result.contains("created"), "add result: {}", result);

        let list = local_dispatch("list", &serde_json::json!({})).unwrap();
        assert!(list.contains("daily-report"), "list: {}", list);
    }

    #[test]
    fn cron_add_missing_schedule_errors() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let args = serde_json::json!({"name": "bad", "goal": "something"});
        assert!(local_dispatch("add", &args).is_err());
    }

    #[test]
    fn cron_add_missing_goal_errors() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let args = serde_json::json!({"name": "bad", "schedule": "* * * * *"});
        assert!(local_dispatch("add", &args).is_err());
    }

    #[test]
    fn cron_remove() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let add_args = serde_json::json!({
            "name": "temp", "schedule": "* * * * *", "goal": "test"
        });
        let added = local_dispatch("add", &add_args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&added).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        let removed = local_dispatch("remove", &serde_json::json!({"jobId": id})).unwrap();
        assert!(removed.contains("removed"), "{}", removed);

        let list = local_dispatch("list", &serde_json::json!({})).unwrap();
        assert!(!list.contains(id.as_str()), "job should be gone");
    }

    #[test]
    fn cron_run_increments_count() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let add_args = serde_json::json!({
            "name": "counter", "schedule": "* * * * *", "goal": "count"
        });
        let added = local_dispatch("add", &add_args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&added).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        local_dispatch("run", &serde_json::json!({"jobId": id})).unwrap();
        local_dispatch("run", &serde_json::json!({"jobId": id})).unwrap();

        let runs = local_dispatch("runs", &serde_json::json!({"jobId": id})).unwrap();
        assert!(runs.contains("\"runCount\":2"), "runs: {}", runs);
    }

    #[test]
    fn cron_status_returns_count() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let s = local_dispatch("status", &serde_json::json!({})).unwrap();
        assert!(s.contains("status"), "{}", s);
    }

    #[test]
    fn cron_unknown_action_errors() {
        assert!(local_dispatch("foobar", &serde_json::json!({})).is_err());
    }

    #[test]
    fn cron_update_name_and_schedule() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let add_args = serde_json::json!({
            "name": "orig", "schedule": "* * * * *", "goal": "do thing"
        });
        let added = local_dispatch("add", &add_args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&added).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        let update_args = serde_json::json!({
            "jobId": id,
            "patch": { "name": "updated-name", "schedule": "0 6 * * *" }
        });
        let result = local_dispatch("update", &update_args).unwrap();
        assert!(result.contains("updated"), "update result: {}", result);

        let list = local_dispatch("list", &serde_json::json!({})).unwrap();
        assert!(list.contains("updated-name"), "updated name in list: {}", list);
        assert!(list.contains("0 6 * * *"), "updated schedule in list: {}", list);
    }

    #[test]
    fn cron_update_missing_job_errors() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let result = local_dispatch(
            "update",
            &serde_json::json!({ "jobId": "nonexistent", "patch": { "name": "x" } }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn cron_remove_nonexistent_errors() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let result = local_dispatch("remove", &serde_json::json!({ "jobId": "no-such" }));
        assert!(result.is_err());
    }

    #[test]
    fn cron_run_nonexistent_job_errors() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let result = local_dispatch("run", &serde_json::json!({ "jobId": "ghost" }));
        assert!(result.is_err());
    }

    #[test]
    fn cron_runs_nonexistent_job_errors() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let result = local_dispatch("runs", &serde_json::json!({ "jobId": "ghost" }));
        assert!(result.is_err());
    }

    #[test]
    fn short_id_is_unique_under_rapid_calls() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..50 {
            let id = short_id();
            assert!(ids.insert(id.clone()), "duplicate short_id: {}", id);
        }
    }

    #[test]
    fn short_id_format_starts_with_cron() {
        let id = short_id();
        assert!(id.starts_with("cron-"), "expected cron- prefix: {}", id);
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 4, "expected 4 parts (cron-pid-secs-counter): {}", id);
    }

    #[test]
    fn cron_update_enabled_false() {
        let _g = TEST_LOCK.lock().unwrap();
        reset();
        let add_args = serde_json::json!({
            "name": "en-test", "schedule": "* * * * *", "goal": "test"
        });
        let added = local_dispatch("add", &add_args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&added).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        let update_args = serde_json::json!({ "jobId": id, "patch": { "enabled": false } });
        local_dispatch("update", &update_args).unwrap();

        let list = local_dispatch("list", &serde_json::json!({})).unwrap();
        assert!(list.contains("false"), "enabled should be false: {}", list);
    }
}
