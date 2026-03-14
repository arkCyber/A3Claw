//! skill-network — URL and network utility skills (pure-Rust, no I/O).
//!
//! Skills exposed:
//!   network.url_parse     { url: string }              → JSON object
//!   network.url_encode    { text: string }             → percent-encoded string
//!   network.url_decode    { text: string }             → decoded string
//!   network.mime_type     { filename: string }         → MIME type string
//!   network.ip_classify   { ip: string }               → "ipv4"|"ipv6"|"invalid"
//!   network.http_status   { code: integer }            → status text description
//!
//! No OS or WASI calls — all logic is pure computation.

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.network",
  "name": "Network Skills",
  "version": "0.1.0",
  "description": "URL parsing, percent-encoding, MIME types, IP classification, HTTP status codes",
  "skills": [
    {
      "name": "network.url_parse",
      "display": "Parse URL",
      "description": "Parse a URL into its components: scheme, host, port, path, query, fragment.",
      "risk": "safe",
      "params": [
        { "name": "url", "type": "string", "description": "URL to parse", "required": true }
      ]
    },
    {
      "name": "network.url_encode",
      "display": "URL Percent-Encode",
      "description": "Percent-encode a string for use in a URL query or path component.",
      "risk": "safe",
      "params": [
        { "name": "text", "type": "string", "description": "Text to encode", "required": true }
      ]
    },
    {
      "name": "network.url_decode",
      "display": "URL Percent-Decode",
      "description": "Decode a percent-encoded URL string.",
      "risk": "safe",
      "params": [
        { "name": "text", "type": "string", "description": "Percent-encoded text to decode", "required": true }
      ]
    },
    {
      "name": "network.mime_type",
      "display": "Get MIME Type",
      "description": "Return the MIME type for a filename based on its extension.",
      "risk": "safe",
      "params": [
        { "name": "filename", "type": "string", "description": "Filename (e.g. 'photo.jpg')", "required": true }
      ]
    },
    {
      "name": "network.ip_classify",
      "display": "Classify IP Address",
      "description": "Classify an IP address as 'ipv4', 'ipv6', or 'invalid'. Also reports private/loopback for IPv4.",
      "risk": "safe",
      "params": [
        { "name": "ip", "type": "string", "description": "IP address string", "required": true }
      ]
    },
    {
      "name": "network.http_status",
      "display": "HTTP Status Description",
      "description": "Return the standard text description for an HTTP status code.",
      "risk": "safe",
      "params": [
        { "name": "code", "type": "integer", "description": "HTTP status code (e.g. 200)", "required": true }
      ]
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
        "network.url_parse" => {
            let url = match req.args["url"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'url'"),
            };
            match parse_url(url) {
                Ok(json) => sdk_respond_ok(rid, &json),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "network.url_encode" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            sdk_respond_ok(rid, &url_encode(text))
        }
        "network.url_decode" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            match url_decode(text) {
                Ok(decoded) => sdk_respond_ok(rid, &decoded),
                Err(e)      => sdk_respond_err(rid, &e),
            }
        }
        "network.mime_type" => {
            let filename = match req.args["filename"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'filename'"),
            };
            sdk_respond_ok(rid, mime_type(filename))
        }
        "network.ip_classify" => {
            let ip = match req.args["ip"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'ip'"),
            };
            sdk_respond_ok(rid, &ip_classify(ip))
        }
        "network.http_status" => {
            let code = match req.args["code"].as_u64() {
                Some(n) => n as u16,
                None    => return sdk_respond_err(rid, "missing or invalid 'code'"),
            };
            sdk_respond_ok(rid, http_status_text(code))
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── URL Parser ────────────────────────────────────────────────────────────────

fn parse_url(url: &str) -> Result<String, String> {
    let (scheme, rest) = if let Some(pos) = url.find("://") {
        (&url[..pos], &url[pos+3..])
    } else {
        return Err(format!("URL missing scheme: {}", url));
    };

    let (authority, path_query_frag) = if let Some(pos) = rest.find('/') {
        (&rest[..pos], &rest[pos..])
    } else {
        (rest, "")
    };

    let (fragment, path_query) = if let Some(pos) = path_query_frag.rfind('#') {
        (&path_query_frag[pos+1..], &path_query_frag[..pos])
    } else {
        ("", path_query_frag)
    };

    let (path, query) = if let Some(pos) = path_query.find('?') {
        (&path_query[..pos], &path_query[pos+1..])
    } else {
        (path_query, "")
    };

    let (userinfo, hostport) = if let Some(pos) = authority.rfind('@') {
        (&authority[..pos], &authority[pos+1..])
    } else {
        ("", authority)
    };

    let (host, port) = if hostport.starts_with('[') {
        // IPv6 literal
        if let Some(pos) = hostport.rfind("]:") {
            (&hostport[..pos+1], Some(&hostport[pos+2..]))
        } else {
            (hostport, None)
        }
    } else if let Some(pos) = hostport.rfind(':') {
        (&hostport[..pos], Some(&hostport[pos+1..]))
    } else {
        (hostport, None)
    };

    let port_num: Option<u16> = port.and_then(|p| p.parse().ok());

    let json = format!(
        r#"{{"scheme":"{}","host":"{}","port":{},"path":"{}","query":"{}","fragment":"{}","userinfo":"{}"}}"#,
        escape_json(scheme),
        escape_json(host),
        port_num.map(|p| p.to_string()).unwrap_or_else(|| "null".to_string()),
        escape_json(path),
        escape_json(query),
        escape_json(fragment),
        escape_json(userinfo),
    );
    Ok(json)
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ── Percent-encoding ──────────────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn url_decode(s: &str) -> Result<String, String> {
    let mut bytes: Vec<u8> = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if i + 2 >= chars.len() { return Err("truncated percent sequence".into()); }
            let hi = chars[i+1].to_digit(16).ok_or_else(|| format!("invalid hex char '{}'", chars[i+1]))?;
            let lo = chars[i+2].to_digit(16).ok_or_else(|| format!("invalid hex char '{}'", chars[i+2]))?;
            bytes.push((hi * 16 + lo) as u8);
            i += 3;
        } else if chars[i] == '+' {
            bytes.push(b' ');
            i += 1;
        } else {
            let mut buf = [0u8; 4];
            let encoded = chars[i].encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
            i += 1;
        }
    }
    String::from_utf8(bytes).map_err(|_| "decoded bytes are not valid UTF-8".into())
}

// ── MIME types ────────────────────────────────────────────────────────────────

fn mime_type(filename: &str) -> &'static str {
    let lower = filename.to_ascii_lowercase();
    let ext = lower.rfind('.').map(|i| &lower[i+1..]).unwrap_or("");
    match ext {
        "html" | "htm"  => "text/html",
        "css"           => "text/css",
        "js" | "mjs"    => "application/javascript",
        "ts"            => "application/typescript",
        "json"          => "application/json",
        "xml"           => "application/xml",
        "csv"           => "text/csv",
        "txt"           => "text/plain",
        "md"            => "text/markdown",
        "png"           => "image/png",
        "jpg" | "jpeg"  => "image/jpeg",
        "gif"           => "image/gif",
        "svg"           => "image/svg+xml",
        "webp"          => "image/webp",
        "ico"           => "image/x-icon",
        "pdf"           => "application/pdf",
        "zip"           => "application/zip",
        "gz"  | "gzip"  => "application/gzip",
        "tar"           => "application/x-tar",
        "mp3"           => "audio/mpeg",
        "mp4"           => "video/mp4",
        "webm"          => "video/webm",
        "wasm"          => "application/wasm",
        "yaml" | "yml"  => "application/x-yaml",
        "toml"          => "application/toml",
        _               => "application/octet-stream",
    }
}

// ── IP classification ─────────────────────────────────────────────────────────

fn ip_classify(ip: &str) -> String {
    let ip = ip.trim();
    if let Some(class) = classify_ipv4(ip) {
        return class;
    }
    if is_ipv6(ip) {
        return "ipv6".to_string();
    }
    "invalid".to_string()
}

fn classify_ipv4(s: &str) -> Option<String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 { return None; }
    let octets: Vec<u8> = parts.iter()
        .filter_map(|p| p.parse::<u8>().ok())
        .collect();
    if octets.len() != 4 { return None; }
    let (a, b) = (octets[0], octets[1]);
    if a == 127 {
        Some("ipv4:loopback".to_string())
    } else if a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168) {
        Some("ipv4:private".to_string())
    } else if a == 169 && b == 254 {
        Some("ipv4:link-local".to_string())
    } else {
        Some("ipv4:public".to_string())
    }
}

