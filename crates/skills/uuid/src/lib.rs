//! skill-uuid — UUID generation and validation skills.
//!
//! Skills exposed:
//!   uuid.v4        {}                             → UUID v4 string
//!   uuid.v5        { namespace: string, name: string } → UUID v5 string
//!   uuid.validate  { value: string }              → bool
//!   uuid.parse     { value: string }              → JSON object with fields
//!   uuid.nil       {}                             → "00000000-0000-0000-0000-000000000000"

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.uuid",
  "name": "UUID Skills",
  "version": "0.1.0",
  "description": "UUID v4 generation, v5 name-based, validation and parsing",
  "skills": [
    {
      "name": "uuid.v4",
      "display": "Generate UUID v4",
      "description": "Generate a random UUID v4.",
      "risk": "safe",
      "params": []
    },
    {
      "name": "uuid.v5",
      "display": "Generate UUID v5",
      "description": "Generate a name-based UUID v5 using SHA-1. Namespace: dns|url|oid|x500 or a UUID string.",
      "risk": "safe",
      "params": [
        { "name": "namespace", "type": "string", "description": "Namespace: dns, url, oid, x500, or UUID string", "required": true },
        { "name": "name",      "type": "string", "description": "Name to hash",                                   "required": true }
      ]
    },
    {
      "name": "uuid.validate",
      "display": "Validate UUID",
      "description": "Check if a string is a valid UUID (any version). Returns 'true' or 'false'.",
      "risk": "safe",
      "params": [
        { "name": "value", "type": "string", "description": "String to validate", "required": true }
      ]
    },
    {
      "name": "uuid.parse",
      "display": "Parse UUID",
      "description": "Parse a UUID string and return its version, variant, and canonical form.",
      "risk": "safe",
      "params": [
        { "name": "value", "type": "string", "description": "UUID string to parse", "required": true }
      ]
    },
    {
      "name": "uuid.nil",
      "display": "Nil UUID",
      "description": "Return the nil UUID (all zeros).",
      "risk": "safe",
      "params": []
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
        "uuid.v4" => {
            sdk_respond_ok(rid, &uuid_v4())
        }
        "uuid.v5" => {
            let ns_raw = match req.args["namespace"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'namespace' argument"),
            };
            let name = match req.args["name"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'name' argument"),
            };
            let ns_bytes = match resolve_namespace(ns_raw) {
                Ok(b)  => b,
                Err(e) => return sdk_respond_err(rid, &e),
            };
            sdk_respond_ok(rid, &uuid_v5(&ns_bytes, name.as_bytes()))
        }
        "uuid.validate" => {
            let value = match req.args["value"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'value' argument"),
            };
            sdk_respond_ok(rid, if is_valid_uuid(value) { "true" } else { "false" })
        }
        "uuid.parse" => {
            let value = match req.args["value"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'value' argument"),
            };
            match parse_uuid(value) {
                Ok(json) => sdk_respond_ok(rid, &json),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "uuid.nil" => {
            sdk_respond_ok(rid, "00000000-0000-0000-0000-000000000000")
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Well-known namespace UUIDs (RFC 4122 §4.3) ────────────────────────────────

fn resolve_namespace(ns: &str) -> Result<[u8; 16], String> {
    match ns {
        "dns"  => Ok(parse_uuid_bytes("6ba7b810-9dad-11d1-80b4-00c04fd430c8")?),
        "url"  => Ok(parse_uuid_bytes("6ba7b811-9dad-11d1-80b4-00c04fd430c8")?),
        "oid"  => Ok(parse_uuid_bytes("6ba7b812-9dad-11d1-80b4-00c04fd430c8")?),
        "x500" => Ok(parse_uuid_bytes("6ba7b814-9dad-11d1-80b4-00c04fd430c8")?),
        other  => Ok(parse_uuid_bytes(other)?),
    }
}

fn parse_uuid_bytes(s: &str) -> Result<[u8; 16], String> {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 { return Err(format!("invalid UUID: {}", s)); }
    let bytes: Vec<u8> = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i+2], 16).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    bytes.try_into().map_err(|_| "bad uuid bytes".to_string())
}

// ── UUID v4 (random, using LCG seeded from memory address) ───────────────────

fn uuid_v4() -> String {
    let mut bytes = lcg_bytes_16(lcg_seed());
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 10xx
    format_uuid(&bytes)
}

fn lcg_seed() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0x9e3779b97f4a7c15);
    let stack_val: u64 = 0;
    let ptr = &stack_val as *const u64 as u64;
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    ptr.wrapping_mul(6364136223846793005)
       .wrapping_add(1442695040888963407)
       ^ count
}

fn lcg_bytes_16(mut seed: u64) -> [u8; 16] {
    let mut out = [0u8; 16];
    for chunk in out.chunks_mut(8) {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let b = seed.to_le_bytes();
        chunk.copy_from_slice(&b[..chunk.len()]);
    }
    out
}

// ── UUID v5 (name-based, SHA-1) ───────────────────────────────────────────────

fn uuid_v5(namespace: &[u8; 16], name: &[u8]) -> String {
    let mut input = namespace.to_vec();
    input.extend_from_slice(name);
    let hash = sha1_digest(&input);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50; // version 5
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 10xx
    format_uuid(&bytes)
}

fn format_uuid(b: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],b[1],b[2],b[3], b[4],b[5], b[6],b[7], b[8],b[9], b[10],b[11],b[12],b[13],b[14],b[15]
    )
}

