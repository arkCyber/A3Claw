//! Canonical Skill definitions — aligned 1:1 with SkillRegistry risk table.
//!
//! Every skill has a name (dot-notation), category, description, parameter
//! schema, and risk classification.  This is the single source of truth for
//! what a Digital Worker can do.

use serde::{Deserialize, Serialize};

// ── Risk level (mirrors SkillRegistry) ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillRisk {
    Safe,
    Confirm,
    Deny,
}

impl std::fmt::Display for SkillRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillRisk::Safe    => write!(f, "Safe"),
            SkillRisk::Confirm => write!(f, "Confirm"),
            SkillRisk::Deny    => write!(f, "Deny"),
        }
    }
}

// ── Skill category ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    FileSystem,
    Web,
    Search,
    Knowledge,
    Email,
    Calendar,
    Shell,
    Agent,
    Security,
    Plugin,
    Network,
    Custom(String),
}

impl std::fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillCategory::FileSystem  => write!(f, "File System"),
            SkillCategory::Web         => write!(f, "Web / Browser"),
            SkillCategory::Search      => write!(f, "Search"),
            SkillCategory::Knowledge   => write!(f, "Knowledge / RAG"),
            SkillCategory::Email       => write!(f, "Email"),
            SkillCategory::Calendar    => write!(f, "Calendar"),
            SkillCategory::Shell       => write!(f, "Shell"),
            SkillCategory::Agent       => write!(f, "Agent"),
            SkillCategory::Security    => write!(f, "Security"),
            SkillCategory::Plugin      => write!(f, "Plugin"),
            SkillCategory::Network     => write!(f, "Network"),
            SkillCategory::Custom(s)   => write!(f, "Custom({})", s),
        }
    }
}

// ── Parameter definition ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SkillParam {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub param_type: &'static str,
}

// ── Single Skill definition ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    /// Dot-notation name matching SkillRegistry (e.g. `"web.fetch"`).
    pub name: &'static str,
    /// Human-readable display name.
    pub display: &'static str,
    /// Short description of what the skill does.
    pub description: &'static str,
    /// Skill category.
    pub category: SkillCategory,
    /// Risk classification.
    pub risk: SkillRisk,
    /// Parameter definitions for LLM tool-call schema generation.
    pub params: &'static [SkillParam],
}

impl Skill {
    /// Returns true if this skill can be added to an agent's capability set.
    pub fn is_grantable(&self) -> bool {
        self.risk != SkillRisk::Deny
    }

    /// Generates a JSON Schema fragment for LLM function-calling.
    pub fn to_tool_schema(&self) -> serde_json::Value {
        let mut props = serde_json::Map::new();
        let mut required: Vec<serde_json::Value> = Vec::new();

        for p in self.params {
            props.insert(p.name.to_string(), serde_json::json!({
                "type": p.param_type,
                "description": p.description
            }));
            if p.required {
                required.push(serde_json::Value::String(p.name.to_string()));
            }
        }

        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": props,
                    "required": required
                }
            }
        })
    }
}

// ── Built-in skill catalogue ──────────────────────────────────────────────────

