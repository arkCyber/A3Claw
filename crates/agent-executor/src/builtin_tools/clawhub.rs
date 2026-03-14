//! `clawhub.*` — ClawHub skill marketplace client.
//!
//! Connects to the ClawHub skill registry at `https://clawhub.io/api/v1`
//! (or the `CLAWHUB_API_URL` env var override) to install, search, list,
//! update, and remove community skills.
//!
//! ## Skills
//! | skill              | description |
//! |--------------------|-------------|
//! | `clawhub.install`  | Install a skill from ClawHub by name |
//! | `clawhub.list`     | List locally installed ClawHub skills |
//! | `clawhub.search`   | Search ClawHub marketplace for skills |
//! | `clawhub.update`   | Update an installed skill to latest version |
//! | `clawhub.remove`   | Remove an installed skill |
//!
//! ## Design
//! - Skills are stored in `~/.openclaw/skills/<name>/` by default.
//! - The `CLAWHUB_SKILLS_DIR` env var overrides the install directory.
//! - All mutations (install/update/remove) are gated at Confirm risk level.
//! - Network errors return descriptive stub messages, never panic.

const CLAWHUB_BASE: &str = "https://clawhub.io/api/v1";
const SKILLS_DIR_DEFAULT: &str = ".openclaw/skills";

fn clawhub_base() -> String {
    std::env::var("CLAWHUB_API_URL").unwrap_or_else(|_| CLAWHUB_BASE.to_string())
}

fn skills_dir() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("CLAWHUB_SKILLS_DIR") {
        return std::path::PathBuf::from(d);
    }
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(SKILLS_DIR_DEFAULT)
}

// ── Public dispatch entry-point ───────────────────────────────────────────────

/// Dispatch a `clawhub.*` skill call.
/// Returns `Ok(String)` on success or graceful degradation, `Err(String)` only on
/// hard parameter validation failure.
pub async fn dispatch_clawhub(
    client: &reqwest::Client,
    skill_name: &str,
    args: &serde_json::Value,
) -> Result<String, String> {
    match skill_name {
        "clawhub.install"  => clawhub_install(client, args).await,
        "clawhub.list"     => clawhub_list().await,
        "clawhub.search"   => clawhub_search(client, args).await,
        "clawhub.update"   => clawhub_update(client, args).await,
        "clawhub.remove"   => clawhub_remove(args).await,
        other => Err(format!("unknown clawhub skill '{}'", other)),
    }
}

// ── clawhub.install ───────────────────────────────────────────────────────────

async fn clawhub_install(
    client: &reqwest::Client,
    args: &serde_json::Value,
) -> Result<String, String> {
    let name = args["name"]
        .as_str()
        .ok_or_else(|| "clawhub.install: missing 'name'".to_string())?
        .trim();

    if name.is_empty() {
        return Err("clawhub.install: 'name' must not be empty".into());
    }

    // Sanitize: only allow alphanumeric, hyphens, underscores, and slashes for scoped names
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/') {
        return Err(format!("clawhub.install: invalid skill name '{}'", name));
    }

    let version = args["version"].as_str().unwrap_or("latest");

    // Fetch skill manifest from ClawHub
    let url = format!("{}/skills/{}", clawhub_base(), name);
    let manifest: serde_json::Value = match client.get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "OpenClaw+/1.0")
        .send().await
    {
        Ok(r) if r.status().is_success() => {
            r.json().await.unwrap_or(serde_json::json!({}))
        }
        Ok(r) => {
            return Ok(format!(
                "(clawhub.install: skill '{}' not found on ClawHub (HTTP {}). \
                 Browse available skills: https://clawhub.io/skills)",
                name, r.status()
            ));
        }
        Err(e) => {
            return Ok(format!(
                "(clawhub.install: ClawHub unreachable — {}. \
                 Set CLAWHUB_API_URL to override the endpoint.)", e
            ));
        }
    };

    // Resolve download URL from manifest
    let resolved_version = if version == "latest" {
        manifest["latest_version"].as_str().unwrap_or("0.0.1")
    } else {
        version
    };

    let download_url = manifest["download_url"]
        .as_str()
        .unwrap_or_else(|| manifest["archive_url"].as_str().unwrap_or(""));

    if download_url.is_empty() {
        return Ok(format!(
            "(clawhub.install: no download URL in manifest for '{}@{}')",
            name, resolved_version
        ));
    }

    // Create skill install directory
    let install_dir = skills_dir().join(name.replace('/', "__"));
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("clawhub.install: cannot create dir: {}", e))?;

    // Download the skill archive
    let archive_bytes = match client.get(download_url)
        .header("User-Agent", "OpenClaw+/1.0")
        .send().await
    {
        Ok(r) if r.status().is_success() => {
            r.bytes().await.map_err(|e| format!("clawhub.install: download error: {}", e))?
        }
        Ok(r) => {
            return Ok(format!("(clawhub.install: download failed HTTP {})", r.status()));
        }
        Err(e) => {
            return Ok(format!("(clawhub.install: download unreachable — {})", e));
        }
    };

    // Write archive to disk
    let archive_path = install_dir.join(format!("{}.tar.gz", name.replace('/', "__")));
    std::fs::write(&archive_path, &archive_bytes)
        .map_err(|e| format!("clawhub.install: cannot write archive: {}", e))?;

    // Write metadata
    let meta = serde_json::json!({
        "name": name,
        "version": resolved_version,
        "installed_at": iso_now_secs(),
        "manifest": manifest,
    });
    std::fs::write(
        install_dir.join("clawhub.json"),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    ).map_err(|e| format!("clawhub.install: cannot write metadata: {}", e))?;

    Ok(format!(
        "Installed skill '{}@{}' to '{}'",
        name,
        resolved_version,
        install_dir.display()
    ))
}