// ── Validation ────────────────────────────────────────────────────────────────

fn is_valid_uuid(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 36 { return false; }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 { return false; }
    let expected_lens = [8, 4, 4, 4, 12];
    parts.iter().zip(expected_lens.iter()).all(|(p, &l)| {
        p.len() == l && p.chars().all(|c| c.is_ascii_hexdigit())
    })
}

// ── Parsing ───────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str) -> Result<String, String> {
    let s = s.trim().to_lowercase();
    if !is_valid_uuid(&s) { return Err(format!("'{}' is not a valid UUID", s)); }
    let bytes = parse_uuid_bytes(&s)?;
    let version = (bytes[6] >> 4) & 0x0f;
    let variant = if bytes[8] & 0x80 == 0 { "NCS" }
                  else if bytes[8] & 0x40 == 0 { "RFC4122" }
                  else if bytes[8] & 0x20 == 0 { "Microsoft" }
                  else { "Future" };
    Ok(format!(
        r#"{{"uuid":"{}","version":{},"variant":"{}"}}"#,
        s, version, variant
    ))
}

// ── SHA-1 (for UUID v5, inlined) ──────────────────────────────────────────────

fn sha1_digest(input: &[u8]) -> [u8; 20] {
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0x00); }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let (mut h0,mut h1,mut h2,mut h3,mut h4):(u32,u32,u32,u32,u32)=
        (0x67452301,0xefcdab89,0x98badcfe,0x10325476,0xc3d2e1f0);
    for chunk in msg.chunks_exact(64) {
        let mut w=[0u32;80];
        for i in 0..16 { w[i]=u32::from_be_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]]); }
        for i in 16..80 { w[i]=(w[i-3]^w[i-8]^w[i-14]^w[i-16]).rotate_left(1); }
        let (mut a,mut b,mut c,mut d,mut e)=(h0,h1,h2,h3,h4);
        for i in 0usize..80 {
            let (f,k)=match i {
                 0..=19=>((b&c)|(!b&d),0x5a827999u32),
                20..=39=>(b^c^d,0x6ed9eba1),
                40..=59=>((b&c)|(b&d)|(c&d),0x8f1bbcdc),
                _=>(b^c^d,0xca62c1d6),
            };
            let t=a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(w[i]);
            e=d;d=c;c=b.rotate_left(30);b=a;a=t;
        }
        h0=h0.wrapping_add(a);h1=h1.wrapping_add(b);h2=h2.wrapping_add(c);
        h3=h3.wrapping_add(d);h4=h4.wrapping_add(e);
    }
    let mut out=[0u8;20];
    for (i,v) in [h0,h1,h2,h3,h4].iter().enumerate() { out[i*4..(i+1)*4].copy_from_slice(&v.to_be_bytes()); }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_format() {
        let u = uuid_v4();
        assert!(is_valid_uuid(&u), "v4 should be valid: {}", u);
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(&parts[2][0..1], "4", "version nibble should be 4");
    }

    #[test]
    fn v5_dns_example() {
        let ns = resolve_namespace("dns").unwrap();
        let u = uuid_v5(&ns, b"python.org");
        assert!(is_valid_uuid(&u));
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(&parts[2][0..1], "5", "version nibble should be 5");
    }

    #[test]
    fn v5_deterministic() {
        let ns = resolve_namespace("url").unwrap();
        let a = uuid_v5(&ns, b"https://openclaw.ai");
        let b = uuid_v5(&ns, b"https://openclaw.ai");
        assert_eq!(a, b);
    }

    #[test]
    fn validate_valid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn validate_invalid_chars() {
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000g"));
    }

    #[test]
    fn validate_nil() {
        assert!(is_valid_uuid("00000000-0000-0000-0000-000000000000"));
    }

    #[test]
    fn validate_too_short() {
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716"));
    }

    #[test]
    fn parse_returns_version() {
        let json = parse_uuid("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert!(json.contains("\"version\":4"), "expected version 4, got: {}", json);
    }

    #[test]
    fn parse_invalid_rejects() {
        assert!(parse_uuid("not-a-uuid").is_err());
    }

    // ── v4 format/bits ─────────────────────────────────────────────────────────────
    #[test]
    fn v4_all_lowercase_hex() {
        let u = uuid_v4();
        assert!(u.chars().all(|c| c == '-' || c.is_ascii_hexdigit()),
                "non-hex chars in UUID: {u}");
    }
    #[test]
    fn v4_version_nibble_is_4() {
        let u = uuid_v4();
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(&parts[2][0..1], "4", "version nibble wrong: {u}");
    }
    #[test]
    fn v4_variant_nibble_correct() {
        let u = uuid_v4();
        let parts: Vec<&str> = u.split('-').collect();
        let v = parts[3].chars().next().unwrap();
        assert!(matches!(v, '8'|'9'|'a'|'b'), "variant wrong: {u}");
    }
    #[test]
    fn v4_unique_per_call() {
        let a = uuid_v4();
        let b = uuid_v4();
        assert_ne!(a, b, "two v4 UUIDs should differ");
    }

    // ── v5 ─────────────────────────────────────────────────────────────────────
    #[test]
    fn v5_different_names_differ() {
        let ns = resolve_namespace("dns").unwrap();
        let a = uuid_v5(&ns, b"example.com");
        let b = uuid_v5(&ns, b"example.org");
        assert_ne!(a, b);
    }
    #[test]
    fn v5_different_namespaces_differ() {
        let ns_dns = resolve_namespace("dns").unwrap();
        let ns_url = resolve_namespace("url").unwrap();
        let a = uuid_v5(&ns_dns, b"example.com");
        let b = uuid_v5(&ns_url, b"example.com");
        assert_ne!(a, b);
    }
    #[test]
    fn v5_format_valid() {
        let ns = resolve_namespace("dns").unwrap();
        let u = uuid_v5(&ns, b"example.com");
        assert!(is_valid_uuid(&u), "v5 UUID should be valid: {u}");
    }

    // ── validate ───────────────────────────────────────────────────────────────
    #[test]
    fn validate_wrong_group_lengths() {
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000"));
    }
    #[test]
    fn validate_extra_dash() {
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-4466554400001"));
    }
    #[test]
    fn v4_is_valid() {
        assert!(is_valid_uuid(&uuid_v4()));
    }

    // ── parse ──────────────────────────────────────────────────────────────────────
    #[test]
    fn parse_returns_correct_fields() {
        let json = parse_uuid("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["version"], 4);
        assert!(v["uuid"].as_str().unwrap().contains("550e8400"));
    }
    #[test]
    fn parse_nil_uuid() {
        let json = parse_uuid("00000000-0000-0000-0000-000000000000").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["version"], 0);
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.uuid");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_start_with_uuid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("uuid."));
        }
    }
}
