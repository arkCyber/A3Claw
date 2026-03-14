//! `document.convert` — document format conversion via Pandoc (graceful fallback).
//!
//! Wraps the system `pandoc` binary. If pandoc is not installed the skill returns
//! a clear error rather than silently failing. Supported conversions include:
//! - Markdown / HTML / RST  → PDF, DOCX, EPUB, HTML, plain text, LaTeX
//! - Any pandoc input/output format pair
//!
//! PDF generation requires either `pandoc` + `pdflatex` / `xelatex` or
//! `pandoc` + `wkhtmltopdf` (via `--pdf-engine`). The skill detects which
//! engine is available and chooses appropriately.

use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_TIMEOUT_SECS: u64 = 300;

// ── Types ─────────────────────────────────────────────────────────────────────

pub struct DocumentConvertArgs {
    /// Source file path (mutually exclusive with `content`).
    pub input_path: Option<String>,
    /// Inline source content (mutually exclusive with `input_path`).
    pub content: Option<String>,
    /// Pandoc input format (e.g. "markdown", "html", "rst"). Auto-detected if absent.
    pub from: Option<String>,
    /// Pandoc output format (e.g. "docx", "pdf", "html", "plain"). Required.
    pub to: String,
    /// Output file path. If omitted, stdout is captured and returned inline.
    pub output_path: Option<String>,
    /// Extra pandoc CLI arguments (passed verbatim).
    pub extra_args: Vec<String>,
    pub timeout_secs: u64,
}