// ── clawhub.list ──────────────────────────────────────────────────────────────

async fn clawhub_list() -> Result<String, String> {
    let dir = skills_dir();
    if !dir.exists() {
        return Ok("No ClawHub skills installed. Use clawhub.install to install skills.".into());
    }

    let mut skills: Vec<serde_json::Value> = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("clawhub.list: cannot read skills dir: {}", e))?;

    for entry in entries.flatten() {
        let meta_path = entry.path().join("clawhub.json");
        if meta_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&raw) {
                    skills.push(serde_json::json!({
                        "name": meta["name"],
                        "version": meta["version"],
                        "installed_at": meta["installed_at"],
                    }));
                }
            }
        }
    }

    if skills.is_empty() {
        return Ok("No ClawHub skills installed.".into());
    }

    let out = skills.iter()
        .map(|s| format!(
            "  {} @ {}",
            s["name"].as_str().unwrap_or("?"),
            s["version"].as_str().unwrap_or("?"),
        ))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!("Installed ClawHub skills ({}):\n{}", skills.len(), out))
}

// ── clawhub.search ────────────────────────────────────────────────────────────

async fn clawhub_search(
    client: &reqwest::Client,
    args: &serde_json::Value,
) -> Result<String, String> {
    let query = args["query"]
        .as_str()
        .ok_or_else(|| "clawhub.search: missing 'query'".to_string())?;

    let url = format!("{}/skills/search?q={}", clawhub_base(), urlenc(query));
    match client.get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "OpenClaw+/1.0")
        .send().await
    {
        Ok(r) if r.status().is_success() => {
            let body = r.text().await.unwrap_or_default();
            if let Ok(j) = serde_json::from_str::<serde_json::Value>(&body) {
                let results = j["skills"]
                    .as_array()
                    .or_else(|| j["results"].as_array())
                    .or_else(|| j.as_array())
                    .map(|arr| arr.iter().take(10).map(|item| {
                        format!("  {} — {} ({})",
                            item["name"].as_str().unwrap_or("?"),
                            item["description"].as_str().unwrap_or(""),
                            item["latest_version"].as_str()
                                .or_else(|| item["version"].as_str())
                                .unwrap_or("?"),
                        )
                    }).collect::<Vec<_>>().join("\n"))
                    .unwrap_or_else(|| body.chars().take(1000).collect());
                Ok(format!("ClawHub search results for '{}':\n{}", query, results))
            } else {
                Ok(body.chars().take(2000).collect())
            }
        }
        Ok(r) => Ok(format!("(clawhub.search: HTTP {})", r.status())),
        Err(e) => Ok(format!(
            "(clawhub.search: ClawHub unreachable — {}. \
             Browse manually: https://clawhub.io/skills?q={})",
            e, urlenc(query)
        )),
    }
}

// ── clawhub.update ────────────────────────────────────────────────────────────

async fn clawhub_update(
    client: &reqwest::Client,
    args: &serde_json::Value,
) -> Result<String, String> {
    let name_opt = args["name"].as_str();

    if let Some(name) = name_opt {
        // Update a single skill by reinstalling it
        let install_args = serde_json::json!({ "name": name, "version": "latest" });
        clawhub_install(client, &install_args).await
            .map(|msg| format!("Updated: {}", msg))
    } else {
        // Update all installed skills
        let dir = skills_dir();
        if !dir.exists() {
            return Ok("No ClawHub skills installed.".into());
        }

        let mut updated = Vec::new();
        let mut failed = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let meta_path = entry.path().join("clawhub.json");
                if let Ok(raw) = std::fs::read_to_string(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if let Some(n) = meta["name"].as_str() {
                            let install_args = serde_json::json!({ "name": n, "version": "latest" });
                            match clawhub_install(client, &install_args).await {
                                Ok(_) => updated.push(n.to_string()),
                                Err(e) => failed.push(format!("{}: {}", n, e)),
                            }
                        }
                    }
                }
            }
        }

        let mut out = format!("Updated {} skill(s).", updated.len());
        if !failed.is_empty() {
            out.push_str(&format!("\nFailed: {}", failed.join(", ")));
        }
        Ok(out)
    }
}

