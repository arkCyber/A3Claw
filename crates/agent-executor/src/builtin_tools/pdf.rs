//! `pdf` — built-in PDF analysis tool (stub implementation).
//!
//! Reads a PDF file from disk and extracts structural metadata
//! without a full parsing library.  For real text extraction,
//! register a `SkillHandler` backed by `lopdf`, `pdfium`, or similar.
//!
//! Supported operations (via `action` arg):
//! - `text`       — extract raw text (best-effort, ASCII/UTF-8 streams only)
//! - `metadata`   — return Title, Author, CreationDate, etc. from the info dict
//! - `page_count` — return the number of pages (default when no action given)

use std::collections::HashMap;

// ── Arg parsing ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PdfArgs {
    pub path:   String,
    pub action: PdfAction,
    /// For `text`: maximum characters to return (default 4000).
    pub max_chars: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfAction {
    Text,
    Metadata,
    PageCount,
}

impl PdfArgs {
    pub fn from_json(args: &serde_json::Value) -> Result<Self, String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| "missing 'path' argument".to_string())?
            .to_string();

        let action = match args["action"].as_str().unwrap_or("page_count") {
            "text"       => PdfAction::Text,
            "metadata"   => PdfAction::Metadata,
            "page_count" | "" => PdfAction::PageCount,
            other => return Err(format!("unknown pdf action '{}'; use text|metadata|page_count", other)),
        };

        let max_chars = args["maxChars"].as_u64().unwrap_or(4000) as usize;

        Ok(Self { path, action, max_chars })
    }
}

// ── Core implementation ───────────────────────────────────────────────────────

/// Analyse a PDF file.  Returns a human-readable summary string.
///
/// This is a **best-effort stub** that parses raw PDF bytes for:
/// - Cross-reference / page count heuristic
/// - Info dictionary key-value pairs
/// - Printable ASCII text embedded in stream objects
///
/// For production quality, register a `SkillHandler` with a proper PDF library.
pub fn analyze_pdf(args: &PdfArgs) -> Result<String, String> {
    let bytes = std::fs::read(&args.path)
        .map_err(|e| format!("cannot read '{}': {}", args.path, e))?;

    if !bytes.starts_with(b"%PDF") {
        return Err(format!("'{}' does not appear to be a PDF file (missing %PDF header)", args.path));
    }

    match args.action {
        PdfAction::PageCount => {
            let count = estimate_page_count(&bytes);
            Ok(format!("PDF '{}': ~{} page(s) (estimated).", args.path, count))
        }
        PdfAction::Metadata => {
            let meta = extract_info_dict(&bytes);
            if meta.is_empty() {
                Ok(format!("PDF '{}': no Info dictionary found.", args.path))
            } else {
                let lines: Vec<String> = meta
                    .iter()
                    .map(|(k, v)| format!("  {}: {}", k, v))
                    .collect();
                Ok(format!("PDF '{}' metadata:\n{}", args.path, lines.join("\n")))
            }
        }
        PdfAction::Text => {
            let text = extract_text(&bytes, args.max_chars);
            if text.trim().is_empty() {
                Ok(format!(
                    "(pdf text: no readable text streams found in '{}'. \
                     Register a SkillHandler with lopdf/pdfium for full extraction.)",
                    args.path
                ))
            } else {
                Ok(format!("PDF '{}' text (up to {} chars):\n{}", args.path, args.max_chars, text))
            }
        }
    }
}

// ── Heuristic helpers ─────────────────────────────────────────────────────────

/// Count `/Page` objects as a proxy for page count.
fn estimate_page_count(bytes: &[u8]) -> usize {
    // Every page has a `/Type /Page` entry (not `/Pages`).
    // We look for the byte sequence `/Type /Page\n`, `/Type /Page\r`, or `/Type /Page ` etc.
    let needle = b"/Type /Page";
    let mut count = 0usize;
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            // Make sure it's not `/Type /Pages` (the page tree node)
            let next = bytes.get(i + needle.len()).copied().unwrap_or(b' ');
            if next != b's' {
                count += 1;
            }
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count.max(1) // at least 1 page if we found a valid PDF header
}

