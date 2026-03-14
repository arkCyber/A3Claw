//! SkillRegistry — runtime Markdown skill document loader.
//!
//! # Purpose
//!
//! `BUILTIN_SKILLS` (in `skill.rs`) carries *metadata* about every skill: name,
//! description, parameter schema, and risk level.  `SkillRegistry` carries the
//! **workflow prose** — the Markdown instruction files that ClawHub distributes
//! and that the agent injects into its system prompt so it knows *how* to use a
//! skill in a real-world scenario (e.g. `github.md`, `morning-briefing.md`).
//!
//! # Lifecycle
//!
//! ```text
//! Process start
//!   └─ SkillRegistry::load(dir)    ← scan  ~/.openclaw/skills/*.md  once
//!         ↓
//!   AgentExecutor::run_task
//!   └─ registry.inject_into_prompt(skill_set, base_prompt)
//!         ↓
//!   LLM system prompt  ← base_prompt + "\n\n## Installed Skill Guides\n..."
//!         ↓
//!   clawhub.install writes new .md  →  registry.reload(dir)   (hot reload)
//! ```
//!
//! # Thread Safety
//!
//! `SkillRegistry` is `Send + Sync`.  An `Arc<SkillRegistry>` is safe to share
//! across multiple concurrent agent tasks; `reload()` uses an `RwLock` so that
//! concurrent reads are never blocked except during the brief reload window.
//!
//! # Token Budget
//!
//! Each `.md` document is capped at [`MAX_DOC_CHARS`] characters before storage
//! so that a single pathological file cannot overflow the LLM context window.
//! `inject_into_prompt` further caps the *total* injected section at
//! [`MAX_INJECT_CHARS`] and appends a truncation marker when the budget is
//! exhausted, guaranteeing that the final prompt fits within small-model limits.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// ── Constants ──────────────────────────────────────────────────────────────────

/// Maximum characters stored per skill document (≈ 1 500 tokens for most models).
pub const MAX_DOC_CHARS: usize = 6_000;

/// Maximum total characters injected into a system prompt.
/// Keeps the combined prompt well within a 4 096-token context budget.
pub const MAX_INJECT_CHARS: usize = 3_000;

/// File extension for skill documents.
const SKILL_EXT: &str = "md";

// ── SkillDoc ───────────────────────────────────────────────────────────────────

/// A single installed skill document.
#[derive(Debug, Clone)]
pub struct SkillDoc {
    /// Skill name derived from the file stem (e.g. `github`, `morning-briefing`).
    pub name: String,
    /// Full path to the `.md` file on disk.
    pub path: PathBuf,
    /// Markdown content, capped at [`MAX_DOC_CHARS`] characters.
    pub content: String,
    /// Byte length of the original file before truncation (for diagnostics).
    pub original_len: usize,
}

impl SkillDoc {
    /// Construct a `SkillDoc` from a file path and its raw content.
    ///
    /// Content is truncated to [`MAX_DOC_CHARS`] if necessary; the final
    /// truncation never cuts in the middle of a UTF-8 character boundary.
    pub fn new(path: PathBuf, raw: String) -> Self {
        let original_len = raw.len();
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content = if raw.len() > MAX_DOC_CHARS {
            // Safe UTF-8 truncation: find the last char boundary ≤ MAX_DOC_CHARS.
            let mut end = MAX_DOC_CHARS;
            while !raw.is_char_boundary(end) {
                end -= 1;
            }
            let mut truncated = raw[..end].to_string();
            truncated.push_str("\n\n... (skill guide truncated for token budget)");
            truncated
        } else {
            raw
        };

        Self { name, path, content, original_len }
    }

    /// Returns `true` if the document was truncated.
    pub fn was_truncated(&self) -> bool {
        self.content.contains("... (skill guide truncated for token budget)")
    }
}

// ── SkillRegistry ──────────────────────────────────────────────────────────────

