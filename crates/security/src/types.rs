use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// A single event emitted by the WasmEdge sandbox host and forwarded to the UI layer.
///
/// Every sensitive operation attempted by OpenClaw (file I/O, network requests,
/// shell execution) is captured as a `SandboxEvent` and routed through the
/// [`crate::interceptor::Interceptor`] before being sent to the monitoring UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxEvent {
    /// Monotonically increasing event identifier within a sandbox session.
    pub id: u64,
    /// Unix timestamp (seconds) at which the event was captured.
    pub timestamp: u64,
    /// The category of operation that triggered this event.
    pub kind: EventKind,
    /// The type of system resource involved in the operation.
    pub resource: ResourceKind,
    /// Optional file path, network host, or other target identifier.
    pub path: Option<String>,
    /// Human-readable description of the operation (e.g. the full command string).
    pub detail: String,
    /// Policy decision: `Some(true)` = allowed, `Some(false)` = denied, `None` = pending user confirmation.
    pub allowed: Option<bool>,
}

impl SandboxEvent {
    /// Creates a new `SandboxEvent` with the current Unix timestamp.
    ///
    /// `allowed` is initialised to `None`; the [`crate::interceptor::Interceptor`]
    /// sets it after the policy decision is made.
    pub fn new(
        id: u64,
        kind: EventKind,
        resource: ResourceKind,
        path: Option<String>,
        detail: impl Into<String>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id,
            timestamp,
            kind,
            resource,
            path,
            detail: detail.into(),
            allowed: None,
        }
    }
}

/// Classifies the type of operation that produced a [`SandboxEvent`].
///
/// Used by the [`crate::policy::PolicyEngine`] to select the appropriate
/// evaluation branch and by the UI to colour-code event rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    /// Read access to a file or directory entry.
    FileAccess,
    /// Write or append to a file.
    FileWrite,
    /// Deletion of a file or directory.
    FileDelete,
    /// Outbound network connection or HTTP/HTTPS request.
    NetworkRequest,
    /// Execution of a shell command via `exec`, `spawn`, or `child_process`.
    ShellExec,
    /// Spawning of a new child process.
    ProcessSpawn,
    /// Sandbox memory usage exceeded the configured limit.
    MemoryLimit,
    /// The sandbox session has started successfully.
    SandboxStart,
    /// The sandbox session has terminated (normally or forcefully).
    SandboxStop,
    /// An operation was denied by the policy engine without user interaction.
    PolicyDenied,
    /// The operation is paused and waiting for explicit user approval or denial.
    UserConfirmRequired,
    /// A git push operation (may be force-push).
    GitPush,
    /// A git commit operation.
    GitCommit,
    /// A git branch deletion.
    GitBranchDelete,
    /// A GitHub API call (REST or GraphQL).
    GitHubApiCall,
    /// A git clone or fetch from a remote.
    GitFetch,
    /// A git reset or rebase that rewrites history.
    GitHistoryRewrite,
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventKind::FileAccess          => write!(f, "File Read"),
            EventKind::FileWrite           => write!(f, "File Write"),
            EventKind::FileDelete          => write!(f, "File Delete"),
            EventKind::NetworkRequest      => write!(f, "Network Request"),
            EventKind::ShellExec           => write!(f, "Shell Exec"),
            EventKind::ProcessSpawn        => write!(f, "Process Spawn"),
            EventKind::MemoryLimit         => write!(f, "Memory Limit"),
            EventKind::SandboxStart        => write!(f, "Sandbox Start"),
            EventKind::SandboxStop         => write!(f, "Sandbox Stop"),
            EventKind::PolicyDenied        => write!(f, "Policy Denied"),
            EventKind::UserConfirmRequired => write!(f, "Awaiting Confirmation"),
            EventKind::GitPush             => write!(f, "Git Push"),
            EventKind::GitCommit           => write!(f, "Git Commit"),
            EventKind::GitBranchDelete     => write!(f, "Git Branch Delete"),
            EventKind::GitHubApiCall       => write!(f, "GitHub API Call"),
            EventKind::GitFetch            => write!(f, "Git Fetch"),
            EventKind::GitHistoryRewrite   => write!(f, "Git History Rewrite"),
        }
    }
}

/// Identifies the category of system resource targeted by an operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceKind {
    /// A regular file.
    File,
    /// A filesystem directory.
    Directory,
    /// A network socket or remote host.
    Network,
    /// An OS process.
    Process,
    /// Heap / virtual memory.
    Memory,
    /// A system-level resource (e.g. sandbox lifecycle).
    System,
    /// A Git repository.
    GitRepo,
    /// A GitHub remote resource (API, PR, issue, etc.).
    GitHubRemote,
}

impl std::fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceKind::File      => write!(f, "File"),
            ResourceKind::Directory => write!(f, "Directory"),
            ResourceKind::Network   => write!(f, "Network"),
            ResourceKind::Process   => write!(f, "Process"),
            ResourceKind::Memory        => write!(f, "Memory"),
            ResourceKind::System        => write!(f, "System"),
            ResourceKind::GitRepo       => write!(f, "Git Repo"),
            ResourceKind::GitHubRemote  => write!(f, "GitHub Remote"),
        }
    }
}

/// Commands sent from the UI process to the sandbox process over the IPC channel.
///
/// These commands allow the operator to respond to pending confirmations,
/// pause/resume execution, or forcefully terminate the sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    /// Allow the pending operation identified by the given event ID.
    Allow(u64),
    /// Deny the pending operation identified by the given event ID.
    Deny(u64),
    /// Pause sandbox execution (future operations will queue until resumed).
    Pause,
    /// Resume a previously paused sandbox.
    Resume,
    /// Forcefully terminate the WasmEdge instance immediately.
    Terminate,
    /// Replace the active security policy with the provided TOML string.
    UpdatePolicy(String),
}