// ── clawhub.remove ────────────────────────────────────────────────────────────

async fn clawhub_remove(args: &serde_json::Value) -> Result<String, String> {
    let name = args["name"]
        .as_str()
        .ok_or_else(|| "clawhub.remove: missing 'name'".to_string())?;

    let install_dir = skills_dir().join(name.replace('/', "__"));
    if !install_dir.exists() {
        return Ok(format!("(clawhub.remove: skill '{}' is not installed)", name));
    }

    std::fs::remove_dir_all(&install_dir)
        .map_err(|e| format!("clawhub.remove: cannot remove '{}': {}", install_dir.display(), e))?;

    Ok(format!("Removed skill '{}' from '{}'", name, install_dir.display()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn urlenc(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() || "-_.~".contains(c) {
                vec![c]
            } else if c == ' ' {
                vec!['+']
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect()
}

fn iso_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn clawhub_install_missing_name_errors() {
        let err = dispatch_clawhub(&client(), "clawhub.install", &serde_json::json!({}))
            .await;
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("missing 'name'"));
    }

    #[tokio::test]
    async fn clawhub_install_invalid_name_errors() {
        let err = dispatch_clawhub(
            &client(),
            "clawhub.install",
            &serde_json::json!({"name": "../../etc/passwd"}),
        ).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("invalid skill name"));
    }

    #[tokio::test]
    async fn clawhub_install_unreachable_returns_stub() {
        // Uses CLAWHUB_API_URL override pointing to unreachable port
        std::env::set_var("CLAWHUB_API_URL", "http://127.0.0.1:1");
        let result = dispatch_clawhub(
            &client(),
            "clawhub.install",
            &serde_json::json!({"name": "test-skill"}),
        ).await;
        // When unreachable, returns Ok(stub) not Err
        match result {
            Ok(s) => assert!(s.contains("ClawHub unreachable") || s.contains("not found")),
            Err(e) => assert!(e.contains("missing") || e.contains("invalid")),
        }
    }

    #[tokio::test]
    async fn clawhub_list_empty_dir_returns_message() {
        std::env::set_var("CLAWHUB_SKILLS_DIR", "/tmp/clawhub_test_empty_xyz987");
        let result = dispatch_clawhub(&client(), "clawhub.list", &serde_json::json!({})).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No ClawHub"));
    }

    #[tokio::test]
    async fn clawhub_search_missing_query_errors() {
        let err = dispatch_clawhub(&client(), "clawhub.search", &serde_json::json!({})).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("missing 'query'"));
    }

    #[tokio::test]
    async fn clawhub_search_unreachable_returns_stub() {
        std::env::set_var("CLAWHUB_API_URL", "http://127.0.0.1:1");
        let result = dispatch_clawhub(
            &client(),
            "clawhub.search",
            &serde_json::json!({"query": "github"}),
        ).await;
        assert!(result.is_ok());
        let s = result.unwrap_or_default();
        assert!(s.contains("ClawHub unreachable") || s.contains("clawhub.search"));
    }

    #[tokio::test]
    async fn clawhub_remove_not_installed_returns_stub() {
        std::env::set_var("CLAWHUB_SKILLS_DIR", "/tmp/clawhub_test_empty_xyz987");
        let result = dispatch_clawhub(
            &client(),
            "clawhub.remove",
            &serde_json::json!({"name": "nonexistent-skill"}),
        ).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("not installed"));
    }

    #[tokio::test]
    async fn clawhub_remove_missing_name_errors() {
        let err = dispatch_clawhub(&client(), "clawhub.remove", &serde_json::json!({})).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("missing 'name'"));
    }

    #[tokio::test]
    async fn clawhub_unknown_skill_errors() {
        let err = dispatch_clawhub(&client(), "clawhub.unknown", &serde_json::json!({})).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("unknown clawhub skill"));
    }

    #[test]
    fn urlenc_spaces() {
        assert_eq!(urlenc("hello world"), "hello+world");
    }

    #[test]
    fn urlenc_alphanumeric_unchanged() {
        assert_eq!(urlenc("github123"), "github123");
    }
}
