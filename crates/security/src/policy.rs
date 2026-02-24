use crate::config::SecurityConfig;
use crate::types::{EventKind, SandboxEvent};
use std::collections::HashSet;
use tracing::{info, warn};

/// Parsed representation of a git command for policy evaluation.
#[derive(Debug)]
#[allow(dead_code)]
struct GitCommand<'a> {
    subcommand: &'a str,
    args: Vec<&'a str>,
    is_force: bool,
    remote: Option<&'a str>,
    branch: Option<&'a str>,
}

/// The outcome of evaluating a [`SandboxEvent`] against the active security policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// The operation is permitted to proceed without user interaction.
    Allow,
    /// The operation is blocked. The inner string describes the reason.
    Deny(String),
    /// The operation is suspended until the user explicitly approves or rejects it.
    /// The inner string is the confirmation prompt shown in the UI.
    RequireConfirmation(String),
}

/// Stateful policy engine that evaluates [`SandboxEvent`]s against a
/// [`SecurityConfig`] and returns a [`PolicyDecision`].
///
/// The engine enforces three layers of protection:
/// 1. **Hard-coded rules** — sensitive paths and dangerous commands are always denied.
/// 2. **Per-session overrides** — paths added via [`add_permanent_allow`] /
///    [`add_permanent_deny`] bypass the normal evaluation for the current session.
/// 3. **Configurable confirmation** — operations that pass the hard rules but are
///    flagged by the config (e.g. file deletion) are escalated to the user.
pub struct PolicyEngine {
    config: SecurityConfig,
    /// Paths the user has chosen to always allow for this session.
    permanent_allow_paths: HashSet<String>,
    /// Paths the user has chosen to always deny for this session.
    permanent_deny_paths: HashSet<String>,
}

