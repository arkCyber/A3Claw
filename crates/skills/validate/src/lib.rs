//! skill-validate — input validation skills (pure-Rust, no I/O).
//!
//! Skills:
//!   validate.email        { value: string } → bool
//!   validate.url          { value: string } → bool
//!   validate.ipv4         { value: string } → bool
//!   validate.ipv6         { value: string } → bool
//!   validate.uuid         { value: string } → bool
//!   validate.credit_card  { value: string } → bool  (Luhn check)
//!   validate.phone        { value: string } → bool  (E.164-ish)
//!   validate.json         { value: string } → bool

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.validate",
  "name": "Validate Skills",
  "version": "0.1.0",
  "description": "Input validation: email, URL, IP, UUID, credit card, phone, JSON",
  "skills": [
    {
      "name": "validate.email",
      "display": "Validate Email",
      "description": "Check if a string is a syntactically valid email address.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "validate.url",
      "display": "Validate URL",
      "description": "Check if a string is a valid http/https URL.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "validate.ipv4",
      "display": "Validate IPv4",
      "description": "Check if a string is a valid IPv4 address.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "validate.ipv6",
      "display": "Validate IPv6",
      "description": "Check if a string is a valid IPv6 address.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "validate.uuid",
      "display": "Validate UUID",
      "description": "Check if a string is a valid UUID (any version).",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "validate.credit_card",
      "display": "Validate Credit Card",
      "description": "Check a credit card number using the Luhn algorithm.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "validate.phone",
      "display": "Validate Phone",
      "description": "Check if a string looks like an E.164 international phone number.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "validate.json",
      "display": "Validate JSON",
      "description": "Check if a string is valid JSON.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r)  => r,
        Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();
    let val = match req.args["value"].as_str() {
        Some(s) => s,
        None    => return sdk_respond_err(rid, "missing string 'value'"),
    };

    let result = match req.skill.as_str() {
        "validate.email"       => is_valid_email(val),
        "validate.url"         => is_valid_url(val),
        "validate.ipv4"        => is_valid_ipv4(val),
        "validate.ipv6"        => is_valid_ipv6(val),
        "validate.uuid"        => is_valid_uuid(val),
        "validate.credit_card" => is_valid_luhn(val),
        "validate.phone"       => is_valid_phone(val),
        "validate.json"        => serde_json::from_str::<serde_json::Value>(val).is_ok(),
        other                  => return sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    };
    sdk_respond_ok(rid, if result { "true" } else { "false" })
}

// ── Validation logic ──────────────────────────────────────────────────────────

fn is_valid_email(s: &str) -> bool {
    let s = s.trim();
    let parts: Vec<&str> = s.splitn(2, '@').collect();
    if parts.len() != 2 { return false; }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
        && domain.len() >= 3
}

fn is_valid_url(s: &str) -> bool {
    let s = s.trim();
    (s.starts_with("http://") || s.starts_with("https://"))
        && s.len() > 10
        && s[8..].contains('.')
}

fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.trim().split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok())
}

fn is_valid_ipv6(s: &str) -> bool {
    let s = s.trim();
    let groups: Vec<&str> = s.split(':').collect();
    if groups.len() < 3 || groups.len() > 8 { return false; }
    groups.iter().all(|g| g.is_empty() || (g.len() <= 4 && g.chars().all(|c| c.is_ascii_hexdigit())))
}

fn is_valid_uuid(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 36 { return false; }
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts[0].len() == 8  && parts[0].chars().all(|c| c.is_ascii_hexdigit())
        && parts[1].len() == 4  && parts[1].chars().all(|c| c.is_ascii_hexdigit())
        && parts[2].len() == 4  && parts[2].chars().all(|c| c.is_ascii_hexdigit())
        && parts[3].len() == 4  && parts[3].chars().all(|c| c.is_ascii_hexdigit())
        && parts[4].len() == 12 && parts[4].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_luhn(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();
    if digits.len() < 2 { return false; }
    let sum: u32 = digits.iter().rev().enumerate().map(|(i, &d)| {
        if i % 2 == 1 {
            let d2 = d * 2;
            if d2 > 9 { d2 - 9 } else { d2 }
        } else { d }
    }).sum();
    sum % 10 == 0
}

fn is_valid_phone(s: &str) -> bool {
    let s = s.trim();
    if !s.starts_with('+') { return false; }
    let digits: String = s[1..].chars().filter(|c| c.is_ascii_digit()).collect();
    (7..=15).contains(&digits.len())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn email_valid()         { assert!(is_valid_email("user@example.com")); }
    #[test] fn email_no_at()         { assert!(!is_valid_email("userexample.com")); }
    #[test] fn email_no_domain_dot() { assert!(!is_valid_email("user@localhost")); }
    #[test] fn email_empty_local()   { assert!(!is_valid_email("@example.com")); }

    #[test] fn url_http()    { assert!(is_valid_url("http://example.com/path")); }
    #[test] fn url_https()   { assert!(is_valid_url("https://example.com")); }
    #[test] fn url_no_scheme(){ assert!(!is_valid_url("example.com")); }
    #[test] fn url_no_dot()  { assert!(!is_valid_url("http://localhost")); }

    #[test] fn ipv4_valid()   { assert!(is_valid_ipv4("192.168.1.1")); }
    #[test] fn ipv4_invalid() { assert!(!is_valid_ipv4("999.0.0.1")); }
    #[test] fn ipv4_short()   { assert!(!is_valid_ipv4("1.2.3")); }

    #[test] fn ipv6_valid()   { assert!(is_valid_ipv6("2001:db8::1")); }
    #[test] fn ipv6_invalid() { assert!(!is_valid_ipv6("not-an-ipv6")); }

    #[test] fn uuid_valid()   { assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000")); }
    #[test] fn uuid_short()   { assert!(!is_valid_uuid("550e8400-e29b-41d4")); }

    #[test] fn luhn_valid()   { assert!(is_valid_luhn("4532015112830366")); }
    #[test] fn luhn_invalid() { assert!(!is_valid_luhn("1234567890123456")); }

    #[test] fn phone_valid()  { assert!(is_valid_phone("+12025551234")); }
    #[test] fn phone_no_plus(){ assert!(!is_valid_phone("12025551234")); }
    #[test] fn phone_short()  { assert!(!is_valid_phone("+123")); }

    #[test] fn json_valid()   { assert!(serde_json::from_str::<serde_json::Value>("{\"a\":1}").is_ok()); }
    #[test] fn json_invalid() { assert!(serde_json::from_str::<serde_json::Value>("not json").is_err()); }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.validate");
        assert_eq!(v["skills"].as_array().unwrap().len(), 8);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("validate."));
        }
    }
}
