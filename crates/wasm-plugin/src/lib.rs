//! OpenClaw+ WASM plugin subsystem.
//!
//! ## Overview
//!
//! Skills not bundled in the core binary can be delivered as `.wasm` files.
//! At startup the runtime scans well-known directories, loads each plugin,
//! reads its [`abi::SkillManifest`], and registers it in the
//! [`registry::WasmPluginRegistry`].  When the dispatcher encounters an
//! unknown skill it falls through to the registry before returning the
//! "no built-in implementation" stub.
//!
//! ## Plugin search order
//!
//! 1. `<cwd>/.openclaw/skills/*.wasm`
//! 2. `~/.openclaw/skills/*.wasm`
//! 3. Paths explicitly added via [`loader::PluginLoader::add_path`].
//!
//! ## Writing a plugin
//!
//! Use the `openclaw-plugin-sdk` crate (target `wasm32-wasip1`).  The SDK
//! provides the `#[skill_plugin]` macro and the `respond!` / `respond_err!`
//! helpers so you only need to implement the business logic.

pub mod abi;
pub mod error;
pub mod executor;
pub mod loader;
pub mod registry;

pub use abi::{ExecuteRequest, ExecuteResponse, SkillDef, SkillManifest};
pub use error::PluginError;
pub use executor::WasmExecutor;
pub use loader::PluginLoader;
pub use registry::WasmPluginRegistry;
