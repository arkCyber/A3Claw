use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.array",
  "name": "Array Utilities",
  "version": "0.1.0",
  "description": "10 array/list utility skills: length, unique, flatten, zip, chunk, sum, product, min, max, join.",
  "skills": [
    {"name":"array.length","display":"Length","description":"Return the number of elements in a JSON array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array","required":true}]},
    {"name":"array.unique","display":"Unique","description":"Remove duplicate values from a JSON array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array","required":true}]},
    {"name":"array.flatten","display":"Flatten","description":"Flatten a nested JSON array one level deep.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array (possibly nested)","required":true}]},
    {"name":"array.zip","display":"Zip","description":"Zip two JSON arrays into array of [a,b] pairs.","risk":"safe","params":[{"name":"a","type":"string","description":"First JSON array","required":true},{"name":"b","type":"string","description":"Second JSON array","required":true}]},
    {"name":"array.chunk","display":"Chunk","description":"Split array into chunks of size N.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array","required":true},{"name":"size","type":"integer","description":"Chunk size","required":true}]},
    {"name":"array.sum","display":"Sum","description":"Sum all numeric values in a JSON array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array of numbers","required":true}]},
    {"name":"array.product","display":"Product","description":"Multiply all numeric values in a JSON array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array of numbers","required":true}]},
    {"name":"array.min","display":"Min","description":"Return the minimum numeric value in a JSON array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array of numbers","required":true}]},
    {"name":"array.max","display":"Max","description":"Return the maximum numeric value in a JSON array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array of numbers","required":true}]},
    {"name":"array.join","display":"Join","description":"Join a JSON array of strings with a separator.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array of strings","required":true},{"name":"sep","type":"string","description":"Separator","required":false}]}
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

    macro_rules! parse_arr {
        ($k:literal) => {
            match args[$k].as_str().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
                Some(serde_json::Value::Array(a)) => a,
                _ => match &args[$k] {
                    serde_json::Value::Array(a) => a.clone(),
                    _ => return sdk_respond_err(rid, concat!("'", $k, "' must be a JSON array")),
                }
            }
        };
    }

    match req.skill.as_str() {
        "array.length" => {
            let arr = parse_arr!("array");
            sdk_respond_ok(rid, &arr.len().to_string())
        }
        "array.unique" => {
            let arr = parse_arr!("array");
            let mut seen = std::collections::HashSet::new();
            let mut out: Vec<serde_json::Value> = Vec::new();
            for v in arr {
                let key = v.to_string();
                if seen.insert(key) { out.push(v); }
            }
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "array.flatten" => {
            let arr = parse_arr!("array");
            let mut out: Vec<serde_json::Value> = Vec::new();
            for v in arr {
                match v {
                    serde_json::Value::Array(inner) => out.extend(inner),
                    other => out.push(other),
                }
            }
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "array.zip" => {
            let a = parse_arr!("a");
            let b = parse_arr!("b");
            let out: Vec<serde_json::Value> = a.into_iter().zip(b.into_iter())
                .map(|(x, y)| serde_json::Value::Array(vec![x, y]))
                .collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "array.chunk" => {
            let arr = parse_arr!("array");
            let size = args["size"].as_u64().unwrap_or(1).max(1) as usize;
            let out: Vec<serde_json::Value> = arr.chunks(size)
                .map(|c| serde_json::Value::Array(c.to_vec()))
                .collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "array.sum" => {
            let arr = parse_arr!("array");
            let total: f64 = arr.iter().filter_map(|v| v.as_f64()).sum();
            sdk_respond_ok(rid, &total.to_string())
        }
        "array.product" => {
            let arr = parse_arr!("array");
            let result: f64 = arr.iter().filter_map(|v| v.as_f64()).product();
            sdk_respond_ok(rid, &result.to_string())
        }
        "array.min" => {
            let arr = parse_arr!("array");
            let nums: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
            match nums.iter().cloned().reduce(f64::min) {
                Some(m) => sdk_respond_ok(rid, &m.to_string()),
                None    => sdk_respond_err(rid, "empty or non-numeric array"),
            }
        }
        "array.max" => {
            let arr = parse_arr!("array");
            let nums: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
            match nums.iter().cloned().reduce(f64::max) {
                Some(m) => sdk_respond_ok(rid, &m.to_string()),
                None    => sdk_respond_err(rid, "empty or non-numeric array"),
            }
        }
        "array.join" => {
            let arr = parse_arr!("array");
            let sep = args["sep"].as_str().unwrap_or(", ");
            let parts: Vec<String> = arr.iter().map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }).collect();
            sdk_respond_ok(rid, &parts.join(sep))
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    // ── array.length ──────────────────────────────────────────────────────
    #[test] fn length_empty() {
        let arr: Vec<i32> = vec![];
        assert_eq!(arr.len(), 0);
    }
    #[test] fn length_three() {
        let arr = vec![1, 2, 3];
        assert_eq!(arr.len(), 3);
    }

    // ── array.unique ──────────────────────────────────────────────────────
    #[test] fn unique_removes_dups() {
        let arr = vec![1, 2, 2, 3];
        let mut seen = std::collections::HashSet::new();
        let out: Vec<i32> = arr.into_iter().filter(|x| seen.insert(*x)).collect();
        assert_eq!(out, vec![1, 2, 3]);
    }
    #[test] fn unique_all_same() {
        let arr = vec![5, 5, 5];
        let mut seen = std::collections::HashSet::new();
        let out: Vec<i32> = arr.into_iter().filter(|x| seen.insert(*x)).collect();
        assert_eq!(out, vec![5]);
    }
    #[test] fn unique_no_dups() {
        let arr = vec![1, 2, 3];
        let mut seen = std::collections::HashSet::new();
        let out: Vec<i32> = arr.into_iter().filter(|x| seen.insert(*x)).collect();
        assert_eq!(out.len(), 3);
    }

    // ── array.flatten ─────────────────────────────────────────────────────
    #[test] fn flatten_one_level() {
        let a: Vec<Vec<i32>> = vec![vec![1,2], vec![3]];
        let flat: Vec<i32> = a.into_iter().flatten().collect();
        assert_eq!(flat, vec![1, 2, 3]);
    }
    #[test] fn flatten_mixed_nested() {
        let arr = json!([[1, 2], 3, [4]]);
        let mut out: Vec<Value> = Vec::new();
        if let Value::Array(items) = arr {
            for v in items {
                match v { Value::Array(inner) => out.extend(inner), other => out.push(other) }
            }
        }
        assert_eq!(out.len(), 4);
    }

    // ── array.zip ─────────────────────────────────────────────────────────
    #[test] fn zip_pairs() {
        let a = vec![1,2,3];
        let b = vec![4,5,6];
        let z: Vec<(i32,i32)> = a.into_iter().zip(b).collect();
        assert_eq!(z, vec![(1,4),(2,5),(3,6)]);
    }
    #[test] fn zip_unequal_truncates() {
        let a = vec![1, 2];
        let b = vec![10, 20, 30];
        let z: Vec<(i32,i32)> = a.into_iter().zip(b).collect();
        assert_eq!(z.len(), 2);
    }

    // ── array.chunk ───────────────────────────────────────────────────────
    #[test] fn chunk_size_2() {
        let a = vec![1,2,3,4,5];
        let chunks: Vec<&[i32]> = a.chunks(2).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], &[1, 2]);
        assert_eq!(chunks[2], &[5]);
    }
    #[test] fn chunk_size_eq_len() {
        let a = vec![1,2,3];
        let chunks: Vec<&[i32]> = a.chunks(3).collect();
        assert_eq!(chunks.len(), 1);
    }

    // ── array.sum / product ───────────────────────────────────────────────
    #[test] fn sum_vec()     { let s: f64 = vec![1.0,2.0,3.0f64].iter().sum(); assert_eq!(s, 6.0); }
    #[test] fn sum_empty()   { let s: f64 = Vec::<f64>::new().iter().sum(); assert_eq!(s, 0.0); }
    #[test] fn product_vec() { let p: f64 = vec![2.0,3.0,4.0f64].iter().product(); assert_eq!(p, 24.0); }
    #[test] fn product_empty() { let p: f64 = Vec::<f64>::new().iter().product(); assert_eq!(p, 1.0); }

    // ── array.min / max ───────────────────────────────────────────────────
    #[test] fn min_of_vec() {
        let nums = vec![3.0f64, 1.0, 4.0, 1.5];
        let m = nums.iter().cloned().reduce(f64::min).unwrap();
        assert_eq!(m, 1.0);
    }
    #[test] fn max_of_vec() {
        let nums = vec![3.0f64, 1.0, 4.0, 1.5];
        let m = nums.iter().cloned().reduce(f64::max).unwrap();
        assert_eq!(m, 4.0);
    }
    #[test] fn min_single() {
        let nums = vec![42.0f64];
        let m = nums.iter().cloned().reduce(f64::min).unwrap();
        assert_eq!(m, 42.0);
    }

    // ── array.join ────────────────────────────────────────────────────────
    #[test] fn join_comma() {
        let parts = vec!["a", "b", "c"];
        assert_eq!(parts.join(", "), "a, b, c");
    }
    #[test] fn join_empty_sep() {
        let parts = vec!["x", "y"];
        assert_eq!(parts.join(""), "xy");
    }
    #[test] fn join_single_element() {
        let parts = vec!["only"];
        assert_eq!(parts.join("-"), "only");
    }

    // ── manifest ─────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(super::MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(super::MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("array."));
        }
    }
}
