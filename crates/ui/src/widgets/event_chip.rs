#![allow(dead_code)]
use crate::app::AppMessage;
use crate::theme;
use cosmic::widget;
use cosmic::Element;
use openclaw_security::EventKind;

/// Inline chip widget displaying the event kind label with a matching colour.
pub fn event_chip(kind: &EventKind) -> Element<'static, AppMessage> {
    let (label, color) = match kind {
        EventKind::FileAccess          => ("File Read",    theme::COLOR_INFO),
        EventKind::FileWrite           => ("File Write",   theme::COLOR_INFO),
        EventKind::FileDelete          => ("File Delete",  theme::COLOR_FILE_DELETE),
        EventKind::NetworkRequest      => ("Network",      theme::COLOR_NETWORK),
        EventKind::ShellExec           => ("Shell Exec",   theme::COLOR_SHELL_EXEC),
        EventKind::ProcessSpawn        => ("Process",      theme::COLOR_SHELL_EXEC),
        EventKind::MemoryLimit         => ("Mem Limit",    theme::COLOR_DENY),
        EventKind::SandboxStart        => ("Sandbox Start",theme::COLOR_SYSTEM),
        EventKind::SandboxStop         => ("Sandbox Stop", theme::COLOR_SYSTEM),
        EventKind::PolicyDenied        => ("Denied",       theme::COLOR_DENY),
        EventKind::UserConfirmRequired => ("Pending",      theme::COLOR_PENDING),
        EventKind::GitPush             => ("Git Push",     theme::COLOR_SHELL_EXEC),
        EventKind::GitCommit           => ("Git Commit",   theme::COLOR_INFO),
        EventKind::GitBranchDelete     => ("Branch Del",   theme::COLOR_FILE_DELETE),
        EventKind::GitHubApiCall       => ("GitHub API",   theme::COLOR_NETWORK),
        EventKind::GitFetch            => ("Git Fetch",    theme::COLOR_INFO),
        EventKind::GitHistoryRewrite   => ("History RW",   theme::COLOR_DENY),
    };

    widget::text(label)
        .size(12)
        .class(cosmic::theme::Text::Color(color))
        .into()
}
