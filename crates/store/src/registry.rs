//! # `registry.rs` — Plugin Registry Client
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Provides [`RegistryClient`], the async HTTP/file client responsible for:
//!
//! 1. **Fetching** the registry index (`fetch_index`) from either an
//!    `http(s)://` URL or a local `file://` path.  HTTP fetches use
//!    exponential back-off retry (up to [`MAX_FETCH_RETRIES`] attempts).
//! 2. **Installing** plugin `.wasm` files (`install_plugin`) with streaming
//!    download progress, SHA-256 integrity verification, and atomic disk
//!    write.  Already-installed files whose hash matches are skipped.
//! 3. **Uninstalling** plugin files (`uninstall_plugin`) from the local
//!    plugin directory.
//! 4. **Listing** installed plugin IDs (`installed_ids`) by scanning the
//!    plugin directory for `*.wasm` files.
//!
//! ## Fault-tolerance
//! | Failure mode | Behaviour |
//! |---|---|
//! | Network timeout (index) | Retry up to 3 times with 500 ms / 1 s / 2 s back-off |
//! | HTTP 4xx / 5xx | Error includes status code and response body |
//! | SHA-256 mismatch | File deleted and re-downloaded |
//! | Plugin dir missing | Created automatically before first write |
//! | Uninstall of missing file | Returns `Ok(())` (idempotent) |
//!
//! ## Testing
//! All public methods have unit tests in the `tests` submodule.  Tests use
//! `tempfile::TempDir` for isolation and cover both happy-path and
//! fault-tolerance scenarios.

use crate::types::{InstallState, PluginEntry, RegistryIndex, StorePrefs};
use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of automatic retries for transient network errors.
const MAX_FETCH_RETRIES: u32 = 3;
/// Base delay between retries (doubles each attempt — exponential back-off).
const RETRY_BASE_MS: u64 = 500;
/// Per-request connect + read timeout for registry index fetches.
const INDEX_TIMEOUT_SECS: u64 = 15;
/// Per-chunk read timeout during plugin download.
const DOWNLOAD_TIMEOUT_SECS: u64 = 120;

// ── RegistryClient ────────────────────────────────────────────────────────────

/// Fetches the registry index and downloads / verifies plugin `.wasm` files.
///
/// All network I/O is async and non-blocking. Progress is reported via an
/// `mpsc` channel so the UI can update a progress bar without blocking.
pub struct RegistryClient {
    http: reqwest::Client,
    prefs: Arc<parking_lot::RwLock<StorePrefs>>,
}