/// All built-in skills.  These are the skills OpenClaw supports and that the
/// SkillRegistry knows about.
pub static BUILTIN_SKILLS: &[Skill] = &[
    // ── File System (read-only / safe) ────────────────────────────────────
    Skill {
        name: "fs.readFile",
        display: "Read File",
        description: "Read the contents of a file at the given path.",
        category: SkillCategory::FileSystem,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "path", description: "Absolute or workspace-relative file path", required: true, param_type: "string" },
            SkillParam { name: "encoding", description: "File encoding (default: utf8)", required: false, param_type: "string" },
        ],
    },
    Skill {
        name: "fs.readDir",
        display: "List Directory",
        description: "List the contents of a directory.",
        category: SkillCategory::FileSystem,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "path", description: "Directory path to list", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "fs.stat",
        display: "File Stat",
        description: "Return metadata (size, mtime, type) for a file or directory.",
        category: SkillCategory::FileSystem,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "path", description: "Path to stat", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "fs.exists",
        display: "File Exists",
        description: "Check whether a file or directory exists.",
        category: SkillCategory::FileSystem,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "path", description: "Path to check", required: true, param_type: "string" },
        ],
    },
    // ── File System (write / confirm) ─────────────────────────────────────
    Skill {
        name: "fs.writeFile",
        display: "Write File",
        description: "Write or overwrite a file with the given content.",
        category: SkillCategory::FileSystem,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "path",    description: "Target file path", required: true,  param_type: "string" },
            SkillParam { name: "content", description: "Content to write", required: true,  param_type: "string" },
            SkillParam { name: "encoding",description: "File encoding",    required: false, param_type: "string" },
        ],
    },
    Skill {
        name: "fs.mkdir",
        display: "Create Directory",
        description: "Create a directory (including parent directories).",
        category: SkillCategory::FileSystem,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "path",      description: "Directory path to create", required: true,  param_type: "string" },
            SkillParam { name: "recursive", description: "Create parents if needed", required: false, param_type: "boolean" },
        ],
    },
    // ── Web / Browser ─────────────────────────────────────────────────────
    Skill {
        name: "web.fetch",
        display: "Fetch URL",
        description: "Fetch a URL and return its HTTP response body (text or JSON).",
        category: SkillCategory::Web,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "url",     description: "URL to fetch",                             required: true,  param_type: "string" },
            SkillParam { name: "method",  description: "HTTP method (default: GET)",               required: false, param_type: "string" },
            SkillParam { name: "headers", description: "JSON object of request headers",           required: false, param_type: "object" },
            SkillParam { name: "body",    description: "Request body (for POST/PUT/PATCH)",        required: false, param_type: "string" },
        ],
    },
    Skill {
        name: "web.screenshot",
        display: "Screenshot",
        description: "Take a screenshot of a web page and return it as base64.",
        category: SkillCategory::Web,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "url",    description: "URL to screenshot",       required: true,  param_type: "string" },
            SkillParam { name: "width",  description: "Viewport width (px)",     required: false, param_type: "integer" },
            SkillParam { name: "height", description: "Viewport height (px)",    required: false, param_type: "integer" },
        ],
    },
    Skill {
        name: "web.navigate",
        display: "Browser Navigate",
        description: "Navigate the browser to a URL.",
        category: SkillCategory::Web,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "url", description: "Target URL", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "web.click",
        display: "Browser Click",
        description: "Click an element on the current page (CSS selector).",
        category: SkillCategory::Web,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "selector", description: "CSS selector of the element to click", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "web.fill",
        display: "Browser Fill Form",
        description: "Fill a form input with a value.",
        category: SkillCategory::Web,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "selector", description: "CSS selector of input field", required: true, param_type: "string" },
            SkillParam { name: "value",    description: "Value to fill",               required: true, param_type: "string" },
        ],
    },
    // ── Search ────────────────────────────────────────────────────────────
    Skill {
        name: "search.web",
        display: "Web Search",
        description: "Perform a web search and return top results.",
        category: SkillCategory::Search,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "query",    description: "Search query",              required: true,  param_type: "string" },
            SkillParam { name: "max_results", description: "Max results (1-20)",     required: false, param_type: "integer" },
        ],
    },
    Skill {
        name: "search.query",
        display: "Knowledge Search",
        description: "Search local knowledge base (RAG) for relevant context.",
        category: SkillCategory::Search,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "query", description: "Search query", required: true, param_type: "string" },
            SkillParam { name: "top_k", description: "Number of results", required: false, param_type: "integer" },
        ],
    },
    // ── Knowledge / RAG ───────────────────────────────────────────────────
    Skill {
        name: "knowledge.query",
        display: "Knowledge Query",
        description: "Query the agent's knowledge base with a natural-language question.",
        category: SkillCategory::Knowledge,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "question", description: "Natural-language question", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "knowledge.retrieve",
        display: "Knowledge Retrieve",
        description: "Retrieve document chunks by ID or semantic similarity.",
        category: SkillCategory::Knowledge,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "query", description: "Query or document ID", required: true, param_type: "string" },
            SkillParam { name: "top_k", description: "Number of results",    required: false, param_type: "integer" },
        ],
    },
    // ── Email ─────────────────────────────────────────────────────────────
    Skill {
        name: "email.list",
        display: "List Emails",
        description: "List recent emails from the inbox.",
        category: SkillCategory::Email,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "folder", description: "Folder name (default: INBOX)", required: false, param_type: "string" },
            SkillParam { name: "limit",  description: "Max number to return",         required: false, param_type: "integer" },
        ],
    },
    Skill {
        name: "email.read",
        display: "Read Email",
        description: "Read the full content of an email by ID.",
        category: SkillCategory::Email,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "email_id", description: "Email message ID", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "email.send",
        display: "Send Email",
        description: "Send an email to one or more recipients.",
        category: SkillCategory::Email,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "to",      description: "Recipient email address(es), comma-separated", required: true, param_type: "string" },
            SkillParam { name: "subject", description: "Email subject",                                required: true, param_type: "string" },
            SkillParam { name: "body",    description: "Email body (plain text or HTML)",              required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "email.reply",
        display: "Reply to Email",
        description: "Reply to an existing email.",
        category: SkillCategory::Email,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "email_id", description: "ID of the email to reply to", required: true, param_type: "string" },
            SkillParam { name: "body",     description: "Reply body",                  required: true, param_type: "string" },
        ],
    },
    // ── Calendar ──────────────────────────────────────────────────────────
    Skill {
        name: "calendar.list",
        display: "List Events",
        description: "List calendar events in a date range.",
        category: SkillCategory::Calendar,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "start", description: "Start date/time (ISO 8601)", required: false, param_type: "string" },
            SkillParam { name: "end",   description: "End date/time (ISO 8601)",   required: false, param_type: "string" },
        ],
    },
    Skill {
        name: "calendar.create",
        display: "Create Event",
        description: "Create a new calendar event.",
        category: SkillCategory::Calendar,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "title",       description: "Event title",               required: true,  param_type: "string" },
            SkillParam { name: "start",       description: "Start time (ISO 8601)",     required: true,  param_type: "string" },
            SkillParam { name: "end",         description: "End time (ISO 8601)",       required: true,  param_type: "string" },
            SkillParam { name: "description", description: "Event description",         required: false, param_type: "string" },
            SkillParam { name: "attendees",   description: "Comma-separated emails",    required: false, param_type: "string" },
        ],
    },
    // ── Agent memory ──────────────────────────────────────────────────────
    Skill {
        name: "agent.getMemory",
        display: "Get Memory",
        description: "Retrieve a value from the agent's persistent memory store.",
        category: SkillCategory::Agent,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "key", description: "Memory key to retrieve", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "agent.setMemory",
        display: "Set Memory",
        description: "Store a value in the agent's persistent memory.",
        category: SkillCategory::Agent,
        risk: SkillRisk::Confirm,
        params: &[
            SkillParam { name: "key",   description: "Memory key",   required: true, param_type: "string" },
            SkillParam { name: "value", description: "Value to store", required: true, param_type: "string" },
        ],
    },
    Skill {
        name: "agent.listSkills",
        display: "List Skills",
        description: "Return the list of skills available to this agent.",
        category: SkillCategory::Agent,
        risk: SkillRisk::Safe,
        params: &[],
    },
    Skill {
        name: "agent.getContext",
        display: "Get Context",
        description: "Return the current task context (goal, steps so far, working memory).",
        category: SkillCategory::Agent,
        risk: SkillRisk::Safe,
        params: &[],
    },
    // ── Security / audit ──────────────────────────────────────────────────
    Skill {
        name: "security.getStatus",
        display: "Security Status",
        description: "Return the current sandbox security status and circuit-breaker state.",
        category: SkillCategory::Security,
        risk: SkillRisk::Safe,
        params: &[],
    },
    Skill {
        name: "security.listEvents",
        display: "Security Events",
        description: "Return recent security audit events.",
        category: SkillCategory::Security,
        risk: SkillRisk::Safe,
        params: &[
            SkillParam { name: "limit", description: "Max events to return (default 50)", required: false, param_type: "integer" },
        ],
    },
];

