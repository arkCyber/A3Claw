//! OpenClaw AI Assistant
//! 
//! Provides intelligent assistance for configuration, troubleshooting,
//! and optimization of OpenClaw+ based on WasmEdge.

pub mod intent;
pub mod knowledge;
pub mod context;
pub mod action;
pub mod response;

use openclaw_config::RagConfig;
use thiserror::Error;

pub use intent::{Intent, IntentParser};
pub use knowledge::{KnowledgeBase, Document, DocumentCategory};
pub use context::{SystemContext, ContextAnalyzer};
pub use action::{ActionExecutor, SuggestedAction, ActionResult};
pub use response::{AssistantResponse, CodeSnippet, DocumentLink};

#[derive(Debug, Error)]
pub enum AssistantError {
    #[error("Failed to parse intent: {0}")]
    IntentParseError(String),
    
    #[error("Knowledge base error: {0}")]
    KnowledgeBaseError(String),
    
    #[error("Context analysis error: {0}")]
    ContextError(String),
    
    #[error("Action execution error: {0}")]
    ActionError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, AssistantError>;

/// OpenClaw AI Assistant Engine
pub struct OpenClawAssistant {
    intent_parser: IntentParser,
    knowledge_base: KnowledgeBase,
    context_analyzer: ContextAnalyzer,
    #[allow(dead_code)]
    action_executor: ActionExecutor,
}

impl OpenClawAssistant {
    /// Creates a new assistant instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            intent_parser: IntentParser::new(),
            knowledge_base: KnowledgeBase::new()?,
            context_analyzer: ContextAnalyzer::new(),
            action_executor: ActionExecutor::new(),
        })
    }
    
    /// Loads knowledge base from RAG configuration
    pub fn with_rag_config(rag_config: &RagConfig) -> Result<Self> {
        Ok(Self {
            intent_parser: IntentParser::new(),
            knowledge_base: KnowledgeBase::from_rag_config(rag_config)?,
            context_analyzer: ContextAnalyzer::new(),
            action_executor: ActionExecutor::new(),
        })
    }
    
    /// Processes a user query and returns a response
    pub fn process_query(
        &self,
        query: &str,
        context: &SystemContext,
    ) -> Result<AssistantResponse> {
        tracing::info!("Processing query: {}", query);
        
        // 1. Parse intent
        let intent = self.intent_parser.parse(query)?;
        tracing::debug!("Parsed intent: {:?}", intent);
        
        // 2. Analyze context
        let analysis = self.context_analyzer.analyze(context)?;
        tracing::debug!("Context analysis: {:?}", analysis);
        
        // 3. Search knowledge base
        let knowledge = self.knowledge_base.search(query, &intent)?;
        tracing::debug!("Found {} relevant documents", knowledge.len());
        
        // 4. Generate response
        let response = self.generate_response(&intent, &knowledge, &analysis)?;
        
        Ok(response)
    }
    
    /// Generates a response based on intent, knowledge, and context
    fn generate_response(
        &self,
        intent: &Intent,
        knowledge: &[Document],
        analysis: &context::ContextAnalysis,
    ) -> Result<AssistantResponse> {
        use Intent::*;
        
        match intent {
            ConfigureRAG { action } => {
                self.generate_rag_config_response(action, knowledge, analysis)
            }
            DiagnoseError { error_type } => {
                self.generate_diagnostic_response(error_type, knowledge, analysis)
            }
            OptimizePerformance { target } => {
                self.generate_optimization_response(target, knowledge, analysis)
            }
            SecurityAudit => {
                self.generate_security_response(knowledge, analysis)
            }
            QueryDocumentation { topic } => {
                self.generate_documentation_response(topic, knowledge)
            }
            Unknown => {
                Ok(AssistantResponse::simple(
                    "I'm not sure what you're asking. Could you rephrase? \
                     I can help with:\n\
                     - Configuring RAG knowledge base\n\
                     - Diagnosing WasmEdge errors\n\
                     - Optimizing performance\n\
                     - Security audits\n\
                     - Documentation lookup"
                ))
            }
        }
    }
    
    fn generate_rag_config_response(
        &self,
        action: &str,
        knowledge: &[Document],
        analysis: &context::ContextAnalysis,
    ) -> Result<AssistantResponse> {
        let mut response = AssistantResponse::new();
        
        if action.contains("add") || action.contains("new") {
            response.text = format!(
                "I'll help you add a new knowledge base folder.\n\n\
                 Current RAG configuration:\n\
                 - {} folders configured\n\
                 - {} files indexed\n\n\
                 What would you like to do?\n\
                 1. Add a folder with automatic settings\n\
                 2. Add a folder with custom settings\n\
                 3. Add individual files",
                analysis.rag_folder_count,
                analysis.rag_file_count
            );
            response.actions.push(SuggestedAction::ConfigureRAG {
                operation: "add_folder".to_string(),
                params: serde_json::json!({
                    "watch_enabled": true,
                    "allow_agent_write": false,
                }),
            });
            response.code_snippets.push(response::CodeSnippet {
                language: "toml".to_string(),
                code: "[rag.folders.my-docs]\npath = \"/path/to/docs\"\nwatch_enabled = true\nallow_agent_write = false".to_string(),
                description: "Example RAG folder config entry".to_string(),
            });
        } else if action.contains("remove") || action.contains("delete") {
            response.text = format!(
                "RAG Knowledge Base — Remove Entry\n\n\
                 You currently have {} folders and {} files in the knowledge base.\n\
                 Use the Settings panel to select and remove an entry.",
                analysis.rag_folder_count,
                analysis.rag_file_count
            );
        } else {
            response.text = format!(
                "RAG Knowledge Base Status\n\n\
                 - Folders: {}\n\
                 - Files: {}\n\
                 - Chunk size: {} chars\n\
                 - OCR enabled: {}\n\n\
                 Use the quick-action buttons to add folders or files, or ask me to help configure specific settings.",
                analysis.rag_folder_count,
                analysis.rag_file_count,
                analysis.rag_chunk_size,
                analysis.rag_ocr_enabled
            );
            response.actions.push(SuggestedAction::ConfigureRAG {
                operation: "check_status".to_string(),
                params: serde_json::json!({}),
            });
        }
        
        // Add relevant documentation
        for doc in knowledge.iter().take(2) {
            response.related_docs.push(DocumentLink {
                title: doc.title.clone(),
                url: format!("#doc-{}", doc.id),
                category: format!("{:?}", doc.category),
            });
        }
        
        Ok(response)
    }
    
    fn generate_diagnostic_response(
        &self,
        error_type: &str,
        knowledge: &[Document],
        analysis: &context::ContextAnalysis,
    ) -> Result<AssistantResponse> {
        let mut response = AssistantResponse::new();
        let et_lower = error_type.to_lowercase();

        if et_lower.contains("policydenied")
            || et_lower.contains("policy denied")
            || et_lower.contains("policy_denied")
        {
            response.text = format!(
                "Security Policy Violation\n\n\
                 Event: {}\n\n\
                 The sandbox blocked an operation that violated the active security policy.\n\n\
                 Common causes:\n\
                 1. Network request to an un-whitelisted host\n\
                 2. Filesystem write to a read-only preopen path\n\
                 3. Sensitive skill (Python / SSH) used without user approval\n\n\
                 Fix:\n\
                 - Add the host to `network_allowlist` in Security Settings\n\
                 - Set `allow_agent_write = true` on the relevant RAG folder\n\
                 - Enable the skill in the Confirmation Policy list\n\n\
                 Recent error count: {}",
                error_type,
                analysis.error_patterns.len()
            );
            response.actions.push(SuggestedAction::RunDiagnostic {
                test_name: "policy_check".to_string(),
            });
            response.code_snippets.push(response::CodeSnippet {
                language: "toml".to_string(),
                code: "[security]\nnetwork_allowlist = [\"api.openai.com\", \"your-trusted-host.example.com\"]".to_string(),
                description: "Add a host to the network allowlist".to_string(),
            });
        } else if et_lower.contains("timeout") || et_lower.contains("timed out") {
            response.text = format!(
                "Timeout Detected\n\n\
                 Event: {}\n\n\
                 The sandbox or agent operation exceeded the configured time limit.\n\n\
                 Recommendations:\n\
                 1. Increase `max_execution_time_secs` in Security Settings\n\
                 2. Check if the target host is reachable (network latency)\n\
                 3. For large file processing, split into smaller batches\n\
                 4. Verify the WasmEdge runtime is not CPU-starved",
                error_type
            );
            response.actions.push(SuggestedAction::RunDiagnostic {
                test_name: "timeout_analysis".to_string(),
            });
        } else if et_lower.contains("network") || et_lower.contains("connection refused")
            || et_lower.contains("dns") || et_lower.contains("tls")
        {
            response.text = format!(
                "Network Error\n\n\
                 Event: {}\n\n\
                 The agent encountered a network connectivity issue.\n\n\
                 Checklist:\n\
                 1. Is the target host in the `network_allowlist`?\n\
                 2. Is there an active internet connection?\n\
                 3. Does the host require a specific port that is blocked?\n\
                 4. For TLS errors: check certificate validity and host name\n\n\
                 Network allowlist entries configured: {}",
                error_type,
                analysis.network_whitelist_count
            );
            response.actions.push(SuggestedAction::ApplySecurity {
                template: "network_review".to_string(),
            });
        } else if et_lower.contains("wasi") || et_lower.contains("error") {
            if let Some(error_code) = self.extract_error_code(error_type) {
                response.text = self.explain_wasi_error(error_code);
                response.actions.push(SuggestedAction::RunDiagnostic {
                    test_name: "wasi_permissions".to_string(),
                });
            } else {
                response.text = format!(
                    "Diagnostic Analysis\n\n\
                     Query: {}\n\n\
                     I detected an error pattern. To narrow down the issue:\n\
                     1. Check the sandbox event log for denied operations\n\
                     2. Verify WASI preopen paths are correctly configured\n\
                     3. Confirm the network whitelist includes all required hosts\n\n\
                     Preopen paths configured: {}\n\
                     Network allowlist entries: {}",
                    error_type,
                    analysis.filesystem_preopen_count,
                    analysis.network_whitelist_count
                );
                response.actions.push(SuggestedAction::RunDiagnostic {
                    test_name: "general".to_string(),
                });
            }
        } else {
            response.text = format!(
                "Security Event Diagnosis\n\n\
                 Analyzing: {}\n\n\
                 No specific error pattern matched. Common causes:\n\
                 - PolicyDenied: network/filesystem access blocked\n\
                 - Missing preopen: guest path not exposed to host\n\
                 - Capability missing: WASI capability not granted\n\n\
                 Check the Dashboard event log for PolicyDenied entries.\n\
                 Recent error patterns detected: {}",
                error_type,
                analysis.error_patterns.len()
            );
        }

        // Add troubleshooting docs
        for doc in knowledge.iter()
            .filter(|d| matches!(d.category, DocumentCategory::Troubleshooting))
        {
            response.related_docs.push(DocumentLink {
                title: doc.title.clone(),
                url: format!("#doc-{}", doc.id),
                category: "Troubleshooting".to_string(),
            });
        }

        Ok(response)
    }
    
    fn generate_optimization_response(
        &self,
        target: &str,
        _knowledge: &[Document],
        analysis: &context::ContextAnalysis,
    ) -> Result<AssistantResponse> {
        let mut response = AssistantResponse::new();
        
        if target.contains("RAG") || target.contains("index") {
            response.text = format!(
                "Performance Analysis:\n\n\
                 Current RAG settings:\n\
                 - Chunk size: {}\n\
                 - Chunk overlap: {}\n\
                 - OCR enabled: {}\n\n\
                 Recommendations:\n\
                 1. For large PDFs (>10MB), disable OCR\n\
                 2. Reduce chunk size to 512-800 for better precision\n\
                 3. Increase batch size to 200 for faster indexing",
                analysis.rag_chunk_size,
                analysis.rag_chunk_overlap,
                analysis.rag_ocr_enabled
            );
            response.actions.push(SuggestedAction::OptimizeRAG {
                params: serde_json::json!({
                    "chunk_size": 800,
                    "max_file_size_mb": 10,
                    "batch_size": 200,
                }),
            });
        } else {
            response.text = format!(
                "Performance Optimization Suggestions\n\n\
                 Target: {}\n\n\
                 General recommendations:\n\
                 - RAG indexing: keep chunk_size 800–1200 for balanced recall\n\
                 - WasmEdge agent: limit concurrent tool calls to 4\n\
                 - Security scanning: use batch mode for large event logs\n\
                 - UI: reduce polling interval if dashboard feels slow\n\n\
                 Open Settings → RAG to adjust indexing parameters.",
                target
            );
            response.actions.push(SuggestedAction::OptimizeRAG {
                params: serde_json::json!({
                    "chunk_size": 1000,
                }),
            });
        }
        
        Ok(response)
    }
    
    fn generate_security_response(
        &self,
        _knowledge: &[Document],
        analysis: &context::ContextAnalysis,
    ) -> Result<AssistantResponse> {
        let mut response = AssistantResponse::new();
        
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        
        if analysis.network_whitelist_count == 0 {
            issues.push("⚠️  No network whitelist configured");
            recommendations.push("Add network whitelist to restrict outbound connections");
        }
        
        if analysis.filesystem_preopen_count == 0 {
            issues.push("⚠️  No filesystem preopen configured");
            recommendations.push("Configure filesystem preopen to restrict file access");
        }
        
        if issues.is_empty() {
            response.text = "✓ Security configuration looks good!\n\n\
                           Your OpenClaw instance follows security best practices.".to_string();
        } else {
            response.text = format!(
                "Security Audit Results:\n\n\
                 Issues found:\n{}\n\n\
                 Recommendations:\n{}",
                issues.join("\n"),
                recommendations.iter().enumerate()
                    .map(|(i, r)| format!("{}. {}", i + 1, r))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            
            response.actions.push(SuggestedAction::ApplySecurity {
                template: "production".to_string(),
            });
        }
        
        Ok(response)
    }
    
    fn generate_documentation_response(
        &self,
        topic: &str,
        knowledge: &[Document],
    ) -> Result<AssistantResponse> {
        let mut response = AssistantResponse::new();
        
        if knowledge.is_empty() {
            response.text = format!(
                "I couldn't find documentation about '{}'.\n\n\
                 Try searching for:\n\
                 - WasmEdge configuration\n\
                 - RAG setup\n\
                 - Security policies\n\
                 - Skill development",
                topic
            );
        } else {
            let doc = &knowledge[0];
            response.text = format!(
                "# {}\n\n{}",
                doc.title,
                doc.content.chars().take(500).collect::<String>()
            );
            
            if doc.content.len() > 500 {
                response.text.push_str("\n\n[Read more...]");
            }
            
            for doc in knowledge.iter().take(5) {
                response.related_docs.push(DocumentLink {
                    title: doc.title.clone(),
                    url: format!("#doc-{}", doc.id),
                    category: format!("{:?}", doc.category),
                });
            }
        }
        
        Ok(response)
    }
    
    fn extract_error_code(&self, error_text: &str) -> Option<i32> {
        let re = regex::Regex::new(r"error\s+(\d+)").ok()?;
        re.captures(error_text)?
            .get(1)?
            .as_str()
            .parse()
            .ok()
    }
    
    fn explain_wasi_error(&self, code: i32) -> String {
        match code {
            8 => {
                "WASI Error 8 (EBADF - Bad File Descriptor)\n\n\
                 Common causes:\n\
                 1. Preopen path doesn't exist\n\
                 2. Incorrect preopen format (should be GUEST:HOST)\n\
                 3. File permissions issue\n\n\
                 Fix:\n\
                 1. Check if the host path exists\n\
                 2. Verify preopen format in config\n\
                 3. Check file permissions".to_string()
            }
            2 => {
                "WASI Error 2 (ENOENT - No Such File or Directory)\n\n\
                 The file or directory doesn't exist.\n\n\
                 Fix:\n\
                 1. Verify the path is correct\n\
                 2. Check if the file was created\n\
                 3. Ensure preopen includes the parent directory".to_string()
            }
            13 => {
                "WASI Error 13 (EACCES - Permission Denied)\n\n\
                 Insufficient permissions to access the resource.\n\n\
                 Fix:\n\
                 1. Check file permissions (chmod)\n\
                 2. Verify preopen configuration\n\
                 3. Run with appropriate user permissions".to_string()
            }
            _ => {
                format!(
                    "WASI Error {}\n\n\
                     This is a WASI system error. Check the WasmEdge documentation \
                     for details about this error code.",
                    code
                )
            }
        }
    }
}

