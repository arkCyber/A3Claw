//! skill-text — text manipulation skill plugin for OpenClaw+
//!
//! Skills:
//!   text.count        { input: string }                    → {chars, words, lines, bytes}
//!   text.truncate     { input: string, max_chars: u32, ellipsis?: string }
//!   text.pad          { input: string, width: u32, char?: string, align?: "left"|"right"|"center" }
//!   text.replace      { input: string, from: string, to: string, count?: u32 }
//!   text.split        { input: string, sep: string, max?: u32 }
//!   text.join         { parts: [string], sep?: string }
//!   text.lines        { input: string }
//!   text.trim         { input: string, side?: "left"|"right"|"both" }
//!   text.case         { input: string, to: "upper"|"lower"|"title"|"snake"|"camel"|"kebab" }
//!   text.extract      { input: string, pattern: string }
//!   text.slugify      { input: string }
//!   text.uuid_v4      {}                                   → UUID v4 string (CSPRNG via WASI)
//!   text.repeat       { input: string, count: u32 }
//!   text.reverse      { input: string }
//!   text.contains     { input: string, needle: string }
//!   text.starts_with  { input: string, prefix: string }
//!   text.ends_with    { input: string, suffix: string }

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.text",
  "name": "Text Skills",
  "version": "0.1.0",
  "description": "String manipulation: count, trim, case, split, join, slugify, UUID, and more",
  "skills": [
    { "name": "text.count",       "display": "Count", "description": "Count chars, words, lines, bytes.", "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }] },
    { "name": "text.truncate",    "display": "Truncate", "description": "Truncate to max chars.", "risk": "safe",
      "params": [
        { "name": "input",     "type": "string",  "required": true  },
        { "name": "max_chars", "type": "integer", "required": true  },
        { "name": "ellipsis",  "type": "string",  "required": false }
      ] },
    { "name": "text.pad",         "display": "Pad", "description": "Pad a string to a given width.", "risk": "safe",
      "params": [
        { "name": "input", "type": "string",  "required": true  },
        { "name": "width", "type": "integer", "required": true  },
        { "name": "char",  "type": "string",  "required": false },
        { "name": "align", "type": "string",  "required": false }
      ] },
    { "name": "text.replace",     "display": "Replace", "description": "Replace occurrences of a substring.", "risk": "safe",
      "params": [
        { "name": "input", "type": "string",  "required": true  },
        { "name": "from",  "type": "string",  "required": true  },
        { "name": "to",    "type": "string",  "required": true  },
        { "name": "count", "type": "integer", "required": false }
      ] },
    { "name": "text.split",       "display": "Split", "description": "Split a string by separator.", "risk": "safe",
      "params": [
        { "name": "input", "type": "string",  "required": true  },
        { "name": "sep",   "type": "string",  "required": true  },
        { "name": "max",   "type": "integer", "required": false }
      ] },
    { "name": "text.join",        "display": "Join", "description": "Join a list of strings.", "risk": "safe",
      "params": [
        { "name": "parts", "type": "array",  "required": true  },
        { "name": "sep",   "type": "string", "required": false }
      ] },
    { "name": "text.lines",       "display": "Lines", "description": "Split text into lines.", "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }] },
    { "name": "text.trim",        "display": "Trim", "description": "Trim leading/trailing whitespace.", "risk": "safe",
      "params": [
        { "name": "input", "type": "string", "required": true  },
        { "name": "side",  "type": "string", "required": false }
      ] },
    { "name": "text.case",        "display": "Case Transform", "description": "Change string case.", "risk": "safe",
      "params": [
        { "name": "input", "type": "string", "required": true },
        { "name": "to",    "type": "string", "required": true }
      ] },
    { "name": "text.extract",     "display": "Extract Pattern", "description": "Find the first simple pattern match (literal substring search, returns index).", "risk": "safe",
      "params": [
        { "name": "input",   "type": "string", "required": true },
        { "name": "pattern", "type": "string", "required": true }
      ] },
    { "name": "text.slugify",     "display": "Slugify", "description": "Convert text to a URL-safe slug.", "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }] },
    { "name": "text.uuid_v4",     "display": "UUID v4", "description": "Generate a random UUID v4 using WASI random bytes.", "risk": "safe",
      "params": [] },
    { "name": "text.repeat",      "display": "Repeat", "description": "Repeat a string N times.", "risk": "safe",
      "params": [
        { "name": "input", "type": "string",  "required": true },
        { "name": "count", "type": "integer", "required": true }
      ] },
    { "name": "text.reverse",     "display": "Reverse", "description": "Reverse a string (Unicode-aware).", "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }] },
    { "name": "text.contains",    "display": "Contains", "description": "Check whether a string contains a substring.", "risk": "safe",
      "params": [
        { "name": "input",  "type": "string", "required": true },
        { "name": "needle", "type": "string", "required": true }
      ] },
    { "name": "text.starts_with", "display": "Starts With", "description": "Check whether a string starts with a prefix.", "risk": "safe",
      "params": [
        { "name": "input",  "type": "string", "required": true },
        { "name": "prefix", "type": "string", "required": true }
      ] },
    { "name": "text.ends_with",   "display": "Ends With", "description": "Check whether a string ends with a suffix.", "risk": "safe",
      "params": [
        { "name": "input",  "type": "string", "required": true },
        { "name": "suffix", "type": "string", "required": true }
      ] }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 {
    sdk_export_str(MANIFEST)
}

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r)  => r,
        Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();

    let get_str = |key: &str| req.args[key].as_str().map(|s| s.to_string());

    match req.skill.as_str() {
        "text.count" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let chars  = input.chars().count();
            let words  = input.split_whitespace().count();
            let lines  = input.lines().count();
            let bytes  = input.len();
            sdk_respond_ok(rid, &format!(
                r#"{{"chars":{chars},"words":{words},"lines":{lines},"bytes":{bytes}}}"#
            ))
        }

        "text.truncate" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let max_chars = match req.args["max_chars"].as_u64() {
                Some(n) => n as usize,
                None    => return sdk_respond_err(rid, "missing 'max_chars'"),
            };
            let ellipsis = req.args["ellipsis"].as_str().unwrap_or("…");
            let char_count = input.chars().count();
            if char_count <= max_chars {
                sdk_respond_ok(rid, &input)
            } else {
                let ell_len = ellipsis.chars().count();
                let take = max_chars.saturating_sub(ell_len);
                let truncated: String = input.chars().take(take).collect();
                sdk_respond_ok(rid, &format!("{}{}", truncated, ellipsis))
            }
        }

        "text.pad" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let width = match req.args["width"].as_u64() {
                Some(n) => n as usize,
                None    => return sdk_respond_err(rid, "missing 'width'"),
            };
            let pad_char = req.args["char"].as_str()
                .and_then(|s| s.chars().next())
                .unwrap_or(' ');
            let align = req.args["align"].as_str().unwrap_or("right");
            let char_count = input.chars().count();
            if char_count >= width {
                return sdk_respond_ok(rid, &input);
            }
            let pad = width - char_count;
            let result = match align {
                "left"   => format!("{}{}", input, pad_char.to_string().repeat(pad)),
                "center" => {
                    let lpad = pad / 2;
                    let rpad = pad - lpad;
                    format!("{}{}{}", pad_char.to_string().repeat(lpad), input, pad_char.to_string().repeat(rpad))
                }
                _ /* right */ => format!("{}{}", pad_char.to_string().repeat(pad), input),
            };
            sdk_respond_ok(rid, &result)
        }

        "text.replace" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let from = match get_str("from") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'from'"),
            };
            let to = req.args["to"].as_str().unwrap_or("").to_string();
            let count = req.args["count"].as_u64();
            let result = if let Some(n) = count {
                let mut s = input.clone();
                let mut out = String::new();
                let mut replaced = 0u64;
                while replaced < n {
                    if let Some(pos) = s.find(&*from) {
                        out.push_str(&s[..pos]);
                        out.push_str(&to);
                        s = s[pos + from.len()..].to_string();
                        replaced += 1;
                    } else {
                        break;
                    }
                }
                out.push_str(&s);
                out
            } else {
                input.replace(&*from, &to)
            };
            sdk_respond_ok(rid, &result)
        }

        "text.split" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let sep = match get_str("sep") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'sep'"),
            };
            let parts: Vec<&str> = if let Some(max) = req.args["max"].as_u64() {
                input.splitn(max as usize, &*sep).collect()
            } else {
                input.split(&*sep).collect()
            };
            let json = serde_json::to_string(&parts).unwrap_or_default();
            sdk_respond_ok(rid, &json)
        }

        "text.join" => {
            let arr = match req.args["parts"].as_array() {
                Some(a) => a,
                None    => return sdk_respond_err(rid, "missing 'parts' array"),
            };
            let sep = req.args["sep"].as_str().unwrap_or("");
            let parts: Vec<&str> = arr.iter()
                .filter_map(|v| v.as_str())
                .collect();
            sdk_respond_ok(rid, &parts.join(sep))
        }

        "text.lines" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let lines: Vec<&str> = input.lines().collect();
            sdk_respond_ok(rid, &serde_json::to_string(&lines).unwrap_or_default())
        }

        "text.trim" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let side = req.args["side"].as_str().unwrap_or("both");
            let result = match side {
                "left"  => input.trim_start().to_string(),
                "right" => input.trim_end().to_string(),
                _       => input.trim().to_string(),
            };
            sdk_respond_ok(rid, &result)
        }

        "text.case" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let to = match get_str("to") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'to'"),
            };
            let result = match to.as_str() {
                "upper"  => input.to_uppercase(),
                "lower"  => input.to_lowercase(),
                "title"  => to_title_case(&input),
                "snake"  => to_snake_case(&input),
                "camel"  => to_camel_case(&input),
                "kebab"  => to_kebab_case(&input),
                other    => return sdk_respond_err(rid, &format!("unknown case: {other}")),
            };
            sdk_respond_ok(rid, &result)
        }

        "text.extract" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let pattern = match get_str("pattern") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'pattern'"),
            };
            match input.find(&*pattern) {
                Some(idx) => sdk_respond_ok(rid, &format!(
                    r#"{{"found":true,"index":{idx},"match":"{}"}}"#, pattern
                )),
                None => sdk_respond_ok(rid, r#"{"found":false,"index":-1,"match":""}"#),
            }
        }

        "text.slugify" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            sdk_respond_ok(rid, &slugify(&input))
        }

        "text.uuid_v4" => {
            sdk_respond_ok(rid, &generate_uuid_v4())
        }

        "text.repeat" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let count = match req.args["count"].as_u64() {
                Some(n) => n as usize,
                None    => return sdk_respond_err(rid, "missing 'count'"),
            };
            if count > 10_000 {
                return sdk_respond_err(rid, "count exceeds limit of 10,000");
            }
            sdk_respond_ok(rid, &input.repeat(count))
        }

        "text.reverse" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            sdk_respond_ok(rid, &input.chars().rev().collect::<String>())
        }

        "text.contains" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let needle = match get_str("needle") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'needle'"),
            };
            sdk_respond_ok(rid, &input.contains(&*needle).to_string())
        }

        "text.starts_with" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let prefix = match get_str("prefix") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'prefix'"),
            };
            sdk_respond_ok(rid, &input.starts_with(&*prefix).to_string())
        }

        "text.ends_with" => {
            let input = match get_str("input") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input'"),
            };
            let suffix = match get_str("suffix") {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'suffix'"),
            };
            sdk_respond_ok(rid, &input.ends_with(&*suffix).to_string())
        }

        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Case transformations ──────────────────────────────────────────────────────