// ── SkillSet — per-agent granted skill set ────────────────────────────────────

/// The set of skills granted to a specific agent.
///
/// Built from the agent's `AgentCapability` list, cross-referenced against
/// `BUILTIN_SKILLS` for risk enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSet {
    granted: Vec<String>,
}

impl SkillSet {
    /// Build a `SkillSet` from a list of capability IDs.
    pub fn from_capability_ids(ids: &[String]) -> Self {
        let granted = ids
            .iter()
            .filter(|id| {
                BUILTIN_SKILLS
                    .iter()
                    .any(|s| s.name == id.as_str() && s.is_grantable())
            })
            .cloned()
            .collect();
        Self { granted }
    }

    /// Returns the `Skill` definition for a given name, if granted.
    pub fn get(&self, name: &str) -> Option<&'static Skill> {
        if self.granted.iter().any(|g| g == name) {
            BUILTIN_SKILLS.iter().find(|s| s.name == name)
        } else {
            None
        }
    }

    pub fn is_granted(&self, name: &str) -> bool {
        self.granted.iter().any(|g| g == name)
    }

    pub fn all_granted(&self) -> Vec<&'static Skill> {
        self.granted
            .iter()
            .filter_map(|id| BUILTIN_SKILLS.iter().find(|s| s.name == id.as_str()))
            .collect()
    }

    /// Generate OpenAI-format tool definitions for all granted safe/confirm skills.
    pub fn to_tool_schemas(&self) -> Vec<serde_json::Value> {
        self.all_granted()
            .iter()
            .filter(|s| s.risk != SkillRisk::Deny)
            .map(|s| s.to_tool_schema())
            .collect()
    }

    /// Default skill set for a role — grantable safe skills only.
    pub fn default_safe() -> Self {
        let granted = BUILTIN_SKILLS
            .iter()
            .filter(|s| s.risk == SkillRisk::Safe)
            .map(|s| s.name.to_string())
            .collect();
        Self { granted }
    }
}

