//! OpenClaw+ Plugin Gateway — library target.
//!
//! Exposes the router, state, skill registry, and types as a library so that
//! unit and integration tests can be compiled with `cargo test --lib`.
//! The binary entry point is in `src/main.rs`.

pub mod router;
pub mod skill_registry;
pub mod state;
pub mod types;