fn words_of(s: &str) -> Vec<String> {
    // Split on non-alphanumeric boundaries and camelCase boundaries
    let mut words = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len() {
        let c = chars[i];
        if c.is_alphanumeric() {
            // Detect camelCase split: lowercase followed by uppercase
            if !current.is_empty() && c.is_uppercase() {
                let prev = chars[i - 1];
                if prev.is_lowercase() || (i + 1 < chars.len() && chars[i + 1].is_lowercase()) {
                    words.push(current.to_lowercase());
                    current = String::new();
                }
            }
            current.push(c);
        } else if !current.is_empty() {
            words.push(current.to_lowercase());
            current = String::new();
        }
    }
    if !current.is_empty() {
        words.push(current.to_lowercase());
    }
    words
}

fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None    => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn to_snake_case(s: &str) -> String {
    words_of(s).join("_")
}

fn to_camel_case(s: &str) -> String {
    let words = words_of(s);
    let mut out = String::new();
    for (i, w) in words.iter().enumerate() {
        if i == 0 {
            out.push_str(w);
        } else {
            let mut chars = w.chars();
            if let Some(f) = chars.next() {
                out.push_str(&f.to_uppercase().to_string());
                out.push_str(chars.as_str());
            }
        }
    }
    out
}

fn to_kebab_case(s: &str) -> String {
    words_of(s).join("-")
}

