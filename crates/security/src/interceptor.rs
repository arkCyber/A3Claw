use crate::audit::AuditLog;
use crate::policy::{PolicyDecision, PolicyEngine};
use crate::types::{ControlCommand, EventKind, ResourceKind, SandboxEvent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tracing::{info, warn};

/// The result of an interception check performed by the [`Interceptor`].
///
/// Returned synchronously to the WasmEdge host function caller so that
/// the sandbox can immediately allow or block the attempted operation.
#[derive(Debug)]
pub enum InterceptResult {
    /// The operation is permitted to proceed.
    Allow,
    /// The operation is blocked. The inner string is the denial reason.
    Deny(String),
}

/// Internal record of an operation awaiting human confirmation.
///
/// The `responder` channel is resolved by [`Interceptor::respond_to_confirmation`]
/// when the UI operator clicks Allow or Deny.
struct PendingConfirmation {
    /// The original event that triggered the confirmation request.
    event: SandboxEvent,
    /// One-shot channel used to deliver the user's decision back to the waiting task.
    responder: oneshot::Sender<bool>,
}

/// Central interception hub connecting the WasmEdge host functions,
/// the [`PolicyEngine`], the [`AuditLog`], and the UI event channel.
///
/// Every sensitive operation inside the sandbox passes through `Interceptor`
/// before being executed. The interceptor:
/// 1. Assigns a unique event ID and creates a [`SandboxEvent`].
/// 2. Asks the [`PolicyEngine`] for a decision.
/// 3. Records the outcome in the [`AuditLog`].
/// 4. Forwards the event to the UI via the `event_tx` channel.
/// 5. For operations requiring confirmation, blocks until the user responds
///    or a 30-second timeout elapses (human-in-the-loop).
pub struct Interceptor {
    /// Shared, async-safe policy engine.
    policy: Arc<Mutex<PolicyEngine>>,
    /// Append-only audit log.
    audit: Arc<AuditLog>,
    /// Channel used to push events to the monitoring UI.
    event_tx: flume::Sender<SandboxEvent>,
    /// Channel used to receive control commands from the UI (Allow/Deny/Terminate…).
    control_rx: Arc<Mutex<flume::Receiver<ControlCommand>>>,
    /// Queue of operations currently awaiting user confirmation.
    pending: Arc<Mutex<Vec<PendingConfirmation>>>,
    /// Monotonically increasing counter used to assign unique event IDs.
    event_counter: Arc<AtomicU64>,
}

impl Interceptor {
    /// Creates a new `Interceptor`.
    ///
    /// # Parameters
    /// - `policy`     — Configured policy engine.
    /// - `audit`      — Audit log sink.
    /// - `event_tx`   — Sender half of the UI event channel.
    /// - `control_rx` — Receiver half of the UI control channel.
    pub fn new(
        policy: PolicyEngine,
        audit: AuditLog,
        event_tx: flume::Sender<SandboxEvent>,
        control_rx: flume::Receiver<ControlCommand>,
    ) -> Self {
        Self {
            policy: Arc::new(Mutex::new(policy)),
            audit: Arc::new(audit),
            event_tx,
            control_rx: Arc::new(Mutex::new(control_rx)),
            pending: Arc::new(Mutex::new(Vec::new())),
            event_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Intercepts a file read operation.
    ///
    /// Called by the `ocplus.check_file_read` WasmEdge host function.
    pub async fn intercept_file_access(&self, path: &str) -> InterceptResult {
        self.intercept(
            EventKind::FileAccess,
            ResourceKind::File,
            Some(path.to_string()),
            format!("Read file: {}", path),
        )
        .await
    }

    /// Intercepts a file write operation.
    ///
    /// Called by the `ocplus.check_file_write` WasmEdge host function.
    pub async fn intercept_file_write(&self, path: &str) -> InterceptResult {
        self.intercept(
            EventKind::FileWrite,
            ResourceKind::File,
            Some(path.to_string()),
            format!("Write file: {}", path),
        )
        .await
    }

    /// Intercepts a file deletion operation.
    ///
    /// Called by the `ocplus.check_file_delete` WasmEdge host function.
    pub async fn intercept_file_delete(&self, path: &str) -> InterceptResult {
        self.intercept(
            EventKind::FileDelete,
            ResourceKind::File,
            Some(path.to_string()),
            format!("Delete file: {}", path),
        )
        .await
    }

    /// Intercepts an outbound network request.
    ///
    /// Called by the `ocplus.check_network` WasmEdge host function.
    ///
    /// # Parameters
    /// - `host` — The target hostname (used for allowlist lookup).
    /// - `url`  — The full URL (recorded in the audit log and event detail).
    pub async fn intercept_network(&self, host: &str, url: &str) -> InterceptResult {
        self.intercept(
            EventKind::NetworkRequest,
            ResourceKind::Network,
            Some(host.to_string()),
            format!("Network request: {}", url),
        )
        .await
    }

    /// Intercepts a shell command execution attempt.
    ///
    /// Automatically detects git commands and routes them to the appropriate
    /// git-specific interceptor for richer policy evaluation.
    ///
    /// Called by the `ocplus.check_shell` WasmEdge host function.
    pub async fn intercept_shell_exec(&self, command: &str) -> InterceptResult {
        let trimmed = command.trim();

        // Route git commands to specialised interceptors
        if trimmed.starts_with("git ") || trimmed == "git" {
            return self.intercept_git_command(command).await;
        }

        self.intercept(
            EventKind::ShellExec,
            ResourceKind::Process,
            None,
            command.to_string(),
        )
        .await
    }

    /// Routes a git command to the appropriate specialised interceptor.
    pub async fn intercept_git_command(&self, command: &str) -> InterceptResult {
        let tokens: Vec<&str> = command.split_whitespace().collect();
        let subcommand = tokens.get(1).copied().unwrap_or("");

        match subcommand {
            "push" => {
                // Detect branch delete via push: git push origin --delete <branch>
                let is_delete = tokens.iter().any(|t| *t == "--delete" || *t == "-d");
                if is_delete {
                    let branch = tokens.last().copied().unwrap_or("unknown");
                    self.intercept(
                        EventKind::GitBranchDelete,
                        ResourceKind::GitRepo,
                        Some(branch.to_string()),
                        command.to_string(),
                    ).await
                } else {
                    self.intercept(
                        EventKind::GitPush,
                        ResourceKind::GitRepo,
                        None,
                        command.to_string(),
                    ).await
                }
            }
            "commit" => {
                self.intercept(
                    EventKind::GitCommit,
                    ResourceKind::GitRepo,
                    None,
                    command.to_string(),
                ).await
            }
            "branch" => {
                let is_delete = tokens.iter().any(|t| *t == "-d" || *t == "-D" || *t == "--delete");
                if is_delete {
                    let branch = tokens.last().copied().unwrap_or("unknown");
                    self.intercept(
                        EventKind::GitBranchDelete,
                        ResourceKind::GitRepo,
                        Some(branch.to_string()),
                        command.to_string(),
                    ).await
                } else {
                    self.intercept(
                        EventKind::ShellExec,
                        ResourceKind::GitRepo,
                        None,
                        command.to_string(),
                    ).await
                }
            }
            "fetch" | "clone" | "pull" => {
                self.intercept(
                    EventKind::GitFetch,
                    ResourceKind::GitRepo,
                    None,
                    command.to_string(),
                ).await
            }
            "reset" | "rebase" | "filter-branch" | "filter-repo" => {
                self.intercept(
                    EventKind::GitHistoryRewrite,
                    ResourceKind::GitRepo,
                    None,
                    command.to_string(),
                ).await
            }
            _ => {
                // Generic git command — use standard shell evaluation
                self.intercept(
                    EventKind::ShellExec,
                    ResourceKind::GitRepo,
                    None,
                    command.to_string(),
                ).await
            }
        }
    }

    /// Intercepts a GitHub API call (REST or GraphQL).
    ///
    /// Called when the agent makes HTTP requests to api.github.com.
    pub async fn intercept_github_api(&self, method: &str, url: &str) -> InterceptResult {
        self.intercept(
            EventKind::GitHubApiCall,
            ResourceKind::GitHubRemote,
            Some(url.to_string()),
            format!("{} {}", method, url),
        )
        .await
    }

    /// Core interception logic shared by all public intercept methods.
    ///
    /// 1. Assigns a unique ID and constructs a [`SandboxEvent`].
    /// 2. Evaluates the event against the [`PolicyEngine`].
    /// 3. Records the outcome in the [`AuditLog`].
    /// 4. Forwards the event to the UI.
    /// 5. Blocks on user confirmation when the policy requires it.
    async fn intercept(
        &self,
        kind: EventKind,
        resource: ResourceKind,
        path: Option<String>,
        detail: String,
    ) -> InterceptResult {
        let id = self.event_counter.fetch_add(1, Ordering::SeqCst);
        let event = SandboxEvent::new(id, kind, resource, path, detail);

        // Evaluate against the policy engine.
        let decision = {
            let policy = self.policy.lock().await;
            policy.evaluate(&event)
        };

        match decision {
            PolicyDecision::Allow => {
                let mut ev = event.clone();
                ev.allowed = Some(true);
                let _ = self.event_tx.send_async(ev.clone()).await;
                self.audit.record(ev, "Allow", None).await;
                InterceptResult::Allow
            }
            PolicyDecision::Deny(reason) => {
                warn!("Policy denied: {} — {}", event.kind, reason);
                let mut ev = event.clone();
                ev.allowed = Some(false);
                let _ = self.event_tx.send_async(ev.clone()).await;
                self.audit
                    .record(ev, &format!("Deny: {}", reason), None)
                    .await;
                InterceptResult::Deny(reason)
            }
            PolicyDecision::RequireConfirmation(prompt) => {
                self.request_user_confirmation(event, prompt).await
            }
        }
    }

    /// Suspends the current operation and waits for the UI operator to
    /// approve or reject it (human-in-the-loop confirmation).
    ///
    /// The event is forwarded to the UI as [`EventKind::UserConfirmRequired`].
    /// If no response arrives within **30 seconds**, the operation is
    /// automatically denied to prevent indefinite blocking.
    async fn request_user_confirmation(
        &self,
        event: SandboxEvent,
        _prompt: String,
    ) -> InterceptResult {
        let (tx, rx) = oneshot::channel::<bool>();

        // Notify the UI that confirmation is needed.
        let mut confirm_event = event.clone();
        confirm_event.kind = EventKind::UserConfirmRequired;
        confirm_event.allowed = None;
        let _ = self.event_tx.send_async(confirm_event).await;

        // Register the pending confirmation so the UI can resolve it.
        {
            let mut pending = self.pending.lock().await;
            pending.push(PendingConfirmation {
                event: event.clone(),
                responder: tx,
            });
        }

        // Block until the user responds or the timeout elapses.
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(true)) => {
                info!("User approved operation: {}", event.detail);
                self.audit
                    .record(event, "Allow", Some("Approved by user"))
                    .await;
                InterceptResult::Allow
            }
            Ok(Ok(false)) | Ok(Err(_)) => {
                warn!("User denied operation: {}", event.detail);
                self.audit
                    .record(event, "Deny", Some("Denied by user"))
                    .await;
                InterceptResult::Deny("Operation denied by user.".to_string())
            }
            Err(_) => {
                warn!("Confirmation timed out, auto-denying: {}", event.detail);
                self.audit
                    .record(event, "Deny", Some("Confirmation timed out (30 s)"))
                    .await;
                InterceptResult::Deny(
                    "Confirmation timed out (30 s) — operation automatically denied.".to_string(),
                )
            }
        }
    }

    /// Resolves a pending confirmation for the event with the given `event_id`.
    ///
    /// Should be called by the UI message handler when the operator clicks
    /// **Allow** or **Deny** on a confirmation card.
    ///
    /// # Parameters
    /// - `event_id` — The [`SandboxEvent::id`] of the operation to resolve.
    /// - `allowed`  — `true` to allow, `false` to deny.
    pub async fn respond_to_confirmation(&self, event_id: u64, allowed: bool) {
        let mut pending = self.pending.lock().await;
        if let Some(pos) = pending.iter().position(|p| p.event.id == event_id) {
            let confirmation = pending.remove(pos);
            let _ = confirmation.responder.send(allowed);
        }
    }

    /// Spawns a background task that drains `control_rx` and routes each
    /// [`ControlCommand`] to the appropriate handler.
    ///
    /// - `Allow(id)` / `Deny(id)` — resolve the pending confirmation for `id`.
    /// - `Terminate` — sends a deny to every pending confirmation and exits.
    /// - `Pause` / `Resume` / `UpdatePolicy` — logged; full implementation
    ///   requires sandbox-runner cooperation (future work).
    ///
    /// Call this once after constructing the `Interceptor`.
    pub fn start_control_loop(self: &Arc<Self>) {
        let interceptor = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                let cmd = {
                    let rx = interceptor.control_rx.lock().await;
                    match rx.recv_async().await {
                        Ok(cmd) => cmd,
                        Err(_) => break, // sender dropped — shut down
                    }
                };

                match cmd {
                    ControlCommand::Allow(id) => {
                        interceptor.respond_to_confirmation(id, true).await;
                    }
                    ControlCommand::Deny(id) => {
                        interceptor.respond_to_confirmation(id, false).await;
                    }
                    ControlCommand::Terminate => {
                        // Deny every pending confirmation and stop the loop.
                        let mut pending = interceptor.pending.lock().await;
                        for p in pending.drain(..) {
                            let _ = p.responder.send(false);
                        }
                        break;
                    }
                    ControlCommand::Pause => {
                        info!("Sandbox pause requested (not yet implemented in embedded mode)");
                    }
                    ControlCommand::Resume => {
                        info!("Sandbox resume requested (not yet implemented in embedded mode)");
                    }
                    ControlCommand::UpdatePolicy(toml) => {
                        info!(bytes = toml.len(), "Policy update received via control channel");
                    }
                }
            }
        });
    }

    /// Returns a clone of the event sender channel.
    ///
    /// Used by the sandbox runner to emit lifecycle events (start/stop)
    /// directly without going through the interception pipeline.
    pub fn event_sender(&self) -> flume::Sender<SandboxEvent> {
        self.event_tx.clone()
    }
}
