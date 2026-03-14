//! skill-compress — lossless compression and decompression skills.
//!
//! Skills exposed:
//!   compress.deflate   { data: string(base64) }  → base64
//!   compress.inflate   { data: string(base64) }  → base64
//!   compress.rle_encode { data: string(base64) } → base64
//!   compress.rle_decode { data: string(base64) } → base64
//!   compress.lz_encode  { text: string }         → base64
//!   compress.lz_decode  { data: string(base64) } → string
//!
//! All algorithms are pure-Rust, no OS/WASI calls needed.
//! deflate/inflate implement RFC 1951 DEFLATE (stored blocks only — lossless, simple).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.compress",
  "name": "Compress Skills",
  "version": "0.1.0",
  "description": "Lossless compression: DEFLATE (stored blocks), RLE, LZ77-lite",
  "skills": [
    {
      "name": "compress.deflate",
      "display": "DEFLATE Compress",
      "description": "Compress base64-encoded bytes using DEFLATE stored blocks (RFC 1951). Returns base64.",
      "risk": "safe",
      "params": [
        { "name": "data", "type": "string", "description": "Base64-encoded input bytes", "required": true }
      ]
    },
    {
      "name": "compress.inflate",
      "display": "DEFLATE Decompress",
      "description": "Decompress DEFLATE stored-block data (base64 input). Returns base64.",
      "risk": "safe",
      "params": [
        { "name": "data", "type": "string", "description": "Base64-encoded DEFLATE data", "required": true }
      ]
    },
    {
      "name": "compress.rle_encode",
      "display": "RLE Encode",
      "description": "Run-length encode base64-encoded bytes. Returns base64.",
      "risk": "safe",
      "params": [
        { "name": "data", "type": "string", "description": "Base64-encoded input bytes", "required": true }
      ]
    },
    {
      "name": "compress.rle_decode",
      "display": "RLE Decode",
      "description": "Decode RLE-encoded data (base64 input). Returns base64.",
      "risk": "safe",
      "params": [
        { "name": "data", "type": "string", "description": "Base64-encoded RLE data", "required": true }
      ]
    },
    {
      "name": "compress.lz_encode",
      "display": "LZ77-lite Encode",
      "description": "Compress a UTF-8 string using LZ77-lite. Returns base64.",
      "risk": "safe",
      "params": [
        { "name": "text", "type": "string", "description": "UTF-8 text to compress", "required": true }
      ]
    },
    {
      "name": "compress.lz_decode",
      "display": "LZ77-lite Decode",
      "description": "Decompress LZ77-lite data (base64 input). Returns UTF-8 string.",
      "risk": "safe",
      "params": [
        { "name": "data", "type": "string", "description": "Base64-encoded LZ77 data", "required": true }
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
        "compress.deflate" => {
            let b64 = match req.args["data"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'data'"),
            };
            match base64_decode(b64) {
                Ok(data) => sdk_respond_ok(rid, &base64_encode(&deflate_stored(&data))),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "compress.inflate" => {
            let b64 = match req.args["data"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'data'"),
            };
            match base64_decode(b64).and_then(|d| inflate_stored(&d)) {
                Ok(data) => sdk_respond_ok(rid, &base64_encode(&data)),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "compress.rle_encode" => {
            let b64 = match req.args["data"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'data'"),
            };
            match base64_decode(b64) {
                Ok(data) => sdk_respond_ok(rid, &base64_encode(&rle_encode(&data))),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "compress.rle_decode" => {
            let b64 = match req.args["data"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'data'"),
            };
            match base64_decode(b64).and_then(|d| rle_decode(&d)) {
                Ok(data) => sdk_respond_ok(rid, &base64_encode(&data)),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "compress.lz_encode" => {
            let text = match req.args["text"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'"),
            };
            sdk_respond_ok(rid, &base64_encode(&lz_encode(text.as_bytes())))
        }
        "compress.lz_decode" => {
            let b64 = match req.args["data"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'data'"),
            };
            match base64_decode(b64).and_then(|d| lz_decode(&d)) {
                Ok(text) => sdk_respond_ok(rid, &text),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── DEFLATE stored blocks (RFC 1951 §3.2.4) ───────────────────────────────────
// "Stored" mode: no compression, just wraps data in DEFLATE block headers.
// This is compliant DEFLATE and can be inflated by any standard library.

const DEFLATE_BLOCK_SIZE: usize = 65535;

fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offset = 0;
    while offset < data.len() || data.is_empty() {
        let end = (offset + DEFLATE_BLOCK_SIZE).min(data.len());
        let chunk = &data[offset..end];
        let is_final = end == data.len();
        let bfinal: u8 = if is_final { 1 } else { 0 };
        out.push(bfinal | 0x00); // BFINAL | BTYPE=00 (no compression)
        let len = chunk.len() as u16;
        let nlen = !len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(chunk);
        if data.is_empty() { break; }
        offset = end;
    }
    out
}

fn inflate_stored(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        if i >= data.len() { return Err("unexpected end of DEFLATE stream".into()); }
        let header = data[i]; i += 1;
        let bfinal = header & 1;
        let btype  = (header >> 1) & 3;
        if btype != 0 { return Err(format!("unsupported DEFLATE block type {}", btype)); }
        if i + 4 > data.len() { return Err("truncated DEFLATE block header".into()); }
        let len  = u16::from_le_bytes([data[i], data[i+1]]) as usize; i += 2;
        let nlen = u16::from_le_bytes([data[i], data[i+1]]) as usize; i += 2;
        if (len ^ nlen) != 0xffff { return Err("DEFLATE LEN/NLEN mismatch".into()); }
        if i + len > data.len() { return Err("DEFLATE block data truncated".into()); }
        out.extend_from_slice(&data[i..i+len]);
        i += len;
        if bfinal == 1 { break; }
    }
    Ok(out)
}

// ── RLE (run-length encoding) ─────────────────────────────────────────────────
// Format: [count: u8][byte: u8] pairs. count=1..=255.

fn rle_encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return Vec::new(); }
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut count: u8 = 1;
        while i + (count as usize) < data.len()
            && data[i + count as usize] == byte
            && count < 255
        {
            count += 1;
        }
        out.push(count);
        out.push(byte);
        i += count as usize;
    }
    out
}

fn rle_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() % 2 != 0 { return Err("RLE data must have even length".into()); }
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let count = data[i] as usize;
        let byte  = data[i + 1];
        if count == 0 { return Err("RLE count of 0 is invalid".into()); }
        for _ in 0..count { out.push(byte); }
        i += 2;
    }
    Ok(out)
}

// ── LZ77-lite ─────────────────────────────────────────────────────────────────
// Simple LZ77 variant:
//   Literal token: 0x00 | byte
//   Back-reference: 0xFF | dist_hi | dist_lo | len  (dist > 0, len >= 3)

fn lz_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let (best_dist, best_len) = find_match(data, i);
        if best_len >= 3 {
            out.push(0xFF);
            out.extend_from_slice(&(best_dist as u16).to_le_bytes());
            out.push(best_len as u8);
            i += best_len;
        } else {
            out.push(0x00);
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

fn find_match(data: &[u8], pos: usize) -> (usize, usize) {
    let window_start = pos.saturating_sub(255);
    let mut best_dist = 0;
    let mut best_len = 0;
    for start in window_start..pos {
        let dist = pos - start;
        let max_len = (data.len() - pos).min(255);
        let mut len = 0;
        while len < max_len && data[start + len] == data[pos + len] {
            len += 1;
        }
        if len > best_len {
            best_len = len;
            best_dist = dist;
        }
    }
    (best_dist, best_len)
}

fn lz_decode(data: &[u8]) -> Result<String, String> {
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0x00 {
            if i + 1 >= data.len() { return Err("LZ literal token truncated".into()); }
            out.push(data[i + 1]);
            i += 2;
        } else if data[i] == 0xFF {
            if i + 3 >= data.len() { return Err("LZ backref token truncated".into()); }
            let dist = u16::from_le_bytes([data[i+1], data[i+2]]) as usize;
            let len  = data[i+3] as usize;
            if dist == 0 || dist > out.len() {
                return Err(format!("LZ invalid back-reference dist={}", dist));
            }
            let base = out.len() - dist;
            for j in 0..len { out.push(out[base + j]); }
            i += 4;
        } else {
            return Err(format!("LZ unknown token 0x{:02x}", data[i]));
        }
    }
    String::from_utf8(out).map_err(|_| "LZ decoded bytes are not valid UTF-8".into())
}

// ── Base64 helpers ────────────────────────────────────────────────────────────

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n  = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[(n >> 18) as usize] as char);
        out.push(B64[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 { out.push(B64[((n >> 6) & 0x3f) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(B64[(n & 0x3f) as usize] as char); }         else { out.push('='); }
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim_end_matches('=');
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let val = |c: u8| -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+'        => Ok(62),
            b'/'        => Ok(63),
            _           => Err(format!("invalid base64 char: {}", c as char)),
        }
    };
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let n0 = val(bytes[i])?;
        let n1 = val(bytes[i+1])?;
        out.push(((n0 << 2) | (n1 >> 4)) as u8);
        if i + 2 < bytes.len() {
            let n2 = val(bytes[i+2])?;
            out.push(((n1 << 4) | (n2 >> 2)) as u8);
            if i + 3 < bytes.len() {
                let n3 = val(bytes[i+3])?;
                out.push(((n2 << 6) | n3) as u8);
            }
        }
        i += 4;
    }
    Ok(out)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deflate_inflate_roundtrip() {
        let data = b"Hello, World! Hello, World! Hello, World!";
        let compressed = deflate_stored(data);
        let decompressed = inflate_stored(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn deflate_empty() {
        let compressed = deflate_stored(b"");
        let decompressed = inflate_stored(&compressed).unwrap();
        assert_eq!(decompressed, b"");
    }

    #[test]
    fn rle_roundtrip() {
        let data = b"AAABBBBBCCDDDDD";
        let enc = rle_encode(data);
        let dec = rle_decode(&enc).unwrap();
        assert_eq!(dec, data);
    }

    #[test]
    fn rle_no_runs() {
        let data = b"abcdef";
        let enc = rle_encode(data);
        assert_eq!(enc.len(), data.len() * 2);
        assert_eq!(rle_decode(&enc).unwrap(), data);
    }

    #[test]
    fn lz_roundtrip_repeated() {
        let text = "abcabcabcabcabc";
        let enc = lz_encode(text.as_bytes());
        let dec = lz_decode(&enc).unwrap();
        assert_eq!(dec, text);
    }

    #[test]
    fn lz_roundtrip_unique() {
        let text = "The quick brown fox";
        let enc = lz_encode(text.as_bytes());
        let dec = lz_decode(&enc).unwrap();
        assert_eq!(dec, text);
    }

    #[test]
    fn base64_roundtrip() {
        let data = b"OpenClaw+ WASM skills";
        assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
    }

    #[test]
    fn deflate_single_byte() {
        let data = b"X";
        let compressed = deflate_stored(data);
        let decompressed = inflate_stored(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn deflate_large_data() {
        let data: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
        let compressed = deflate_stored(&data);
        let decompressed = inflate_stored(&compressed).unwrap();
        assert_eq!(decompressed, data.as_slice());
    }

    #[test]
    fn deflate_all_zeros() {
        let data = vec![0u8; 64];
        assert_eq!(inflate_stored(&deflate_stored(&data)).unwrap(), data);
    }

    #[test]
    fn rle_empty() {
        let enc = rle_encode(b"");
        assert_eq!(rle_decode(&enc).unwrap(), b"");
    }

    #[test]
    fn rle_single_byte() {
        let data = b"A";
        assert_eq!(rle_decode(&rle_encode(data)).unwrap(), data.as_ref());
    }

    #[test]
    fn rle_max_run() {
        let data = vec![0xFFu8; 255];
        let enc = rle_encode(&data);
        assert_eq!(rle_decode(&enc).unwrap(), data);
    }

    #[test]
    fn lz_empty() {
        let enc = lz_encode(b"");
        assert_eq!(lz_decode(&enc).unwrap(), "");
    }

    #[test]
    fn lz_single_char() {
        let text = "Z";
        assert_eq!(lz_decode(&lz_encode(text.as_bytes())).unwrap(), text);
    }

    #[test]
    fn lz_longer_repeated() {
        let text = "aaaaaaaaaaaaaaaa";
        assert_eq!(lz_decode(&lz_encode(text.as_bytes())).unwrap(), text);
    }

    #[test]
    fn base64_empty() {
        assert_eq!(base64_decode(&base64_encode(b"")).unwrap(), b"");
    }

    #[test]
    fn base64_known_vector() {
        assert_eq!(base64_encode(b"Man"), "TWFu");
    }

    #[test]
    fn base64_decode_known() {
        assert_eq!(base64_decode("SGVsbG8=").unwrap(), b"Hello");
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.compress");
        assert_eq!(v["skills"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn manifest_skill_names_start_with_compress() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("compress."));
        }
    }
}