/// Extract string values from the PDF Info dictionary (very simplified).
fn extract_info_dict(bytes: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let text = String::from_utf8_lossy(bytes);

    // Info dict keys we care about
    let keys = [
        "Title", "Author", "Subject", "Keywords",
        "Creator", "Producer", "CreationDate", "ModDate",
    ];

    for key in &keys {
        let needle = format!("/{} (", key);
        if let Some(start) = text.find(&needle) {
            let after = &text[start + needle.len()..];
            // PDF strings end at `)` (ignoring escaped parens for simplicity)
            if let Some(end) = after.find(')') {
                let value = after[..end].trim().to_string();
                if !value.is_empty() {
                    map.insert(key.to_string(), value);
                }
            }
        }
    }
    map
}

/// Extract printable ASCII text from PDF stream objects.
fn extract_text(bytes: &[u8], max_chars: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    let mut in_stream = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "stream" {
            in_stream = true;
            continue;
        }
        if trimmed == "endstream" {
            in_stream = false;
            continue;
        }
        if in_stream {
            // Keep lines that look like text operators: Tj, TJ, ', "
            // or parenthesised literal strings
            if trimmed.ends_with("Tj")
                || trimmed.ends_with("TJ")
                || trimmed.starts_with('(')
            {
                let printable: String = trimmed
                    .chars()
                    .filter(|c| c.is_ascii_graphic() || *c == ' ')
                    .collect();
                if !printable.is_empty() {
                    out.push_str(&printable);
                    out.push('\n');
                    if out.len() >= max_chars {
                        break;
                    }
                }
            }
        }
    }

    out.chars().take(max_chars).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_pdf(extra: &[u8]) -> Vec<u8> {
        let mut v = b"%PDF-1.4\n".to_vec();
        v.extend_from_slice(extra);
        v
    }

    #[test]
    fn from_json_requires_path() {
        let err = PdfArgs::from_json(&serde_json::json!({})).unwrap_err();
        assert!(err.contains("path"));
    }

    #[test]
    fn from_json_defaults_to_page_count() {
        let a = PdfArgs::from_json(&serde_json::json!({"path": "/tmp/x.pdf"})).unwrap();
        assert_eq!(a.action, PdfAction::PageCount);
    }

    #[test]
    fn from_json_unknown_action_errors() {
        let err = PdfArgs::from_json(
            &serde_json::json!({"path": "/tmp/x.pdf", "action": "bogus"})
        ).unwrap_err();
        assert!(err.contains("bogus"));
    }

    #[test]
    fn estimate_page_count_single() {
        let pdf = minimal_pdf(b"/Type /Page\n");
        assert_eq!(estimate_page_count(&pdf), 1);
    }

    #[test]
    fn estimate_page_count_skips_pages_node() {
        let pdf = minimal_pdf(b"/Type /Pages\n/Type /Page\n/Type /Page\n");
        assert_eq!(estimate_page_count(&pdf), 2);
    }

    #[test]
    fn extract_info_dict_finds_title() {
        let pdf = minimal_pdf(b"/Title (Hello World)\n");
        let meta = extract_info_dict(&pdf);
        assert_eq!(meta.get("Title").map(|s| s.as_str()), Some("Hello World"));
    }

    #[test]
    fn analyze_pdf_rejects_non_pdf() {
        let args = PdfArgs { path: "/dev/null".into(), action: PdfAction::PageCount, max_chars: 1000 };
        let err = analyze_pdf(&args).unwrap_err();
        assert!(err.contains("does not appear"));
    }

    #[test]
    fn analyze_pdf_missing_file_errors() {
        let args = PdfArgs {
            path: "/nonexistent/__test_openclaw_pdf__.pdf".into(),
            action: PdfAction::PageCount,
            max_chars: 1000,
        };
        let err = analyze_pdf(&args).unwrap_err();
        assert!(err.contains("cannot read"));
    }
}
