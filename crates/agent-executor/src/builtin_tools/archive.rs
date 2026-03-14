//! `fs.archive` / `fs.extract` — file compression and decompression.
//!
//! Delegates to system `zip`/`unzip` or `tar` binaries (zero new dependencies).
//! Format is auto-detected from the output/input path extension.
//!
//! ## Supported formats
//! | extension         | tool used |
//! |-------------------|-----------|
//! | `.zip`            | zip / unzip |
//! | `.tar`            | tar cf / tar xf |
//! | `.tar.gz` `.tgz`  | tar czf / tar xzf |
//! | `.tar.bz2` `.tbz` | tar cjf / tar xjf |
//! | `.tar.xz` `.txz`  | tar cJf / tar xJf |

use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const MAX_TIMEOUT_SECS: u64 = 600;

// ── Argument types ────────────────────────────────────────────────────────────

pub struct ArchiveArgs {
    /// Paths to include (files and/or directories).
    pub sources: Vec<String>,
    /// Output archive file path (extension determines format).
    pub dest: String,
    pub timeout_secs: u64,
}

pub struct ExtractArgs {
    /// Archive file to extract.
    pub archive: String,
    /// Directory to extract into (created if absent, default: same dir as archive).
    pub dest: Option<String>,
    pub timeout_secs: u64,
}

impl ArchiveArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        let dest = v["dest"]
            .as_str()
            .ok_or("fs.archive: missing 'dest' (output archive path)")?
            .to_string();

        let sources: Vec<String> = match v["sources"].as_array() {
            Some(arr) if !arr.is_empty() => arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect(),
            _ => return Err("fs.archive: 'sources' must be a non-empty array of paths".into()),
        };

        Ok(Self {
            sources,
            dest,
            timeout_secs: v["timeout_secs"]
                .as_u64()
                .unwrap_or(DEFAULT_TIMEOUT_SECS)
                .min(MAX_TIMEOUT_SECS),
        })
    }
}

impl ExtractArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        let archive = v["archive"]
            .as_str()
            .ok_or("fs.extract: missing 'archive' path")?
            .to_string();
        Ok(Self {
            archive,
            dest: v["dest"].as_str().map(|s| s.to_string()),
            timeout_secs: v["timeout_secs"]
                .as_u64()
                .unwrap_or(DEFAULT_TIMEOUT_SECS)
                .min(MAX_TIMEOUT_SECS),
        })
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn archive(args: &ArchiveArgs) -> Result<String, String> {
    let dest_path = std::path::Path::new(&args.dest);
    let format = detect_format(dest_path)?;

    // Create parent directory if needed.
    if let Some(parent) = dest_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("fs.archive: cannot create parent dir: {}", e))?;
        }
    }

    let mut cmd = match format {
        ArchiveFormat::Zip => {
            verify_binary("zip")?;
            let mut c = std::process::Command::new("zip");
            c.arg("-r").arg(&args.dest);
            for src in &args.sources { c.arg(src); }
            c
        }
        ArchiveFormat::Tar => build_tar_create_cmd(&args.dest, &args.sources, "cf")?,
        ArchiveFormat::TarGz => build_tar_create_cmd(&args.dest, &args.sources, "czf")?,
        ArchiveFormat::TarBz2 => build_tar_create_cmd(&args.dest, &args.sources, "cjf")?,
        ArchiveFormat::TarXz => build_tar_create_cmd(&args.dest, &args.sources, "cJf")?,
    };

    run_subprocess(&mut cmd, args.timeout_secs, "fs.archive").map(|_| {
        format!(
            "Archive created: {} ({} source(s))",
            args.dest,
            args.sources.len()
        )
    })
}