/// Runtime registry of installed skill Markdown documents.
///
/// Backed by an `Arc<RwLock<HashMap>>` so that:
/// - Many concurrent reads (agent prompt construction) never contend.
/// - A single writer (`reload`) holds the lock only for the `HashMap` swap,
///   not during the I/O scan.
#[derive(Debug, Clone)]
pub struct SkillRegistry {
    inner: Arc<RwLock<HashMap<String, SkillDoc>>>,
}

impl SkillRegistry {
    // ── Construction ──────────────────────────────────────────────────────────

    /// Create an empty registry (no documents loaded).
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all `*.md` files from `dir` into the registry.
    ///
    /// Non-existent directories are silently skipped — the agent will run
    /// without any installed skill guides.  Individual file I/O errors are
    /// logged as warnings and skipped rather than failing the whole load.
    pub async fn load(dir: impl AsRef<Path>) -> Self {
        let registry = Self::empty();
        registry.reload(dir).await;
        registry
    }

    /// Load from the standard user skill directory: `~/.openclaw/skills/`.
    ///
    /// Falls back to `empty()` if the home directory cannot be determined.
    pub async fn load_default() -> Self {
        match default_skills_dir() {
            Some(dir) => Self::load(dir).await,
            None => {
                warn!("SkillRegistry: could not determine home directory; starting empty");
                Self::empty()
            }
        }
    }

    // ── Reload (hot reload) ──────────────────────────────────────────────────

    /// Rescan `dir` for `*.md` files and atomically replace the in-memory map.
    ///
    /// I/O is performed *outside* the write lock; only the final `HashMap` swap
    /// is performed under the lock, minimising contention.
    pub async fn reload(&self, dir: impl AsRef<Path>) {
        let dir = dir.as_ref();

        if !dir.exists() {
            debug!(path = %dir.display(), "SkillRegistry: skills directory does not exist — skipping scan");
            return;
        }

        let entries = match scan_md_files(dir) {
            Ok(e) => e,
            Err(e) => {
                warn!(path = %dir.display(), error = %e, "SkillRegistry: directory scan failed");
                return;
            }
        };

        let mut new_map: HashMap<String, SkillDoc> = HashMap::with_capacity(entries.len());

        for path in entries {
            match tokio::fs::read_to_string(&path).await {
                Ok(raw) => {
                    let doc = SkillDoc::new(path.clone(), raw);
                    if doc.was_truncated() {
                        warn!(
                            name = %doc.name,
                            original_bytes = doc.original_len,
                            "SkillRegistry: skill guide truncated to {} chars",
                            MAX_DOC_CHARS
                        );
                    }
                    debug!(name = %doc.name, "SkillRegistry: loaded skill guide");
                    new_map.insert(doc.name.clone(), doc);
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "SkillRegistry: failed to read skill file");
                }
            }
        }

        let count = new_map.len();
        *self.inner.write().await = new_map;
        info!(count, "SkillRegistry: loaded {} skill guide(s)", count);
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Return the number of loaded skill documents.
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Return `true` if no skill documents are loaded.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    /// Look up a skill document by name (file stem).
    pub async fn get(&self, name: &str) -> Option<SkillDoc> {
        self.inner.read().await.get(name).cloned()
    }

