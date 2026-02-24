//! Skill risk registry.
//!
//! Maps OpenClaw Skill names to a [`RiskLevel`] so the policy engine can make
//! fast, consistent decisions without inspecting every argument.
//!
//! The registry is seeded with well-known built-in Skills and can be extended
//! at runtime via [`SkillRegistry::register`].

use std::collections::HashMap;

/// Risk classification for an OpenClaw Skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Safe to run without user interaction (read-only, no side-effects).
    Safe,
    /// Needs user confirmation before running.
    Confirm,
    /// Always blocked regardless of user input.
    Deny,
}

/// Maps Skill names to their [`RiskLevel`].
///
/// Skill names use dot-notation matching OpenClaw's convention:
/// `"<category>.<action>"` (e.g. `"shell.exec"`, `"fs.writeFile"`).
/// A trailing `".*"` acts as a category-level wildcard.
#[derive(Debug, Clone)]
pub struct SkillRegistry {
    exact: HashMap<String, RiskLevel>,
    /// Category-level wildcards stored as `"category"` (without the `.*`).
    wildcards: HashMap<String, RiskLevel>,
    /// Default level returned when no rule matches.
    default: RiskLevel,
}

impl SkillRegistry {
    /// Builds the registry pre-loaded with OpenClaw's known built-in Skills.
    pub fn with_defaults() -> Self {
        let mut r = Self {
            exact: HashMap::new(),
            wildcards: HashMap::new(),
            default: RiskLevel::Confirm,
        };

        // ── Always-safe Skills ────────────────────────────────────────────────
        // Read-only operations with no persistent side-effects.
        for name in [
            "agent.getContext",
            "agent.listSkills",
            "agent.getMemory",
            "fs.readFile",
            "fs.readDir",
            "fs.stat",
            "fs.exists",
            "web.fetch",          // network — but read-only; policy engine checks host
            "web.screenshot",
            "search.query",
            "search.web",
            "knowledge.query",
            "knowledge.retrieve",
            "calendar.list",
            "calendar.get",
            "email.list",
            "email.read",
            "security.getStatus",
            "security.listEvents",
        ] {
            r.exact.insert(name.to_string(), RiskLevel::Safe);
        }

        // ── Confirm-before-run Skills ─────────────────────────────────────────
        // Mutating or side-effecting operations that need human approval.
        for name in [
            "fs.writeFile",
            "fs.appendFile",
            "fs.mkdir",
            "fs.rename",
            "fs.copy",
            "web.click",
            "web.fill",
            "web.submit",
            "web.navigate",
            "email.send",
            "email.reply",
            "email.delete",
            "calendar.create",
            "calendar.update",
            "calendar.delete",
            "agent.setMemory",
            "agent.clearMemory",
            "security.updatePolicy",
            "plugin.install",
            "plugin.uninstall",
            "plugin.enable",
            "plugin.disable",
        ] {
            r.exact.insert(name.to_string(), RiskLevel::Confirm);
        }

        // ── Always-denied Skills ──────────────────────────────────────────────
        // High-risk operations blocked unconditionally.
        for name in [
            "shell.exec",
            "shell.spawn",
            "shell.sudo",
            "process.kill",
            "process.spawn",
            "fs.delete",
            "fs.unlink",
            "fs.rmdir",
            "fs.chmod",
            "fs.chown",
            "system.reboot",
            "system.shutdown",
            "system.update",
            "network.openPort",
            "network.proxy",
        ] {
            r.exact.insert(name.to_string(), RiskLevel::Deny);
        }

        // ── Category wildcards ────────────────────────────────────────────────
        // Applied when no exact match is found.
        r.wildcards.insert("shell".to_string(),   RiskLevel::Deny);
        r.wildcards.insert("process".to_string(), RiskLevel::Deny);
        r.wildcards.insert("system".to_string(),  RiskLevel::Deny);
        r.wildcards.insert("network".to_string(), RiskLevel::Confirm);
        r.wildcards.insert("fs".to_string(),      RiskLevel::Confirm);
        r.wildcards.insert("web".to_string(),     RiskLevel::Confirm);
        r.wildcards.insert("email".to_string(),   RiskLevel::Confirm);
        r.wildcards.insert("calendar".to_string(),RiskLevel::Confirm);
        r.wildcards.insert("plugin".to_string(),  RiskLevel::Confirm);

        r
    }

    /// Registers or overrides the risk level for a specific Skill name.
    ///
    /// Use `"category.*"` to set a category-level wildcard.
    #[allow(dead_code)]
    pub fn register(&mut self, skill_name: impl Into<String>, level: RiskLevel) {
        let name = skill_name.into();
        if let Some(category) = name.strip_suffix(".*") {
            self.wildcards.insert(category.to_string(), level);
        } else {
            self.exact.insert(name, level);
        }
    }

    /// Returns the [`RiskLevel`] for the given Skill name.
    ///
    /// Lookup order:
    /// 1. Exact match.
    /// 2. Category wildcard (the part before the first `.`).
    /// 3. Registry default (`Confirm`).
    pub fn risk_level(&self, skill_name: &str) -> RiskLevel {
        if let Some(&level) = self.exact.get(skill_name) {
            return level;
        }

        if let Some(category) = skill_name.split('.').next() {
            if let Some(&level) = self.wildcards.get(category) {
                return level;
            }
        }

        self.default
    }

    /// Returns `true` if the Skill is known to the registry (exact match only).
    #[allow(dead_code)]
    pub fn is_known(&self, skill_name: &str) -> bool {
        self.exact.contains_key(skill_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_wins() {
        let r = SkillRegistry::with_defaults();
        assert_eq!(r.risk_level("shell.exec"),    RiskLevel::Deny);
        assert_eq!(r.risk_level("fs.readFile"),   RiskLevel::Safe);
        assert_eq!(r.risk_level("fs.writeFile"),  RiskLevel::Confirm);
        assert_eq!(r.risk_level("email.send"),    RiskLevel::Confirm);
    }

    #[test]
    fn category_wildcard_fallback() {
        let r = SkillRegistry::with_defaults();
        // "shell.unknownAction" has no exact entry but category "shell" => Deny
        assert_eq!(r.risk_level("shell.unknownAction"), RiskLevel::Deny);
        // "network.unknownAction" => Confirm
        assert_eq!(r.risk_level("network.unknownAction"), RiskLevel::Confirm);
    }

    #[test]
    fn default_for_unknown() {
        let r = SkillRegistry::with_defaults();
        assert_eq!(r.risk_level("completely.unknown.skill"), RiskLevel::Confirm);
    }

    #[test]
    fn runtime_override() {
        let mut r = SkillRegistry::with_defaults();
        r.register("custom.dangerousOp", RiskLevel::Deny);
        assert_eq!(r.risk_level("custom.dangerousOp"), RiskLevel::Deny);
        // Wildcard override
        r.register("custom.*", RiskLevel::Safe);
        assert_eq!(r.risk_level("custom.safeOp"), RiskLevel::Safe);
    }
}
