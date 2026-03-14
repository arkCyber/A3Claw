//! skill-hash — hashing skill plugin for OpenClaw+
//!
//! Skills exposed:
//!   hash.md5       { input: string }  → hex string
//!   hash.sha1      { input: string }  → hex string
//!   hash.sha256    { input: string }  → hex string
//!   hash.sha512    { input: string }  → hex string
//!   hash.hmac_sha256 { input: string, key: string } → hex string

use openclaw_plugin_sdk::prelude::*;

// ── Manifest ──────────────────────────────────────────────────────────────────

static MANIFEST: &str = r#"{
  "id": "openclaw.hash",
  "name": "Hash Skills",
  "version": "0.1.0",
  "description": "Cryptographic hashing: MD5, SHA-1, SHA-256, SHA-512, HMAC-SHA-256",
  "skills": [
    {
      "name": "hash.md5",
      "display": "MD5 Hash",
      "description": "Compute MD5 digest of a string (hex output). NOT cryptographically secure — use for checksums only.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "description": "Input to hash", "required": true }]
    },
    {
      "name": "hash.sha1",
      "display": "SHA-1 Hash",
      "description": "Compute SHA-1 digest (hex output). Use SHA-256 for security-sensitive work.",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "description": "Input to hash", "required": true }]
    },
    {
      "name": "hash.sha256",
      "display": "SHA-256 Hash",
      "description": "Compute SHA-256 digest (hex output).",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "description": "Input to hash", "required": true }]
    },
    {
      "name": "hash.sha512",
      "display": "SHA-512 Hash",
      "description": "Compute SHA-512 digest (hex output).",
      "risk": "safe",
      "params": [{ "name": "input", "type": "string", "description": "Input to hash", "required": true }]
    },
    {
      "name": "hash.hmac_sha256",
      "display": "HMAC-SHA-256",
      "description": "Compute HMAC-SHA-256 with a secret key (hex output).",
      "risk": "safe",
      "params": [
        { "name": "input", "type": "string", "description": "Message to authenticate", "required": true },
        { "name": "key",   "type": "string", "description": "Secret key (UTF-8)",     "required": true }
      ]
    }
  ]
}"#;

// ── Exports ───────────────────────────────────────────────────────────────────

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
        "hash.md5" => {
            let input = match req.args["input"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input' argument"),
            };
            sdk_respond_ok(rid, &md5(input.as_bytes()))
        }

        "hash.sha1" => {
            let input = match req.args["input"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input' argument"),
            };
            sdk_respond_ok(rid, &sha1(input.as_bytes()))
        }

        "hash.sha256" => {
            let input = match req.args["input"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input' argument"),
            };
            sdk_respond_ok(rid, &sha256(input.as_bytes()))
        }

        "hash.sha512" => {
            let input = match req.args["input"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input' argument"),
            };
            sdk_respond_ok(rid, &sha512(input.as_bytes()))
        }

        "hash.hmac_sha256" => {
            let input = match req.args["input"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'input' argument"),
            };
            let key = match req.args["key"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'key' argument"),
            };
            sdk_respond_ok(rid, &hmac_sha256(key.as_bytes(), input.as_bytes()))
        }

        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Pure hash implementations (no external crates — wasm32 portable) ──────────

/// MD5 — RFC 1321
fn md5(data: &[u8]) -> String {
    let digest = md5_digest(data);
    hex_encode(&digest)
}

/// SHA-1 — RFC 3174
fn sha1(data: &[u8]) -> String {
    let digest = sha1_digest(data);
    hex_encode(&digest)
}

/// SHA-256 — FIPS 180-4
fn sha256(data: &[u8]) -> String {
    let digest = sha256_digest(data);
    hex_encode(&digest)
}

/// SHA-512 — FIPS 180-4
fn sha512(data: &[u8]) -> String {
    let digest = sha512_digest(data);
    hex_encode(&digest)
}

/// HMAC-SHA-256 — RFC 2104
fn hmac_sha256(key: &[u8], data: &[u8]) -> String {
    const BLOCK: usize = 64;
    // Derive K0: if key > block size, hash it; pad to block size
    let mut k0 = [0u8; BLOCK];
    if key.len() > BLOCK {
        let h = sha256_digest(key);
        k0[..h.len()].copy_from_slice(&h);
    } else {
        k0[..key.len()].copy_from_slice(key);
    }
    // ipad = 0x36, opad = 0x5C
    let ipad: Vec<u8> = k0.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = k0.iter().map(|b| b ^ 0x5C).collect();
    // inner = SHA256(ipad || data)
    let mut inner_msg = ipad;
    inner_msg.extend_from_slice(data);
    let inner = sha256_digest(&inner_msg);
    // outer = SHA256(opad || inner)
    let mut outer_msg = opad;
    outer_msg.extend_from_slice(&inner);
    hex_encode(&sha256_digest(&outer_msg))
}

