use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.bits",
  "name": "Bitwise Utilities",
  "version": "0.1.0",
  "description": "10 bitwise/binary skills: and, or, xor, not, shift_left, shift_right, popcount, to_binary, from_binary, to_hex.",
  "skills": [
    {"name":"bits.and","display":"Bitwise AND","description":"Bitwise AND of two 64-bit integers.","risk":"safe","params":[{"name":"a","type":"integer","required":true},{"name":"b","type":"integer","required":true}]},
    {"name":"bits.or","display":"Bitwise OR","description":"Bitwise OR of two 64-bit integers.","risk":"safe","params":[{"name":"a","type":"integer","required":true},{"name":"b","type":"integer","required":true}]},
    {"name":"bits.xor","display":"Bitwise XOR","description":"Bitwise XOR of two 64-bit integers.","risk":"safe","params":[{"name":"a","type":"integer","required":true},{"name":"b","type":"integer","required":true}]},
    {"name":"bits.not","display":"Bitwise NOT","description":"Bitwise NOT (bitwise complement) of a 64-bit integer.","risk":"safe","params":[{"name":"a","type":"integer","required":true}]},
    {"name":"bits.shift_left","display":"Shift Left","description":"Left-shift a by n bits.","risk":"safe","params":[{"name":"a","type":"integer","required":true},{"name":"n","type":"integer","required":true}]},
    {"name":"bits.shift_right","display":"Shift Right","description":"Logical right-shift a by n bits.","risk":"safe","params":[{"name":"a","type":"integer","required":true},{"name":"n","type":"integer","required":true}]},
    {"name":"bits.popcount","display":"Popcount","description":"Count the number of set bits (1s) in a 64-bit integer.","risk":"safe","params":[{"name":"a","type":"integer","required":true}]},
    {"name":"bits.to_binary","display":"To Binary","description":"Convert an integer to its binary string representation.","risk":"safe","params":[{"name":"a","type":"integer","required":true},{"name":"width","type":"integer","description":"Zero-pad to this width","required":false}]},
    {"name":"bits.from_binary","display":"From Binary","description":"Parse a binary string (e.g. '1011') to an integer.","risk":"safe","params":[{"name":"binary","type":"string","required":true}]},
    {"name":"bits.to_hex","display":"To Hex","description":"Convert an integer to its hexadecimal string representation.","risk":"safe","params":[{"name":"a","type":"integer","required":true},{"name":"uppercase","type":"boolean","description":"Use uppercase letters (default true)","required":false}]}
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
    let args = &req.args;

    let a = args["a"].as_i64().unwrap_or(0);
    let b = args["b"].as_i64().unwrap_or(0);

    match req.skill.as_str() {
        "bits.and"         => sdk_respond_ok(rid, &(a & b).to_string()),
        "bits.or"          => sdk_respond_ok(rid, &(a | b).to_string()),
        "bits.xor"         => sdk_respond_ok(rid, &(a ^ b).to_string()),
        "bits.not"         => sdk_respond_ok(rid, &(!a).to_string()),
        "bits.shift_left"  => {
            let n = args["n"].as_u64().unwrap_or(0).min(63) as u32;
            sdk_respond_ok(rid, &(a << n).to_string())
        }
        "bits.shift_right" => {
            let n = args["n"].as_u64().unwrap_or(0).min(63) as u32;
            sdk_respond_ok(rid, &((a as u64 >> n) as i64).to_string())
        }
        "bits.popcount"    => sdk_respond_ok(rid, &(a as u64).count_ones().to_string()),
        "bits.to_binary"   => {
            let width = args["width"].as_u64().unwrap_or(0) as usize;
            let bin = format!("{:b}", a as u64);
            let result = if width > bin.len() {
                format!("{:0>width$}", bin, width = width)
            } else {
                bin
            };
            sdk_respond_ok(rid, &result)
        }
        "bits.from_binary" => {
            let s = args["binary"].as_str().unwrap_or("0");
            match i64::from_str_radix(s.trim_start_matches("0b"), 2) {
                Ok(v)  => sdk_respond_ok(rid, &v.to_string()),
                Err(_) => sdk_respond_err(rid, "invalid binary string"),
            }
        }
        "bits.to_hex" => {
            let upper = args["uppercase"].as_bool().unwrap_or(true);
            let result = if upper { format!("{:X}", a as u64) } else { format!("{:x}", a as u64) };
            sdk_respond_ok(rid, &result)
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    // ── bits.and ────────────────────────────────────────────────────────────
    #[test] fn and_bits()    { assert_eq!(0b1010i64 & 0b1100, 0b1000); }
    #[test] fn and_zero()    { assert_eq!(0xFF_i64 & 0, 0); }
    #[test] fn and_identity(){ assert_eq!(0b1111i64 & 0b1111, 0b1111); }

    // ── bits.or ─────────────────────────────────────────────────────────────
    #[test] fn or_bits()     { assert_eq!(0b1010i64 | 0b0101, 0b1111); }
    #[test] fn or_zero()     { assert_eq!(0b1010i64 | 0, 0b1010); }

    // ── bits.xor ────────────────────────────────────────────────────────────
    #[test] fn xor_bits()    { assert_eq!(0b1010i64 ^ 0b1111, 0b0101); }
    #[test] fn xor_self()    { assert_eq!(0b1010i64 ^ 0b1010, 0); }

    // ── bits.not ────────────────────────────────────────────────────────────
    #[test] fn not_zero()    { assert_eq!(!0i64, -1i64); }
    #[test] fn not_neg1()    { assert_eq!(!(-1i64), 0i64); }

    // ── bits.shift_left / shift_right ──────────────────────────────────
    #[test] fn shift_left()  { assert_eq!(1i64 << 3, 8); }
    #[test] fn shift_left_4() { assert_eq!(1i64 << 4, 16); }
    #[test] fn shift_right() { assert_eq!((16u64 >> 2) as i64, 4); }
    #[test] fn shift_right_0() { assert_eq!((8u64 >> 0) as i64, 8); }

    // ── bits.popcount ──────────────────────────────────────────────────────
    #[test] fn popcount()    { assert_eq!((0b10110100u64).count_ones(), 4); }
    #[test] fn popcount_zero(){ assert_eq!((0u64).count_ones(), 0); }
    #[test] fn popcount_ff() { assert_eq!((0xFFu64).count_ones(), 8); }

    // ── bits.to_binary ────────────────────────────────────────────────────
    #[test] fn to_binary()   { assert_eq!(format!("{:b}", 42u64), "101010"); }
    #[test] fn to_binary_1() { assert_eq!(format!("{:b}", 1u64), "1"); }
    #[test] fn to_binary_padded() {
        let bin = format!("{:b}", 5u64);
        let padded = format!("{:0>8}", bin);
        assert_eq!(padded, "00000101");
    }

    // ── bits.from_binary ───────────────────────────────────────────────────
    #[test] fn from_binary() { assert_eq!(i64::from_str_radix("101010", 2).unwrap(), 42); }
    #[test] fn from_binary_0b_prefix() {
        let s = "0b1011";
        let stripped = s.trim_start_matches("0b");
        assert_eq!(i64::from_str_radix(stripped, 2).unwrap(), 11);
    }
    #[test] fn from_binary_zero() { assert_eq!(i64::from_str_radix("0", 2).unwrap(), 0); }

    // ── bits.to_hex ────────────────────────────────────────────────────────
    #[test] fn to_hex_upper()  { assert_eq!(format!("{:X}", 255u64), "FF"); }
    #[test] fn to_hex_lower()  { assert_eq!(format!("{:x}", 255u64), "ff"); }
    #[test] fn to_hex_zero()   { assert_eq!(format!("{:X}", 0u64), "0"); }

    // ── manifest ───────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(super::MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(super::MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("bits."));
        }
    }
}