    /// Return the names of all loaded skill documents, sorted.
    pub async fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.inner.read().await.keys().cloned().collect();
        names.sort();
        names
    }

    /// Return all loaded documents, sorted by name.
    pub async fn all(&self) -> Vec<SkillDoc> {
        let map = self.inner.read().await;
        let mut docs: Vec<SkillDoc> = map.values().cloned().collect();
        docs.sort_by(|a, b| a.name.cmp(&b.name));
        docs
    }

    // ── Prompt injection ──────────────────────────────────────────────────────

    /// Append relevant skill guides to a base system prompt.
    ///
    /// Only documents whose name matches a skill in `granted_skill_names` are
    /// injected.  If `granted_skill_names` is empty, *all* loaded documents
    /// are candidates (useful for unrestricted agents).
    ///
    /// The injected section is capped at [`MAX_INJECT_CHARS`] total characters;
    /// a truncation marker is appended when the budget is exhausted so the LLM
    /// can see that additional guides exist.
    ///
    /// Returns the original `base_prompt` unchanged if no matching guides exist.
    pub async fn inject_into_prompt(
        &self,
        base_prompt: &str,
        granted_skill_names: &[&str],
    ) -> String {
        let map = self.inner.read().await;
        if map.is_empty() {
            return base_prompt.to_string();
        }

        // Collect matching docs in deterministic order.
        let mut matching: Vec<&SkillDoc> = map
            .values()
            .filter(|doc| {
                granted_skill_names.is_empty()
                    || granted_skill_names.iter().any(|n| *n == doc.name.as_str())
            })
            .collect();
        matching.sort_by(|a, b| a.name.cmp(&b.name));

        if matching.is_empty() {
            return base_prompt.to_string();
        }

        let mut section = String::with_capacity(MAX_INJECT_CHARS + 128);
        section.push_str("\n\n## Installed Skill Guides\n\n");

        let mut budget = MAX_INJECT_CHARS;
        let mut truncated = false;

        for doc in &matching {
            let header = format!("### {}\n", doc.name);
            let body   = format!("{}\n\n", doc.content);
            let needed = header.len() + body.len();

            if needed > budget {
                truncated = true;
                break;
            }

            section.push_str(&header);
            section.push_str(&body);
            budget -= needed;
        }

        if truncated {
            section.push_str("... (additional skill guides omitted — token budget exhausted)\n");
        }

        format!("{}{}", base_prompt, section)
    }

    // ── Write helpers (used by clawhub.install) ───────────────────────────────

    /// Write a skill document to `dir/<name>.md` and insert it into the registry
    /// without a full rescan.  Creates `dir` if it does not exist.
    ///
    /// Returns the path of the written file on success.
    pub async fn install_doc(
        &self,
        dir: impl AsRef<Path>,
        name: &str,
        content: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let dir = dir.as_ref();
        tokio::fs::create_dir_all(dir).await?;

        // Sanitise the name: only allow [a-z0-9_-] to prevent path traversal.
        let safe_name = sanitise_skill_name(name);
        let path = dir.join(format!("{}.{}", safe_name, SKILL_EXT));

        tokio::fs::write(&path, content).await?;

        let doc = SkillDoc::new(path.clone(), content.to_string());
        self.inner.write().await.insert(doc.name.clone(), doc);

        info!(name = safe_name, path = %path.display(), "SkillRegistry: installed skill guide");
        Ok(path)
    }

    /// Remove a skill document by name from the registry and delete its file
    /// from `dir`.  Returns `true` if the document existed and was removed.
    pub async fn remove_doc(
        &self,
        dir: impl AsRef<Path>,
        name: &str,
    ) -> bool {
        let safe_name = sanitise_skill_name(name);
        let path = dir.as_ref().join(format!("{}.{}", safe_name, SKILL_EXT));

        // Remove from map first; ignore file-delete errors (file may not exist).
        let existed = self.inner.write().await.remove(&safe_name).is_some();
        let _ = tokio::fs::remove_file(&path).await;

        if existed {
            info!(name = safe_name, "SkillRegistry: removed skill guide");
        }
        existed
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Return the default skills directory: `~/.openclaw/skills/`.
pub fn default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".openclaw").join("skills"))
}

/// Scan a directory for `*.md` files, returning their paths.
/// Does *not* recurse into sub-directories.
fn scan_md_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == SKILL_EXT {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort(); // Deterministic ordering across platforms.
    Ok(paths)
}