impl PolicyEngine {
    /// Creates a new `PolicyEngine` with the given configuration and empty
    /// per-session override sets.
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            permanent_allow_paths: HashSet::new(),
            permanent_deny_paths: HashSet::new(),
        }
    }

    /// Evaluates a sandbox event and returns the appropriate policy decision.
    ///
    /// Dispatches to a specialised evaluator based on [`EventKind`].
    pub fn evaluate(&self, event: &SandboxEvent) -> PolicyDecision {
        match &event.kind {
            EventKind::FileDelete => self.evaluate_file_delete(event),
            EventKind::FileWrite => self.evaluate_file_write(event),
            EventKind::FileAccess => self.evaluate_file_access(event),
            EventKind::NetworkRequest => self.evaluate_network(event),
            EventKind::ShellExec | EventKind::ProcessSpawn => self.evaluate_shell(event),
            EventKind::MemoryLimit => {
                PolicyDecision::Deny("Memory usage exceeded the configured limit.".to_string())
            }
            EventKind::GitPush => self.evaluate_git_push(event),
            EventKind::GitCommit => self.evaluate_git_commit(event),
            EventKind::GitBranchDelete => self.evaluate_git_branch_delete(event),
            EventKind::GitHubApiCall => self.evaluate_github_api(event),
            EventKind::GitFetch => self.evaluate_git_fetch(event),
            EventKind::GitHistoryRewrite => self.evaluate_git_history_rewrite(event),
            _ => PolicyDecision::Allow,
        }
    }

    /// Evaluates a file deletion attempt.
    ///
    /// - Paths in the permanent-deny set are always blocked.
    /// - Paths in the permanent-allow set are always permitted.
    /// - When `folder_access` whitelist is non-empty, the path must be inside
    ///   a whitelisted folder that has `allow_delete = true`.
    /// - Paths outside the sandbox workspace are unconditionally denied.
    fn evaluate_file_delete(&self, event: &SandboxEvent) -> PolicyDecision {
        let path = event.path.as_deref().unwrap_or("");

        if self.permanent_deny_paths.contains(path) {
            return PolicyDecision::Deny(format!("Path '{}' is in the permanent-deny list.", path));
        }
        if self.permanent_allow_paths.contains(path) {
            return PolicyDecision::Allow;
        }

        // Folder whitelist check (when whitelist is configured).
        if !self.config.folder_access.is_empty() {
            let p = std::path::Path::new(path);
            match self.config.folder_access.iter().find(|fa| fa.contains(p)) {
                None => {
                    warn!("Denied file delete — path not in folder whitelist: {}", path);
                    return PolicyDecision::Deny(format!(
                        "Access denied: '{}' is not inside any authorised folder.",
                        path
                    ));
                }
                Some(fa) if !fa.allow_delete => {
                    return PolicyDecision::Deny(format!(
                        "Delete not permitted in folder '{}' ({}). Enable delete access in Settings → Folder Access.",
                        fa.host_path.display(), fa.label
                    ));
                }
                Some(_) => {}
            }
        } else if !self.is_within_workspace(path) {
            // Legacy behaviour when no whitelist is configured.
            warn!("Denied file deletion outside sandbox workspace: {}", path);
            return PolicyDecision::Deny(format!(
                "Security policy: deletion outside the sandbox workspace is not allowed — '{}'",
                path
            ));
        }

        if self.config.confirm_file_delete {
            PolicyDecision::RequireConfirmation(format!(
                "OpenClaw wants to delete:\n{}\n\nAllow this operation?",
                path
            ))
        } else {
            PolicyDecision::Allow
        }
    }

    /// Evaluates a file write attempt.
    ///
    /// When `folder_access` whitelist is non-empty, the path must be inside
    /// a whitelisted folder that has `allow_write = true`.
    fn evaluate_file_write(&self, event: &SandboxEvent) -> PolicyDecision {
        let path = event.path.as_deref().unwrap_or("");

        if !self.config.folder_access.is_empty() {
            let p = std::path::Path::new(path);
            match self.config.folder_access.iter().find(|fa| fa.contains(p)) {
                None => {
                    warn!("Denied file write — path not in folder whitelist: {}", path);
                    return PolicyDecision::Deny(format!(
                        "Access denied: '{}' is not inside any authorised folder.",
                        path
                    ));
                }
                Some(fa) if !fa.allow_write => {
                    return PolicyDecision::Deny(format!(
                        "Write not permitted in folder '{}' ({}). Enable write access in Settings → Folder Access.",
                        fa.host_path.display(), fa.label
                    ));
                }
                Some(fa) if !fa.extension_allowed(p) => {
                    return PolicyDecision::Deny(format!(
                        "File extension not allowed in folder '{}'. Allowed: [{}]",
                        fa.label,
                        fa.allowed_extensions.join(", ")
                    ));
                }
                Some(_) => {}
            }
        } else if !self.is_within_workspace(path) {
            warn!("Denied file write outside sandbox workspace: {}", path);
            return PolicyDecision::Deny(format!(
                "Security policy: writing outside the sandbox workspace is not allowed — '{}'",
                path
            ));
        }

        PolicyDecision::Allow
    }

    /// Evaluates a file read attempt.
    ///
    /// - Sensitive paths (SSH keys, credentials, etc.) are always denied.
    /// - When `folder_access` whitelist is non-empty, the path must be inside
    ///   a whitelisted folder (read is always allowed for whitelisted folders).
    fn evaluate_file_access(&self, event: &SandboxEvent) -> PolicyDecision {
        let path = event.path.as_deref().unwrap_or("");

        // Hard-coded sensitive path list — always denied regardless of whitelist.
        let sensitive_paths = [
            "/etc/passwd",
            "/etc/shadow",
            "/etc/sudoers",
            ".ssh/",
            ".gnupg/",
            ".aws/credentials",
            ".env",
            "id_rsa",
            "id_ed25519",
        ];
        for sensitive in &sensitive_paths {
            if path.contains(sensitive) {
                warn!("Denied access to sensitive path: {}", path);
                return PolicyDecision::Deny(format!(
                    "Security policy: access to sensitive path '{}' is not allowed.",
                    path
                ));
            }
        }

        // Folder whitelist check (when whitelist is configured).
        if !self.config.folder_access.is_empty() {
            let p = std::path::Path::new(path);
            match self.config.folder_access.iter().find(|fa| fa.contains(p)) {
                None => {
                    warn!("Denied file read — path not in folder whitelist: {}", path);
                    return PolicyDecision::Deny(format!(
                        "Access denied: '{}' is not inside any authorised folder. \
                         Add the folder in Settings → Folder Access.",
                        path
                    ));
                }
                Some(fa) if !fa.extension_allowed(p) => {
                    return PolicyDecision::Deny(format!(
                        "File extension not allowed in folder '{}'. Allowed: [{}]",
                        fa.label,
                        fa.allowed_extensions.join(", ")
                    ));
                }
                Some(_) => {}
            }
        }

        PolicyDecision::Allow
    }

    /// Evaluates an outbound network request.
    ///
    /// - Hosts in the allowlist are permitted immediately.
    /// - Unknown hosts are denied, or escalated for confirmation when
    ///   `confirm_network` is enabled.
    fn evaluate_network(&self, event: &SandboxEvent) -> PolicyDecision {
        let host = event.path.as_deref().unwrap_or("");

        if self.config.is_network_allowed(host) {
            info!("Network request allowed: {}", host);
            return PolicyDecision::Allow;
        }

        if self.config.confirm_network {
            PolicyDecision::RequireConfirmation(format!(
                "OpenClaw wants to connect to an unknown host:\n{}\n\nAllow this network request?",
                host
            ))
        } else {
            warn!("Denied unauthorised network request to: {}", host);
            PolicyDecision::Deny(format!(
                "Security policy: host '{}' is not in the network allowlist.",
                host
            ))
        }
    }

    /// Evaluates a shell command execution attempt.
    ///
    /// - If `intercept_shell` is disabled, all commands are allowed.
    /// - Commands matching the hard-coded dangerous-command list are always denied.
    /// - All other commands are escalated for confirmation when `confirm_shell_exec`
    ///   is enabled.
    fn evaluate_shell(&self, event: &SandboxEvent) -> PolicyDecision {
        if !self.config.intercept_shell {
            return PolicyDecision::Allow;
        }

        let cmd = &event.detail;

        // Hard-coded dangerous command patterns — always denied.
        let dangerous_exact = [
            "rm -rf /",
            "rm -rf ~",
            "mkfs",
            "dd if=",
            "chmod 777 /",
            "> /dev/sda",
            ":(){ :|:& };:",
            "bash <(",
        ];

        for dangerous in &dangerous_exact {
            if cmd.contains(dangerous) {
                warn!("Denied dangerous shell command: {}", cmd);
                return PolicyDecision::Deny(format!(
                    "Security policy: dangerous command detected and automatically denied:\n{}",
                    cmd
                ));
            }
        }

        // Detect pipe-to-shell patterns: `curl <url> | sh`, `wget <url> | bash`, etc.
        let pipe_to_shell = cmd.contains("| sh") || cmd.contains("| bash")
            || cmd.contains("|sh") || cmd.contains("|bash");
        let fetcher = cmd.trim_start().starts_with("curl")
            || cmd.trim_start().starts_with("wget")
            || cmd.trim_start().starts_with("fetch");
        if pipe_to_shell && fetcher {
            warn!("Denied pipe-to-shell command: {}", cmd);
            return PolicyDecision::Deny(format!(
                "Security policy: dangerous pipe-to-shell command detected and automatically denied:\n{}",
                cmd
            ));
        }

        if self.config.confirm_shell_exec {
            PolicyDecision::RequireConfirmation(format!(
                "OpenClaw wants to run a shell command:\n\n```\n{}\n```\n\nAllow execution?",
                cmd
            ))
        } else {
            PolicyDecision::Allow
        }
    }

    /// Returns `true` if `path` is considered to be inside the sandbox workspace.
    ///
    /// Paths starting with `/workspace` (the WASI guest mount point), relative
    /// paths (`./`), or paths without a leading `/` or `~` are treated as safe.
    fn is_within_workspace(&self, path: &str) -> bool {
        path.starts_with("/workspace")
            || path.starts_with("./")
            || (!path.starts_with('/') && !path.starts_with('~'))
    }

    /// Adds `path` to the per-session permanent-allow set.
    ///
    /// Future file-delete evaluations for this exact path will return
    /// [`PolicyDecision::Allow`] without prompting the user.
    pub fn add_permanent_allow(&mut self, path: String) {
        self.permanent_allow_paths.insert(path);
    }

    /// Adds `path` to the per-session permanent-deny set.
    ///
    /// Future evaluations for this exact path will return
    /// [`PolicyDecision::Deny`] without prompting the user.
    pub fn add_permanent_deny(&mut self, path: String) {
        self.permanent_deny_paths.insert(path);
    }

    /// Returns a reference to the active [`SecurityConfig`].
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Replaces the active configuration with `config`.
    ///
    /// Per-session override sets are preserved across config updates.
    pub fn update_config(&mut self, config: SecurityConfig) {
        self.config = config;
    }

    // ── Git / GitHub evaluators ───────────────────────────────────────────────

    /// Parses a git command string into a structured [`GitCommand`].
    fn parse_git_command<'a>(cmd: &'a str) -> Option<GitCommand<'a>> {
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        // Must start with "git"
        let start = tokens.iter().position(|t| *t == "git")?;
        let rest = &tokens[start + 1..];
        if rest.is_empty() {
            return None;
        }
        let subcommand = rest[0];
        let args: Vec<&str> = rest[1..].to_vec();
        let is_force = args.iter().any(|a| *a == "--force" || *a == "-f" || *a == "--force-with-lease");

        // Extract remote and branch from push/fetch args
        let (remote, branch) = if args.len() >= 2 && !args[0].starts_with('-') {
            (Some(args[0]), Some(args[1]))
        } else if args.len() == 1 && !args[0].starts_with('-') {
            (Some(args[0]), None)
        } else {
            (None, None)
        };

        Some(GitCommand { subcommand, args, is_force, remote, branch })
    }

    /// Checks if a branch name is a default/protected branch.
    fn is_default_branch(branch: &str) -> bool {
        matches!(branch, "main" | "master" | "develop" | "dev" | "production" | "prod" | "release")
    }

    /// Checks if a GitHub org/repo is in the allowed list.
    fn is_github_repo_allowed(&self, repo_path: &str) -> bool {
        let gh = &self.config.github;
        // If both lists are empty, allow all
        if gh.allowed_orgs.is_empty() && gh.allowed_repos.is_empty() {
            return true;
        }
        // Check exact repo match first
        if gh.allowed_repos.iter().any(|r| repo_path.starts_with(r.as_str())) {
            return true;
        }
        // Check org prefix
        let org = repo_path.split('/').next().unwrap_or("");
        gh.allowed_orgs.iter().any(|o| o == org)
    }

    /// Evaluates a `git push` command.
    ///
    /// - Force push is always denied when `deny_force_push` is set.
    /// - Pushes to default/protected branches require confirmation when
    ///   `protect_default_branch` is set.
    /// - All other pushes require confirmation when `confirm_push` is set.
    fn evaluate_git_push(&self, event: &SandboxEvent) -> PolicyDecision {
        let gh = &self.config.github;
        if !gh.intercept_push {
            return PolicyDecision::Allow;
        }

        let cmd = event.detail.as_str();
        let parsed = Self::parse_git_command(cmd);

        // Detect force push
        let is_force = parsed.as_ref().map(|p| p.is_force).unwrap_or_else(|| {
            cmd.contains("--force") || cmd.contains(" -f ") || cmd.contains("--force-with-lease")
        });

        if is_force && gh.deny_force_push {
            warn!("Denied force push: {}", cmd);
            return PolicyDecision::Deny(format!(
                "🚫 Security policy: force push is not allowed.\n\
                 Command: `{}`\n\n\
                 Force pushing rewrites remote history and can cause data loss for collaborators.\n\
                 Disable `github.deny_force_push` in settings to allow this.",
                cmd
            ));
        }

        // Check branch protection
        if gh.protect_default_branch {
            let branch = parsed.as_ref().and_then(|p| p.branch);
            let branch_from_detail = event.path.as_deref().unwrap_or("");
            let target_branch = branch.unwrap_or(branch_from_detail);

            if Self::is_default_branch(target_branch) {
                return PolicyDecision::RequireConfirmation(format!(
                    "⚠️  AI Agent wants to push to protected branch: **{}**\n\n\
                     Command: `{}`\n\n\
                     Pushing to the default branch can affect all collaborators.\n\
                     Allow this push?",
                    target_branch, cmd
                ));
            }
        }

        if gh.confirm_push {
            let remote = parsed.as_ref().and_then(|p| p.remote).unwrap_or("origin");
            PolicyDecision::RequireConfirmation(format!(
                "📤 AI Agent wants to push code to remote: **{}**\n\n\
                 Command: `{}`\n\n\
                 Allow this push?",
                remote, cmd
            ))
        } else {
            info!("Git push allowed: {}", cmd);
            PolicyDecision::Allow
        }
    }

    /// Evaluates a `git commit` command.
    ///
    /// Checks `max_files_per_commit` threshold if configured.
    fn evaluate_git_commit(&self, event: &SandboxEvent) -> PolicyDecision {
        let gh = &self.config.github;
        let cmd = &event.detail;

        // Check file count from detail (format: "git commit — N files changed")
        if let Some(max) = gh.max_files_per_commit {
            // Parse "N files" from detail string
            let file_count: Option<u32> = cmd
                .split_whitespace()
                .zip(cmd.split_whitespace().skip(1))
                .find(|(_, next)| next.starts_with("file"))
                .and_then(|(n, _)| n.parse().ok());

            if let Some(count) = file_count {
                if count > max {
                    return PolicyDecision::RequireConfirmation(format!(
                        "⚠️  AI Agent wants to commit **{} files** (limit: {}).\n\n\
                         Command: `{}`\n\n\
                         Large commits may include unintended changes. Allow?",
                        count, max, cmd
                    ));
                }
            }
        }

        info!("Git commit allowed: {}", cmd);
        PolicyDecision::Allow
    }

    /// Evaluates a `git branch -d` / `git push origin --delete` command.
    fn evaluate_git_branch_delete(&self, event: &SandboxEvent) -> PolicyDecision {
        let gh = &self.config.github;
        let cmd = &event.detail;
        let branch = event.path.as_deref().unwrap_or("unknown");

        if Self::is_default_branch(branch) {
            warn!("Denied deletion of default branch: {}", branch);
            return PolicyDecision::Deny(format!(
                "🚫 Security policy: deletion of default branch '{}' is not allowed.\n\
                 Command: `{}`",
                branch, cmd
            ));
        }

        if gh.confirm_branch_delete {
            PolicyDecision::RequireConfirmation(format!(
                "🗑️  AI Agent wants to delete branch: **{}**\n\n\
                 Command: `{}`\n\n\
                 This will permanently remove the branch from the remote. Allow?",
                branch, cmd
            ))
        } else {
            PolicyDecision::Allow
        }
    }

    /// Evaluates a GitHub API call (REST or GraphQL).
    ///
    /// Checks the target org/repo against the allowlist.
    fn evaluate_github_api(&self, event: &SandboxEvent) -> PolicyDecision {
        let gh = &self.config.github;
        if !gh.intercept_github_api {
            return PolicyDecision::Allow;
        }

        let url = event.path.as_deref().unwrap_or("");
        let detail = &event.detail;

        // Extract org/repo from GitHub API URL: /repos/{owner}/{repo}/...
        let repo_path = url
            .trim_start_matches("https://api.github.com/repos/")
            .trim_start_matches("https://api.github.com/");

        if !self.is_github_repo_allowed(repo_path) {
            warn!("Denied GitHub API call to non-allowed repo: {}", url);
            return PolicyDecision::Deny(format!(
                "🚫 Security policy: GitHub API call to '{}' is not in the allowed organisations/repos list.\n\
                 Configure `github.allowed_orgs` or `github.allowed_repos` in settings.",
                url
            ));
        }

        // Detect destructive API operations
        let is_destructive = detail.contains("DELETE")
            || detail.contains("delete")
            || detail.contains("dismiss")
            || detail.contains("close");

        if is_destructive {
            return PolicyDecision::RequireConfirmation(format!(
                "⚠️  AI Agent wants to perform a destructive GitHub API operation:\n\n\
                 `{}`\n\nURL: {}\n\nAllow this operation?",
                detail, url
            ));
        }

        info!("GitHub API call allowed: {}", url);
        PolicyDecision::Allow
    }

    /// Evaluates a `git fetch` or `git clone` command.
    fn evaluate_git_fetch(&self, event: &SandboxEvent) -> PolicyDecision {
        let cmd = &event.detail;
        // Clone from non-github sources may be suspicious
        let is_suspicious = cmd.contains("http://")  // non-HTTPS
            || (cmd.contains("github.com") && !self.is_github_repo_allowed(
                cmd.split("github.com/").nth(1).unwrap_or("")
            ));

        if is_suspicious {
            return PolicyDecision::RequireConfirmation(format!(
                "⚠️  AI Agent wants to fetch/clone from a remote:\n\n\
                 `{}`\n\nAllow this operation?",
                cmd
            ));
        }

        info!("Git fetch allowed: {}", cmd);
        PolicyDecision::Allow
    }

    /// Evaluates a `git reset --hard`, `git rebase`, or `git filter-branch` command.
    fn evaluate_git_history_rewrite(&self, event: &SandboxEvent) -> PolicyDecision {
        let gh = &self.config.github;
        let cmd = &event.detail;

        // filter-branch and filter-repo are always dangerous
        if cmd.contains("filter-branch") || cmd.contains("filter-repo") {
            warn!("Denied history rewrite with filter-branch/filter-repo: {}", cmd);
            return PolicyDecision::Deny(format!(
                "🚫 Security policy: `git filter-branch` and `git filter-repo` are not allowed.\n\
                 These commands permanently rewrite repository history.\n\
                 Command: `{}`",
                cmd
            ));
        }

        if gh.confirm_history_rewrite {
            PolicyDecision::RequireConfirmation(format!(
                "⚠️  AI Agent wants to rewrite git history:\n\n\
                 `{}`\n\n\
                 This operation modifies commit history and cannot be easily undone.\n\
                 Allow this operation?",
                cmd
            ))
        } else {
            PolicyDecision::Allow
        }
    }
}
