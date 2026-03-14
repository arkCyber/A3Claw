//! skill-sort — sorting, ranking, and searching skills (pure-Rust, no I/O).
//!
//! Skills:
//!   sort.numbers   { values: [f64], order?: "asc"|"desc" }          → sorted array JSON
//!   sort.strings   { values: [str], order?: "asc"|"desc" }          → sorted array JSON
//!   sort.by_key    { values: [{...}], key: str, order?: "asc"|"desc" } → sorted array JSON
//!   sort.rank      { values: [f64] }                                 → [{value, rank}] JSON
//!   sort.unique    { values: [any] }                                 → deduplicated array JSON
//!   sort.binary_search { values: [f64], target: f64 }               → index or -1

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.sort",
  "name": "Sort Skills",
  "version": "0.1.0",
  "description": "Sorting, ranking, deduplication, and binary search",
  "skills": [
    {
      "name": "sort.numbers",
      "display": "Sort Numbers",
      "description": "Sort a numeric array ascending or descending.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array",  "required": true  },
        { "name": "order",  "type": "string", "required": false }
      ]
    },
    {
      "name": "sort.strings",
      "display": "Sort Strings",
      "description": "Sort a string array ascending or descending.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array",  "required": true  },
        { "name": "order",  "type": "string", "required": false }
      ]
    },
    {
      "name": "sort.by_key",
      "display": "Sort Objects by Key",
      "description": "Sort an array of objects by a given key.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array",  "required": true  },
        { "name": "key",    "type": "string", "required": true  },
        { "name": "order",  "type": "string", "required": false }
      ]
    },
    {
      "name": "sort.rank",
      "display": "Rank Values",
      "description": "Assign ranks to numeric values (1 = smallest).",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array", "required": true }
      ]
    },
    {
      "name": "sort.unique",
      "display": "Unique Values",
      "description": "Remove duplicate values from an array (preserves first occurrence order).",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array", "required": true }
      ]
    },
    {
      "name": "sort.binary_search",
      "display": "Binary Search",
      "description": "Search for target in a sorted numeric array. Returns index or -1.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array",  "required": true },
        { "name": "target", "type": "number", "required": true }
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

    match req.skill.as_str() {
        "sort.numbers" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let desc = req.args["order"].as_str().unwrap_or("asc") == "desc";
            let mut nums: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
            sort_numbers(&mut nums, desc);
            let out: Vec<serde_json::Value> = nums.iter().map(|&n| serde_json::Value::from(n)).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap())
        }
        "sort.strings" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let desc = req.args["order"].as_str().unwrap_or("asc") == "desc";
            let mut strs: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            if desc { strs.sort_by(|a, b| b.cmp(a)); } else { strs.sort(); }
            let out: Vec<serde_json::Value> = strs.iter().map(|s| serde_json::Value::from(s.as_str())).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap())
        }
        "sort.by_key" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let key = match req.args["key"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'key'") };
            let desc = req.args["order"].as_str().unwrap_or("asc") == "desc";
            let mut objs = arr.clone();
            objs.sort_by(|a, b| {
                let av = a.get(key).and_then(|v| v.as_str()).unwrap_or("");
                let bv = b.get(key).and_then(|v| v.as_str()).unwrap_or("");
                if desc { bv.cmp(av) } else { av.cmp(bv) }
            });
            sdk_respond_ok(rid, &serde_json::to_string(&objs).unwrap())
        }
        "sort.rank" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let nums: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
            let ranked = rank_values(&nums);
            sdk_respond_ok(rid, &serde_json::to_string(&ranked).unwrap())
        }
        "sort.unique" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let unique = unique_values(arr);
            sdk_respond_ok(rid, &serde_json::to_string(&unique).unwrap())
        }
        "sort.binary_search" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let target = match req.args["target"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing number 'target'") };
            let nums: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
            let idx = binary_search(&nums, target);
            sdk_respond_ok(rid, &idx.to_string())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Sort logic ────────────────────────────────────────────────────────────────

fn sort_numbers(nums: &mut Vec<f64>, desc: bool) {
    if desc {
        nums.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }
}

fn rank_values(nums: &[f64]) -> Vec<serde_json::Value> {
    let mut indexed: Vec<(usize, f64)> = nums.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut ranks = vec![0usize; nums.len()];
    for (rank, (orig_idx, _)) in indexed.iter().enumerate() {
        ranks[*orig_idx] = rank + 1;
    }
    nums.iter().zip(ranks.iter()).map(|(&v, &r)| {
        serde_json::json!({"value": v, "rank": r})
    }).collect()
}

fn unique_values(arr: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut seen = std::collections::HashSet::new();
    arr.iter().filter(|v| seen.insert(v.to_string())).cloned().collect()
}

fn binary_search(nums: &[f64], target: f64) -> i64 {
    let mut lo = 0i64;
    let mut hi = nums.len() as i64 - 1;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let mv = nums[mid as usize];
        if (mv - target).abs() < f64::EPSILON {
            return mid;
        } else if mv < target {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    -1
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_asc() {
        let mut v = vec![3.0, 1.0, 2.0];
        sort_numbers(&mut v, false);
        assert_eq!(v, vec![1.0, 2.0, 3.0]);
    }
    #[test]
    fn sort_desc() {
        let mut v = vec![3.0, 1.0, 2.0];
        sort_numbers(&mut v, true);
        assert_eq!(v, vec![3.0, 2.0, 1.0]);
    }
    #[test]
    fn sort_empty() {
        let mut v: Vec<f64> = vec![];
        sort_numbers(&mut v, false);
        assert!(v.is_empty());
    }
    #[test]
    fn sort_single() {
        let mut v = vec![42.0];
        sort_numbers(&mut v, false);
        assert_eq!(v, vec![42.0]);
    }
    #[test]
    fn sort_already_sorted() {
        let mut v = vec![1.0, 2.0, 3.0];
        sort_numbers(&mut v, false);
        assert_eq!(v, vec![1.0, 2.0, 3.0]);
    }
    #[test]
    fn rank_basic() {
        let r = rank_values(&[10.0, 30.0, 20.0]);
        assert_eq!(r[0]["rank"], 1);
        assert_eq!(r[1]["rank"], 3);
        assert_eq!(r[2]["rank"], 2);
    }
    #[test]
    fn rank_single() {
        let r = rank_values(&[5.0]);
        assert_eq!(r[0]["rank"], 1);
    }
    #[test]
    fn unique_removes_dupes() {
        let arr: Vec<serde_json::Value> = vec![1.into(), 2.into(), 1.into(), 3.into()];
        let u = unique_values(&arr);
        assert_eq!(u.len(), 3);
    }
    #[test]
    fn unique_preserves_order() {
        let arr: Vec<serde_json::Value> = vec!["b".into(), "a".into(), "b".into()];
        let u = unique_values(&arr);
        assert_eq!(u[0], serde_json::Value::String("b".into()));
        assert_eq!(u[1], serde_json::Value::String("a".into()));
    }
    #[test]
    fn binary_search_found() {
        assert_eq!(binary_search(&[1.0, 2.0, 3.0, 4.0, 5.0], 3.0), 2);
    }
    #[test]
    fn binary_search_not_found() {
        assert_eq!(binary_search(&[1.0, 2.0, 4.0, 5.0], 3.0), -1);
    }
    #[test]
    fn binary_search_first() {
        assert_eq!(binary_search(&[1.0, 2.0, 3.0], 1.0), 0);
    }
    #[test]
    fn binary_search_last() {
        assert_eq!(binary_search(&[1.0, 2.0, 3.0], 3.0), 2);
    }
    #[test]
    fn binary_search_empty() {
        assert_eq!(binary_search(&[], 1.0), -1);
    }
    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.sort");
        assert_eq!(v["skills"].as_array().unwrap().len(), 6);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("sort."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