pub fn extract(args: &ExtractArgs) -> Result<String, String> {
    let archive_path = std::path::Path::new(&args.archive);
    if !archive_path.exists() {
        return Err(format!("fs.extract: archive not found: {}", args.archive));
    }
    let format = detect_format(archive_path)?;

    let dest = args.dest.clone().unwrap_or_else(|| {
        archive_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string())
    });

    std::fs::create_dir_all(&dest)
        .map_err(|e| format!("fs.extract: cannot create dest dir '{}': {}", dest, e))?;

    let mut cmd = match format {
        ArchiveFormat::Zip => {
            verify_binary("unzip")?;
            let mut c = std::process::Command::new("unzip");
            c.arg("-o").arg(&args.archive).arg("-d").arg(&dest);
            c
        }
        ArchiveFormat::Tar => build_tar_extract_cmd(&args.archive, &dest, "xf")?,
        ArchiveFormat::TarGz => build_tar_extract_cmd(&args.archive, &dest, "xzf")?,
        ArchiveFormat::TarBz2 => build_tar_extract_cmd(&args.archive, &dest, "xjf")?,
        ArchiveFormat::TarXz => build_tar_extract_cmd(&args.archive, &dest, "xJf")?,
    };

    run_subprocess(&mut cmd, args.timeout_secs, "fs.extract")
        .map(|_| format!("Extracted '{}' to '{}'", args.archive, dest))
}

// ── Format detection ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
}

fn detect_format(path: &std::path::Path) -> Result<ArchiveFormat, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Ok(ArchiveFormat::TarGz)
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz") || name.ends_with(".tbz2") {
        Ok(ArchiveFormat::TarBz2)
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        Ok(ArchiveFormat::TarXz)
    } else if name.ends_with(".tar") {
        Ok(ArchiveFormat::Tar)
    } else if name.ends_with(".zip") {
        Ok(ArchiveFormat::Zip)
    } else {
        Err(format!(
            "archive: unrecognized format for '{}' — use .zip, .tar, .tar.gz, .tar.bz2, or .tar.xz",
            path.display()
        ))
    }
}

// ── Command builders ──────────────────────────────────────────────────────────

fn build_tar_create_cmd(dest: &str, sources: &[String], flags: &str) -> Result<std::process::Command, String> {
    verify_binary("tar")?;
    let mut c = std::process::Command::new("tar");
    c.arg(flags).arg(dest);
    for src in sources { c.arg(src); }
    Ok(c)
}

fn build_tar_extract_cmd(archive: &str, dest: &str, flags: &str) -> Result<std::process::Command, String> {
    verify_binary("tar")?;
    let mut c = std::process::Command::new("tar");
    c.arg(flags).arg(archive).arg("-C").arg(dest);
    Ok(c)
}

// ── Subprocess runner ─────────────────────────────────────────────────────────

fn run_subprocess(
    cmd: &mut std::process::Command,
    timeout_secs: u64,
    skill: &str,
) -> Result<(), String> {
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{skill}: failed to spawn: {e}"))?;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let stderr = drain(child.stderr.take());
                return Err(format!(
                    "{skill}: exited with {}: {}",
                    status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&stderr).trim()
                ));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(format!("{skill}: timed out after {timeout_secs}s"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("{skill}: wait error: {e}")),
        }
    }
}

fn drain(stream: Option<impl std::io::Read>) -> Vec<u8> {
    stream
        .map(|mut r| {
            let mut b = Vec::new();
            let _ = std::io::Read::read_to_end(&mut r, &mut b);
            b
        })
        .unwrap_or_default()
}

