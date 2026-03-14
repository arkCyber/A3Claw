//! Built-in tool implementations — bundled at compile time, no runtime fetch.
//!
//! Each sub-module corresponds to one or more official OpenClaw tools from
//! <https://docs.openclaw.ai/tools>. All tools are available out-of-the-box;
//! optional functionality (e.g. Brave API key for `web_search`) degrades
//! gracefully when credentials are absent.

pub mod apply_patch;
pub mod browser;
pub mod exec;
pub mod web_fetch;
pub mod image;
pub mod cron;
pub mod cron_scheduler;
pub mod sessions;
pub mod pdf;
pub mod heartbeat;
pub mod messaging;
pub mod mcp;
pub mod rag;
pub mod python;
pub mod ssh;
pub mod document;
pub mod archive;
pub mod data;
pub mod clawhub;
