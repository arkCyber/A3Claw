//! skill-diff — text and JSON diffing skills (pure-Rust, no I/O).
//!
//! Skills:
//!   diff.lines       { a: string, b: string }  → unified-diff style JSON array
//!   diff.chars       { a: string, b: string }  → edit distance + operations
//!   diff.json_patch  { a: object, b: object }  → JSON Patch (RFC 6902) array
//!   diff.similarity  { a: string, b: string }  → 0.0–1.0 similarity ratio
//!   diff.lcs         { a: string, b: string }  → longest common subsequence

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.diff",
  "name": "Diff Skills",
  "version": "0.1.0",
  "description": "Text and JSON diffing, LCS, edit distance, similarity",
  "skills": [
    {
      "name": "diff.lines",
      "display": "Line Diff",
      "description": "Compare two multi-line strings line by line, returning added/removed/context lines.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "string", "required": true },
        { "name": "b", "type": "string", "required": true }
      ]
    },
    {
      "name": "diff.chars",
      "display": "Char Edit Distance",
      "description": "Return the Levenshtein edit distance between two strings.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "string", "required": true },
        { "name": "b", "type": "string", "required": true }
      ]
    },
    {
      "name": "diff.json_patch",
      "display": "JSON Patch",
      "description": "Compute a JSON Patch (RFC 6902) array to transform object a into b.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "object", "required": true },
        { "name": "b", "type": "object", "required": true }
      ]
    },
    {
      "name": "diff.similarity",
      "display": "String Similarity",
      "description": "Return a 0.0-1.0 similarity ratio based on edit distance.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "string", "required": true },
        { "name": "b", "type": "string", "required": true }
      ]
    },
    {
      "name": "diff.lcs",
      "display": "Longest Common Subsequence",
      "description": "Return the longest common subsequence of two strings.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "string", "required": true },
        { "name": "b", "type": "string", "required": true }
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
        "diff.lines" => {
            let a = match req.args["a"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'b'") };
            let result = diff_lines(a, b);
            sdk_respond_ok(rid, &serde_json::to_string(&result).unwrap())
        }
        "diff.chars" => {
            let a = match req.args["a"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'b'") };
            sdk_respond_ok(rid, &edit_distance(a, b).to_string())
        }
        "diff.json_patch" => {
            let patch = json_patch(&req.args["a"], &req.args["b"]);
            sdk_respond_ok(rid, &serde_json::to_string(&patch).unwrap())
        }
        "diff.similarity" => {
            let a = match req.args["a"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'b'") };
            let sim = similarity(a, b);
            sdk_respond_ok(rid, &format!("{:.6}", sim))
        }
        "diff.lcs" => {
            let a = match req.args["a"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'b'") };
            sdk_respond_ok(rid, &lcs(a, b))
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Diff logic ────────────────────────────────────────────────────────────────

fn diff_lines(a: &str, b: &str) -> Vec<serde_json::Value> {
    let la: Vec<&str> = a.lines().collect();
    let lb: Vec<&str> = b.lines().collect();
    let mut result = Vec::new();
    let max = la.len().max(lb.len());
    for i in 0..max {
        match (la.get(i), lb.get(i)) {
            (Some(l), Some(r)) if l == r => result.push(serde_json::json!({"op": "=", "line": l})),
            (Some(l), Some(r))           => {
                result.push(serde_json::json!({"op": "-", "line": l}));
                result.push(serde_json::json!({"op": "+", "line": r}));
            }
            (Some(l), None)              => result.push(serde_json::json!({"op": "-", "line": l})),
            (None, Some(r))              => result.push(serde_json::json!({"op": "+", "line": r})),
            (None, None)                 => {}
        }
    }
    result
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] {
                dp[i-1][j-1]
            } else {
                1 + dp[i-1][j].min(dp[i][j-1]).min(dp[i-1][j-1])
            };
        }
    }
    dp[m][n]
}

fn similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    let max_len = a.chars().count().max(b.chars().count());
    if max_len == 0 { return 1.0; }
    let dist = edit_distance(a, b);
    1.0 - (dist as f64 / max_len as f64)
}

fn lcs(a: &str, b: &str) -> String {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] {
                dp[i-1][j-1] + 1
            } else {
                dp[i-1][j].max(dp[i][j-1])
            };
        }
    }
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i-1] == b[j-1] { result.push(a[i-1]); i -= 1; j -= 1; }
        else if dp[i-1][j] > dp[i][j-1] { i -= 1; }
        else { j -= 1; }
    }
    result.reverse();
    result.iter().collect()
}