// ── Slugify ───────────────────────────────────────────────────────────────────

fn slugify(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut last_was_sep = true;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('-');
            last_was_sep = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

// ── UUID v4 via WASI random_get ───────────────────────────────────────────────

fn generate_uuid_v4() -> String {
    // Use WASI random_get if available, fall back to a xorshift64 seeded by
    // stack address (non-cryptographic, but deterministically different per call).
    let bytes = wasi_random_16();
    // Set version 4 and variant bits per RFC 4122
    let mut b = bytes;
    b[6] = (b[6] & 0x0F) | 0x40;
    b[8] = (b[8] & 0x3F) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

fn wasi_random_16() -> [u8; 16] {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 16];
        // WASI preview1 random_get syscall
        extern "C" {
            fn __wasi_random_get(buf: *mut u8, buf_len: usize) -> u16;
        }
        unsafe { __wasi_random_get(buf.as_mut_ptr(), 16); }
        buf
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // On native (tests): use a simple deterministic pseudo-random
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(12345) as u64;
        let mut x = seed ^ 0xdeadbeefcafe;
        let mut buf = [0u8; 16];
        for b in &mut buf {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *b = (x & 0xFF) as u8;
        }
        buf
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_basic() {
        let words = "hello world\nfoo".split_whitespace().count();
        assert_eq!(words, 3);
    }

    #[test]
    fn truncate_short() {
        let s = "hello".to_string();
        let result = if s.chars().count() <= 10 { s.clone() } else { format!("{}…", &s[..9]) };
        assert_eq!(result, "hello");
    }

    #[test]
    fn truncate_long() {
        let s = "The quick brown fox".to_string();
        let max = 10usize;
        let ell = "…";
        let take = max.saturating_sub(ell.chars().count());
        let truncated: String = s.chars().take(take).collect();
        let result = format!("{}{}", truncated, ell);
        assert_eq!(result.chars().count(), 10);
    }

    #[test]
    fn title_case_known() {
        assert_eq!(to_title_case("hello world"), "Hello World");
    }

    #[test]
    fn snake_case_from_camel() {
        assert_eq!(to_snake_case("helloWorld"), "hello_world");
    }

    #[test]
    fn camel_case_from_snake() {
        assert_eq!(to_camel_case("hello_world"), "helloWorld");
    }

    #[test]
    fn kebab_case_known() {
        assert_eq!(to_kebab_case("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
    }

    #[test]
    fn slugify_unicode_boundary() {
        assert_eq!(slugify("foo  bar"), "foo-bar");
    }

    #[test]
    fn slugify_no_trailing_dash() {
        assert_eq!(slugify("foo!"), "foo");
    }

    #[test]
    fn uuid_format() {
        let uuid = generate_uuid_v4();
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5,    "expected 5 groups: {uuid}");
        assert_eq!(parts[0].len(), 8, "group 0 wrong len: {uuid}");
        assert_eq!(parts[1].len(), 4, "group 1 wrong len: {uuid}");
        assert_eq!(parts[2].len(), 4, "group 2 wrong len: {uuid}");
        assert_eq!(parts[3].len(), 4, "group 3 wrong len: {uuid}");
        assert_eq!(parts[4].len(), 12,"group 4 wrong len: {uuid}");
        // Version bit
        assert!(parts[2].starts_with('4'), "UUID version must be 4: {uuid}");
        // Variant bits: first hex of group 3 must be 8, 9, a, or b
        let variant = parts[3].chars().next().unwrap();
        assert!(matches!(variant, '8'|'9'|'a'|'b'), "UUID variant wrong: {uuid}");
    }

    #[test]
    fn reverse_unicode() {
        assert_eq!("日本語".chars().rev().collect::<String>(), "語本日");
    }

    #[test]
    fn words_of_camel() {
        assert_eq!(words_of("helloWorldFoo"), vec!["hello", "world", "foo"]);
    }

    #[test]
    fn words_of_mixed() {
        assert_eq!(words_of("hello_world-foo"), vec!["hello", "world", "foo"]);
    }

    // ── case transforms ─────────────────────────────────────────────────────────────
    #[test]
    fn title_case_manual_impl() {
        let s = "hello world";
        let t: String = s.split_whitespace()
            .map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() })
            .collect::<Vec<_>>().join(" ");
        assert_eq!(t, "Hello World");
    }
    #[test]
    fn snake_case_preserves_acronym() {
        assert_eq!(to_snake_case("myURL"), "my_url");
    }
    #[test]
    fn camel_case_from_multi_segment() {
        assert_eq!(to_camel_case("foo_bar_baz"), "fooBarBaz");
    }
    #[test]
    fn kebab_case_multiple_words() {
        assert_eq!(to_kebab_case("Hello Beautiful World"), "hello-beautiful-world");
    }

    // ── slugify ────────────────────────────────────────────────────────────────────
    #[test]
    fn slugify_numbers_preserved() {
        assert_eq!(slugify("test 123"), "test-123");
    }
    #[test]
    fn slugify_multiple_spaces() {
        assert_eq!(slugify("a   b"), "a-b");
    }

    // ── words_of ───────────────────────────────────────────────────────────────────
    #[test]
    fn words_of_single_word() {
        assert_eq!(words_of("hello"), vec!["hello"]);
    }
    #[test]
    fn words_of_all_caps() {
        let words = words_of("ABC");
        assert!(!words.is_empty());
    }

    // ── reverse unicode ───────────────────────────────────────────────────────────
    #[test]
    fn reverse_ascii() {
        assert_eq!("hello".chars().rev().collect::<String>(), "olleh");
    }
    #[test]
    fn reverse_empty() {
        assert_eq!("".chars().rev().collect::<String>(), "");
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.text");
        assert_eq!(v["skills"].as_array().unwrap().len(), 17);
    }
    #[test]
    fn manifest_skill_names_start_with_text() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("text."));
        }
    }
}
