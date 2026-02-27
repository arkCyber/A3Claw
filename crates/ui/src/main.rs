mod app;
mod env_check;
mod i18n;
mod icons;
mod ipc_client;
mod pages;
mod widgets;
mod theme;

use app::OpenClawApp;
use tracing_subscriber::EnvFilter;
use std::io::Write;

/// A simple synchronous file writer for tracing so logs reach disk even when
/// the process is launched via `open App.app` and stdout/stderr are /dev/null.
struct SyncFileWriter(std::sync::Mutex<std::fs::File>);
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SyncFileWriter {
    type Writer = SyncGuardWriter<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        SyncGuardWriter(self.0.lock().unwrap())
    }
}
struct SyncGuardWriter<'a>(std::sync::MutexGuard<'a, std::fs::File>);
impl<'a> Write for SyncGuardWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { self.0.write(buf) }
    fn flush(&mut self) -> std::io::Result<()> { self.0.flush() }
}

fn main() -> cosmic::iced::Result {
    // Write logs to /tmp/openclaw.log (absolute path, sync) so they are visible
    // even when the app is launched via `open /tmp/OpenClawPlus.app`.
    let log_file = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("/tmp/openclaw.log")
        .expect("open /tmp/openclaw.log");
    let writer = SyncFileWriter(std::sync::Mutex::new(log_file));
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("openclaw_ui=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();

    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(1200.0, 820.0))
        .size_limits(
            cosmic::iced::core::layout::Limits::NONE
                .min_width(900.0)
                .min_height(600.0)
                .max_width(1600.0)
                .max_height(1200.0),
        )
        .resizable(Some(4.0))
        .debug(false)
        .theme(theme::warm_dark_theme());

    cosmic::app::run::<OpenClawApp>(settings, ())
}
