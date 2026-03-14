# Contributing to OpenClaw+ WASM Skills

OpenClaw+ uses a plugin architecture where skills are compiled to `wasm32-wasip1` and loaded at runtime from `~/.openclaw/skills/`. This guide explains how to build and contribute a new community skill crate.

---

## Quick Start

```bash
# 1. Clone the repo
git clone https://github.com/your-org/openclaw-plus
cd openclaw-plus

# 2. Install the wasm32-wasip1 target
rustup target add wasm32-wasip1

# 3. Build all official skill crates and install to ~/.openclaw/skills/
cargo build -p skills --release

# 4. Run all tests
cargo test -p skill-hash -p skill-encode -p skill-math -p skill-text \
           -p skill-datetime -p skill-crypto -p skill-uuid \
           -p skill-compress -p skill-json -p skill-regex -p skill-network
```

---

## Official WASM Skill Registry (62 skills across 11 crates)

| Crate | Skill prefix | Skills |
|---|---|---|
| `skill-hash` | `hash.*` | md5, sha1, sha256, sha512, hmac_sha256 |
| `skill-encode` | `encode.*` | base64_encode/decode, hex_encode/decode, url_encode/decode, html_escape, morse, rot13 |
| `skill-math` | `math.*` | eval, gcd, lcm, prime_check |
| `skill-text` | `text.*` | truncate, word_count, slugify, title_case, pad_left, pad_right |
| `skill-datetime` | `datetime.*` | now, parse, format, diff, add, tz_convert |
| `skill-crypto` | `crypto.*` | aes_encrypt, aes_decrypt, chacha20, pbkdf2, hmac_sha512, constant_time_eq |
| `skill-uuid` | `uuid.*` | v4, v5, validate, parse, nil |
| `skill-compress` | `compress.*` | deflate, inflate, rle_encode, rle_decode, lz_encode, lz_decode |
| `skill-json` | `json.*` | validate, format, minify, get, set, merge, keys, to_csv |
| `skill-regex` | `regex.*` | test, find, find_all, replace, split, count |
| `skill-network` | `network.*` | url_parse, url_encode, url_decode, mime_type, ip_classify, http_status |

---

## Creating a Community Skill Crate

### 1. Use the template

```bash
# Copy the template into your own directory (outside this workspace)
cp -r crates/skills/hash my-skill
cd my-skill
```

Or scaffold from scratch:

```
my-skill/
  Cargo.toml
  src/
    lib.rs
```

### 2. `Cargo.toml`

```toml
[package]
name = "skill-my-skill"
version = "0.1.0"
edition = "2021"
description = "My community skill plugin for OpenClaw+"

[lib]
crate-type = ["cdylib"]
name = "skill_my_skill"

[dependencies]
openclaw-plugin-sdk = { git = "https://github.com/your-org/openclaw-plus", package = "openclaw-plugin-sdk" }
serde_json = "1.0"

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### 3. `src/lib.rs` — minimum implementation

```rust
use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.my-skill",
  "name": "My Skill",
  "version": "0.1.0",
  "description": "What this skill does.",
  "skills": [
    {
      "name": "my_skill.greet",
      "display": "Greet",
      "description": "Return a greeting.",
      "risk": "safe",
      "params": [
        { "name": "name", "type": "string", "description": "Name to greet", "required": true }
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
        "my_skill.greet" => {
            let name = match req.args["name"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'name'"),
            };
            sdk_respond_ok(rid, &format!("Hello, {}!", name))
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}
```

### 4. Build and install

```bash
# Compile to WASM
cargo build --target wasm32-wasip1 --release

# Install to the OpenClaw+ skill directory
mkdir -p ~/.openclaw/skills
cp target/wasm32-wasip1/release/skill_my_skill.wasm ~/.openclaw/skills/my_skill.wasm
```

The OpenClaw+ gateway auto-discovers all `.wasm` files in `~/.openclaw/skills/` at startup.

### 5. Test locally

```bash
# Unit tests run on native target
cargo test
```

---

## Manifest Fields

| Field | Required | Notes |
|---|---|---|
| `id` | yes | Unique reverse-domain string, e.g. `community.my-skill` |
| `name` | yes | Human-readable display name |
| `version` | yes | semver string |
| `description` | yes | One-sentence summary |
| `skills[].name` | yes | Dot-namespaced, e.g. `my_skill.greet` |
| `skills[].risk` | yes | `safe` / `confirm` / `deny` |
| `skills[].params` | yes | Array of param descriptors |

**Risk levels:**
- `safe` — executes without user confirmation
- `confirm` — pauses and asks the user before executing
- `deny` — always blocked (use for skills not yet ready)

---

## Param Types

| Type string | Rust equivalent |
|---|---|
| `"string"` | `&str` from `args["key"].as_str()` |
| `"integer"` | `i64` from `args["key"].as_i64()` |
| `"number"` | `f64` from `args["key"].as_f64()` |
| `"boolean"` | `bool` from `args["key"].as_bool()` |

---

## Naming Conventions

- **Crate name:** `skill-<namespace>` (kebab-case)
- **Lib name:** `skill_<namespace>` (snake_case, used in `.wasm` filename)
- **Skill names:** `<namespace>.<verb>` — e.g. `crypto.aes_encrypt`
- **Namespace prefix:** must not conflict with official namespaces listed above

---

## Pull Request Checklist

Before submitting a PR to add a community skill to this repository:

- [ ] Crate compiles with `cargo build --target wasm32-wasip1 --release`
- [ ] All unit tests pass with `cargo test`
- [ ] Manifest JSON is valid (tested in a `manifest_is_valid_json` test)
- [ ] Every exported skill has at least one test
- [ ] No OS-level I/O in the implementation (no `std::fs`, `std::net`, etc.)
- [ ] Risk levels are correctly assigned
- [ ] Crate added to `crates/skills/Cargo.toml` workspace members
- [ ] Crate added to `crates/skills/build.rs` skill list
- [ ] Crate added to root `Cargo.toml` workspace members

---

## Plugin SDK Reference

The `openclaw-plugin-sdk` crate (`crates/plugin-sdk/`) provides:

| Symbol | Description |
|---|---|
| `sdk_read_request(ptr, len)` | Deserialize the incoming `ExecuteRequest` from WASM linear memory |
| `sdk_respond_ok(rid, output)` | Write a success response and return its pointer+length as `u64` |
| `sdk_respond_err(rid, msg)` | Write an error response and return its pointer+length as `u64` |
| `sdk_export_str(s)` | Export a static string (used for `skill_manifest`) |
| `ExecuteRequest` | `{ request_id, skill, args: serde_json::Value }` |

Import via:

```rust
use openclaw_plugin_sdk::prelude::*;
```

---

## Community Registry

Once your skill crate is ready, open a GitHub issue with the label `community-skill` and include:

- Crate name and namespace prefix
- Short description of what the skills do
- Link to your repository or PR

We will review and add it to the community skill registry.

---

## Design Principles

- **Pure computation only.** Skills must not perform I/O (no network calls, no file system access). Network or file skills belong in built-in Rust handlers in `agent-executor`.
- **Zero dependencies preferred.** Avoid pulling in large crates. The `serde_json` dependency is always acceptable.
- **WASM-first.** All code must compile to `wasm32-wasip1`. Avoid OS-specific APIs.
- **Test everything.** Every skill variant must have at least one unit test.