// ── Lookup helpers ────────────────────────────────────────────────────────────

/// Find a skill by exact name.
pub fn find_skill(name: &str) -> Option<&'static Skill> {
    BUILTIN_SKILLS.iter().find(|s| s.name == name)
}

/// All skills in a category.
pub fn skills_by_category(cat: &SkillCategory) -> Vec<&'static Skill> {
    BUILTIN_SKILLS.iter().filter(|s| &s.category == cat).collect()
}

/// All grantable (non-Deny) skills.
pub fn grantable_skills() -> Vec<&'static Skill> {
    BUILTIN_SKILLS.iter().filter(|s| s.is_grantable()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_skills_have_unique_names() {
        let mut names = std::collections::HashSet::new();
        for s in BUILTIN_SKILLS {
            assert!(names.insert(s.name), "Duplicate skill name: {}", s.name);
        }
    }

    #[test]
    fn shell_skills_are_deny() {
        for s in BUILTIN_SKILLS {
            if s.name.starts_with("shell.") || s.name.starts_with("process.") {
                assert_eq!(s.risk, SkillRisk::Deny, "Shell/process skill must be Deny: {}", s.name);
            }
        }
    }

    #[test]
    fn skill_set_rejects_denied_skills() {
        let ids = vec!["shell.exec".to_string(), "web.fetch".to_string()];
        let set = SkillSet::from_capability_ids(&ids);
        assert!(!set.is_granted("shell.exec"));
        assert!(set.is_granted("web.fetch"));
    }

    #[test]
    fn tool_schema_structure() {
        let skill = find_skill("web.fetch").unwrap();
        let schema = skill.to_tool_schema();
        assert_eq!(schema["type"], "function");
        assert_eq!(schema["function"]["name"], "web.fetch");
        let params = &schema["function"]["parameters"]["required"];
        assert!(params.as_array().unwrap().contains(&serde_json::json!("url")));
    }

    #[test]
    fn skill_set_default_safe_all_safe() {
        let set = SkillSet::default_safe();
        for s in set.all_granted() {
            assert_eq!(s.risk, SkillRisk::Safe, "Default safe set must not contain: {}", s.name);
        }
    }
}