fn is_ipv6(s: &str) -> bool {
    // Minimal IPv6 check: contains at least one colon, only hex digits + colons + optional brackets
    let s = s.trim_matches(|c| c == '[' || c == ']');
    if !s.contains(':') { return false; }
    let colon_count = s.chars().filter(|&c| c == ':').count();
    if colon_count < 2 || colon_count > 7 { return false; }
    s.chars().all(|c| c.is_ascii_hexdigit() || c == ':')
}

// ── HTTP status codes ─────────────────────────────────────────────────────────

fn http_status_text(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Content Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Content",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _   => "Unknown Status",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_parse_basic() {
        let json = parse_url("https://user:pass@example.com:8080/path/to?q=1&r=2#section").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["scheme"], "https");
        assert_eq!(v["host"], "example.com");
        assert_eq!(v["port"], 8080);
        assert_eq!(v["path"], "/path/to");
        assert_eq!(v["query"], "q=1&r=2");
        assert_eq!(v["fragment"], "section");
        assert_eq!(v["userinfo"], "user:pass");
    }

    #[test]
    fn url_parse_no_port() {
        let json = parse_url("http://example.com/").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["port"], serde_json::Value::Null);
    }

    #[test]
    fn url_parse_missing_scheme_errors() {
        assert!(parse_url("example.com/path").is_err());
    }

    #[test]
    fn url_encode_special_chars() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a=1&b=2"), "a%3D1%26b%3D2");
    }

    #[test]
    fn url_encode_unreserved_unchanged() {
        assert_eq!(url_encode("abc-_.~"), "abc-_.~");
    }

    #[test]
    fn url_decode_basic() {
        assert_eq!(url_decode("hello%20world").unwrap(), "hello world");
        assert_eq!(url_decode("a%3D1").unwrap(), "a=1");
    }

    #[test]
    fn url_decode_plus_as_space() {
        assert_eq!(url_decode("hello+world").unwrap(), "hello world");
    }

    #[test]
    fn url_roundtrip() {
        let original = "hello world & more=things!";
        let encoded = url_encode(original);
        let decoded = url_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn mime_type_html() {
        assert_eq!(mime_type("index.html"), "text/html");
    }

    #[test]
    fn mime_type_wasm() {
        assert_eq!(mime_type("skill.wasm"), "application/wasm");
    }

    #[test]
    fn mime_type_unknown() {
        assert_eq!(mime_type("file.xyz"), "application/octet-stream");
    }

    #[test]
    fn ip_classify_loopback() {
        assert_eq!(ip_classify("127.0.0.1"), "ipv4:loopback");
    }

    #[test]
    fn ip_classify_private() {
        assert_eq!(ip_classify("192.168.1.1"), "ipv4:private");
        assert_eq!(ip_classify("10.0.0.1"), "ipv4:private");
        assert_eq!(ip_classify("172.16.0.1"), "ipv4:private");
    }

    #[test]
    fn ip_classify_public() {
        assert_eq!(ip_classify("8.8.8.8"), "ipv4:public");
    }

    #[test]
    fn ip_classify_ipv6() {
        assert_eq!(ip_classify("2001:db8::1"), "ipv6");
        assert_eq!(ip_classify("::1"), "ipv6");
    }

    #[test]
    fn ip_classify_invalid() {
        assert_eq!(ip_classify("not-an-ip"), "invalid");
        assert_eq!(ip_classify("256.0.0.1"), "invalid");
    }

    #[test]
    fn http_status_200() {
        assert_eq!(http_status_text(200), "OK");
    }

    #[test]
    fn http_status_404() {
        assert_eq!(http_status_text(404), "Not Found");
    }

    #[test]
    fn http_status_500() {
        assert_eq!(http_status_text(500), "Internal Server Error");
    }

    #[test]
    fn http_status_unknown() {
        assert_eq!(http_status_text(999), "Unknown Status");
    }

    // ── url_parse edge cases ────────────────────────────────────────────────
    #[test]
    fn url_parse_simple() {
        let json = parse_url("https://example.com/path?q=1").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["scheme"], "https");
        assert_eq!(v["host"], "example.com");
        assert_eq!(v["path"], "/path");
        assert_eq!(v["query"], "q=1");
    }
    #[test]
    fn url_parse_no_path() {
        let json = parse_url("http://example.com").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["scheme"], "http");
        assert_eq!(v["host"], "example.com");
    }
    #[test]
    fn url_parse_with_port() {
        let json = parse_url("http://localhost:8080/api").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["port"], 8080);
    }
    #[test]
    fn url_parse_missing_scheme_fails() {
        assert!(parse_url("not-a-url").is_err());
    }

    // ── percent encode/decode ─────────────────────────────────────────────
    #[test]
    fn percent_encode_space()  { assert_eq!(url_encode(" "), "%20"); }
    #[test]
    fn percent_encode_slash()  { assert_eq!(url_encode("/"), "%2F"); }
    #[test]
    fn percent_encode_empty()  { assert_eq!(url_encode(""), ""); }
    #[test]
    fn percent_decode_roundtrip() {
        let s = "hello world/test";
        assert_eq!(url_decode(&url_encode(s)).unwrap(), s);
    }
    #[test]
    fn percent_encode_alphanumeric_unchanged() {
        assert_eq!(url_encode("abc123"), "abc123");
    }

    // ── mime types ─────────────────────────────────────────────────────────────
    #[test]
    fn mime_type_json()   { assert_eq!(mime_type("data.json"), "application/json"); }
    #[test]
    fn mime_type_png()    { assert_eq!(mime_type("image.png"), "image/png"); }
    #[test]
    fn mime_type_css()    { assert_eq!(mime_type("style.css"), "text/css"); }

    // ── http status ──────────────────────────────────────────────────────────
    #[test]
    fn http_status_201() { assert_eq!(http_status_text(201), "Created"); }
    #[test]
    fn http_status_301() { assert_eq!(http_status_text(301), "Moved Permanently"); }
    #[test]
    fn http_status_400() { assert_eq!(http_status_text(400), "Bad Request"); }
    #[test]
    fn http_status_401() { assert_eq!(http_status_text(401), "Unauthorized"); }
    #[test]
    fn http_status_403() { assert_eq!(http_status_text(403), "Forbidden"); }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.network");
        assert_eq!(v["skills"].as_array().unwrap().len(), 6);
    }
    #[test]
    fn manifest_skill_names_start_with_network() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("network."));
        }
    }
}