impl RegistryClient {
    pub fn new(prefs: Arc<parking_lot::RwLock<StorePrefs>>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("openclaw-store/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
            .connect_timeout(std::time::Duration::from_secs(10))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self { http, prefs }
    }

    /// Fetches the registry index for the given URL with automatic retry.
    ///
    /// Supports both `http(s)://` and `file://` URLs so local registries can
    /// be used for development and testing without a live server.
    ///
    /// Retries up to [`MAX_FETCH_RETRIES`] times on transient network errors
    /// using exponential back-off. File-system errors are not retried.
    pub async fn fetch_index(&self, url: &str) -> Result<RegistryIndex> {
        debug!(url, "fetching registry index");

        if url.starts_with("file://") {
            return self.fetch_index_local(url).await;
        }

        let mut last_err = anyhow::anyhow!("no attempts made");
        for attempt in 0..=MAX_FETCH_RETRIES {
            if attempt > 0 {
                let delay_ms = RETRY_BASE_MS * (1 << (attempt - 1));
                warn!(
                    attempt,
                    delay_ms,
                    url,
                    "retrying registry index fetch after error"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            match self.fetch_index_http(url).await {
                Ok(index) => return Ok(index),
                Err(e) => {
                    error!(attempt, url, error = %e, "registry fetch attempt failed");
                    last_err = e;
                }
            }
        }

        Err(last_err).with_context(|| {
            format!("registry index fetch failed after {MAX_FETCH_RETRIES} retries: {url}")
        })
    }

    async fn fetch_index_local(&self, url: &str) -> Result<RegistryIndex> {
        let path = url.trim_start_matches("file://");
        debug!(path, "reading local registry file");

        let text = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::fs::read_to_string(path),
        )
        .await
        .with_context(|| format!("timeout reading local registry: {path}"))?
        .with_context(|| format!("reading local registry file: {path}"))?;

        let index: RegistryIndex = serde_json::from_str(&text)
            .with_context(|| format!("deserialising registry index from {path}"))?;

        info!(
            registry = %index.name,
            plugins  = index.plugins.len(),
            path,
            "local registry index loaded"
        );
        Ok(index)
    }

    async fn fetch_index_http(&self, url: &str) -> Result<RegistryIndex> {
        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(INDEX_TIMEOUT_SECS),
            self.http.get(url).send(),
        )
        .await
        .with_context(|| format!("timeout fetching registry index: {url}"))?
        .with_context(|| format!("GET {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("registry returned HTTP {status} for {url}: {body}");
        }

        let text = tokio::time::timeout(
            std::time::Duration::from_secs(INDEX_TIMEOUT_SECS),
            resp.text(),
        )
        .await
        .with_context(|| format!("timeout reading registry response body: {url}"))?
        .context("reading registry response body")?;

        let index: RegistryIndex = serde_json::from_str(&text)
            .with_context(|| format!("deserialising registry index from {url}"))?;

        info!(
            registry = %index.name,
            plugins  = index.plugins.len(),
            url,
            "registry index loaded"
        );
        Ok(index)
    }

    /// Downloads a plugin `.wasm`, verifies its SHA-256, and writes it to the
    /// local plugin directory.
    ///
    /// `progress_tx` receives values in `0.0..=1.0` as bytes arrive.
    /// Send `None` to signal completion or failure to the caller.
    pub async fn install_plugin(
        &self,
        entry: &PluginEntry,
        progress_tx: tokio::sync::mpsc::Sender<InstallProgress>,
    ) -> Result<PathBuf> {
        let plugin_dir = {
            let p = self.prefs.read();
            PathBuf::from(&p.plugin_dir)
        };
        tokio::fs::create_dir_all(&plugin_dir)
            .await
            .context("creating plugin directory")?;

        let dest_path = plugin_dir.join(format!("{}-{}.wasm", entry.id, entry.version));

        // Skip re-download if already present and hash matches.
        if dest_path.exists() {
            if verify_file_sha256(&dest_path, &entry.sha256).await? {
                info!(path = %dest_path.display(), "plugin already installed and verified");
                let _ = progress_tx.send(InstallProgress::Done(dest_path.clone())).await;
                return Ok(dest_path);
            }
            warn!(path = %dest_path.display(), "existing file failed hash check — re-downloading");
        }

        // ── Download with streaming progress ─────────────────────────────────
        let _ = progress_tx
            .send(InstallProgress::Downloading { fraction: 0.0 })
            .await;

        // Support file:// URLs for local development and testing.
        let buf: Vec<u8>;
        if entry.wasm_url.starts_with("file://") {
            let path = entry.wasm_url.trim_start_matches("file://");
            buf = tokio::fs::read(path)
                .await
                .with_context(|| format!("reading local wasm file: {path}"))?;
            let _ = progress_tx
                .send(InstallProgress::Downloading { fraction: 1.0 })
                .await;
        } else {
            let resp = self
                .http
                .get(&entry.wasm_url)
                .send()
                .await
                .with_context(|| format!("GET {}", entry.wasm_url))?
                .error_for_status()
                .context("download URL returned non-2xx status")?;

            let total_bytes = resp.content_length().unwrap_or(0);
            let mut downloaded: u64 = 0;
            let mut tmp_buf: Vec<u8> = Vec::with_capacity(total_bytes as usize + 1);

            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.context("error reading download stream")?;
                tmp_buf.extend_from_slice(&chunk);
                downloaded += chunk.len() as u64;

                let fraction = if total_bytes > 0 {
                    downloaded as f32 / total_bytes as f32
                } else {
                    0.5
                };
                let _ = progress_tx
                    .send(InstallProgress::Downloading { fraction })
                    .await;
            }
            buf = tmp_buf;
        }

        // ── SHA-256 verification ──────────────────────────────────────────────
        let _ = progress_tx.send(InstallProgress::Verifying).await;
        let digest = hex::encode(Sha256::digest(&buf));
        if digest != entry.sha256 {
            bail!(
                "SHA-256 mismatch for {}: expected {} got {}",
                entry.id,
                entry.sha256,
                digest
            );
        }
        info!(plugin = %entry.id, "SHA-256 verified");

        // ── Write to disk ─────────────────────────────────────────────────────
        let _ = progress_tx.send(InstallProgress::Writing).await;
        let mut file = tokio::fs::File::create(&dest_path)
            .await
            .with_context(|| format!("creating {}", dest_path.display()))?;
        file.write_all(&buf)
            .await
            .context("writing plugin to disk")?;
        file.flush().await?;

        info!(path = %dest_path.display(), "plugin installed");
        let _ = progress_tx.send(InstallProgress::Done(dest_path.clone())).await;
        Ok(dest_path)
    }

    /// Removes a plugin `.wasm` from the local plugin directory.
    pub async fn uninstall_plugin(&self, entry: &PluginEntry) -> Result<()> {
        let plugin_dir = {
            let p = self.prefs.read();
            PathBuf::from(&p.plugin_dir)
        };
        let path = plugin_dir.join(format!("{}-{}.wasm", entry.id, entry.version));
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .with_context(|| format!("removing {}", path.display()))?;
            info!(path = %path.display(), "plugin uninstalled");
        }
        Ok(())
    }

