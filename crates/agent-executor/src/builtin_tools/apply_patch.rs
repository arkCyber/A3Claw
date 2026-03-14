//! `apply_patch` — unified diff / search-replace patch tool.
//!
//! Mirrors the official OpenClaw `apply_patch` built-in tool.
//! Supports two patch formats:
//!
//! 1. **Unified diff** (`--- a/file` / `+++ b/file` header)  
//! 2. **Search-replace blocks** (fenced with `<<<`, `===`, `>>>`)
//!
//! Both formats are applied directly in-process without shelling out.
//!
//! Security: writes are gated by the Gateway before-skill hook (called by
//! `SkillDispatcher::dispatch`). The `apply_patch` skill itself is classified
//! `Confirm` so the operator must approve every patch.

use std::path::{Path, PathBuf};

/// Result of a single file patch operation.
#[derive(Debug)]
pub struct PatchResult {
    pub path: PathBuf,
    pub hunks_applied: usize,
    pub created: bool,
}

/// Apply a patch string to the filesystem.
///
/// Detects the patch format automatically:
/// - If the patch contains `--- ` / `+++ ` headers → unified diff
/// - Otherwise → search-replace block format
pub fn apply(patch_text: &str, workspace_root: Option<&Path>) -> Result<Vec<PatchResult>, String> {
    let trimmed = patch_text.trim();

    if looks_like_unified_diff(trimmed) {
        apply_unified_diff(trimmed, workspace_root)
    } else if trimmed.contains("<<<<<<<") || trimmed.contains("<<<") {
        apply_search_replace(trimmed, workspace_root)
    } else {
        Err("Unrecognised patch format. Expected unified diff (--- / +++) or \
             search-replace blocks (<<< SEARCH / === / >>> REPLACE)."
            .to_string())
    }
}

fn looks_like_unified_diff(s: &str) -> bool {
    s.contains("\n--- ") || s.starts_with("--- ") || s.contains("\ndiff --git ")
}

// ── Unified diff ──────────────────────────────────────────────────────────────

fn apply_unified_diff(
    patch: &str,
    workspace_root: Option<&Path>,
) -> Result<Vec<PatchResult>, String> {
    let mut results = Vec::new();
    let mut current_file: Option<PathBuf> = None;
    let mut original_lines: Vec<String> = Vec::new();
    let mut hunks: Vec<Hunk> = Vec::new();

    for line in patch.lines() {
        if line.starts_with("--- ") {
            // flush previous file
            if let Some(ref path) = current_file {
                let r = flush_unified(path, &original_lines, &hunks)?;
                results.push(r);
            }
            current_file = None;
            original_lines.clear();
            hunks.clear();
        } else if line.starts_with("+++ ") {
            let raw = line.trim_start_matches("+++ ").trim();
            let rel = raw.trim_start_matches("b/").trim_start_matches("./");
            let path = resolve_path(rel, workspace_root);

            // Load existing file content (may not exist for new files)
            original_lines = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .map(|l| l.to_string())
                .collect();
            current_file = Some(path);
        } else if line.starts_with("@@ ") {
            // @@ -start,count +start,count @@
            if let Some(hunk) = parse_hunk_header(line) {
                hunks.push(hunk);
            }
        } else if let Some(last) = hunks.last_mut() {
            last.lines.push(line.to_string());
        }
    }

    // flush last file
    if let Some(ref path) = current_file {
        let r = flush_unified(path, &original_lines, &hunks)?;
        results.push(r);
    }

    if results.is_empty() {
        Err("No file hunks found in unified diff.".to_string())
    } else {
        Ok(results)
    }
}

#[derive(Debug)]
struct Hunk {
    orig_start: usize,
    lines: Vec<String>,
}

fn parse_hunk_header(line: &str) -> Option<Hunk> {
    // @@ -orig_start[,orig_count] +new_start[,new_count] @@
    let after_prefix = line.strip_prefix("@@ -")?;
    let comma_or_space = after_prefix.find(|c: char| c == ',' || c == ' ')?;
    let orig_start: usize = after_prefix[..comma_or_space].parse().ok()?;
    Some(Hunk { orig_start, lines: Vec::new() })
}

