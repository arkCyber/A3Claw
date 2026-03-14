//! skill-template — copy this crate to build a new OpenClaw+ WASM skill plugin.
//!
//! Steps:
//!   1. Copy this directory to a new path, e.g. `crates/skills/my-skill/`
//!   2. Rename the crate in Cargo.toml: `name = "skill-my-skill"`
//!   3. Change the lib name:           `name = "skill_my_skill"`
//!   4. Update MANIFEST below with your skill id, name, and skill definitions.
//!   5. Add dispatch arms to `skill_execute` for each skill name.
//!   6. Add your skill logic as pure functions below.
//!   7. Write unit tests.
//!   8. Register the crate in the root Cargo.toml workspace members.
//!   9. Add an entry to crates/skills/build.rs skill_crates list.
//!  10. Build: cargo build --target wasm32-wasip1 --release
//!  11. Install: cp target/wasm32-wasip1/release/skill_my_skill.wasm ~/.openclaw/skills/

use openclaw_plugin_sdk::prelude::*;

// ── Manifest ──────────────────────────────────────────────────────────────────
// Change "community.template" to your own reverse-domain id.
// Each skill needs a unique dot-namespaced name, a risk level, and param list.
// Risk levels: "safe" | "confirm" | "deny"

static MANIFEST: &str = r#"{
  "id": "community.template",
  "name": "Template Skill",
  "version": "0.1.0",
  "description": "A minimal example skill plugin. Replace with your own description.",
  "skills": [
    {
      "name": "template.hello",
      "display": "Hello",
      "description": "Return a greeting for the given name.",
      "risk": "safe",
      "params": [
        {
          "name": "name",
          "type": "string",
          "description": "Name to greet",
          "required": true
        }
      ]
    },
    {
      "name": "template.echo",
      "display": "Echo",
      "description": "Return the input value unchanged.",
      "risk": "safe",
      "params": [
        {
          "name": "value",
          "type": "string",
          "description": "Value to echo back",
          "required": true
        }
      ]
    }
  ]
}"#;

// ── Exported entry points — do not rename these two functions ──────────────────

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
        // ── Add one arm per skill listed in MANIFEST ──────────────────────────
        "template.hello" => {
            let name = match req.args["name"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'name'"),
            };
            sdk_respond_ok(rid, &skill_hello(name))
        }
        "template.echo" => {
            let value = match req.args["value"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'value'"),
            };
            sdk_respond_ok(rid, value)
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Skill logic — pure functions, no I/O ─────────────────────────────────────

fn skill_hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(skill_hello("World"), "Hello, World!");
        assert_eq!(skill_hello("OpenClaw"), "Hello, OpenClaw!");
    }

    #[test]
    fn hello_empty_name() {
        assert_eq!(skill_hello(""), "Hello, !");
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "community.template");
        assert_eq!(v["skills"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn all_skills_have_risk_field() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for skill in v["skills"].as_array().unwrap() {
            assert!(
                skill["risk"].is_string(),
                "skill {} missing risk field",
                skill["name"]
            );
        }
    }
}