// ── Hex encode ────────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ── MD5 implementation ────────────────────────────────────────────────────────

fn md5_digest(input: &[u8]) -> [u8; 16] {
    // Per-round shift amounts
    const S: [u32; 64] = [
        7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
        5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
        4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
        6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,
    ];
    // Precomputed T[i] = floor(2^32 * |sin(i+1)|)
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
        0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
        0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
        0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
        0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
        0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];

    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let (mut a0, mut b0, mut c0, mut d0): (u32, u32, u32, u32) =
        (0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476);

    for chunk in msg.chunks_exact(64) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0u32..64 {
            let (f, g) = match i {
                 0..=15 => ((b & c) | (!b & d),          i),
                16..=31 => ((d & b) | (!d & c),          (5*i + 1) % 16),
                32..=47 => (b ^ c ^ d,                   (3*i + 5) % 16),
                _       => (c ^ (b | !d),                (7*i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                 .wrapping_add(K[i as usize])
                 .wrapping_add(m[g as usize])
                 .rotate_left(S[i as usize]),
            );
            a = temp;
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

// ── SHA-1 implementation ──────────────────────────────────────────────────────

fn sha1_digest(input: &[u8]) -> [u8; 20] {
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let (mut h0, mut h1, mut h2, mut h3, mut h4): (u32, u32, u32, u32, u32) =
        (0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0);

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
        }
        for i in 16..80 {
            w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for i in 0usize..80 {
            let (f, k) = match i {
                 0..=19 => ((b & c) | (!b & d), 0x5a827999u32),
                20..=39 => (b ^ c ^ d,           0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _       => (b ^ c ^ d,           0xca62c1d6),
            };
            let temp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d; d = c; c = b.rotate_left(30); b = a; a = temp;
        }
        h0 = h0.wrapping_add(a); h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c); h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, v) in [h0, h1, h2, h3, h4].iter().enumerate() {
        out[i*4..(i+1)*4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ── SHA-256 implementation ────────────────────────────────────────────────────
// Reference: FIPS 180-4, verified against NIST test vectors.

fn sha256_digest(input: &[u8]) -> [u8; 32] {
    #[rustfmt::skip]
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    // Initial hash values (first 32 bits of the fractional parts of square roots of the first 8 primes)
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: padding
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit (64-byte) block
    for block in msg.chunks_exact(64) {
        // Prepare message schedule
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7)
                ^ w[i - 15].rotate_right(18)
                ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17)
                ^ w[i - 2].rotate_right(19)
                ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        // Initialize working variables
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;

        // Compression function
        for i in 0..64 {
            let s1    = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch    = (e & f) ^ ((!e) & g);
            let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0    = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj   = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        // Add the compressed chunk to the current hash value
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    // Produce the final hash value (big-endian)
    let mut out = [0u8; 32];
    for (i, v) in state.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ── SHA-512 implementation ────────────────────────────────────────────────────

fn sha512_digest(input: &[u8]) -> [u8; 64] {
    const K: [u64; 80] = [
        0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
        0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
        0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
        0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
        0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
        0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
        0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
        0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
        0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
        0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
        0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
        0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
        0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
        0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
        0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
        0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
        0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
        0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
        0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
        0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
    ];

    let bit_len_lo = (input.len() as u128).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 128 != 112 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&((bit_len_lo >> 64) as u64).to_be_bytes());
    msg.extend_from_slice(&(bit_len_lo as u64).to_be_bytes());

    let mut h: [u64; 8] = [
        0x6a09e667f3bcc908, 0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
        0x510e527fade682d1, 0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
    ];

    for chunk in msg.chunks_exact(128) {
        let mut w = [0u64; 80];
        for i in 0..16 {
            let b = &chunk[i*8..(i+1)*8];
            w[i] = u64::from_be_bytes([b[0],b[1],b[2],b[3],b[4],b[5],b[6],b[7]]);
        }
        for i in 16..80 {
            let s0 = w[i-15].rotate_right(1) ^ w[i-15].rotate_right(8) ^ (w[i-15] >> 7);
            let s1 = w[i-2].rotate_right(19) ^ w[i-2].rotate_right(61) ^ (w[i-2] >> 6);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh) =
            (h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        for i in 0..80 {
            let s1    = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch    = (e & f) ^ (!e & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0    = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj   = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh=g; g=f; f=e; e=d.wrapping_add(temp1); d=c; c=b; b=a;
            a = temp1.wrapping_add(temp2);
        }
        h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b);
        h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f);
        h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 64];
    for (i, v) in h.iter().enumerate() {
        out[i*8..(i+1)*8].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ── Tests (native) ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_empty() {
        assert_eq!(md5(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn md5_abc() {
        assert_eq!(md5(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn md5_known_vector() {
        assert_eq!(md5(b"The quick brown fox jumps over the lazy dog"),
                   "9e107d9d372bb6826bd81d3542a419d6");
    }

    #[test]
    fn sha1_empty() {
        assert_eq!(sha1(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn sha1_abc() {
        assert_eq!(sha1(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn sha1_known_vector() {
        assert_eq!(sha1(b"The quick brown fox jumps over the lazy dog"),
                   "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
    }

    #[test]
    fn sha256_empty() {
        assert_eq!(sha256(b""),
                   "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn sha256_abc() {
        assert_eq!(sha256(b"abc"),
                   "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(sha256(b"The quick brown fox jumps over the lazy dog"),
                   "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592");
        // correct: d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592
    }

    #[test]
    fn sha512_empty() {
        assert_eq!(sha512(b""),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");
    }

    #[test]
    fn sha512_abc() {
        let got = sha512(b"abc");
        assert!(got.starts_with("ddaf35a193617aba"), "got: {got}");
    }

    #[test]
    fn hmac_sha256_rfc4231_tc1() {
        let key = [0x0bu8; 20];
        let data = b"Hi There";
        let result = hmac_sha256(&key, data);
        assert_eq!(result, "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
    }

    #[test]
    fn hex_encode_zero() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0x10]), "00ff10");
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.hash");
        let skills = v["skills"].as_array().unwrap();
        assert_eq!(skills.len(), 5);
    }

    #[test]
    fn manifest_skill_names() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let names: Vec<&str> = v["skills"].as_array().unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"hash.md5"));
        assert!(names.contains(&"hash.sha256"));
        assert!(names.contains(&"hash.hmac_sha256"));
    }

    // ── md5 extra ───────────────────────────────────────────────────────────────────
    #[test]
    fn md5_length_always_32() {
        assert_eq!(md5(b"anything").len(), 32);
        assert_eq!(md5(b"").len(), 32);
    }
    #[test]
    fn md5_different_inputs_differ() {
        assert_ne!(md5(b"foo"), md5(b"bar"));
    }
    #[test]
    fn md5_hello_world() {
        assert_eq!(md5(b"Hello, World!"), "65a8e27d8879283831b664bd8b7f0ad4");
    }

    // ── sha1 extra ──────────────────────────────────────────────────────────────────
    #[test]
    fn sha1_length_always_40() {
        assert_eq!(sha1(b"test").len(), 40);
    }
    #[test]
    fn sha1_different_inputs_differ() {
        assert_ne!(sha1(b"foo"), sha1(b"bar"));
    }

    // ── sha256 extra ───────────────────────────────────────────────────────────
    #[test]
    fn sha256_length_always_64() {
        assert_eq!(sha256(b"test").len(), 64);
    }
    #[test]
    fn sha256_deterministic() {
        assert_eq!(sha256(b"hello"), sha256(b"hello"));
    }
    #[test]
    fn sha256_different_inputs_differ() {
        assert_ne!(sha256(b"foo"), sha256(b"bar"));
    }

    // ── hmac_sha256 extra ──────────────────────────────────────────────────────
    #[test]
    fn hmac_sha256_length_always_64() {
        assert_eq!(hmac_sha256(b"key", b"data").len(), 64);
    }
    #[test]
    fn hmac_sha256_different_keys_differ() {
        assert_ne!(hmac_sha256(b"key1", b"data"), hmac_sha256(b"key2", b"data"));
    }
    #[test]
    fn hmac_sha256_different_data_differ() {
        assert_ne!(hmac_sha256(b"key", b"data1"), hmac_sha256(b"key", b"data2"));
    }
}