impl Default for OpenClawAssistant {
    fn default() -> Self {
        Self::new().expect("Failed to create default assistant")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use context::SystemContext;

    fn assistant() -> OpenClawAssistant {
        OpenClawAssistant::new().unwrap()
    }

    #[test]
    fn new_succeeds() {
        assert!(OpenClawAssistant::new().is_ok());
    }

    #[test]
    fn default_succeeds() {
        let _a = OpenClawAssistant::default();
    }

    #[test]
    fn with_rag_config_succeeds() {
        let rag = openclaw_config::RagConfig::default();
        assert!(OpenClawAssistant::with_rag_config(&rag).is_ok());
    }

    #[test]
    fn process_query_empty_context_not_empty() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("add new knowledge base folder", &ctx).unwrap();
        assert!(!r.text.is_empty());
    }

    #[test]
    fn process_query_unknown_returns_fallback() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("zzz xyz 123 abc", &ctx).unwrap();
        assert!(!r.text.is_empty());
    }

    #[test]
    fn process_query_wasi_error_8() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("WASI error 8 bad file descriptor", &ctx).unwrap();
        assert!(r.text.contains("8") || r.text.contains("EBADF") || r.text.contains("descriptor"));
    }

    #[test]
    fn process_query_wasi_error_2() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("WASI error 2 file not found", &ctx).unwrap();
        assert!(r.text.contains("2") || r.text.contains("ENOENT") || r.text.contains("exist"));
    }

    #[test]
    fn process_query_wasi_error_13() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("WASI error 13 permission denied", &ctx).unwrap();
        assert!(r.text.contains("13") || r.text.contains("EACCES") || r.text.contains("ermission"));
    }

    #[test]
    fn process_query_policy_denied_returns_code_snippet() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("policy_denied network access blocked", &ctx).unwrap();
        assert!(!r.text.is_empty());
        assert!(!r.code_snippets.is_empty());
    }

    #[test]
    fn process_query_timeout_returns_actions() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("agent operation timed out", &ctx).unwrap();
        assert!(!r.text.is_empty());
        assert!(!r.actions.is_empty());
    }

    #[test]
    fn wasi_description_code_8_via_query() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("WASI error 8 EBADF bad descriptor", &ctx).unwrap();
        assert!(r.text.contains("EBADF") || r.text.contains("Bad") || r.text.contains("8"));
    }

    #[test]
    fn wasi_description_code_2_via_query() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("WASI error 2 ENOENT no such file", &ctx).unwrap();
        assert!(r.text.contains("ENOENT") || r.text.contains("No Such") || r.text.contains("2"));
    }

    #[test]
    fn wasi_description_code_13_via_query() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("WASI error 13 EACCES permission denied", &ctx).unwrap();
        assert!(r.text.contains("EACCES") || r.text.contains("ermission") || r.text.contains("13"));
    }

    #[test]
    fn wasi_description_unknown_code_via_query() {
        let a = assistant();
        let ctx = SystemContext::new();
        let r = a.process_query("WASI error 99 unknown code", &ctx).unwrap();
        assert!(!r.text.is_empty());
    }

    #[test]
    fn process_query_security_audit_non_empty_text() {
        let a = assistant();
        let ctx = SystemContext::new()
            .with_security_config(openclaw_security::SecurityConfig::default());
        let r = a.process_query("Run security audit", &ctx).unwrap();
        assert!(!r.text.is_empty());
        // text should contain Security or Audit (either pass or issues path)
        assert!(
            r.text.contains("Security") || r.text.contains("security")
                || r.text.contains("Audit") || r.text.contains("audit")
        );
    }

    #[test]
    fn process_query_security_audit_empty_config_has_actions() {
        let a = assistant();
        let mut sec = openclaw_security::SecurityConfig::default();
        sec.network_allowlist.clear();
        sec.fs_mounts.clear();
        let ctx = SystemContext::new().with_security_config(sec);
        let r = a.process_query("Run security audit", &ctx).unwrap();
        assert!(!r.text.is_empty());
        // With empty config, issues are found → actions must be suggested
        assert!(!r.actions.is_empty());
    }

    #[test]
    fn process_query_optimize_returns_text() {
        let a = assistant();
        let mut rag = openclaw_config::RagConfig::default();
        rag.settings.chunk_size = 2000;
        let ctx = SystemContext::new().with_rag_config(rag);
        let r = a.process_query("RAG indexing is slow, optimize performance", &ctx).unwrap();
        assert!(!r.text.is_empty());
    }
}
