//! Example OpenClaw+ WASM skill plugin.
//!
//! Provides two skills:
//! - `hello.greet` — greet a person by name
//! - `hello.echo`  — echo back the given text
//!
//! Build:
//!   cargo build --target wasm32-wasip1 --release
//!   cp target/wasm32-wasip1/release/hello_skill_plugin.wasm ~/.openclaw/skills/

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.hello",
  "name": "Hello Skill Plugin",
  "version": "1.0.0",
  "description": "Example plugin: greet and echo",
  "skills": [
    {
      "name": "hello.greet",
      "display": "Greet",
      "description": "Greet a person by name.",
      "risk": "safe",
      "params": [
        { "name": "name", "type": "string", "description": "Person to greet", "required": true }
      ]
    },
    {
      "name": "hello.echo",
      "display": "Echo",
      "description": "Echo back the given text.",
      "risk": "safe",
      "params": [
        { "name": "text", "type": "string", "description": "Text to echo", "required": true }
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
        Ok(r) => r,
        Err(e) => return sdk_respond_err("", &format!("Failed to parse request: {}", e)),
    };

    match req.skill.as_str() {
        "hello.greet" => {
            let name = req.args["name"].as_str().unwrap_or("World");
            sdk_respond_ok(&req.request_id, &format!("Hello, {}! Greetings from the WASM plugin.", name))
        }
        "hello.echo" => {
            let text = req.args["text"].as_str().unwrap_or("");
            sdk_respond_ok(&req.request_id, text)
        }
        other => sdk_respond_err(&req.request_id, &format!("Unknown skill: {}", other)),
    }
}