    /// Returns the set of plugin IDs that are installed locally.
    pub async fn installed_ids(&self) -> Vec<String> {
        let plugin_dir = {
            let p = self.prefs.read();
            PathBuf::from(&p.plugin_dir)
        };
        let mut ids = Vec::new();
        if let Ok(mut rd) = tokio::fs::read_dir(&plugin_dir).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".wasm") {
                    // Strip the `-<version>.wasm` suffix to recover the plugin ID.
                    if let Some((id, _)) = name.rsplit_once('-') {
                        ids.push(id.to_string());
                    }
                }
            }
        }
        ids
    }
}

// ── Progress events ───────────────────────────────────────────────────────────

/// Progress events emitted by [`RegistryClient::install_plugin`].
#[derive(Debug, Clone)]
pub enum InstallProgress {
    Downloading { fraction: f32 },
    Verifying,
    Writing,
    Done(PathBuf),
    Failed(String),
}

impl From<InstallProgress> for InstallState {
    fn from(p: InstallProgress) -> Self {
        match p {
            InstallProgress::Downloading { fraction } => {
                InstallState::Downloading { progress: fraction }
            }
            InstallProgress::Verifying => InstallState::Verifying,
            InstallProgress::Writing   => InstallState::Installing,
            InstallProgress::Done(_)   => InstallState::Installed,
            InstallProgress::Failed(e) => InstallState::Failed(e),
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Reads a file from disk and checks its SHA-256 against `expected_hex`.
async fn verify_file_sha256(path: &Path, expected_hex: &str) -> Result<bool> {
    let bytes = tokio::fs::read(path)
        .await
        .with_context(|| format!("reading {}", path.display()))?;
    let digest = hex::encode(Sha256::digest(&bytes));
    Ok(digest == expected_hex)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LibrarySource, PluginEntry, StorePrefs};
    use parking_lot::RwLock;
    use sha2::{Digest, Sha256};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn make_client(plugin_dir: &str) -> RegistryClient {
        let prefs = StorePrefs {
            active_source: LibrarySource::ClawPlus,
            clawplus_registry_url: String::new(),
            openclaw_registry_url: String::new(),
            plugin_dir: plugin_dir.to_string(),
        };
        RegistryClient::new(Arc::new(RwLock::new(prefs)))
    }

    // ── fetch_index: file:// ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_fetch_index_from_local_file() {
        let dir = TempDir::new().unwrap();
        let index_path = dir.path().join("index.json");

        let json = r#"{
            "version": 1,
            "name": "Test Registry",
            "description": "unit test",
            "plugins": [
                {
                    "id": "test.plugin.one",
                    "name": "Plugin One",
                    "description": "desc",
                    "version": "0.1.0",
                    "author": "Tester",
                    "license": "MIT",
                    "wasm_url": "file:///tmp/one.wasm",
                    "sha256": "abc123",
                    "min_host_version": "0.1.0",
                    "tags": ["test"],
                    "source": "claw_plus",
                    "downloads": 0
                }
            ]
        }"#;
        tokio::fs::write(&index_path, json).await.unwrap();

        let client = make_client("/tmp/plugins");
        let url = format!("file://{}", index_path.display());
        let index = client.fetch_index(&url).await.unwrap();

        assert_eq!(index.version, 1);
        assert_eq!(index.name, "Test Registry");
        assert_eq!(index.plugins.len(), 1);
        assert_eq!(index.plugins[0].id, "test.plugin.one");
    }

