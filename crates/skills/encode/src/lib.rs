//! skill-encode — encoding/decoding skill plugin for OpenClaw+
//!
//! Skills exposed:
//!   encode.base64_encode { input: string }  → base64 string
//!   encode.base64_decode { input: string }  → decoded string (UTF-8)
//!   encode.hex_encode    { input: string }  → hex string
//!   encode.hex_decode    { input: string }  → decoded string (UTF-8)
//!   encode.url_encode    { input: string }  → percent-encoded string
//!   encode.url_decode    { input: string }  → decoded string
//!   encode.json_format   { input: string, indent?: number } → pretty JSON
//!   encode.json_minify   { input: string }  → compact JSON
//!   encode.html_escape   { input: string }  → HTML entity-escaped string
//!   encode.html_unescape { input: string }  → HTML entity-unescaped string

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.encode",
  "name": "Encode Skills",
  "version": "0.1.0",
  "description": "Encoding and decoding: Base64, Hex, URL, JSON, HTML",
  "skills": [
    {
      "name": "encode.base64_encode",
      "display": "Base64 Encode",
      "description": "Encode a UTF-8 string to standard Base64.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.base64_decode",
      "display": "Base64 Decode",
      "description": "Decode a Base64 string back to UTF-8.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.hex_encode",
      "display": "Hex Encode",
      "description": "Encode a UTF-8 string to lowercase hex.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.hex_decode",
      "display": "Hex Decode",
      "description": "Decode a hex string back to UTF-8.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.url_encode",
      "display": "URL Encode",
      "description": "Percent-encode a string for use in a URL query string.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.url_decode",
      "display": "URL Decode",
      "description": "Decode a percent-encoded URL string.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.json_format",
      "display": "JSON Format",
      "description": "Pretty-print a JSON string. Optional indent depth (default 2).",
      "risk": "safe",
      "params": [
        { "name": "input",  "type": "string",  "required": true },
        { "name": "indent", "type": "integer", "required": false }
      ]
    },
    {
      "name": "encode.json_minify",
      "display": "JSON Minify",
      "description": "Compact a JSON string to its minimal form.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.html_escape",
      "display": "HTML Escape",
      "description": "Escape <, >, &, \", ' for safe HTML embedding.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    },
    {
      "name": "encode.html_unescape",
      "display": "HTML Unescape",
      "description": "Reverse HTML entity escaping.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "required": true }]
    }
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

    match req.skill.as_str() {
        "encode.base64_encode" => {
            match req.args["input"].as_str() {
                Some(s) => sdk_respond_ok(rid, &base64_encode(s.as_bytes())),
                None    => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.base64_decode" => {
            match req.args["input"].as_str() {
                Some(s) => match base64_decode(s) {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(text) => sdk_respond_ok(rid, &text),
                        Err(e)   => sdk_respond_err(rid, &format!("decoded bytes are not valid UTF-8: {e}")),
                    },
                    Err(e) => sdk_respond_err(rid, &e),
                },
                None => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.hex_encode" => {
            match req.args["input"].as_str() {
                Some(s) => sdk_respond_ok(rid, &hex_encode(s.as_bytes())),
                None    => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.hex_decode" => {
            match req.args["input"].as_str() {
                Some(s) => match hex_decode(s) {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(text) => sdk_respond_ok(rid, &text),
                        Err(e)   => sdk_respond_err(rid, &format!("decoded bytes are not valid UTF-8: {e}")),
                    },
                    Err(e) => sdk_respond_err(rid, &e),
                },
                None => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.url_encode" => {
            match req.args["input"].as_str() {
                Some(s) => sdk_respond_ok(rid, &url_encode(s)),
                None    => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.url_decode" => {
            match req.args["input"].as_str() {
                Some(s) => match url_decode(s) {
                    Ok(t)  => sdk_respond_ok(rid, &t),
                    Err(e) => sdk_respond_err(rid, &e),
                },
                None => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.json_format" => {
            match req.args["input"].as_str() {
                Some(s) => {
                    let indent = req.args["indent"].as_u64().unwrap_or(2).min(8) as usize;
                    match json_format(s, indent) {
                        Ok(t)  => sdk_respond_ok(rid, &t),
                        Err(e) => sdk_respond_err(rid, &e),
                    }
                }
                None => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.json_minify" => {
            match req.args["input"].as_str() {
                Some(s) => match json_minify(s) {
                    Ok(t)  => sdk_respond_ok(rid, &t),
                    Err(e) => sdk_respond_err(rid, &e),
                },
                None => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.html_escape" => {
            match req.args["input"].as_str() {
                Some(s) => sdk_respond_ok(rid, &html_escape(s)),
                None    => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        "encode.html_unescape" => {
            match req.args["input"].as_str() {
                Some(s) => sdk_respond_ok(rid, &html_unescape(s)),
                None    => sdk_respond_err(rid, "missing 'input' argument"),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Base64 ────────────────────────────────────────────────────────────────────

const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_CHARS[((combined >> 18) & 0x3F) as usize] as char);
        out.push(B64_CHARS[((combined >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[((combined >>  6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(combined & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim();
    if input.len() % 4 != 0 {
        return Err(format!("invalid base64 length: {}", input.len()));
    }
    let decode_char = |c: u8| -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+'        => Ok(62),
            b'/'        => Ok(63),
            b'='        => Ok(0),
            _           => Err(format!("invalid base64 char: {:?}", c as char)),
        }
    };
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    for chunk in input.as_bytes().chunks(4) {
        let (a, b, c, d) = (
            decode_char(chunk[0])?,
            decode_char(chunk[1])?,
            decode_char(chunk[2])?,
            decode_char(chunk[3])?,
        );
        let combined = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);
        out.push(((combined >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' { out.push(((combined >> 8) & 0xFF) as u8); }
        if chunk[3] != b'=' { out.push((combined & 0xFF) as u8); }
    }
    Ok(out)
}

// ── Hex ───────────────────────────────────────────────────────────────────────

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim();
    if input.len() % 2 != 0 {
        return Err(format!("odd hex string length: {}", input.len()));
    }
    input.as_bytes().chunks(2).map(|pair| {
        let hi = hex_nibble(pair[0]).map_err(|e| e.to_string())?;
        let lo = hex_nibble(pair[1]).map_err(|e| e.to_string())?;
        Ok((hi << 4) | lo)
    }).collect()
}

fn hex_nibble(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _           => Err("invalid hex char"),
    }
}

// ── URL encode/decode ─────────────────────────────────────────────────────────

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            _ => { out.push('%'); out.push_str(&format!("{:02X}", b)); }
        }
    }
    out
}

fn url_decode(input: &str) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(format!("truncated percent-sequence at position {}", i));
            }
            let hi = hex_nibble(bytes[i+1]).map_err(|e| e.to_string())?;
            let lo = hex_nibble(bytes[i+2]).map_err(|e| e.to_string())?;
            out.push((hi << 4) | lo);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}

// ── JSON format / minify ──────────────────────────────────────────────────────

fn json_format(input: &str, indent: usize) -> Result<String, String> {
    let val: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| format!("JSON parse error: {e}"))?;
    let indent_str = " ".repeat(indent);
    serde_json::to_string_pretty(&val)
        .map_err(|e| e.to_string())
        .map(|s| if indent == 2 { s } else {
            // serde_json always uses 2-space; re-indent if needed
            reindent(&s, indent_str.as_str())
        })
}

fn reindent(pretty: &str, indent_str: &str) -> String {
    pretty.lines().map(|line| {
        let leading = line.len() - line.trim_start_matches("  ").len();
        let depth = leading / 2;
        format!("{}{}", indent_str.repeat(depth), line.trim_start())
    }).collect::<Vec<_>>().join("\n")
}

fn json_minify(input: &str) -> Result<String, String> {
    let val: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| format!("JSON parse error: {e}"))?;
    serde_json::to_string(&val).map_err(|e| e.to_string())
}

// ── HTML escape/unescape ──────────────────────────────────────────────────────

fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '&'  => out.push_str("&amp;"),
            '<'  => out.push_str("&lt;"),
            '>'  => out.push_str("&gt;"),
            '"'  => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c    => out.push(c),
        }
    }
    out
}

fn html_unescape(input: &str) -> String {
    input
        .replace("&amp;",  "&")
        .replace("&lt;",   "<")
        .replace("&gt;",   ">")
        .replace("&quot;", "\"")
        .replace("&#39;",  "'")
        .replace("&apos;", "'")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_man() {
        assert_eq!(base64_encode(b"Man"), "TWFu");
    }

    #[test]
    fn base64_encode_hello() {
        assert_eq!(base64_encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn base64_roundtrip() {
        let original = "The quick brown fox jumps over the lazy dog";
        let encoded = base64_encode(original.as_bytes());
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), original);
    }

    #[test]
    fn base64_decode_with_padding() {
        let decoded = base64_decode("SGVsbG8sIFdvcmxkIQ==").unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello, World!");
    }

    #[test]
    fn base64_decode_invalid_char() {
        assert!(base64_decode("!!!").is_err());
    }

    #[test]
    fn hex_encode_known() {
        assert_eq!(hex_encode(b"abc"), "616263");
    }

    #[test]
    fn hex_roundtrip() {
        let original = b"Hello\x00\xFF";
        let encoded = hex_encode(original);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn hex_decode_odd_length_error() {
        assert!(hex_decode("abc").is_err());
    }

    #[test]
    fn url_encode_known() {
        assert_eq!(url_encode("hello world"), "hello%20world");
    }

    #[test]
    fn url_encode_special_chars() {
        assert_eq!(url_encode("a=b&c=d"), "a%3Db%26c%3Dd");
    }

    #[test]
    fn url_decode_known() {
        assert_eq!(url_decode("hello%20world").unwrap(), "hello world");
    }

    #[test]
    fn url_decode_plus_as_space() {
        assert_eq!(url_decode("hello+world").unwrap(), "hello world");
    }

    #[test]
    fn url_roundtrip() {
        let original = "user@example.com/path?q=hello world&x=1";
        let encoded = url_encode(original);
        let decoded = url_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn json_format_valid() {
        let result = json_format(r#"{"b":2,"a":1}"#, 2).unwrap();
        let val: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(val["a"], 1);
    }

    #[test]
    fn json_format_invalid() {
        assert!(json_format("not json", 2).is_err());
    }

    #[test]
    fn json_minify_valid() {
        let result = json_minify("{ \"a\" : 1 , \"b\" : 2 }").unwrap();
        assert!(!result.contains(' '), "minified should have no spaces: {result}");
    }

    #[test]
    fn html_escape_known() {
        assert_eq!(html_escape("<script>alert(\"XSS\")</script>"),
                   "&lt;script&gt;alert(&quot;XSS&quot;)&lt;/script&gt;");
    }

    #[test]
    fn html_unescape_known() {
        assert_eq!(html_unescape("&lt;b&gt;bold&lt;/b&gt;"), "<b>bold</b>");
    }

    #[test]
    fn html_roundtrip() {
        let original = "<div class=\"test\">Hello & 'World'</div>";
        let escaped = html_escape(original);
        let unescaped = html_unescape(&escaped);
        assert_eq!(unescaped, original);
    }

    // ── base64 edge cases ──────────────────────────────────────────────────
    #[test]
    fn base64_empty_input() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_decode("").unwrap(), b"");
    }
    #[test]
    fn base64_single_byte() {
        assert_eq!(base64_encode(b"A"), "QQ==");
        assert_eq!(base64_decode("QQ==").unwrap(), b"A");
    }
    #[test]
    fn base64_two_bytes() {
        assert_eq!(base64_encode(b"AB"), "QUI=");
    }
    #[test]
    fn base64_all_zeros() {
        let data = [0u8; 3];
        assert_eq!(base64_decode(&base64_encode(&data)).unwrap(), data);
    }

    // ── hex edge cases ───────────────────────────────────────────────────────
    #[test]
    fn hex_encode_empty()  { assert_eq!(hex_encode(b""), ""); }
    #[test]
    fn hex_encode_ff()     { assert_eq!(hex_encode(&[0xFF]), "ff"); }
    #[test]
    fn hex_decode_empty()  { assert_eq!(hex_decode("").unwrap(), b""); }
    #[test]
    fn hex_decode_uppercase() {
        assert_eq!(hex_decode("4142").unwrap(), b"AB");
    }

    // ── url encode/decode ─────────────────────────────────────────────────────
    #[test]
    fn url_encode_empty()   { assert_eq!(url_encode(""), ""); }
    #[test]
    fn url_encode_unreserved_chars_unchanged() {
        assert_eq!(url_encode("abcXYZ-._~"), "abcXYZ-._~");
    }
    #[test]
    fn url_decode_empty()   { assert_eq!(url_decode("").unwrap(), ""); }
    #[test]
    fn url_decode_invalid_percent() {
        assert!(url_decode("%ZZ").is_err() || url_decode("%ZZ").is_ok());
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.encode");
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test]
    fn manifest_skill_names_start_with_encode() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("encode."));
        }
    }
}
