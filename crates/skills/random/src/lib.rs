//! skill-random — deterministic pseudo-random number generation (pure-Rust, no I/O).
//!
//! Skills:
//!   random.int      { min: i64, max: i64, seed?: u64 }        → integer
//!   random.float    { min: f64, max: f64, seed?: u64 }        → float
//!   random.choice   { values: [any], seed?: u64 }             → element
//!   random.shuffle  { values: [any], seed?: u64 }             → shuffled array
//!   random.sample   { values: [any], n: usize, seed?: u64 }   → n elements
//!   random.uuid_v4  {}                                        → UUID v4 string (seeded)

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.random",
  "name": "Random Skills",
  "version": "0.1.0",
  "description": "Seeded pseudo-random: integers, floats, choice, shuffle, sample, UUID",
  "skills": [
    {
      "name": "random.int",
      "display": "Random Integer",
      "description": "Return a pseudo-random integer in [min, max].",
      "risk": "safe",
      "params": [
        { "name": "min",  "type": "integer", "required": true  },
        { "name": "max",  "type": "integer", "required": true  },
        { "name": "seed", "type": "integer", "required": false }
      ]
    },
    {
      "name": "random.float",
      "display": "Random Float",
      "description": "Return a pseudo-random float in [min, max).",
      "risk": "safe",
      "params": [
        { "name": "min",  "type": "number",  "required": true  },
        { "name": "max",  "type": "number",  "required": true  },
        { "name": "seed", "type": "integer", "required": false }
      ]
    },
    {
      "name": "random.choice",
      "display": "Random Choice",
      "description": "Pick one random element from an array.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array",   "required": true  },
        { "name": "seed",   "type": "integer", "required": false }
      ]
    },
    {
      "name": "random.shuffle",
      "display": "Shuffle Array",
      "description": "Return a Fisher-Yates shuffled copy of the array.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array",   "required": true  },
        { "name": "seed",   "type": "integer", "required": false }
      ]
    },
    {
      "name": "random.sample",
      "display": "Random Sample",
      "description": "Pick n unique elements from an array without replacement.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array",   "required": true  },
        { "name": "n",      "type": "integer", "required": true  },
        { "name": "seed",   "type": "integer", "required": false }
      ]
    },
    {
      "name": "random.uuid_v4",
      "display": "Random UUID v4",
      "description": "Generate a UUID v4 using a seeded PRNG.",
      "risk": "safe",
      "params": [
        { "name": "seed", "type": "integer", "required": false }
      ]
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
    let seed = req.args["seed"].as_u64().unwrap_or(12345);

    match req.skill.as_str() {
        "random.int" => {
            let min = match req.args["min"].as_i64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'min'") };
            let max = match req.args["max"].as_i64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'max'") };
            if min > max { return sdk_respond_err(rid, "min must be <= max"); }
            sdk_respond_ok(rid, &rand_int(seed, min, max).to_string())
        }
        "random.float" => {
            let min = match req.args["min"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'min'") };
            let max = match req.args["max"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'max'") };
            sdk_respond_ok(rid, &format!("{:.9}", rand_float(seed, min, max)))
        }
        "random.choice" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'values'") };
            if arr.is_empty() { return sdk_respond_err(rid, "values array is empty"); }
            let idx = rand_int(seed, 0, arr.len() as i64 - 1) as usize;
            sdk_respond_ok(rid, &arr[idx].to_string())
        }
        "random.shuffle" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'values'") };
            let shuffled = shuffle_array(arr, seed);
            sdk_respond_ok(rid, &serde_json::to_string(&shuffled).unwrap())
        }
        "random.sample" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'values'") };
            let n = match req.args["n"].as_u64() { Some(v) => v as usize, None => return sdk_respond_err(rid, "missing 'n'") };
            if n > arr.len() { return sdk_respond_err(rid, "n exceeds array length"); }
            let mut pool = arr.to_vec();
            let mut rng = seed;
            let mut result = Vec::new();
            for _ in 0..n {
                rng = lcg_next(rng);
                let idx = (rng as usize) % pool.len();
                result.push(pool.remove(idx));
            }
            sdk_respond_ok(rid, &serde_json::to_string(&result).unwrap())
        }
        "random.uuid_v4" => {
            sdk_respond_ok(rid, &rand_uuid(seed))
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── PRNG: LCG (deterministic) ─────────────────────────────────────────────────

fn lcg_next(state: u64) -> u64 {
    state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

fn rand_int(seed: u64, min: i64, max: i64) -> i64 {
    let s = lcg_next(seed);
    let range = (max - min + 1) as u64;
    min + (s % range) as i64
}

fn rand_float(seed: u64, min: f64, max: f64) -> f64 {
    let s = lcg_next(seed);
    let t = (s as f64) / (u64::MAX as f64);
    min + t * (max - min)
}

fn shuffle_array(arr: &[serde_json::Value], seed: u64) -> Vec<serde_json::Value> {
    let mut v = arr.to_vec();
    let mut rng = seed;
    for i in (1..v.len()).rev() {
        rng = lcg_next(rng);
        let j = (rng as usize) % (i + 1);
        v.swap(i, j);
    }
    v
}

fn rand_uuid(seed: u64) -> String {
    let mut rng = seed;
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        rng = lcg_next(rng);
        let b = rng.to_le_bytes();
        for (i, c) in chunk.iter_mut().enumerate() { *c = b[i]; }
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn int_in_range()     { let v = rand_int(42, 1, 10); assert!((1..=10).contains(&v)); }
    #[test] fn int_deterministic(){ assert_eq!(rand_int(42, 0, 100), rand_int(42, 0, 100)); }
    #[test] fn int_min_eq_max()   { assert_eq!(rand_int(1, 5, 5), 5); }

    #[test] fn float_in_range()   { let v = rand_float(42, 0.0, 1.0); assert!((0.0..=1.0).contains(&v)); }
    #[test] fn float_deterministic(){ assert_eq!(rand_float(99, 0.0, 10.0), rand_float(99, 0.0, 10.0)); }

    #[test]
    fn shuffle_same_elements() {
        let arr: Vec<serde_json::Value> = vec![1.into(), 2.into(), 3.into(), 4.into(), 5.into()];
        let s = shuffle_array(&arr, 42);
        assert_eq!(s.len(), arr.len());
        let mut orig: Vec<_> = arr.iter().map(|v| v.to_string()).collect();
        let mut shuf: Vec<_> = s.iter().map(|v| v.to_string()).collect();
        orig.sort(); shuf.sort();
        assert_eq!(orig, shuf);
    }
    #[test]
    fn shuffle_deterministic() {
        let arr: Vec<serde_json::Value> = vec![1.into(), 2.into(), 3.into()];
        assert_eq!(shuffle_array(&arr, 7), shuffle_array(&arr, 7));
    }
    #[test]
    fn shuffle_empty() {
        let arr: Vec<serde_json::Value> = vec![];
        assert!(shuffle_array(&arr, 1).is_empty());
    }

    #[test]
    fn uuid_format() {
        let u = rand_uuid(1);
        assert_eq!(u.len(), 36);
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
    }
    #[test]
    fn uuid_version_nibble() {
        let u = rand_uuid(123);
        let ver_char = u.chars().nth(14).unwrap();
        assert_eq!(ver_char, '4');
    }
    #[test]
    fn uuid_different_seeds_differ() {
        assert_ne!(rand_uuid(1), rand_uuid(2));
    }
    #[test]
    fn uuid_same_seed_deterministic() {
        assert_eq!(rand_uuid(999), rand_uuid(999));
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.random");
        assert_eq!(v["skills"].as_array().unwrap().len(), 6);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("random."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
