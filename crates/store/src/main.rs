//! # `main.rs` — OpenClaw+ Store Entry Point
//!
//! **Author:** arksong2018@gmail.com
//!
//! ## Purpose
//! Binary entry point for the `openclaw-store` UI process.  Responsibilities:
//!
//! 1. **Logging** — initialises `tracing-subscriber` with a daily-rolling log
//!    file written to:
//!    - macOS  : `~/Library/Logs/OpenClawStore/`
//!    - Linux  : `~/.local/share/openclaw-store/logs/`
//!    - Windows: `%APPDATA%\OpenClawStore\logs\`
//!
//!    The log level is controlled by the `RUST_LOG` environment variable;
//!    defaults to `openclaw_store=info,warn`.
//!
//! 2. **Panic hook** — installs a custom panic hook that writes the panic
//!    location and payload to the structured log before the default handler
//!    unwinds the stack.  This ensures crashes are always captured on disk.
//!
//! 3. **UI launch** — calls `cosmic::app::run::<StoreApp>()` with a
//!    960 × 680 resizable window.  The `.app` bundle is started via
//!    `open -n /tmp/OpenClawStore.app`; macOS activates the window
//!    automatically so no manual `NSApp activate` call is needed.
//!
//! ## Why no manual NSApp activation?
//! Earlier versions called `[NSApp activateIgnoringOtherApps]` via raw
//! `objc2::msg_send!` before `cosmic::app::run` had initialised
//! `NSApplication`.  This caused a dyld / IPC crash
//! (`(ipc/send) invalid destination port`).  The fix is to rely on macOS
//! to activate the window when the `.app` bundle is opened with `open -n`.

use openclaw_store::app::StoreApp;
use tracing::{error, info};

fn main() -> anyhow::Result<()> {
    // ── Logging setup ─────────────────────────────────────────────────────────
    // Write structured logs to both stderr and a rolling file under
    // ~/Library/Logs/OpenClawStore/ (macOS) or ~/.local/share/openclaw-store/logs/
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "openclaw-store.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("openclaw_store=info,warn")
        });

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    // ── Panic hook — log panics before unwinding ──────────────────────────────
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = info.payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic>");
        error!(
            panic.location = %location,
            panic.payload  = %payload,
            "PANIC — openclaw-store crashed"
        );
        default_hook(info);
    }));

    info!(
        version = env!("CARGO_PKG_VERSION"),
        log_dir = %log_dir.display(),
        "openclaw-store starting"
    );

    // ── Window settings ───────────────────────────────────────────────────────
    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(960.0, 680.0))
        .resizable(Some(1.0))
        .debug(false);

    // ── Run ───────────────────────────────────────────────────────────────────
    // NOTE: On macOS we intentionally do NOT call NSApp activate here.
    // The .app bundle is launched via `open -n` which causes macOS to
    // automatically bring the window to the foreground.  Any attempt to call
    // [NSApp activateIgnoringOtherApps] before winit has initialised
    // NSApplication results in a dyld/ObjC crash (invalid destination port).
    if let Err(e) = cosmic::app::run::<StoreApp>(settings, ()) {
        error!(error = %e, "cosmic::app::run returned an error");
        return Err(anyhow::anyhow!("UI runtime error: {e}"));
    }

    info!("openclaw-store exiting cleanly");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn log_directory() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Library")
            .join("Logs")
            .join("OpenClawStore")
    }
    #[cfg(target_os = "windows")]
    {
        // %APPDATA%\OpenClawStore\logs
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("OpenClawStore")
            .join("logs")
    }
    #[cfg(target_os = "linux")]
    {
        // ~/.local/share/openclaw-store/logs
        dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("openclaw-store")
            .join("logs")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        std::path::PathBuf::from(".").join("logs")
    }
}