/// Sanitise a skill name so it is safe to use as a file stem.
///
/// Keeps only ASCII lowercase letters, digits, hyphens, and underscores.
/// Replaces everything else with `-`.  Limits to 64 characters.
fn sanitise_skill_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .to_lowercase();

    let trimmed: String = cleaned
        .trim_matches('-')
        .chars()
        .take(64)
        .collect();

    if trimmed.is_empty() { "skill".to_string() } else { trimmed }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn tmp_dir_with_skills(skills: &[(&str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, content) in skills {
            let path = dir.path().join(format!("{}.md", name));
            std::fs::write(&path, content).expect("write skill file");
        }
        dir
    }

    // ── SkillDoc ──────────────────────────────────────────────────────────────

    #[test]
    fn skill_doc_name_from_stem() {
        let path = PathBuf::from("/home/user/.openclaw/skills/github.md");
        let doc = SkillDoc::new(path, "# GitHub Skill\n\nDo things.".to_string());
        assert_eq!(doc.name, "github");
    }

    #[test]
    fn skill_doc_short_content_not_truncated() {
        let content = "# Short\n\nSome instructions.".to_string();
        let doc = SkillDoc::new(PathBuf::from("test.md"), content.clone());
        assert_eq!(doc.content, content);
        assert!(!doc.was_truncated());
        assert_eq!(doc.original_len, content.len());
    }

    #[test]
    fn skill_doc_long_content_truncated() {
        let long = "x".repeat(MAX_DOC_CHARS + 1000);
        let doc = SkillDoc::new(PathBuf::from("big.md"), long.clone());
        assert!(doc.was_truncated(), "oversized doc must be truncated");
        assert!(doc.content.len() <= MAX_DOC_CHARS + 60,
            "truncated doc must fit within MAX_DOC_CHARS + marker overhead");
        assert_eq!(doc.original_len, long.len());
    }

    #[test]
    fn skill_doc_truncation_stays_on_char_boundary() {
        // Create content that has a multi-byte character right at the boundary.
        let mut s = "a".repeat(MAX_DOC_CHARS - 1);
        s.push('é'); // 2 bytes — straddles boundary at MAX_DOC_CHARS
        s.push_str(&"b".repeat(100));
        let doc = SkillDoc::new(PathBuf::from("boundary.md"), s);
        assert!(doc.was_truncated());
        // Content must be valid UTF-8 (no panic means it is).
        let _ = doc.content.chars().count();
    }

    #[test]
    fn skill_doc_exactly_max_doc_chars_not_truncated() {
        let content = "a".repeat(MAX_DOC_CHARS);
        let doc = SkillDoc::new(PathBuf::from("exact.md"), content);
        assert!(!doc.was_truncated());
    }

    #[test]
    fn skill_doc_empty_content() {
        let doc = SkillDoc::new(PathBuf::from("empty.md"), String::new());
        assert!(!doc.was_truncated());
        assert!(doc.content.is_empty());
    }

    // ── sanitise_skill_name ───────────────────────────────────────────────────

    #[test]
    fn sanitise_allows_alphanumeric_hyphen_underscore() {
        assert_eq!(sanitise_skill_name("morning-briefing"), "morning-briefing");
        assert_eq!(sanitise_skill_name("github_actions"), "github_actions");
        assert_eq!(sanitise_skill_name("skill123"), "skill123");
    }

    #[test]
    fn sanitise_replaces_special_chars() {
        // Spaces and '!' become '-', then trailing hyphens are trimmed.
        assert_eq!(sanitise_skill_name("My Skill!"), "my-skill");
    }

    #[test]
    fn sanitise_trims_leading_trailing_hyphens() {
        assert_eq!(sanitise_skill_name("--github--"), "github");
    }

    #[test]
    fn sanitise_empty_returns_skill_fallback() {
        assert_eq!(sanitise_skill_name(""), "skill");
        assert_eq!(sanitise_skill_name("!!!"), "skill");
    }

    #[test]
    fn sanitise_caps_at_64_chars() {
        let long = "a".repeat(100);
        let result = sanitise_skill_name(&long);
        assert!(result.len() <= 64, "name must be capped at 64 chars: len={}", result.len());
    }

    #[test]
    fn sanitise_path_traversal_rejected() {
        // "../../etc/passwd" must not survive sanitisation as a path component.
        let result = sanitise_skill_name("../../etc/passwd");
        assert!(!result.contains(".."), "path traversal must be removed: {}", result);
        assert!(!result.contains('/'), "slashes must be removed: {}", result);
    }

    // ── SkillRegistry::empty ──────────────────────────────────────────────────

    #[tokio::test]
    async fn empty_registry_has_zero_docs() {
        let r = SkillRegistry::empty();
        assert_eq!(r.len().await, 0);
        assert!(r.is_empty().await);
    }

    #[tokio::test]
    async fn empty_registry_names_returns_empty_vec() {
        let r = SkillRegistry::empty();
        assert!(r.names().await.is_empty());
    }

    // ── SkillRegistry::load ───────────────────────────────────────────────────

    #[tokio::test]
    async fn load_from_nonexistent_dir_gives_empty_registry() {
        let r = SkillRegistry::load("/this/path/does/not/exist/12345").await;
        assert!(r.is_empty().await);
    }

    #[tokio::test]
    async fn load_scans_md_files() {
        let dir = tmp_dir_with_skills(&[
            ("github",           "# GitHub\nManage repos."),
            ("morning-briefing", "# Morning Briefing\nDaily summary."),
        ]);
        let r = SkillRegistry::load(dir.path()).await;
        assert_eq!(r.len().await, 2);
        assert!(r.names().await.contains(&"github".to_string()));
        assert!(r.names().await.contains(&"morning-briefing".to_string()));
    }

    #[tokio::test]
    async fn load_ignores_non_md_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("skill.txt"), "not md").unwrap();
        std::fs::write(dir.path().join("skill.json"), "{}").unwrap();
        std::fs::write(dir.path().join("real.md"), "# Real").unwrap();
        let r = SkillRegistry::load(dir.path()).await;
        assert_eq!(r.len().await, 1);
        assert_eq!(r.names().await, vec!["real"]);
    }

    #[tokio::test]
    async fn load_names_are_sorted() {
        let dir = tmp_dir_with_skills(&[
            ("zebra", "z"), ("alpha", "a"), ("mango", "m"),
        ]);
        let r = SkillRegistry::load(dir.path()).await;
        let names = r.names().await;
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "names() must return sorted list");
    }

    // ── SkillRegistry::get ────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_existing_doc_returns_some() {
        let dir = tmp_dir_with_skills(&[("github", "# GitHub")]);
        let r = SkillRegistry::load(dir.path()).await;
        let doc = r.get("github").await;
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().content, "# GitHub");
    }

    #[tokio::test]
    async fn get_missing_doc_returns_none() {
        let r = SkillRegistry::empty();
        assert!(r.get("nonexistent").await.is_none());
    }

    // ── SkillRegistry::reload (hot reload) ───────────────────────────────────

    #[tokio::test]
    async fn reload_adds_new_files() {
        let dir = tmp_dir_with_skills(&[("github", "v1")]);
        let r = SkillRegistry::load(dir.path()).await;
        assert_eq!(r.len().await, 1);

        // Add a second file, then reload.
        std::fs::write(dir.path().join("slack.md"), "# Slack").unwrap();
        r.reload(dir.path()).await;

        assert_eq!(r.len().await, 2);
        assert!(r.get("slack").await.is_some());
    }

    #[tokio::test]
    async fn reload_reflects_updated_content() {
        let dir = tmp_dir_with_skills(&[("github", "v1")]);
        let r = SkillRegistry::load(dir.path()).await;
        assert_eq!(r.get("github").await.unwrap().content, "v1");

        std::fs::write(dir.path().join("github.md"), "v2 updated").unwrap();
        r.reload(dir.path()).await;

        assert_eq!(r.get("github").await.unwrap().content, "v2 updated");
    }

    #[tokio::test]
    async fn reload_removes_deleted_files() {
        let dir = tmp_dir_with_skills(&[("github", "g"), ("slack", "s")]);
        let r = SkillRegistry::load(dir.path()).await;
        assert_eq!(r.len().await, 2);

        std::fs::remove_file(dir.path().join("slack.md")).unwrap();
        r.reload(dir.path()).await;

        assert_eq!(r.len().await, 1);
        assert!(r.get("slack").await.is_none());
    }

    // ── inject_into_prompt ────────────────────────────────────────────────────

    #[tokio::test]
    async fn inject_empty_registry_returns_base_prompt() {
        let r = SkillRegistry::empty();
        let result = r.inject_into_prompt("Base prompt.", &["github"]).await;
        assert_eq!(result, "Base prompt.");
    }

    #[tokio::test]
    async fn inject_matching_skill_appends_section() {
        let dir = tmp_dir_with_skills(&[("github", "## GitHub\n\nCreate PRs.")]);
        let r = SkillRegistry::load(dir.path()).await;
        let result = r.inject_into_prompt("Base.", &["github"]).await;
        assert!(result.starts_with("Base."));
        assert!(result.contains("## Installed Skill Guides"));
        assert!(result.contains("Create PRs."));
    }

    #[tokio::test]
    async fn inject_non_matching_skill_not_included() {
        let dir = tmp_dir_with_skills(&[("github", "GitHub content")]);
        let r = SkillRegistry::load(dir.path()).await;
        let result = r.inject_into_prompt("Base.", &["slack"]).await;
        // "slack" not installed — no injection
        assert_eq!(result, "Base.");
    }

    #[tokio::test]
    async fn inject_empty_granted_list_injects_all() {
        let dir = tmp_dir_with_skills(&[
            ("github", "GH content"),
            ("slack",  "SL content"),
        ]);
        let r = SkillRegistry::load(dir.path()).await;
        let result = r.inject_into_prompt("Base.", &[]).await;
        assert!(result.contains("GH content"));
        assert!(result.contains("SL content"));
    }

    #[tokio::test]
    async fn inject_token_budget_caps_output() {
        // Create many large skill docs to exceed MAX_INJECT_CHARS.
        let dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            let name = format!("skill{:02}", i);
            let content = "x".repeat(MAX_INJECT_CHARS / 5);  // each ~600 chars
            std::fs::write(dir.path().join(format!("{}.md", name)), content).unwrap();
        }
        let r = SkillRegistry::load(dir.path()).await;
        let result = r.inject_into_prompt("Base.", &[]).await;
        // Result must not exceed base + inject section header + budget + marker overhead.
        let injected_len = result.len() - "Base.".len();
        assert!(
            injected_len <= MAX_INJECT_CHARS + 256,
            "injected section too large: {} chars", injected_len
        );
        assert!(result.contains("omitted"), "truncation marker must appear");
    }

    #[tokio::test]
    async fn inject_prompt_is_base_plus_section() {
        let dir = tmp_dir_with_skills(&[("morning-briefing", "Daily briefing steps.")]);
        let r = SkillRegistry::load(dir.path()).await;
        let base = "You are a helpful agent.";
        let result = r.inject_into_prompt(base, &["morning-briefing"]).await;
        assert!(result.starts_with(base));
        assert!(result.contains("morning-briefing"));
        assert!(result.contains("Daily briefing steps."));
    }

    // ── install_doc / remove_doc ──────────────────────────────────────────────

    #[tokio::test]
    async fn install_doc_writes_file_and_registers() {
        let dir = tempfile::tempdir().unwrap();
        let r = SkillRegistry::empty();
        let path = r.install_doc(dir.path(), "github", "# GitHub guide").await.unwrap();
        assert!(path.exists(), "installed file must exist on disk");
        assert_eq!(r.len().await, 1);
        assert_eq!(r.get("github").await.unwrap().content, "# GitHub guide");
    }

    #[tokio::test]
    async fn install_doc_sanitises_name() {
        let dir = tempfile::tempdir().unwrap();
        let r = SkillRegistry::empty();
        // Names with spaces/special chars must be sanitised.
        let path = r.install_doc(dir.path(), "My Skill!", "content").await.unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy();
        assert!(!stem.contains(' '), "spaces must be removed from file stem");
        assert!(!stem.contains('!'), "special chars must be removed");
    }

    #[tokio::test]
    async fn install_doc_path_traversal_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let r = SkillRegistry::empty();
        let path = r.install_doc(dir.path(), "../../etc/evil", "evil").await.unwrap();
        // The file must be inside `dir`, not at ../../etc/evil.
        assert!(
            path.starts_with(dir.path()),
            "install_doc must not escape the skills directory: {:?}", path
        );
    }

    #[tokio::test]
    async fn remove_doc_removes_from_registry_and_disk() {
        let dir = tempfile::tempdir().unwrap();
        let r = SkillRegistry::empty();
        r.install_doc(dir.path(), "slack", "# Slack").await.unwrap();
        assert_eq!(r.len().await, 1);

        let removed = r.remove_doc(dir.path(), "slack").await;
        assert!(removed, "remove_doc must return true for existing skill");
        assert_eq!(r.len().await, 0);
        assert!(!dir.path().join("slack.md").exists(), "file must be deleted from disk");
    }

    #[tokio::test]
    async fn remove_doc_returns_false_for_unknown_skill() {
        let r = SkillRegistry::empty();
        let dir = tempfile::tempdir().unwrap();
        let removed = r.remove_doc(dir.path(), "nonexistent").await;
        assert!(!removed, "remove_doc must return false for unknown skill");
    }

    // ── Concurrent access safety ──────────────────────────────────────────────

    #[tokio::test]
    async fn concurrent_reads_do_not_panic() {
        let dir = tmp_dir_with_skills(&[("github", "GH"), ("slack", "SL")]);
        let r = Arc::new(SkillRegistry::load(dir.path()).await);

        let mut handles = Vec::new();
        for _ in 0..16 {
            let r2 = r.clone();
            handles.push(tokio::spawn(async move {
                let _ = r2.names().await;
                let _ = r2.get("github").await;
                let _ = r2.inject_into_prompt("Base.", &["github", "slack"]).await;
            }));
        }
        for h in handles {
            h.await.expect("task must not panic");
        }
    }

    #[tokio::test]
    async fn concurrent_reload_and_read_no_panic() {
        let dir = tmp_dir_with_skills(&[("github", "GH")]);
        let r = Arc::new(SkillRegistry::load(dir.path()).await);
        let dir_path = dir.path().to_path_buf();

        let r_reader = r.clone();
        let reader = tokio::spawn(async move {
            for _ in 0..50 {
                let _ = r_reader.names().await;
                let _ = r_reader.get("github").await;
            }
        });

        let r_writer = r.clone();
        let dir_w = dir_path.clone();
        let writer = tokio::spawn(async move {
            for i in 0..5u8 {
                std::fs::write(dir_w.join(format!("extra{}.md", i)), format!("content {}", i))
                    .unwrap();
                r_writer.reload(&dir_w).await;
            }
        });

        reader.await.expect("reader must not panic");
        writer.await.expect("writer must not panic");
    }

    // ── default_skills_dir ────────────────────────────────────────────────────

    #[test]
    fn default_skills_dir_ends_with_openclaw_skills() {
        if let Some(dir) = default_skills_dir() {
            let s = dir.to_string_lossy();
            assert!(s.contains(".openclaw"), "default dir must contain .openclaw: {}", s);
            assert!(s.ends_with("skills"), "default dir must end with 'skills': {}", s);
        }
        // If home dir is unavailable in CI, None is acceptable.
    }
}