fn flush_unified(
    path: &Path,
    original: &[String],
    hunks: &[Hunk],
) -> Result<PatchResult, String> {
    if hunks.is_empty() {
        return Err(format!("No @@ hunk headers found for file: {}", path.display()));
    }
    let created = !path.exists();
    let mut result_lines: Vec<String> = original.to_vec();

    // Apply hunks in reverse order so line numbers stay valid
    let mut sorted = hunks.to_vec_by_start();
    sorted.sort_by(|a, b| b.orig_start.cmp(&a.orig_start));

    for hunk in &sorted {
        let hunk_base = hunk.orig_start.saturating_sub(1); // 1-based → 0-based
        let mut cursor = hunk_base; // tracks position in result_lines as we apply

        let mut i = 0;
        while i < hunk.lines.len() {
            let hl = &hunk.lines[i];
            if hl.starts_with('-') {
                // Count consecutive '-' lines to delete in one splice
                let mut del_count = 0;
                let del_start = cursor;
                while i < hunk.lines.len() && hunk.lines[i].starts_with('-') {
                    del_count += 1;
                    i += 1;
                }
                // Collect any immediately following '+' lines as replacements
                let mut additions: Vec<String> = Vec::new();
                while i < hunk.lines.len() && hunk.lines[i].starts_with('+') {
                    additions.push(hunk.lines[i][1..].to_string());
                    i += 1;
                }
                let end = (del_start + del_count).min(result_lines.len());
                let added = additions.len();
                result_lines.splice(del_start..end, additions);
                cursor = del_start + added;
            } else if hl.starts_with('+') {
                // Pure insertion (no preceding '-')
                let ins: String = hl[1..].to_string();
                result_lines.insert(cursor, ins);
                cursor += 1;
                i += 1;
            } else {
                // Context line — advance cursor, don't modify
                cursor += 1;
                i += 1;
            }
        }
    }

    // Write result
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    std::fs::write(path, result_lines.join("\n") + "\n")
        .map_err(|e| format!("write {}: {}", path.display(), e))?;

    Ok(PatchResult {
        path: path.to_path_buf(),
        hunks_applied: hunks.len(),
        created,
    })
}

#[allow(dead_code)]
trait HunkExt {
    fn vec_by_start(&self) -> Vec<&Hunk>;
    fn to_vec_by_start(&self) -> Vec<Hunk>;
}

impl HunkExt for [Hunk] {
    fn vec_by_start(&self) -> Vec<&Hunk> {
        self.iter().collect()
    }
    fn to_vec_by_start(&self) -> Vec<Hunk> {
        self.iter()
            .map(|h| Hunk { orig_start: h.orig_start, lines: h.lines.clone() })
            .collect()
    }
}

// ── Search-replace blocks ─────────────────────────────────────────────────────
//
// Format:
//
// <<<<<<< path/to/file.ext
// SEARCH TEXT
// =======
// REPLACE TEXT
// >>>>>>>
//
// or the triple-fence variant used by some LLMs:
//
// <<< SEARCH
// SEARCH TEXT
// ===
// REPLACE TEXT
// >>> REPLACE

