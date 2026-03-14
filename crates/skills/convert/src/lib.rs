//! skill-convert — type and format conversion skills (pure-Rust, no I/O).
//!
//! Skills:
//!   convert.to_int      { value: any }          → integer string
//!   convert.to_float    { value: any }          → float string
//!   convert.to_bool     { value: any }          → "true"|"false"
//!   convert.to_string   { value: any }          → string
//!   convert.int_to_base { value: i64, base: u32 } → string representation
//!   convert.base_to_int { value: str, base: u32 } → decimal integer string
//!   convert.bytes_to_human { bytes: u64 }       → "1.23 MB" etc.

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.convert",
  "name": "Convert Skills",
  "version": "0.1.0",
  "description": "Type conversions, base conversions, and human-readable formatting",
  "skills": [
    {
      "name": "convert.to_int",
      "display": "To Integer",
      "description": "Convert a value to an integer (truncates floats, parses strings).",
      "risk": "safe",
      "params": [{ "name": "value", "type": "any", "required": true }]
    },
    {
      "name": "convert.to_float",
      "display": "To Float",
      "description": "Convert a value to a floating-point number.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "any", "required": true }]
    },
    {
      "name": "convert.to_bool",
      "display": "To Boolean",
      "description": "Convert a value to boolean. 0/empty/null/false → false, otherwise true.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "any", "required": true }]
    },
    {
      "name": "convert.to_string",
      "display": "To String",
      "description": "Convert any JSON value to its string representation.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "any", "required": true }]
    },
    {
      "name": "convert.int_to_base",
      "display": "Integer to Base",
      "description": "Convert integer to a string in the given base (2–36).",
      "risk": "safe",
      "params": [
        { "name": "value", "type": "integer", "required": true },
        { "name": "base",  "type": "integer", "required": true }
      ]
    },
    {
      "name": "convert.base_to_int",
      "display": "Base to Integer",
      "description": "Parse a string in the given base (2–36) and return decimal integer.",
      "risk": "safe",
      "params": [
        { "name": "value", "type": "string",  "required": true },
        { "name": "base",  "type": "integer", "required": true }
      ]
    },
    {
      "name": "convert.bytes_to_human",
      "display": "Bytes to Human",
      "description": "Format a byte count as a human-readable string (KB/MB/GB/TB).",
      "risk": "safe",
      "params": [{ "name": "bytes", "type": "integer", "required": true }]
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

    match req.skill.as_str() {
        "convert.to_int" => {
            match to_int(&req.args["value"]) {
                Ok(n)  => sdk_respond_ok(rid, &n.to_string()),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "convert.to_float" => {
            match to_float(&req.args["value"]) {
                Ok(f)  => sdk_respond_ok(rid, &format!("{}", f)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "convert.to_bool" => {
            sdk_respond_ok(rid, if to_bool(&req.args["value"]) { "true" } else { "false" })
        }
        "convert.to_string" => {
            sdk_respond_ok(rid, &to_str(&req.args["value"]))
        }
        "convert.int_to_base" => {
            let n = match req.args["value"].as_i64() { Some(v) => v, None => return sdk_respond_err(rid, "missing integer 'value'") };
            let base = match req.args["base"].as_u64() { Some(v) => v as u32, None => return sdk_respond_err(rid, "missing integer 'base'") };
            match int_to_base(n, base) {
                Ok(s)  => sdk_respond_ok(rid, &s),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "convert.base_to_int" => {
            let s = match req.args["value"].as_str() { Some(v) => v, None => return sdk_respond_err(rid, "missing string 'value'") };
            let base = match req.args["base"].as_u64() { Some(v) => v as u32, None => return sdk_respond_err(rid, "missing integer 'base'") };
            match base_to_int(s, base) {
                Ok(n)  => sdk_respond_ok(rid, &n.to_string()),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "convert.bytes_to_human" => {
            let bytes = match req.args["bytes"].as_u64() { Some(v) => v, None => return sdk_respond_err(rid, "missing integer 'bytes'") };
            sdk_respond_ok(rid, &bytes_to_human(bytes))
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Conversion logic ──────────────────────────────────────────────────────────

fn to_int(v: &serde_json::Value) -> Result<i64, String> {
    if let Some(n) = v.as_i64() { return Ok(n); }
    if let Some(f) = v.as_f64() { return Ok(f as i64); }
    if let Some(s) = v.as_str() { return s.trim().parse::<i64>().map_err(|e| e.to_string()); }
    if let Some(b) = v.as_bool() { return Ok(if b { 1 } else { 0 }); }
    Err(format!("cannot convert {} to integer", v))
}

fn to_float(v: &serde_json::Value) -> Result<f64, String> {
    if let Some(f) = v.as_f64() { return Ok(f); }
    if let Some(s) = v.as_str() { return s.trim().parse::<f64>().map_err(|e| e.to_string()); }
    if let Some(b) = v.as_bool() { return Ok(if b { 1.0 } else { 0.0 }); }
    Err(format!("cannot convert {} to float", v))
}

fn to_bool(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Bool(b)   => *b,
        serde_json::Value::Null      => false,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        serde_json::Value::Array(a)  => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

fn to_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn int_to_base(n: i64, base: u32) -> Result<String, String> {
    if !(2..=36).contains(&base) { return Err(format!("base must be 2–36, got {base}")); }
    if n == 0 { return Ok("0".to_string()); }
    let digits: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let neg = n < 0;
    let mut m = n.unsigned_abs();
    let mut chars = Vec::new();
    while m > 0 {
        chars.push(digits[(m % base as u64) as usize] as char);
        m /= base as u64;
    }
    if neg { chars.push('-'); }
    chars.reverse();
    Ok(chars.iter().collect())
}

fn base_to_int(s: &str, base: u32) -> Result<i64, String> {
    if !(2..=36).contains(&base) { return Err(format!("base must be 2–36, got {base}")); }
    i64::from_str_radix(s.trim(), base).map_err(|e| e.to_string())
}

fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    if bytes >= TB      { format!("{:.2} TB", bytes as f64 / TB as f64) }
    else if bytes >= GB { format!("{:.2} GB", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.2} MB", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.2} KB", bytes as f64 / KB as f64) }
    else                { format!("{} B", bytes) }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test] fn to_int_from_int()   { assert_eq!(to_int(&json!(42)).unwrap(), 42); }
    #[test] fn to_int_from_float() { assert_eq!(to_int(&json!(3.9)).unwrap(), 3); }
    #[test] fn to_int_from_str()   { assert_eq!(to_int(&json!("99")).unwrap(), 99); }
    #[test] fn to_int_from_bool()  { assert_eq!(to_int(&json!(true)).unwrap(), 1); }
    #[test] fn to_int_bad_str()    { assert!(to_int(&json!("abc")).is_err()); }

    #[test] fn to_float_from_int() { assert!((to_float(&json!(5)).unwrap() - 5.0).abs() < 1e-9); }
    #[test] fn to_float_from_str() { assert!((to_float(&json!("3.14")).unwrap() - 3.14).abs() < 1e-6); }

    #[test] fn to_bool_false_vals() {
        assert!(!to_bool(&json!(false)));
        assert!(!to_bool(&serde_json::Value::Null));
        assert!(!to_bool(&json!(0)));
        assert!(!to_bool(&json!("")));
    }
    #[test] fn to_bool_true_vals() {
        assert!(to_bool(&json!(1)));
        assert!(to_bool(&json!("hello")));
        assert!(to_bool(&json!(true)));
    }

    #[test] fn to_str_string()  { assert_eq!(to_str(&json!("hi")), "hi"); }
    #[test] fn to_str_number()  { assert_eq!(to_str(&json!(42)), "42"); }

    #[test] fn base2()    { assert_eq!(int_to_base(10, 2).unwrap(), "1010"); }
    #[test] fn base16()   { assert_eq!(int_to_base(255, 16).unwrap(), "ff"); }
    #[test] fn base_neg() { assert_eq!(int_to_base(-10, 2).unwrap(), "-1010"); }
    #[test] fn base_zero(){ assert_eq!(int_to_base(0, 10).unwrap(), "0"); }
    #[test] fn base_bad() { assert!(int_to_base(10, 1).is_err()); }

    #[test] fn from_base2()  { assert_eq!(base_to_int("1010", 2).unwrap(), 10); }
    #[test] fn from_base16() { assert_eq!(base_to_int("ff", 16).unwrap(), 255); }

    #[test] fn bytes_b()  { assert_eq!(bytes_to_human(512), "512 B"); }
    #[test] fn bytes_kb() { assert!(bytes_to_human(2048).contains("KB")); }
    #[test] fn bytes_mb() { assert!(bytes_to_human(1024 * 1024 * 3).contains("MB")); }
    #[test] fn bytes_gb() { assert!(bytes_to_human(1024 * 1024 * 1024 * 2).contains("GB")); }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.convert");
        assert_eq!(v["skills"].as_array().unwrap().len(), 7);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("convert."));
        }
    }
}
