//! skill-crypto — symmetric encryption, key derivation, and MAC skills.
//!
//! Skills exposed:
//!   crypto.aes_encrypt   { plaintext: string, key: string, mode?: "gcm"|"ctr" }  → base64 ciphertext
//!   crypto.aes_decrypt   { ciphertext: string, key: string, mode?: "gcm"|"ctr" } → plaintext
//!   crypto.chacha20      { plaintext: string, key: string, nonce: string }        → base64 ciphertext
//!   crypto.pbkdf2        { password: string, salt: string, iterations?: u32, len?: u32 } → hex key
//!   crypto.hmac_sha512   { input: string, key: string }                           → hex string
//!   crypto.constant_time_eq { a: string, b: string }                              → bool
//!
//! All algorithms are pure-Rust, no OS/WASI calls needed.

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.crypto",
  "name": "Crypto Skills",
  "version": "0.1.0",
  "description": "Symmetric encryption (AES-CTR, ChaCha20), PBKDF2 key derivation, HMAC-SHA-512",
  "skills": [
    {
      "name": "crypto.aes_encrypt",
      "display": "AES-128-CTR Encrypt",
      "description": "Encrypt plaintext with AES-128-CTR. Key must be 16 bytes (hex). Returns base64(iv||ciphertext).",
      "risk": "safe",
      "params": [
        { "name": "plaintext", "type": "string", "description": "UTF-8 plaintext",         "required": true },
        { "name": "key",       "type": "string", "description": "16-byte key as hex (32 chars)", "required": true }
      ]
    },
    {
      "name": "crypto.aes_decrypt",
      "display": "AES-128-CTR Decrypt",
      "description": "Decrypt base64(iv||ciphertext) produced by crypto.aes_encrypt.",
      "risk": "safe",
      "params": [
        { "name": "ciphertext", "type": "string", "description": "Base64 encoded iv||ciphertext", "required": true },
        { "name": "key",        "type": "string", "description": "16-byte key as hex (32 chars)", "required": true }
      ]
    },
    {
      "name": "crypto.chacha20",
      "display": "ChaCha20 Encrypt/Decrypt",
      "description": "XOR stream cipher. Same function for encrypt and decrypt. Key=32 bytes hex, nonce=12 bytes hex.",
      "risk": "safe",
      "params": [
        { "name": "data",  "type": "string", "description": "Input bytes as base64",   "required": true },
        { "name": "key",   "type": "string", "description": "32-byte key as hex",      "required": true },
        { "name": "nonce", "type": "string", "description": "12-byte nonce as hex",    "required": true }
      ]
    },
    {
      "name": "crypto.pbkdf2",
      "display": "PBKDF2-SHA256 Key Derivation",
      "description": "Derive a cryptographic key from a password using PBKDF2-HMAC-SHA256.",
      "risk": "safe",
      "params": [
        { "name": "password",   "type": "string",  "description": "Password string",        "required": true },
        { "name": "salt",       "type": "string",  "description": "Salt string",             "required": true },
        { "name": "iterations", "type": "integer", "description": "Iteration count (default 100000)", "required": false },
        { "name": "len",        "type": "integer", "description": "Output key length in bytes (default 32)", "required": false }
      ]
    },
    {
      "name": "crypto.hmac_sha512",
      "display": "HMAC-SHA-512",
      "description": "Compute HMAC-SHA-512 message authentication code (hex output).",
      "risk": "safe",
      "params": [
        { "name": "input", "type": "string", "description": "Message",    "required": true },
        { "name": "key",   "type": "string", "description": "Secret key", "required": true }
      ]
    },
    {
      "name": "crypto.constant_time_eq",
      "display": "Constant-Time String Compare",
      "description": "Compare two strings in constant time to prevent timing attacks. Returns true/false.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "string", "description": "First value",  "required": true },
        { "name": "b", "type": "string", "description": "Second value", "required": true }
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
        "crypto.aes_encrypt" => {
            let pt  = match req.args["plaintext"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'plaintext'") };
            let key = match req.args["key"].as_str()       { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'") };
            match aes_ctr_encrypt(pt.as_bytes(), key) {
                Ok(b64) => sdk_respond_ok(rid, &b64),
                Err(e)  => sdk_respond_err(rid, &e),
            }
        }
        "crypto.aes_decrypt" => {
            let ct  = match req.args["ciphertext"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'ciphertext'") };
            let key = match req.args["key"].as_str()        { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'") };
            match aes_ctr_decrypt(ct, key) {
                Ok(pt) => sdk_respond_ok(rid, &pt),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "crypto.chacha20" => {
            let data  = match req.args["data"].as_str()  { Some(s) => s, None => return sdk_respond_err(rid, "missing 'data'") };
            let key   = match req.args["key"].as_str()   { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'") };
            let nonce = match req.args["nonce"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'nonce'") };
            let bytes = match base64_decode(data) {
                Ok(b)  => b,
                Err(e) => return sdk_respond_err(rid, &e),
            };
            match chacha20_xor(&bytes, key, nonce) {
                Ok(out) => sdk_respond_ok(rid, &base64_encode(&out)),
                Err(e)  => sdk_respond_err(rid, &e),
            }
        }
        "crypto.pbkdf2" => {
            let password   = match req.args["password"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'password'") };
            let salt       = match req.args["salt"].as_str()     { Some(s) => s, None => return sdk_respond_err(rid, "missing 'salt'") };
            let iterations = req.args["iterations"].as_u64().unwrap_or(100_000) as u32;
            let out_len    = req.args["len"].as_u64().unwrap_or(32) as usize;
            let derived    = pbkdf2_sha256(password.as_bytes(), salt.as_bytes(), iterations, out_len);
            sdk_respond_ok(rid, &hex_encode(&derived))
        }
        "crypto.hmac_sha512" => {
            let input = match req.args["input"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'input'") };
            let key   = match req.args["key"].as_str()   { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'") };
            sdk_respond_ok(rid, &hmac_sha512(key.as_bytes(), input.as_bytes()))
        }
        "crypto.constant_time_eq" => {
            let a = match req.args["a"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'b'") };
            let eq = constant_time_eq(a.as_bytes(), b.as_bytes());
            sdk_respond_ok(rid, if eq { "true" } else { "false" })
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Hex / Base64 helpers ──────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 { return Err("hex string must have even length".into()); }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i+2], 16).map_err(|e| e.to_string()))
        .collect()
}

const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n  = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_CHARS[(n >> 18) as usize] as char);
        out.push(B64_CHARS[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 { out.push(B64_CHARS[((n >> 6) & 0x3f) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(B64_CHARS[(n & 0x3f) as usize] as char); }         else { out.push('='); }
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

// ── AES-128-CTR ───────────────────────────────────────────────────────────────
// Tiny pure-Rust AES-128 for wasm32 (no std, no heap except Vec).

fn aes_ctr_encrypt(plaintext: &[u8], key_hex: &str) -> Result<String, String> {
    let key = hex_decode(key_hex)?;
    if key.len() != 16 { return Err("AES-128 key must be 16 bytes (32 hex chars)".into()); }
    let iv = pseudo_random_iv(plaintext.len() as u64 ^ 0xdeadbeef);
    let keystream = aes_ctr_keystream(&key, &iv, plaintext.len());
    let mut ct: Vec<u8> = plaintext.iter().zip(keystream.iter()).map(|(a, b)| a ^ b).collect();
    let mut out = iv.to_vec();
    out.append(&mut ct);
    Ok(base64_encode(&out))
}

fn aes_ctr_decrypt(ciphertext_b64: &str, key_hex: &str) -> Result<String, String> {
    let data = base64_decode(ciphertext_b64)?;
    if data.len() < 16 { return Err("ciphertext too short".into()); }
    let key = hex_decode(key_hex)?;
    if key.len() != 16 { return Err("AES-128 key must be 16 bytes (32 hex chars)".into()); }
    let (iv, ct) = data.split_at(16);
    let iv: [u8; 16] = iv.try_into().unwrap();
    let keystream = aes_ctr_keystream(&key, &iv, ct.len());
    let pt: Vec<u8> = ct.iter().zip(keystream.iter()).map(|(a, b)| a ^ b).collect();
    String::from_utf8(pt).map_err(|_| "decrypted bytes are not valid UTF-8".into())
}

fn pseudo_random_iv(seed: u64) -> [u8; 16] {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut iv = [0u8; 16];
    for chunk in iv.chunks_mut(8) {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let b = state.to_le_bytes();
        chunk.copy_from_slice(&b[..chunk.len()]);
    }
    iv
}

fn aes_ctr_keystream(key: &[u8], iv: &[u8; 16], len: usize) -> Vec<u8> {
    let round_keys = aes128_key_expansion(key.try_into().unwrap());
    let mut counter = u128::from_be_bytes(*iv);
    let mut stream = Vec::with_capacity(len);
    while stream.len() < len {
        let block = counter.to_be_bytes();
        let enc = aes128_encrypt_block(&block, &round_keys);
        stream.extend_from_slice(&enc);
        counter = counter.wrapping_add(1);
    }
    stream.truncate(len);
    stream
}

// AES-128: FIPS-197 key expansion
fn aes128_key_expansion(key: [u8; 16]) -> [[u8; 16]; 11] {
    const SBOX: [u8; 256] = [
        0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
        0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
        0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
        0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
        0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
        0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
        0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
        0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
        0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
        0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
        0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
        0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
        0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
        0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
        0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
        0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
    ];
    const RCON: [u8; 10] = [0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80,0x1b,0x36];

    let mut w = [[0u8; 4]; 44];
    for i in 0..4 { w[i].copy_from_slice(&key[i*4..i*4+4]); }
    for i in 4..44 {
        let mut temp = w[i-1];
        if i % 4 == 0 {
            temp = [SBOX[temp[1] as usize] ^ RCON[i/4-1],
                    SBOX[temp[2] as usize],
                    SBOX[temp[3] as usize],
                    SBOX[temp[0] as usize]];
        }
        for j in 0..4 { w[i][j] = w[i-4][j] ^ temp[j]; }
    }
    let mut rk = [[0u8; 16]; 11];
    for i in 0..11 {
        for j in 0..4 { rk[i][j*4..j*4+4].copy_from_slice(&w[i*4+j]); }
    }
    rk
}

fn aes128_encrypt_block(block: &[u8; 16], rk: &[[u8; 16]; 11]) -> [u8; 16] {
    const SBOX: [u8; 256] = [
        0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
        0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
        0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
        0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
        0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
        0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
        0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
        0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
        0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
        0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
        0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
        0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
        0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
        0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
        0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
        0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
    ];
    #[allow(dead_code)]
    fn xtime(b: u8) -> u8 { if b & 0x80 != 0 { (b << 1) ^ 0x1b } else { b << 1 } }
    fn gmul(mut a: u8, mut b: u8) -> u8 {
        let mut p = 0u8;
        for _ in 0..8 { if b & 1 != 0 { p ^= a; } let hi = a & 0x80; a <<= 1; if hi != 0 { a ^= 0x1b; } b >>= 1; }
        p
    }

    let mut state = *block;
    // AddRoundKey 0
    for i in 0..16 { state[i] ^= rk[0][i]; }

    for round in 1..=10 {
        // SubBytes
        for b in state.iter_mut() { *b = SBOX[*b as usize]; }
        // ShiftRows
        let s = state;
        state[1]  = s[5];  state[5]  = s[9];  state[9]  = s[13]; state[13] = s[1];
        state[2]  = s[10]; state[6]  = s[14]; state[10] = s[2];  state[14] = s[6];
        state[3]  = s[15]; state[7]  = s[3];  state[11] = s[7];  state[15] = s[11];
        // MixColumns (skip last round)
        if round < 10 {
            for col in 0..4 {
                let i = col * 4;
                let (a, b, c, d) = (state[i], state[i+1], state[i+2], state[i+3]);
                state[i]   = gmul(a,2) ^ gmul(b,3) ^ c ^ d;
                state[i+1] = a ^ gmul(b,2) ^ gmul(c,3) ^ d;
                state[i+2] = a ^ b ^ gmul(c,2) ^ gmul(d,3);
                state[i+3] = gmul(a,3) ^ b ^ c ^ gmul(d,2);
            }
        }
        // AddRoundKey
        for i in 0..16 { state[i] ^= rk[round][i]; }
    }
    state
}

// ── ChaCha20 (RFC 7539) ───────────────────────────────────────────────────────

fn chacha20_xor(data: &[u8], key_hex: &str, nonce_hex: &str) -> Result<Vec<u8>, String> {
    let key   = hex_decode(key_hex)?;
    let nonce = hex_decode(nonce_hex)?;
    if key.len() != 32 { return Err("ChaCha20 key must be 32 bytes (64 hex chars)".into()); }
    if nonce.len() != 12 { return Err("ChaCha20 nonce must be 12 bytes (24 hex chars)".into()); }

    fn quarter_round(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(16);
        s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(12);
        s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(8);
        s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(7);
    }

    let mut out = Vec::with_capacity(data.len());
    let key_words: Vec<u32> = key.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    let nonce_words: Vec<u32> = nonce.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();

    let mut block_counter: u32 = 1;
    let mut offset = 0;
    while offset < data.len() {
        let mut state: [u32; 16] = [
            0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
            key_words[0], key_words[1], key_words[2], key_words[3],
            key_words[4], key_words[5], key_words[6], key_words[7],
            block_counter, nonce_words[0], nonce_words[1], nonce_words[2],
        ];
        let initial = state;
        for _ in 0..10 {
            quarter_round(&mut state, 0,4,8,12); quarter_round(&mut state, 1,5,9,13);
            quarter_round(&mut state, 2,6,10,14); quarter_round(&mut state, 3,7,11,15);
            quarter_round(&mut state, 0,5,10,15); quarter_round(&mut state, 1,6,11,12);
            quarter_round(&mut state, 2,7,8,13); quarter_round(&mut state, 3,4,9,14);
        }
        for i in 0..16 { state[i] = state[i].wrapping_add(initial[i]); }
        let keystream: Vec<u8> = state.iter().flat_map(|w| w.to_le_bytes()).collect();
        for i in 0..64.min(data.len() - offset) {
            out.push(data[offset + i] ^ keystream[i]);
        }
        offset += 64;
        block_counter = block_counter.wrapping_add(1);
    }
    Ok(out)
}

// ── PBKDF2-HMAC-SHA256 ────────────────────────────────────────────────────────

fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32, out_len: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(out_len);
    let mut block_num: u32 = 1;
    while result.len() < out_len {
        let mut u = {
            let mut s = salt.to_vec();
            s.extend_from_slice(&block_num.to_be_bytes());
            hmac_sha256_raw(password, &s)
        };
        let mut xor = u;
        for _ in 1..iterations {
            u = hmac_sha256_raw(password, &u);
            for (a, b) in xor.iter_mut().zip(u.iter()) { *a ^= b; }
        }
        result.extend_from_slice(&xor);
        block_num += 1;
    }
    result.truncate(out_len);
    result
}

fn hmac_sha256_raw(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k0 = [0u8; BLOCK];
    if key.len() > BLOCK {
        let h = sha256_digest(key);
        k0[..32].copy_from_slice(&h);
    } else {
        k0[..key.len()].copy_from_slice(key);
    }
    let ipad: Vec<u8> = k0.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = k0.iter().map(|b| b ^ 0x5c).collect();
    let mut inner = ipad;
    inner.extend_from_slice(data);
    let inner_hash = sha256_digest(&inner);
    let mut outer = opad;
    outer.extend_from_slice(&inner_hash);
    sha256_digest(&outer)
}

// ── HMAC-SHA-512 ──────────────────────────────────────────────────────────────

fn hmac_sha512(key: &[u8], data: &[u8]) -> String {
    const BLOCK: usize = 128;
    let mut k0 = [0u8; BLOCK];
    if key.len() > BLOCK {
        let h = sha512_digest(key);
        k0[..64].copy_from_slice(&h);
    } else {
        k0[..key.len()].copy_from_slice(key);
    }
    let ipad: Vec<u8> = k0.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = k0.iter().map(|b| b ^ 0x5c).collect();
    let mut inner = ipad;
    inner.extend_from_slice(data);
    let inner_hash = sha512_digest(&inner);
    let mut outer = opad;
    outer.extend_from_slice(&inner_hash);
    hex_encode(&sha512_digest(&outer))
}

// ── Constant-time compare ─────────────────────────────────────────────────────

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) { diff |= x ^ y; }
    diff == 0
}

// ── SHA-256 (reused from skill-hash, inlined for WASM self-containment) ────────

fn sha256_digest(input: &[u8]) -> [u8; 32] {
    #[rustfmt::skip]
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut state: [u32; 8] = [0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0x00); }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 { w[i] = u32::from_be_bytes([block[i*4],block[i*4+1],block[i*4+2],block[i*4+3]]); }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3);
            let s1 = w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h] = state;
        for i in 0..64 {
            let s1 = e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25);
            let ch = (e&f)^((!e)&g);
            let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22);
            let mj = (a&b)^(a&c)^(b&c);
            let t2 = s0.wrapping_add(mj);
            h=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2);
        }
        state[0]=state[0].wrapping_add(a); state[1]=state[1].wrapping_add(b);
        state[2]=state[2].wrapping_add(c); state[3]=state[3].wrapping_add(d);
        state[4]=state[4].wrapping_add(e); state[5]=state[5].wrapping_add(f);
        state[6]=state[6].wrapping_add(g); state[7]=state[7].wrapping_add(h);
    }
    let mut out = [0u8; 32];
    for (i, v) in state.iter().enumerate() { out[i*4..(i+1)*4].copy_from_slice(&v.to_be_bytes()); }
    out
}

// ── SHA-512 (inlined) ─────────────────────────────────────────────────────────

fn sha512_digest(input: &[u8]) -> [u8; 64] {
    const K: [u64; 80] = [
        0x428a2f98d728ae22,0x7137449123ef65cd,0xb5c0fbcfec4d3b2f,0xe9b5dba58189dbbc,
        0x3956c25bf348b538,0x59f111f1b605d019,0x923f82a4af194f9b,0xab1c5ed5da6d8118,
        0xd807aa98a3030242,0x12835b0145706fbe,0x243185be4ee4b28c,0x550c7dc3d5ffb4e2,
        0x72be5d74f27b896f,0x80deb1fe3b1696b1,0x9bdc06a725c71235,0xc19bf174cf692694,
        0xe49b69c19ef14ad2,0xefbe4786384f25e3,0x0fc19dc68b8cd5b5,0x240ca1cc77ac9c65,
        0x2de92c6f592b0275,0x4a7484aa6ea6e483,0x5cb0a9dcbd41fbd4,0x76f988da831153b5,
        0x983e5152ee66dfab,0xa831c66d2db43210,0xb00327c898fb213f,0xbf597fc7beef0ee4,
        0xc6e00bf33da88fc2,0xd5a79147930aa725,0x06ca6351e003826f,0x142929670a0e6e70,
        0x27b70a8546d22ffc,0x2e1b21385c26c926,0x4d2c6dfc5ac42aed,0x53380d139d95b3df,
        0x650a73548baf63de,0x766a0abb3c77b2a8,0x81c2c92e47edaee6,0x92722c851482353b,
        0xa2bfe8a14cf10364,0xa81a664bbc423001,0xc24b8b70d0f89791,0xc76c51a30654be30,
        0xd192e819d6ef5218,0xd69906245565a910,0xf40e35855771202a,0x106aa07032bbd1b8,
        0x19a4c116b8d2d0c8,0x1e376c085141ab53,0x2748774cdf8eeb99,0x34b0bcb5e19b48a8,
        0x391c0cb3c5c95a63,0x4ed8aa4ae3418acb,0x5b9cca4f7763e373,0x682e6ff3d6b2b8a3,
        0x748f82ee5defb2fc,0x78a5636f43172f60,0x84c87814a1f0ab72,0x8cc702081a6439ec,
        0x90befffa23631e28,0xa4506cebde82bde9,0xbef9a3f7b2c67915,0xc67178f2e372532b,
        0xca273eceea26619c,0xd186b8c721c0c207,0xeada7dd6cde0eb1e,0xf57d4f7fee6ed178,
        0x06f067aa72176fba,0x0a637dc5a2c898a6,0x113f9804bef90dae,0x1b710b35131c471b,
        0x28db77f523047d84,0x32caab7b40c72493,0x3c9ebe0a15c9bebc,0x431d67c49c100d4c,
        0x4cc5d4becb3e42b6,0x597f299cfc657e2a,0x5fcb6fab3ad6faec,0x6c44198c4a475817,
    ];
    let bit_len = (input.len() as u128).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 128 != 112 { msg.push(0x00); }
    msg.extend_from_slice(&((bit_len >> 64) as u64).to_be_bytes());
    msg.extend_from_slice(&(bit_len as u64).to_be_bytes());
    let mut h: [u64; 8] = [
        0x6a09e667f3bcc908,0xbb67ae8584caa73b,0x3c6ef372fe94f82b,0xa54ff53a5f1d36f1,
        0x510e527fade682d1,0x9b05688c2b3e6c1f,0x1f83d9abfb41bd6b,0x5be0cd19137e2179,
    ];
    for chunk in msg.chunks_exact(128) {
        let mut w = [0u64; 80];
        for i in 0..16 { let b=&chunk[i*8..(i+1)*8]; w[i]=u64::from_be_bytes([b[0],b[1],b[2],b[3],b[4],b[5],b[6],b[7]]); }
        for i in 16..80 {
            let s0=w[i-15].rotate_right(1)^w[i-15].rotate_right(8)^(w[i-15]>>7);
            let s1=w[i-2].rotate_right(19)^w[i-2].rotate_right(61)^(w[i-2]>>6);
            w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh)=(h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        for i in 0..80 {
            let s1=e.rotate_right(14)^e.rotate_right(18)^e.rotate_right(41);
            let ch=(e&f)^(!e&g);
            let t1=hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0=a.rotate_right(28)^a.rotate_right(34)^a.rotate_right(39);
            let mj=(a&b)^(a&c)^(b&c);
            let t2=s0.wrapping_add(mj);
            hh=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(t2);
        }
        h[0]=h[0].wrapping_add(a);h[1]=h[1].wrapping_add(b);h[2]=h[2].wrapping_add(c);h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e);h[5]=h[5].wrapping_add(f);h[6]=h[6].wrapping_add(g);h[7]=h[7].wrapping_add(hh);
    }
    let mut out=[0u8;64];
    for (i,v) in h.iter().enumerate() { out[i*8..(i+1)*8].copy_from_slice(&v.to_be_bytes()); }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_roundtrip() {
        let key = "000102030405060708090a0b0c0d0e0f";
        let enc = aes_ctr_encrypt(b"Hello, World!", key).unwrap();
        let dec = aes_ctr_decrypt(&enc, key).unwrap();
        assert_eq!(dec, "Hello, World!");
    }

    #[test]
    fn aes_bad_key_length() {
        assert!(aes_ctr_encrypt(b"test", "0001020304").is_err());
    }

    #[test]
    fn chacha20_roundtrip() {
        let key   = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let nonce = "000000000000000000000000";
        let plain = b"Attack at dawn!";
        let enc = base64_encode(plain);
        let enc_b64 = chacha20_xor(plain, key, nonce).map(|v| base64_encode(&v)).unwrap();
        let dec = chacha20_xor(&base64_decode(&enc_b64).unwrap(), key, nonce).map(|v| base64_encode(&v)).unwrap();
        assert_eq!(dec, enc);
    }

    #[test]
    fn pbkdf2_deterministic() {
        let a = pbkdf2_sha256(b"password", b"salt", 1, 32);
        let b = pbkdf2_sha256(b"password", b"salt", 1, 32);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn pbkdf2_different_passwords_differ() {
        let a = pbkdf2_sha256(b"password1", b"salt", 1, 32);
        let b = pbkdf2_sha256(b"password2", b"salt", 1, 32);
        assert_ne!(a, b);
    }

    #[test]
    fn pbkdf2_more_iterations_differs() {
        let a = pbkdf2_sha256(b"password", b"salt", 1, 32);
        let b = pbkdf2_sha256(b"password", b"salt", 2, 32);
        assert_ne!(a, b);
    }

    #[test]
    fn hmac_sha512_non_empty() {
        let result = hmac_sha512(b"key", b"data");
        assert_eq!(result.len(), 128);
    }

    #[test]
    fn constant_time_eq_same() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"hi", b"hii"));
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.crypto");
        assert_eq!(v["skills"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn base64_roundtrip() {
        let data = b"The quick brown fox";
        assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
    }

    #[test]
    fn hex_roundtrip() {
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        assert_eq!(hex_decode(&hex_encode(&bytes)).unwrap(), bytes);
    }

    #[test]
    fn aes_empty_plaintext() {
        let key = "000102030405060708090a0b0c0d0e0f";
        let enc = aes_ctr_encrypt(b"", key).unwrap();
        let dec = aes_ctr_decrypt(&enc, key).unwrap();
        assert_eq!(dec, "");
    }

    #[test]
    fn aes_different_keys_produce_different_output() {
        let key1 = "000102030405060708090a0b0c0d0e0f";
        let key2 = "0f0e0d0c0b0a09080706050403020100";
        let enc1 = aes_ctr_encrypt(b"secret", key1).unwrap();
        let enc2 = aes_ctr_encrypt(b"secret", key2).unwrap();
        assert_ne!(enc1, enc2);
    }

    #[test]
    fn aes_longer_message() {
        let key = "aabbccddeeff00112233445566778899";
        let plain = b"This is a longer message that spans multiple AES blocks!";
        let enc = aes_ctr_encrypt(plain, key).unwrap();
        let dec = aes_ctr_decrypt(&enc, key).unwrap();
        assert_eq!(dec.as_bytes(), plain);
    }

    #[test]
    fn chacha20_empty() {
        let key   = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let nonce = "000000000000000000000000";
        let enc = chacha20_xor(b"", key, nonce).unwrap();
        assert_eq!(enc, b"");
    }

    #[test]
    fn chacha20_bad_key_length() {
        let key   = "0001020304";
        let nonce = "000000000000000000000000";
        assert!(chacha20_xor(b"test", key, nonce).is_err());
    }

    #[test]
    fn pbkdf2_different_salts_differ() {
        let a = pbkdf2_sha256(b"password", b"salt1", 1, 32);
        let b = pbkdf2_sha256(b"password", b"salt2", 1, 32);
        assert_ne!(a, b);
    }

    #[test]
    fn pbkdf2_output_length_respected() {
        let out16 = pbkdf2_sha256(b"pw", b"salt", 1, 16);
        let out64 = pbkdf2_sha256(b"pw", b"salt", 1, 64);
        assert_eq!(out16.len(), 16);
        assert_eq!(out64.len(), 64);
    }

    #[test]
    fn hmac_sha512_different_keys_differ() {
        let a = hmac_sha512(b"key1", b"data");
        let b = hmac_sha512(b"key2", b"data");
        assert_ne!(a, b);
    }

    #[test]
    fn hmac_sha512_different_data_differ() {
        let a = hmac_sha512(b"key", b"data1");
        let b = hmac_sha512(b"key", b"data2");
        assert_ne!(a, b);
    }

    #[test]
    fn constant_time_eq_empty() {
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn hex_encode_all_zeros() {
        assert_eq!(hex_encode(&[0u8; 4]), "00000000");
    }

    #[test]
    fn hex_decode_invalid_chars() {
        assert!(hex_decode("zzzz").is_err());
    }
}