fn json_patch(a: &serde_json::Value, b: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut ops = Vec::new();
    let empty = serde_json::Map::new();
    let ao = a.as_object().unwrap_or(&empty);
    let bo = b.as_object().unwrap_or(&empty);
    for (k, bv) in bo {
        match ao.get(k) {
            None     => ops.push(serde_json::json!({"op": "add",     "path": format!("/{k}"), "value": bv})),
            Some(av) if av != bv => ops.push(serde_json::json!({"op": "replace", "path": format!("/{k}"), "value": bv})),
            _ => {}
        }
    }
    for k in ao.keys() {
        if !bo.contains_key(k) {
            ops.push(serde_json::json!({"op": "remove", "path": format!("/{k}")}));
        }
    }
    ops
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn edit_same()    { assert_eq!(edit_distance("abc", "abc"), 0); }
    #[test] fn edit_empty()   { assert_eq!(edit_distance("", "abc"), 3); }
    #[test] fn edit_insert()  { assert_eq!(edit_distance("ac", "abc"), 1); }
    #[test] fn edit_replace() { assert_eq!(edit_distance("abc", "axc"), 1); }
    #[test] fn edit_delete()  { assert_eq!(edit_distance("abcd", "abc"), 1); }
    #[test] fn edit_both_empty() { assert_eq!(edit_distance("", ""), 0); }

    #[test] fn similarity_identical() { assert!((similarity("hello", "hello") - 1.0).abs() < 1e-9); }
    #[test] fn similarity_empty()     { assert!((similarity("", "") - 1.0).abs() < 1e-9); }
    #[test] fn similarity_different() { assert!(similarity("abc", "xyz") < 1.0); }
    #[test] fn similarity_range()     { let s = similarity("hello", "helo"); assert!((0.0..=1.0).contains(&s)); }

    #[test] fn lcs_basic()  { let r = lcs("ABCBDAB", "BDCAB"); assert_eq!(r.len(), 4); }
    #[test] fn lcs_empty()  { assert_eq!(lcs("", "abc"), ""); }
    #[test] fn lcs_same()   { assert_eq!(lcs("abc", "abc"), "abc"); }
    #[test] fn lcs_no_common() { assert_eq!(lcs("abc", "xyz"), ""); }

    #[test]
    fn diff_lines_same() {
        let d = diff_lines("a\nb", "a\nb");
        assert!(d.iter().all(|v| v["op"] == "="));
    }
    #[test]
    fn diff_lines_added() {
        let d = diff_lines("a", "a\nb");
        assert!(d.iter().any(|v| v["op"] == "+"));
    }
    #[test]
    fn diff_lines_removed() {
        let d = diff_lines("a\nb", "a");
        assert!(d.iter().any(|v| v["op"] == "-"));
    }

    #[test]
    fn json_patch_add() {
        let a = serde_json::json!({"x": 1});
        let b = serde_json::json!({"x": 1, "y": 2});
        let p = json_patch(&a, &b);
        assert!(p.iter().any(|op| op["op"] == "add" && op["path"] == "/y"));
    }
    #[test]
    fn json_patch_remove() {
        let a = serde_json::json!({"x": 1, "y": 2});
        let b = serde_json::json!({"x": 1});
        let p = json_patch(&a, &b);
        assert!(p.iter().any(|op| op["op"] == "remove" && op["path"] == "/y"));
    }
    #[test]
    fn json_patch_replace() {
        let a = serde_json::json!({"x": 1});
        let b = serde_json::json!({"x": 2});
        let p = json_patch(&a, &b);
        assert!(p.iter().any(|op| op["op"] == "replace" && op["path"] == "/x"));
    }
    #[test]
    fn json_patch_no_change() {
        let a = serde_json::json!({"x": 1});
        let p = json_patch(&a, &a);
        assert!(p.is_empty());
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.diff");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("diff."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
