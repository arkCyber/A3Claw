//! skill-path — file path manipulation skills (pure-Rust, no I/O).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.path",
  "name": "Path Skills",
  "version": "0.1.0",
  "description": "File path manipulation: join, split, basename, dirname, extension, normalize",
  "skills": [
    {
      "name": "path.join",
      "display": "Join Paths",
      "description": "Join path segments with the system separator.",
      "risk": "safe",
      "params": [{ "name": "parts", "type": "array", "required": true }]
    },
    {
      "name": "path.basename",
      "display": "Basename",
      "description": "Return the final component of a path.",
      "risk": "safe",
      "params": [{ "name": "path", "type": "string", "required": true }]
    },
    {
      "name": "path.dirname",
      "display": "Dirname",
      "description": "Return the directory portion of a path.",
      "risk": "safe",
      "params": [{ "name": "path", "type": "string", "required": true }]
    },
    {
      "name": "path.extension",
      "display": "File Extension",
      "description": "Return the file extension (without dot), or empty string.",
      "risk": "safe",
      "params": [{ "name": "path", "type": "string", "required": true }]
    },
    {
      "name": "path.normalize",
      "display": "Normalize Path",
      "description": "Resolve . and .. components and remove duplicate separators.",
      "risk": "safe",
      "params": [{ "name": "path", "type": "string", "required": true }]
    },
    {
      "name": "path.split",
      "display": "Split Path",
      "description": "Split a path into its components.",
      "risk": "safe",
      "params": [{ "name": "path", "type": "string", "required": true }]
    },
    {
      "name": "path.is_absolute",
      "display": "Is Absolute",
      "description": "Return true if the path starts with /.",
      "risk": "safe",
      "params": [{ "name": "path", "type": "string", "required": true }]
    }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r) => r, Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();

    match req.skill.as_str() {
        "path.join" => {
            let arr = match req.args["parts"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'parts'") };
            let parts: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            sdk_respond_ok(rid, &path_join(&parts))
        }
        "path.basename" => {
            let p = match req.args["path"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'") };
            sdk_respond_ok(rid, path_basename(p))
        }
        "path.dirname" => {
            let p = match req.args["path"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'") };
            sdk_respond_ok(rid, &path_dirname(p))
        }
        "path.extension" => {
            let p = match req.args["path"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'") };
            sdk_respond_ok(rid, path_extension(p))
        }
        "path.normalize" => {
            let p = match req.args["path"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'") };
            sdk_respond_ok(rid, &path_normalize(p))
        }
        "path.split" => {
            let p = match req.args["path"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'") };
            let parts = path_split(p);
            sdk_respond_ok(rid, &serde_json::to_string(&parts).unwrap())
        }
        "path.is_absolute" => {
            let p = match req.args["path"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'path'") };
            sdk_respond_ok(rid, if p.starts_with('/') { "true" } else { "false" })
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Path logic ────────────────────────────────────────────────────────────────

fn path_join(parts: &[&str]) -> String {
    parts.iter().enumerate().fold(String::new(), |mut acc, (i, &p)| {
        if i > 0 && !acc.ends_with('/') && !p.starts_with('/') { acc.push('/'); }
        acc.push_str(p);
        acc
    })
}

fn path_basename(p: &str) -> &str {
    p.trim_end_matches('/').rsplit('/').next().unwrap_or(p)
}

fn path_dirname(p: &str) -> String {
    let p = p.trim_end_matches('/');
    if let Some(i) = p.rfind('/') {
        if i == 0 { "/".to_string() } else { p[..i].to_string() }
    } else {
        ".".to_string()
    }
}

fn path_extension(p: &str) -> &str {
    let base = path_basename(p);
    if let Some(i) = base.rfind('.') {
        if i > 0 { &base[i+1..] } else { "" }
    } else { "" }
}

fn path_normalize(p: &str) -> String {
    let absolute = p.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".."     => { parts.pop(); }
            s        => parts.push(s),
        }
    }
    let result = parts.join("/");
    if absolute { format!("/{}", result) } else if result.is_empty() { ".".to_string() } else { result }
}

fn path_split(p: &str) -> Vec<&str> {
    p.split('/').filter(|s| !s.is_empty()).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn join_basic()    { assert_eq!(path_join(&["a", "b", "c"]), "a/b/c"); }
    #[test] fn join_trailing() { assert_eq!(path_join(&["a/", "b"]), "a/b"); }
    #[test] fn join_empty()    { assert_eq!(path_join(&[]), ""); }
    #[test] fn join_single()   { assert_eq!(path_join(&["foo"]), "foo"); }

    #[test] fn basename_basic()    { assert_eq!(path_basename("/a/b/file.txt"), "file.txt"); }
    #[test] fn basename_no_slash() { assert_eq!(path_basename("file.txt"), "file.txt"); }
    #[test] fn basename_trailing_slash() { assert_eq!(path_basename("/a/b/"), "b"); }

    #[test] fn dirname_basic()  { assert_eq!(path_dirname("/a/b/c"), "/a/b"); }
    #[test] fn dirname_root()   { assert_eq!(path_dirname("/file"), "/"); }
    #[test] fn dirname_rel()    { assert_eq!(path_dirname("file"), "."); }

    #[test] fn ext_basic()   { assert_eq!(path_extension("file.txt"), "txt"); }
    #[test] fn ext_no_ext()  { assert_eq!(path_extension("file"), ""); }
    #[test] fn ext_dotfile() { assert_eq!(path_extension(".hidden"), ""); }
    #[test] fn ext_multi()   { assert_eq!(path_extension("archive.tar.gz"), "gz"); }

    #[test] fn normalize_dotdot()  { assert_eq!(path_normalize("/a/b/../c"), "/a/c"); }
    #[test] fn normalize_dot()     { assert_eq!(path_normalize("/a/./b"), "/a/b"); }
    #[test] fn normalize_double_slash() { assert_eq!(path_normalize("/a//b"), "/a/b"); }
    #[test] fn normalize_relative() { assert_eq!(path_normalize("a/b/../c"), "a/c"); }

    #[test] fn split_basic() { assert_eq!(path_split("/a/b/c"), vec!["a", "b", "c"]); }
    #[test] fn split_rel()   { assert_eq!(path_split("foo/bar"), vec!["foo", "bar"]); }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.path");
        assert_eq!(v["skills"].as_array().unwrap().len(), 7);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("path."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