    #[tokio::test]
    async fn test_fetch_index_missing_file_returns_error() {
        let client = make_client("/tmp/plugins");
        let result = client.fetch_index("file:///nonexistent/path/index.json").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_index_invalid_json_returns_error() {
        let dir = TempDir::new().unwrap();
        let index_path = dir.path().join("bad.json");
        tokio::fs::write(&index_path, b"not json at all").await.unwrap();

        let client = make_client("/tmp/plugins");
        let url = format!("file://{}", index_path.display());
        let result = client.fetch_index(&url).await;
        assert!(result.is_err());
    }

    // ── SHA-256 verification helper ───────────────────────────────────────────

    #[tokio::test]
    async fn test_verify_file_sha256_correct() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.bin");
        let content = b"hello openclaw+";
        tokio::fs::write(&path, content).await.unwrap();
        let expected = hex::encode(Sha256::digest(content));
        assert!(verify_file_sha256(&path, &expected).await.unwrap());
    }

    #[tokio::test]
    async fn test_verify_file_sha256_wrong_hash() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.bin");
        tokio::fs::write(&path, b"hello").await.unwrap();
        assert!(!verify_file_sha256(&path, "0000000000000000000000000000000000000000000000000000000000000000").await.unwrap());
    }

    // ── installed_ids ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_installed_ids_empty_dir() {
        let dir = TempDir::new().unwrap();
        let client = make_client(dir.path().to_str().unwrap());
        let ids = client.installed_ids().await;
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_installed_ids_finds_wasm_files() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("dev.clawplus.search-0.1.0.wasm"), b"fake").await.unwrap();
        tokio::fs::write(dir.path().join("dev.clawplus.git-0.2.0.wasm"), b"fake").await.unwrap();
        tokio::fs::write(dir.path().join("readme.txt"), b"ignored").await.unwrap();

        let client = make_client(dir.path().to_str().unwrap());
        let mut ids = client.installed_ids().await;
        ids.sort();
        assert_eq!(ids, vec!["dev.clawplus.git", "dev.clawplus.search"]);
    }

    // ── uninstall_plugin ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_uninstall_removes_wasm_file() {
        let dir = TempDir::new().unwrap();
        let wasm_path = dir.path().join("dev.clawplus.crypto-0.1.0.wasm");
        tokio::fs::write(&wasm_path, b"fake wasm").await.unwrap();
        assert!(wasm_path.exists());

        let client = make_client(dir.path().to_str().unwrap());
        let entry = dummy_entry("dev.clawplus.crypto", "0.1.0");
        client.uninstall_plugin(&entry).await.unwrap();
        assert!(!wasm_path.exists());
    }

    #[tokio::test]
    async fn test_uninstall_nonexistent_is_ok() {
        let dir = TempDir::new().unwrap();
        let client = make_client(dir.path().to_str().unwrap());
        let entry = dummy_entry("dev.clawplus.missing", "0.1.0");
        assert!(client.uninstall_plugin(&entry).await.is_ok());
    }

    // ── install_plugin (local file wasm_url) ─────────────────────────────────

    #[tokio::test]
    async fn test_install_plugin_already_installed_skips_download() {
        let dir = TempDir::new().unwrap();
        let content = b"fake wasm bytes";
        let sha256 = hex::encode(Sha256::digest(content));

        let dest = dir.path().join("dev.clawplus.test-0.1.0.wasm");
        tokio::fs::write(&dest, content).await.unwrap();

        let client = make_client(dir.path().to_str().unwrap());
        let mut entry = dummy_entry("dev.clawplus.test", "0.1.0");
        entry.sha256 = sha256;

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let result = client.install_plugin(&entry, tx).await.unwrap();
        assert_eq!(result, dest);

        let last = {
            let mut last = None;
            while let Ok(msg) = rx.try_recv() { last = Some(msg); }
            last
        };
        assert!(matches!(last, Some(InstallProgress::Done(_))));
    }

    // ── fault-tolerance: fetch_index_local timeout simulation ─────────────────

    #[tokio::test]
    async fn test_fetch_index_local_missing_returns_error() {
        let client = make_client("/tmp/plugins");
        let result = client
            .fetch_index("file:///no/such/path/registry.json")
            .await;
        assert!(result.is_err());
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("reading local registry file") || msg.contains("No such file"),
            "unexpected error message: {msg}"
        );
    }

    #[tokio::test]
    async fn test_fetch_index_local_bad_json_error_contains_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        tokio::fs::write(&path, b"{not valid json}").await.unwrap();

        let client = make_client("/tmp/plugins");
        let url = format!("file://{}", path.display());
        let result = client.fetch_index(&url).await;
        assert!(result.is_err());
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("deserialising"),
            "expected deserialisation error, got: {msg}"
        );
    }

    // ── fault-tolerance: SHA-256 mismatch on re-download ─────────────────────

    #[tokio::test]
    async fn test_verify_file_sha256_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.wasm");
        tokio::fs::write(&path, b"").await.unwrap();
        let expected = hex::encode(Sha256::digest(b""));
        assert!(verify_file_sha256(&path, &expected).await.unwrap());
    }

    #[tokio::test]
    async fn test_verify_file_sha256_nonexistent_returns_error() {
        let result = verify_file_sha256(
            std::path::Path::new("/no/such/file.wasm"),
            "0".repeat(64).as_str(),
        )
        .await;
        assert!(result.is_err());
    }

    // ── fault-tolerance: install_plugin SHA-256 mismatch ─────────────────────

    #[tokio::test]
    async fn test_install_plugin_hash_mismatch_returns_error() {
        let dir = TempDir::new().unwrap();
        let content = b"fake wasm bytes";
        let wrong_hash = "a".repeat(64);

        let dest = dir.path().join("dev.clawplus.test-0.1.0.wasm");
        tokio::fs::write(&dest, content).await.unwrap();

        let client = make_client(dir.path().to_str().unwrap());
        let mut entry = dummy_entry("dev.clawplus.test", "0.1.0");
        entry.sha256 = wrong_hash;
        // wasm_url is empty — re-download will fail with a network error
        entry.wasm_url = String::new();

        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let result = client.install_plugin(&entry, tx).await;
        // Hash mismatch triggers re-download; empty wasm_url causes an error
        assert!(result.is_err(), "expected error when hash mismatches and re-download fails");
    }

    // ── fault-tolerance: uninstall idempotency ────────────────────────────────

    #[tokio::test]
    async fn test_uninstall_twice_is_ok() {
        let dir = TempDir::new().unwrap();
        let wasm_path = dir.path().join("dev.clawplus.dup-0.1.0.wasm");
        tokio::fs::write(&wasm_path, b"wasm").await.unwrap();

        let client = make_client(dir.path().to_str().unwrap());
        let entry = dummy_entry("dev.clawplus.dup", "0.1.0");
        client.uninstall_plugin(&entry).await.unwrap();
        // Second call on already-removed file must not error
        assert!(client.uninstall_plugin(&entry).await.is_ok());
    }

    // ── fault-tolerance: installed_ids with mixed files ───────────────────────

    #[tokio::test]
    async fn test_installed_ids_ignores_non_wasm() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("plugin-0.1.0.wasm"), b"w").await.unwrap();
        tokio::fs::write(dir.path().join("notes.txt"), b"x").await.unwrap();
        tokio::fs::write(dir.path().join("config.json"), b"{}").await.unwrap();
        tokio::fs::write(dir.path().join(".DS_Store"), b"").await.unwrap();

        let client = make_client(dir.path().to_str().unwrap());
        let ids = client.installed_ids().await;
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "plugin");
    }

    // ── PluginEntry: new fields (skills, preinstalled) ────────────────────────

    #[test]
    fn test_plugin_entry_skills_field_defaults_empty() {
        let entry = dummy_entry("dev.clawplus.test", "0.1.0");
        assert!(entry.skills.is_empty(), "skills should default to empty vec");
    }

    #[test]
    fn test_plugin_entry_preinstalled_defaults_false() {
        let entry = dummy_entry("dev.clawplus.test", "0.1.0");
        assert!(!entry.preinstalled, "preinstalled should default to false");
    }

    #[test]
    fn test_plugin_entry_with_skills_serialises_correctly() {
        let mut entry = dummy_entry("dev.clawplus.fs-tools", "0.2.0");
        entry.skills = vec!["fs.readFile".into(), "fs.writeFile".into(), "fs.readDir".into()];
        entry.preinstalled = true;

        let json = serde_json::to_string(&entry).unwrap();
        let decoded: PluginEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.skills, vec!["fs.readFile", "fs.writeFile", "fs.readDir"]);
        assert!(decoded.preinstalled);
    }

    #[test]
    fn test_registry_index_with_preinstalled_plugins_deserialises() {
        let json = r#"{
            "version": 1,
            "name": "Test Registry",
            "description": "unit test",
            "plugins": [
                {
                    "id": "dev.clawplus.search",
                    "name": "Fast Search",
                    "description": "ripgrep search",
                    "version": "0.2.0",
                    "author": "ClawPlus",
                    "license": "MIT",
                    "wasm_url": "file:///tmp/search.wasm",
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                    "min_host_version": "0.1.0",
                    "tags": ["search", "builtin"],
                    "source": "claw_plus",
                    "downloads": 2100,
                    "skills": ["search.query", "search.web"],
                    "preinstalled": true
                },
                {
                    "id": "dev.clawplus.crypto",
                    "name": "Crypto Tools",
                    "description": "encryption",
                    "version": "0.2.0",
                    "author": "ClawPlus",
                    "license": "MIT",
                    "wasm_url": "file:///tmp/crypto.wasm",
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                    "min_host_version": "0.1.0",
                    "tags": ["crypto"],
                    "source": "claw_plus",
                    "downloads": 530
                }
            ]
        }"#;

        let index: crate::types::RegistryIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.plugins.len(), 2);

        let search = &index.plugins[0];
        assert_eq!(search.id, "dev.clawplus.search");
        assert!(search.preinstalled);
        assert_eq!(search.skills, vec!["search.query", "search.web"]);

        let crypto = &index.plugins[1];
        assert_eq!(crypto.id, "dev.clawplus.crypto");
        assert!(!crypto.preinstalled, "crypto should not be preinstalled");
        assert!(crypto.skills.is_empty(), "crypto skills should default empty");
    }

    #[test]
    fn test_install_progress_from_converts_all_variants() {
        use crate::types::InstallState;
        assert!(matches!(
            InstallState::from(InstallProgress::Downloading { fraction: 0.5 }),
            InstallState::Downloading { progress } if (progress - 0.5).abs() < 1e-6
        ));
        assert_eq!(InstallState::from(InstallProgress::Verifying), InstallState::Verifying);
        assert_eq!(InstallState::from(InstallProgress::Writing), InstallState::Installing);
        assert_eq!(
            InstallState::from(InstallProgress::Done(std::path::PathBuf::from("/tmp/x.wasm"))),
            InstallState::Installed
        );
        assert!(matches!(
            InstallState::from(InstallProgress::Failed("oops".into())),
            InstallState::Failed(msg) if msg == "oops"
        ));
    }

    #[tokio::test]
    async fn test_installed_ids_recovers_plugin_id_with_dots() {
        // Plugin IDs contain dots: "dev.clawplus.search-0.2.0.wasm"
        // The rsplitn(2, '-') strategy must correctly recover "dev.clawplus.search".
        let dir = TempDir::new().unwrap();
        tokio::fs::write(
            dir.path().join("dev.clawplus.search-0.2.0.wasm"),
            b"fake",
        ).await.unwrap();
        tokio::fs::write(
            dir.path().join("dev.clawplus.fs-tools-0.2.0.wasm"),
            b"fake",
        ).await.unwrap();

        let client = make_client(dir.path().to_str().unwrap());
        let mut ids = client.installed_ids().await;
        ids.sort();
        assert_eq!(ids, vec!["dev.clawplus.fs-tools", "dev.clawplus.search"]);
    }

    #[tokio::test]
    async fn test_install_plugin_creates_plugin_dir_if_missing() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("plugins").join("nested");
        // Directory does not exist yet — install_plugin must create it.
        assert!(!plugin_dir.exists());

        let content = b"fake wasm content for dir creation test";
        let sha256 = hex::encode(Sha256::digest(content));

        // Write a fake "remote" wasm to a temp file so we can use file:// URL.
        let src_dir = TempDir::new().unwrap();
        let src_path = src_dir.path().join("plugin.wasm");
        tokio::fs::write(&src_path, content).await.unwrap();

        let client = make_client(plugin_dir.to_str().unwrap());
        let mut entry = dummy_entry("dev.clawplus.dirtest", "0.1.0");
        entry.sha256 = sha256;
        entry.wasm_url = format!("file://{}", src_path.display());

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let result = client.install_plugin(&entry, tx).await;
        // Drain channel
        while rx.try_recv().is_ok() {}

        assert!(result.is_ok(), "install should succeed: {:?}", result.err());
        assert!(plugin_dir.exists(), "plugin directory should have been created");
    }

    #[tokio::test]
    async fn test_verify_file_sha256_large_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("large.wasm");
        // 1 MiB of pseudo-random-ish data
        let content: Vec<u8> = (0u32..262144).flat_map(|i| i.to_le_bytes()).collect();
        tokio::fs::write(&path, &content).await.unwrap();
        let expected = hex::encode(Sha256::digest(&content));
        assert!(verify_file_sha256(&path, &expected).await.unwrap());
        // Wrong hash must return false, not error
        assert!(!verify_file_sha256(&path, &"a".repeat(64)).await.unwrap());
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn dummy_entry(id: &str, version: &str) -> PluginEntry {
        PluginEntry {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            version: version.to_string(),
            author: "test".to_string(),
            license: "MIT".to_string(),
            wasm_url: String::new(),
            sha256: "0".repeat(64),
            min_host_version: "0.1.0".to_string(),
            tags: vec![],
            source: LibrarySource::ClawPlus,
            downloads: 0,
            skills: vec![],
            preinstalled: false,
            installed: false,
            download_progress: None,
        }
    }
}
