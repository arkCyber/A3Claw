//! skill-semver — semantic versioning skills (pure-Rust, no I/O).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.semver",
  "name": "SemVer Skills",
  "version": "0.1.0",
  "description": "Semantic version parsing, comparison, bumping, and range checking",
  "skills": [
    {
      "name": "semver.parse",
      "display": "Parse SemVer",
      "description": "Parse a semantic version string into major/minor/patch/pre/build components.",
      "risk": "safe",
      "params": [{ "name": "version", "type": "string", "required": true }]
    },
    {
      "name": "semver.compare",
      "display": "Compare Versions",
      "description": "Compare two semver strings. Returns -1, 0, or 1.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "string", "required": true },
        { "name": "b", "type": "string", "required": true }
      ]
    },
    {
      "name": "semver.bump",
      "display": "Bump Version",
      "description": "Increment major, minor, or patch component of a version.",
      "risk": "safe",
      "params": [
        { "name": "version",    "type": "string", "required": true },
        { "name": "component",  "type": "string", "required": true }
      ]
    },
    {
      "name": "semver.satisfies",
      "display": "Satisfies Range",
      "description": "Check if a version satisfies a range like >=1.0.0 <2.0.0.",
      "risk": "safe",
      "params": [
        { "name": "version", "type": "string", "required": true },
        { "name": "range",   "type": "string", "required": true }
      ]
    },
    {
      "name": "semver.sort",
      "display": "Sort Versions",
      "description": "Sort an array of semver strings ascending or descending.",
      "risk": "safe",
      "params": [
        { "name": "versions", "type": "array",  "required": true  },
        { "name": "order",    "type": "string", "required": false }
      ]
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
        "semver.parse" => {
            let v = match req.args["version"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'version'") };
            match parse_semver(v) {
                Ok(p)  => {
                    let json = format!(
                        r#"{{"major":{},"minor":{},"patch":{},"pre":{},"build":{}}}"#,
                        p.major, p.minor, p.patch,
                        serde_json::Value::String(p.pre),
                        serde_json::Value::String(p.build),
                    );
                    sdk_respond_ok(rid, &json)
                }
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "semver.compare" => {
            let a = match req.args["a"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'b'") };
            match (parse_semver(a), parse_semver(b)) {
                (Ok(va), Ok(vb)) => sdk_respond_ok(rid, &cmp_ver(&va, &vb).to_string()),
                (Err(e), _) | (_, Err(e)) => sdk_respond_err(rid, &e),
            }
        }
        "semver.bump" => {
            let v = match req.args["version"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'version'") };
            let comp = req.args["component"].as_str().unwrap_or("patch");
            match bump_version(v, comp) {
                Ok(s)  => sdk_respond_ok(rid, &s),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "semver.satisfies" => {
            let v = match req.args["version"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'version'") };
            let r = match req.args["range"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'range'") };
            match satisfies(v, r) {
                Ok(b)  => sdk_respond_ok(rid, if b { "true" } else { "false" }),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "semver.sort" => {
            let arr = match req.args["versions"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'versions'") };
            let desc = req.args["order"].as_str().unwrap_or("asc") == "desc";
            let mut vs: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            vs.sort_by(|a, b| {
                let (pa, pb) = (parse_semver(a).ok(), parse_semver(b).ok());
                let ord = match (pa, pb) {
                    (Some(a), Some(b)) => cmp_ver(&a, &b),
                    _ => 0,
                };
                if desc { ord.cmp(&0).reverse() } else { ord.cmp(&0) }
            });
            let out: Vec<serde_json::Value> = vs.iter().map(|s| serde_json::Value::String(s.clone())).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── SemVer logic ──────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
struct SemVer { major: u64, minor: u64, patch: u64, pre: String, build: String }

fn parse_semver(s: &str) -> Result<SemVer, String> {
    let s = s.trim().trim_start_matches('v');
    let (core, build) = if let Some(i) = s.find('+') {
        (&s[..i], s[i+1..].to_string())
    } else { (s, String::new()) };
    let (core, pre) = if let Some(i) = core.find('-') {
        (&core[..i], core[i+1..].to_string())
    } else { (core, String::new()) };
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 { return Err(format!("invalid semver: {s}")); }
    Ok(SemVer {
        major: parts[0].parse().map_err(|_| format!("invalid major: {}", parts[0]))?,
        minor: parts[1].parse().map_err(|_| format!("invalid minor: {}", parts[1]))?,
        patch: parts[2].parse().map_err(|_| format!("invalid patch: {}", parts[2]))?,
        pre, build,
    })
}

fn cmp_ver(a: &SemVer, b: &SemVer) -> i32 {
    for (x, y) in [(a.major, b.major), (a.minor, b.minor), (a.patch, b.patch)] {
        if x < y { return -1; }
        if x > y { return  1; }
    }
    0
}

fn bump_version(v: &str, comp: &str) -> Result<String, String> {
    let mut sv = parse_semver(v)?;
    match comp {
        "major" => { sv.major += 1; sv.minor = 0; sv.patch = 0; }
        "minor" => { sv.minor += 1; sv.patch = 0; }
        "patch" => { sv.patch += 1; }
        other   => return Err(format!("unknown component: {other}")),
    }
    Ok(format!("{}.{}.{}", sv.major, sv.minor, sv.patch))
}

fn satisfies(version: &str, range: &str) -> Result<bool, String> {
    let ver = parse_semver(version)?;
    for part in range.split_whitespace() {
        let (op, rest) = if part.starts_with(">=") { (">=", &part[2..]) }
            else if part.starts_with("<=") { ("<=", &part[2..]) }
            else if part.starts_with('>') { (">", &part[1..]) }
            else if part.starts_with('<') { ("<", &part[1..]) }
            else if part.starts_with('^') { ("^", &part[1..]) }
            else { ("=", part) };
        let rv = parse_semver(rest)?;
        let c = cmp_ver(&ver, &rv);
        let ok = match op {
            ">=" => c >= 0,
            "<=" => c <= 0,
            ">"  => c > 0,
            "<"  => c < 0,
            "^"  => ver.major == rv.major && c >= 0,
            _    => c == 0,
        };
        if !ok { return Ok(false); }
    }
    Ok(true)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn parse_basic()   { let v = parse_semver("1.2.3").unwrap(); assert_eq!((v.major, v.minor, v.patch), (1, 2, 3)); }
    #[test] fn parse_v_prefix(){ let v = parse_semver("v2.0.0").unwrap(); assert_eq!(v.major, 2); }
    #[test] fn parse_pre()     { let v = parse_semver("1.0.0-alpha").unwrap(); assert_eq!(v.pre, "alpha"); }
    #[test] fn parse_build()   { let v = parse_semver("1.0.0+build.1").unwrap(); assert_eq!(v.build, "build.1"); }
    #[test] fn parse_invalid() { assert!(parse_semver("1.2").is_err()); }

    #[test] fn cmp_lt()  { let (a, b) = (parse_semver("1.0.0").unwrap(), parse_semver("2.0.0").unwrap()); assert_eq!(cmp_ver(&a, &b), -1); }
    #[test] fn cmp_eq()  { let (a, b) = (parse_semver("1.2.3").unwrap(), parse_semver("1.2.3").unwrap()); assert_eq!(cmp_ver(&a, &b), 0); }
    #[test] fn cmp_gt()  { let (a, b) = (parse_semver("1.2.4").unwrap(), parse_semver("1.2.3").unwrap()); assert_eq!(cmp_ver(&a, &b), 1); }

    #[test] fn bump_patch() { assert_eq!(bump_version("1.2.3", "patch").unwrap(), "1.2.4"); }
    #[test] fn bump_minor() { assert_eq!(bump_version("1.2.3", "minor").unwrap(), "1.3.0"); }
    #[test] fn bump_major() { assert_eq!(bump_version("1.2.3", "major").unwrap(), "2.0.0"); }
    #[test] fn bump_bad()   { assert!(bump_version("1.2.3", "build").is_err()); }

    #[test] fn satisfies_gte()  { assert!(satisfies("1.5.0", ">=1.0.0").unwrap()); }
    #[test] fn satisfies_lt()   { assert!(satisfies("0.9.0", "<1.0.0").unwrap()); }
    #[test] fn satisfies_caret(){ assert!(satisfies("1.2.0", "^1.0.0").unwrap()); }
    #[test] fn satisfies_fail() { assert!(!satisfies("2.0.0", "^1.0.0").unwrap()); }

    #[test]
    fn sort_versions() {
        let mut vs = vec!["2.0.0".to_string(), "1.0.0".to_string(), "1.5.0".to_string()];
        vs.sort_by(|a, b| {
            let c = cmp_ver(&parse_semver(a).unwrap(), &parse_semver(b).unwrap());
            c.cmp(&0)
        });
        assert_eq!(vs, vec!["1.0.0", "1.5.0", "2.0.0"]);
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.semver");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("semver."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