fn verify_binary(name: &str) -> Result<(), String> {
    let ok = std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(format!("archive: '{name}' not found in PATH"))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ArchiveArgs::from_json ────────────────────────────────────────────

    #[test]
    fn archive_args_missing_dest_errors() {
        let v = serde_json::json!({"sources": ["/tmp/a"]});
        assert!(ArchiveArgs::from_json(&v).is_err());
    }

    #[test]
    fn archive_args_missing_sources_errors() {
        let v = serde_json::json!({"dest": "/tmp/out.zip"});
        assert!(ArchiveArgs::from_json(&v).is_err());
    }

    #[test]
    fn archive_args_empty_sources_errors() {
        let v = serde_json::json!({"dest": "/tmp/out.zip", "sources": []});
        assert!(ArchiveArgs::from_json(&v).is_err());
    }

    #[test]
    fn archive_args_ok() {
        let v = serde_json::json!({"dest": "/tmp/out.tar.gz", "sources": ["/tmp/a", "/tmp/b"]});
        let a = ArchiveArgs::from_json(&v).unwrap();
        assert_eq!(a.dest, "/tmp/out.tar.gz");
        assert_eq!(a.sources.len(), 2);
    }

    #[test]
    fn archive_args_timeout_capped() {
        let v = serde_json::json!({"dest": "/tmp/o.zip", "sources": ["/a"], "timeout_secs": 9999});
        let a = ArchiveArgs::from_json(&v).unwrap();
        assert_eq!(a.timeout_secs, MAX_TIMEOUT_SECS);
    }

    // ── ExtractArgs::from_json ────────────────────────────────────────────

    #[test]
    fn extract_args_missing_archive_errors() {
        let v = serde_json::json!({"dest": "/tmp/out"});
        assert!(ExtractArgs::from_json(&v).is_err());
    }

    #[test]
    fn extract_args_ok_no_dest() {
        let v = serde_json::json!({"archive": "/tmp/a.zip"});
        let a = ExtractArgs::from_json(&v).unwrap();
        assert_eq!(a.archive, "/tmp/a.zip");
        assert!(a.dest.is_none());
    }

    #[test]
    fn extract_args_ok_with_dest() {
        let v = serde_json::json!({"archive": "/tmp/a.tar.gz", "dest": "/out"});
        let a = ExtractArgs::from_json(&v).unwrap();
        assert_eq!(a.dest.as_deref(), Some("/out"));
    }

    // ── detect_format ─────────────────────────────────────────────────────

    #[test]
    fn detect_zip() {
        assert_eq!(detect_format(std::path::Path::new("a.zip")).unwrap(), ArchiveFormat::Zip);
    }

    #[test]
    fn detect_tar() {
        assert_eq!(detect_format(std::path::Path::new("a.tar")).unwrap(), ArchiveFormat::Tar);
    }

    #[test]
    fn detect_tar_gz() {
        assert_eq!(detect_format(std::path::Path::new("a.tar.gz")).unwrap(), ArchiveFormat::TarGz);
        assert_eq!(detect_format(std::path::Path::new("a.tgz")).unwrap(), ArchiveFormat::TarGz);
    }

    #[test]
    fn detect_tar_bz2() {
        assert_eq!(detect_format(std::path::Path::new("a.tar.bz2")).unwrap(), ArchiveFormat::TarBz2);
        assert_eq!(detect_format(std::path::Path::new("a.tbz")).unwrap(), ArchiveFormat::TarBz2);
    }

    #[test]
    fn detect_tar_xz() {
        assert_eq!(detect_format(std::path::Path::new("a.tar.xz")).unwrap(), ArchiveFormat::TarXz);
        assert_eq!(detect_format(std::path::Path::new("a.txz")).unwrap(), ArchiveFormat::TarXz);
    }

    #[test]
    fn detect_unknown_errors() {
        assert!(detect_format(std::path::Path::new("a.rar")).is_err());
        assert!(detect_format(std::path::Path::new("file.txt")).is_err());
    }

    // ── integration (requires tar/zip/unzip on PATH) ──────────────────────

    #[test]
    #[ignore = "requires tar on PATH"]
    fn archive_and_extract_tar_gz_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("hello.txt");
        std::fs::write(&src, b"hello openclaw").unwrap();

        let archive_path = dir.path().join("out.tar.gz");
        let args = ArchiveArgs {
            sources: vec![src.to_string_lossy().to_string()],
            dest: archive_path.to_string_lossy().to_string(),
            timeout_secs: 10,
        };
        archive(&args).unwrap();
        assert!(archive_path.exists());

        let out_dir = dir.path().join("extracted");
        let xargs = ExtractArgs {
            archive: archive_path.to_string_lossy().to_string(),
            dest: Some(out_dir.to_string_lossy().to_string()),
            timeout_secs: 10,
        };
        extract(&xargs).unwrap();
        assert!(out_dir.exists());
    }

    #[test]
    fn extract_nonexistent_archive_errors() {
        let args = ExtractArgs {
            archive: "/nonexistent/path/file.tar.gz".to_string(),
            dest: None,
            timeout_secs: 5,
        };
        assert!(extract(&args).is_err());
    }
}