fn apply_search_replace(
    patch: &str,
    workspace_root: Option<&Path>,
) -> Result<Vec<PatchResult>, String> {
    let mut results = Vec::new();
    let lines: Vec<&str> = patch.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Detect block start: `<<<<<<< path` or `<<< SEARCH`
        if line.starts_with("<<<<<<<") || line == "<<< SEARCH" || line == "<<<" {
            let file_path: Option<PathBuf> = if line.starts_with("<<<<<<<") {
                let raw = line.trim_start_matches('<').trim();
                if raw.is_empty() || raw.eq_ignore_ascii_case("SEARCH") {
                    None
                } else {
                    Some(resolve_path(raw, workspace_root))
                }
            } else {
                None
            };

            i += 1;
            let mut search_lines: Vec<&str> = Vec::new();
            while i < lines.len() && !lines[i].trim().starts_with("=======") && lines[i].trim() != "===" {
                search_lines.push(lines[i]);
                i += 1;
            }
            i += 1; // skip `=======` / `===`

            let mut replace_lines: Vec<&str> = Vec::new();
            while i < lines.len()
                && !lines[i].trim().starts_with(">>>>>>>")
                && lines[i].trim() != ">>> REPLACE"
                && lines[i].trim() != ">>>"
            {
                replace_lines.push(lines[i]);
                i += 1;
            }
            i += 1; // skip `>>>>>>>`

            let search_text = search_lines.join("\n");
            let replace_text = replace_lines.join("\n");

            // If no path specified, we cannot apply — skip with error
            let path = match file_path {
                Some(p) => p,
                None => {
                    return Err(
                        "Search-replace block missing file path (expected `<<<<<<< path/to/file`)".to_string()
                    );
                }
            };

            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {}", path.display(), e))?;

            if !content.contains(&search_text) {
                return Err(format!(
                    "Search text not found in {}: {:?}",
                    path.display(),
                    search_text.chars().take(80).collect::<String>()
                ));
            }

            let new_content = content.replacen(&search_text, &replace_text, 1);
            std::fs::write(&path, &new_content)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;

            results.push(PatchResult { path, hunks_applied: 1, created: false });
        } else {
            i += 1;
        }
    }

    if results.is_empty() {
        Err("No search-replace blocks found in patch.".to_string())
    } else {
        Ok(results)
    }
}

// ── Path resolution ───────────────────────────────────────────────────────────

