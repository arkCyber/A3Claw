//! skill-logic — boolean / conditional / comparison skills (pure-Rust, no I/O).
//!
//! Skills exposed:
//!   logic.and        { a: bool, b: bool }            → bool
//!   logic.or         { a: bool, b: bool }            → bool
//!   logic.not        { a: bool }                     → bool
//!   logic.xor        { a: bool, b: bool }            → bool
//!   logic.if_else    { cond: bool, then: any, else: any } → any
//!   logic.all        { values: [bool] }              → bool
//!   logic.any        { values: [bool] }              → bool
//!   logic.coalesce   { values: [any] }               → first non-null value

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.logic",
  "name": "Logic Skills",
  "version": "0.1.0",
  "description": "Boolean operations, conditionals, and coalescing",
  "skills": [
    {
      "name": "logic.and",
      "display": "Logical AND",
      "description": "Return true if both a and b are true.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "boolean", "required": true },
        { "name": "b", "type": "boolean", "required": true }
      ]
    },
    {
      "name": "logic.or",
      "display": "Logical OR",
      "description": "Return true if either a or b is true.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "boolean", "required": true },
        { "name": "b", "type": "boolean", "required": true }
      ]
    },
    {
      "name": "logic.not",
      "display": "Logical NOT",
      "description": "Return the boolean negation of a.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "boolean", "required": true }
      ]
    },
    {
      "name": "logic.xor",
      "display": "Logical XOR",
      "description": "Return true if exactly one of a, b is true.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "boolean", "required": true },
        { "name": "b", "type": "boolean", "required": true }
      ]
    },
    {
      "name": "logic.if_else",
      "display": "If/Else",
      "description": "Return then_val if cond is true, else_val otherwise.",
      "risk": "safe",
      "params": [
        { "name": "cond",     "type": "boolean", "required": true },
        { "name": "then_val", "type": "any",     "required": true },
        { "name": "else_val", "type": "any",     "required": true }
      ]
    },
    {
      "name": "logic.all",
      "display": "All True",
      "description": "Return true if every element in values is true.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array", "required": true }
      ]
    },
    {
      "name": "logic.any",
      "display": "Any True",
      "description": "Return true if at least one element in values is true.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array", "required": true }
      ]
    },
    {
      "name": "logic.coalesce",
      "display": "Coalesce",
      "description": "Return the first non-null value from the array.",
      "risk": "safe",
      "params": [
        { "name": "values", "type": "array", "required": true }
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
        "logic.and" => {
            let a = match req.args["a"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'a'") };
            let b = match req.args["b"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'b'") };
            sdk_respond_ok(rid, if logic_and(a, b) { "true" } else { "false" })
        }
        "logic.or" => {
            let a = match req.args["a"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'a'") };
            let b = match req.args["b"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'b'") };
            sdk_respond_ok(rid, if logic_or(a, b) { "true" } else { "false" })
        }
        "logic.not" => {
            let a = match req.args["a"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'a'") };
            sdk_respond_ok(rid, if logic_not(a) { "true" } else { "false" })
        }
        "logic.xor" => {
            let a = match req.args["a"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'a'") };
            let b = match req.args["b"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'b'") };
            sdk_respond_ok(rid, if logic_xor(a, b) { "true" } else { "false" })
        }
        "logic.if_else" => {
            let cond = match req.args["cond"].as_bool() { Some(v) => v, None => return sdk_respond_err(rid, "missing bool 'cond'") };
            let result = if cond { &req.args["then_val"] } else { &req.args["else_val"] };
            sdk_respond_ok(rid, &result.to_string())
        }
        "logic.all" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let result = arr.iter().all(|v| v.as_bool().unwrap_or(false));
            sdk_respond_ok(rid, if result { "true" } else { "false" })
        }
        "logic.any" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            let result = arr.iter().any(|v| v.as_bool().unwrap_or(false));
            sdk_respond_ok(rid, if result { "true" } else { "false" })
        }
        "logic.coalesce" => {
            let arr = match req.args["values"].as_array() { Some(v) => v, None => return sdk_respond_err(rid, "missing array 'values'") };
            match arr.iter().find(|v| !v.is_null()) {
                Some(v) => sdk_respond_ok(rid, &v.to_string()),
                None    => sdk_respond_ok(rid, "null"),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Logic functions ───────────────────────────────────────────────────────────

fn logic_and(a: bool, b: bool) -> bool { a && b }
fn logic_or(a: bool, b: bool)  -> bool { a || b }
fn logic_not(a: bool)          -> bool { !a }
fn logic_xor(a: bool, b: bool) -> bool { a ^ b }

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn and_tt() { assert!(logic_and(true, true)); }
    #[test] fn and_tf() { assert!(!logic_and(true, false)); }
    #[test] fn and_ff() { assert!(!logic_and(false, false)); }
    #[test] fn or_tf()  { assert!(logic_or(true, false)); }
    #[test] fn or_ff()  { assert!(!logic_or(false, false)); }
    #[test] fn not_t()  { assert!(!logic_not(true)); }
    #[test] fn not_f()  { assert!(logic_not(false)); }
    #[test] fn xor_tt() { assert!(!logic_xor(true, true)); }
    #[test] fn xor_tf() { assert!(logic_xor(true, false)); }
    #[test] fn xor_ff() { assert!(!logic_xor(false, false)); }

    #[test]
    fn all_true() {
        let arr: Vec<serde_json::Value> = vec![true.into(), true.into(), true.into()];
        assert!(arr.iter().all(|v| v.as_bool().unwrap_or(false)));
    }
    #[test]
    fn all_with_false() {
        let arr: Vec<serde_json::Value> = vec![true.into(), false.into()];
        assert!(!arr.iter().all(|v| v.as_bool().unwrap_or(false)));
    }
    #[test]
    fn any_one_true() {
        let arr: Vec<serde_json::Value> = vec![false.into(), true.into()];
        assert!(arr.iter().any(|v| v.as_bool().unwrap_or(false)));
    }
    #[test]
    fn any_all_false() {
        let arr: Vec<serde_json::Value> = vec![false.into(), false.into()];
        assert!(!arr.iter().any(|v| v.as_bool().unwrap_or(false)));
    }
    #[test]
    fn coalesce_first_non_null() {
        let arr: Vec<serde_json::Value> = vec![serde_json::Value::Null, 42.into(), "hello".into()];
        let first = arr.iter().find(|v| !v.is_null()).unwrap();
        assert_eq!(first, &serde_json::Value::Number(42.into()));
    }
    #[test]
    fn coalesce_all_null() {
        let arr: Vec<serde_json::Value> = vec![serde_json::Value::Null, serde_json::Value::Null];
        assert!(arr.iter().find(|v| !v.is_null()).is_none());
    }
    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.logic");
        assert_eq!(v["skills"].as_array().unwrap().len(), 8);
    }
    #[test]
    fn manifest_skill_names_start_with_logic() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("logic."));
        }
    }
    #[test]
    fn all_skills_have_risk_field() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["risk"].is_string());
        }
    }
}
