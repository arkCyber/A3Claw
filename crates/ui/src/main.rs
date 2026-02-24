mod app;
mod i18n;
mod icons;
mod ipc_client;
mod pages;
mod widgets;
mod theme;

use app::OpenClawApp;
use tracing_subscriber::EnvFilter;

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("openclaw_ui=info".parse().unwrap()))
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