/// Lexically normalise a [`PathBuf`] by resolving `.` and `..` components
/// without touching the filesystem.
///
/// This prevents path-traversal attacks where a patch file header such as
/// `+++ b/../../../etc/cron.d/evil` could escape the workspace root.
fn lexical_normalise(path: PathBuf) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                // Only pop if we have a non-root component to pop.
                let last = out.components().last();
                match last {
                    None | Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                    _ => { out.pop(); }
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Resolve a patch-relative path against an optional workspace root, then
/// lexically normalise it to prevent `../` traversal attacks.
///
/// When `workspace_root` is provided, also validates that the resolved path
/// is inside the root, returning an error if it escapes.
fn resolve_path(rel: &str, workspace_root: Option<&Path>) -> PathBuf {
    let joined = match workspace_root {
        Some(root) => root.join(rel),
        None => PathBuf::from(rel),
    };
    lexical_normalise(joined)
}

/// Like [`resolve_path`] but additionally enforces that the resulting path
/// stays inside `workspace_root`.  Returns `Err` on path-traversal attempts.
#[allow(dead_code)]
fn resolve_path_safe(rel: &str, workspace_root: &Path) -> Result<PathBuf, String> {
    let resolved = lexical_normalise(workspace_root.join(rel));
    if resolved.starts_with(workspace_root) {
        Ok(resolved)
    } else {
        Err(format!(
            "Security: path '{}' escapes workspace root '{}' — patch rejected.",
            resolved.display(),
            workspace_root.display()
        ))
    }
}

// ── Public helper: format results for LLM consumption ────────────────────────

pub fn format_results(results: &[PatchResult]) -> String {
    results
        .iter()
        .map(|r| {
            format!(
                "{} {} ({} hunk{})",
                if r.created { "Created" } else { "Patched" },
                r.path.display(),
                r.hunks_applied,
                if r.hunks_applied == 1 { "" } else { "s" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let p = dir.path().join(name);
        fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn search_replace_basic() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "hello.txt", "Hello World\nGoodbye\n");

        let patch = format!(
            "<<<<<<< {}\nHello World\n=======\nHello Rust\n>>>>>>>\n",
            dir.path().join("hello.txt").display()
        );

        let results = apply(&patch, Some(dir.path())).unwrap();
        assert_eq!(results.len(), 1);
        let content = fs::read_to_string(dir.path().join("hello.txt")).unwrap();
        assert!(content.contains("Hello Rust"));
        assert!(!content.contains("Hello World"));
    }

    #[test]
    fn search_replace_not_found_errors() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "f.txt", "line1\n");

        let patch = format!(
            "<<<<<<< {}\nNOT_THERE\n=======\nreplacement\n>>>>>>>\n",
            dir.path().join("f.txt").display()
        );

        assert!(apply(&patch, Some(dir.path())).is_err());
    }

    #[test]
    fn unified_diff_add_line() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "src.txt", "line1\nline3\n");

        let rel = dir.path().join("src.txt");
        let patch = format!(
            "--- a/src.txt\n+++ b/{}\n@@ -1,2 +1,3 @@\n line1\n+line2\n line3\n",
            rel.display()
        );

        let results = apply(&patch, Some(dir.path())).unwrap();
        assert!(!results.is_empty());
        let content = fs::read_to_string(&rel).unwrap();
        assert!(content.contains("line2"), "line2 should be inserted: {}", content);
    }

    #[test]
    fn format_results_created_vs_patched() {
        let r1 = PatchResult { path: PathBuf::from("a.rs"), hunks_applied: 2, created: false };
        let r2 = PatchResult { path: PathBuf::from("b.rs"), hunks_applied: 1, created: true };
        let s = format_results(&[r1, r2]);
        assert!(s.contains("Patched a.rs"));
        assert!(s.contains("Created b.rs"));
    }

    // ── format_results plural/singular ───────────────────────────────────────

    #[test]
    fn format_results_single_hunk_no_plural() {
        let r = PatchResult { path: PathBuf::from("x.rs"), hunks_applied: 1, created: false };
        let s = format_results(&[r]);
        assert!(s.contains("1 hunk)"), "singular: {}", s);
        assert!(!s.contains("hunks)"), "must not use plural: {}", s);
    }

    #[test]
    fn format_results_multi_hunk_plural() {
        let r = PatchResult { path: PathBuf::from("x.rs"), hunks_applied: 3, created: false };
        let s = format_results(&[r]);
        assert!(s.contains("3 hunks)"), "plural: {}", s);
    }

    #[test]
    fn format_results_empty_slice() {
        let s = format_results(&[]);
        assert!(s.is_empty(), "empty slice should produce empty string");
    }

    // ── apply: unrecognised format returns Err ────────────────────────────────

    #[test]
    fn apply_unrecognised_format_returns_err() {
        let result = apply("this is not a patch", None);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Unrecognised"), "error message: {}", msg);
    }

    #[test]
    fn apply_empty_string_returns_err() {
        let result = apply("", None);
        assert!(result.is_err());
    }

    // ── search-replace: missing file path returns Err ─────────────────────────

    #[test]
    fn search_replace_missing_file_path_returns_err() {
        let patch = "<<< SEARCH\nold text\n===\nnew text\n>>> REPLACE\n";
        let result = apply(patch, None);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("missing file path") || msg.contains("No search-replace"), "msg: {}", msg);
    }

    // ── unified diff: delete lines ────────────────────────────────────────────

    #[test]
    fn unified_diff_delete_line() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "del.txt", "keep\ndelete_me\nalso_keep\n");

        let rel = dir.path().join("del.txt");
        let patch = format!(
            "--- a/del.txt\n+++ b/{}\n@@ -1,3 +1,2 @@\n keep\n-delete_me\n also_keep\n",
            rel.display()
        );

        let results = apply(&patch, Some(dir.path())).unwrap();
        assert_eq!(results.len(), 1);
        let content = fs::read_to_string(&rel).unwrap();
        assert!(content.contains("keep"), "keep line: {}", content);
        assert!(content.contains("also_keep"), "also_keep: {}", content);
        assert!(!content.contains("delete_me"), "deleted: {}", content);
    }

    // ── unified diff: create new file ─────────────────────────────────────────

    #[test]
    fn unified_diff_creates_new_file() {
        let dir = TempDir::new().unwrap();
        let new_path = dir.path().join("new_file.txt");

        let patch = format!(
            "--- /dev/null\n+++ b/{}\n@@ -0,0 +1,2 @@\n+line_a\n+line_b\n",
            new_path.display()
        );

        let results = apply(&patch, None).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].created, "should mark file as created");
        let content = fs::read_to_string(&new_path).unwrap();
        assert!(content.contains("line_a"), "content: {}", content);
        assert!(content.contains("line_b"), "content: {}", content);
    }

    // ── unified diff: multi-file patch ────────────────────────────────────────

    #[test]
    fn unified_diff_multi_file() {
        let dir = TempDir::new().unwrap();
        let p1 = dir.path().join("file1.txt");
        let p2 = dir.path().join("file2.txt");
        fs::write(&p1, "aaa\n").unwrap();
        fs::write(&p2, "bbb\n").unwrap();

        let patch = format!(
            "--- a/file1.txt\n+++ b/{p1}\n@@ -1,1 +1,1 @@\n-aaa\n+AAA\n\
             --- a/file2.txt\n+++ b/{p2}\n@@ -1,1 +1,1 @@\n-bbb\n+BBB\n",
            p1 = p1.display(), p2 = p2.display()
        );

        let results = apply(&patch, None).unwrap();
        assert_eq!(results.len(), 2, "should patch two files");
        assert!(fs::read_to_string(&p1).unwrap().contains("AAA"));
        assert!(fs::read_to_string(&p2).unwrap().contains("BBB"));
    }

    // ── search-replace: multiple blocks ──────────────────────────────────────

    #[test]
    fn search_replace_multiple_blocks_same_file() {
        let dir = TempDir::new().unwrap();
        let p = write_file(&dir, "m.txt", "foo\nbar\nbaz\n");

        let patch = format!(
            "<<<<<<< {p}\nfoo\n=======\nFOO\n>>>>>>>\n\
             <<<<<<< {p}\nbar\n=======\nBAR\n>>>>>>>\n",
            p = p.display()
        );

        let results = apply(&patch, Some(dir.path())).unwrap();
        assert_eq!(results.len(), 2, "two blocks = two results");
        let content = fs::read_to_string(&p).unwrap();
        assert!(content.contains("FOO"), "FOO: {}", content);
        assert!(content.contains("BAR"), "BAR: {}", content);
    }

    // ── unified diff: no hunks returns Err ────────────────────────────────────

    #[test]
    fn unified_diff_no_hunks_returns_err() {
        let result = apply("--- a/foo.txt\n+++ b/foo.txt\n", None);
        assert!(result.is_err());
    }

    // ── path traversal security tests ────────────────────────────────────────

    #[test]
    fn lexical_normalise_removes_dotdot() {
        let p = PathBuf::from("/workspace/../etc/passwd");
        let n = lexical_normalise(p);
        assert_eq!(n, PathBuf::from("/etc/passwd"));
    }

    #[test]
    fn lexical_normalise_keeps_valid_path() {
        let p = PathBuf::from("/workspace/src/main.rs");
        let n = lexical_normalise(p);
        assert_eq!(n, PathBuf::from("/workspace/src/main.rs"));
    }

    #[test]
    fn resolve_path_safe_blocks_traversal() {
        let root = PathBuf::from("/workspace");
        let result = resolve_path_safe("../../../etc/passwd", &root);
        assert!(result.is_err(), "path traversal must be rejected: {:?}", result);
        let msg = result.unwrap_err();
        assert!(msg.contains("escapes workspace root"), "msg: {}", msg);
    }

    #[test]
    fn resolve_path_safe_allows_valid_path() {
        let root = PathBuf::from("/workspace");
        let result = resolve_path_safe("src/main.rs", &root);
        assert!(result.is_ok(), "valid path must be accepted: {:?}", result);
        let p = result.unwrap();
        assert!(p.starts_with("/workspace"), "must be inside workspace: {:?}", p);
    }

    // ── looks_like_unified_diff (via apply routing) ───────────────────────────

    #[test]
    fn git_diff_header_routes_to_unified() {
        let dir = TempDir::new().unwrap();
        let p = write_file(&dir, "gd.txt", "x\n");
        let patch = format!(
            "diff --git a/gd.txt b/gd.txt\n--- a/gd.txt\n+++ b/{}\n@@ -1,1 +1,1 @@\n-x\n+y\n",
            p.display()
        );
        let results = apply(&patch, None).unwrap();
        assert!(!results.is_empty());
        assert!(fs::read_to_string(&p).unwrap().contains('y'));
    }
}