impl DocumentConvertArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        let input_path = v["input_path"].as_str().map(|s| s.to_string());
        let content = v["content"].as_str().map(|s| s.to_string());

        if input_path.is_none() && content.is_none() {
            return Err("document.convert: one of 'input_path' or 'content' is required".into());
        }
        if input_path.is_some() && content.is_some() {
            return Err("document.convert: 'input_path' and 'content' are mutually exclusive".into());
        }

        let to = v["to"]
            .as_str()
            .ok_or("document.convert: 'to' (output format) is required")?
            .to_string();

        let extra_args = v["extra_args"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        Ok(Self {
            input_path,
            content,
            from: v["from"].as_str().map(|s| s.to_string()),
            to,
            output_path: v["output_path"].as_str().map(|s| s.to_string()),
            extra_args,
            timeout_secs: v["timeout_secs"]
                .as_u64()
                .unwrap_or(DEFAULT_TIMEOUT_SECS)
                .min(MAX_TIMEOUT_SECS),
        })
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn convert(args: &DocumentConvertArgs) -> Result<String, String> {
    verify_pandoc()?;

    let tmp_input: Option<std::path::PathBuf>;
    let input_path_ref: std::path::PathBuf;

    // If inline content, write to a temp file so pandoc can read it.
    if let Some(content) = &args.content {
        let ext = args.from.as_deref().unwrap_or("md");
        let tmp = std::env::temp_dir().join(format!("openclaw_doc_{}.{}", std::process::id(), ext));
        std::fs::write(&tmp, content.as_bytes())
            .map_err(|e| format!("document.convert: failed to write temp input: {}", e))?;
        tmp_input = Some(tmp.clone());
        input_path_ref = tmp;
    } else {
        tmp_input = None;
        let input_path = args.input_path.as_deref()
            .ok_or_else(|| "document.convert: either 'content' or 'input_path' must be provided".to_string())?;
        input_path_ref = std::path::PathBuf::from(input_path);
    }

    let mut cmd = std::process::Command::new("pandoc");
    cmd.arg(&input_path_ref);

    if let Some(from) = &args.from {
        cmd.arg("--from").arg(from);
    }
    cmd.arg("--to").arg(&args.to);

    // PDF engine selection: prefer xelatex (better Unicode), fall back to pdflatex.
    if args.to == "pdf" {
        let engine = detect_pdf_engine();
        cmd.arg("--pdf-engine").arg(engine);
    }

    if let Some(out) = &args.output_path {
        cmd.arg("--output").arg(out);
    } else {
        cmd.arg("--output").arg("-"); // write to stdout
    }

    for extra in &args.extra_args {
        cmd.arg(extra);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let result = run_subprocess(cmd, args.timeout_secs);

    // Clean up temp input file.
    if let Some(tmp) = tmp_input {
        let _ = std::fs::remove_file(tmp);
    }

    match result {
        Ok(output) => {
            if let Some(out_path) = &args.output_path {
                Ok(format!("Converted to {} — output written to {}", args.to, out_path))
            } else {
                Ok(output)
            }
        }
        Err(e) => Err(e),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn verify_pandoc() -> Result<(), String> {
    let ok = std::process::Command::new("which")
        .arg("pandoc")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err("document.convert: 'pandoc' not found in PATH — install it with: brew install pandoc".into())
    }
}

fn detect_pdf_engine() -> &'static str {
    for engine in &["xelatex", "pdflatex", "lualatex", "wkhtmltopdf"] {
        let found = std::process::Command::new("which")
            .arg(engine)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if found {
            return engine;
        }
    }
    "pdflatex"
}

fn run_subprocess(mut cmd: std::process::Command, timeout_secs: u64) -> Result<String, String> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut child = cmd.spawn().map_err(|e| format!("document.convert: failed to spawn pandoc: {}", e))?;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = drain(child.stdout.take());
                let stderr = drain(child.stderr.take());
                if status.success() {
                    return Ok(String::from_utf8_lossy(&stdout).to_string());
                } else {
                    return Err(format!(
                        "document.convert: pandoc exited with {}: {}",
                        status.code().unwrap_or(-1),
                        String::from_utf8_lossy(&stderr).trim()
                    ));
                }
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(format!("document.convert: timed out after {}s", timeout_secs));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("document.convert: wait error: {}", e)),
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_json_neither_input_errors() {
        assert!(DocumentConvertArgs::from_json(&serde_json::json!({"to": "html"})).is_err());
    }

    #[test]
    fn from_json_both_inputs_errors() {
        let v = serde_json::json!({"input_path": "/a.md", "content": "# hi", "to": "html"});
        assert!(DocumentConvertArgs::from_json(&v).is_err());
    }

    #[test]
    fn from_json_missing_to_errors() {
        let v = serde_json::json!({"content": "# hi"});
        assert!(DocumentConvertArgs::from_json(&v).is_err());
    }

    #[test]
    fn from_json_content_ok() {
        let v = serde_json::json!({"content": "# Title", "to": "html"});
        let a = DocumentConvertArgs::from_json(&v).unwrap();
        assert_eq!(a.to, "html");
        assert_eq!(a.content.as_deref(), Some("# Title"));
        assert!(a.input_path.is_none());
    }

    #[test]
    fn from_json_input_path_ok() {
        let v = serde_json::json!({"input_path": "/doc.md", "to": "docx"});
        let a = DocumentConvertArgs::from_json(&v).unwrap();
        assert_eq!(a.input_path.as_deref(), Some("/doc.md"));
        assert!(a.content.is_none());
    }

    #[test]
    fn from_json_timeout_capped() {
        let v = serde_json::json!({"content": "x", "to": "html", "timeout_secs": 9999});
        let a = DocumentConvertArgs::from_json(&v).unwrap();
        assert_eq!(a.timeout_secs, MAX_TIMEOUT_SECS);
    }

    #[test]
    fn from_json_extra_args_parsed() {
        let v = serde_json::json!({"content": "x", "to": "html", "extra_args": ["--standalone", "--toc"]});
        let a = DocumentConvertArgs::from_json(&v).unwrap();
        assert_eq!(a.extra_args, vec!["--standalone", "--toc"]);
    }

    #[test]
    #[ignore = "requires pandoc on PATH"]
    fn convert_markdown_to_html() {
        let a = DocumentConvertArgs {
            content: Some("# Hello\n\nWorld.".to_string()),
            input_path: None,
            from: Some("markdown".to_string()),
            to: "html".to_string(),
            output_path: None,
            extra_args: vec![],
            timeout_secs: 10,
        };
        let result = convert(&a).unwrap();
        assert!(result.contains("<h1"), "result: {result}");
    }

    #[test]
    #[ignore = "requires pandoc on PATH"]
    fn convert_to_file_reports_path() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.html");
        let a = DocumentConvertArgs {
            content: Some("# Test".to_string()),
            input_path: None,
            from: Some("markdown".to_string()),
            to: "html".to_string(),
            output_path: Some(out.to_string_lossy().to_string()),
            extra_args: vec![],
            timeout_secs: 10,
        };
        let result = convert(&a).unwrap();
        assert!(result.contains("output written to"), "result: {result}");
        assert!(out.exists());
    }
}
