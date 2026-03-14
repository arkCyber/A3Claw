//! System context analysis

use crate::Result;
use openclaw_config::RagConfig;
use openclaw_security::SecurityConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub rag_config: Option<RagConfig>,
    pub security_config: Option<SecurityConfig>,
    pub error_logs: Vec<String>,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub wasmedge_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    pub rag_folder_count: usize,
    pub rag_file_count: usize,
    pub rag_chunk_size: usize,
    pub rag_chunk_overlap: usize,
    pub rag_ocr_enabled: bool,
    pub network_whitelist_count: usize,
    pub filesystem_preopen_count: usize,
    pub has_recent_errors: bool,
    pub error_patterns: Vec<String>,
}

pub struct ContextAnalyzer;

impl ContextAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    pub fn analyze(&self, context: &SystemContext) -> Result<ContextAnalysis> {
        let mut analysis = ContextAnalysis {
            rag_folder_count: 0,
            rag_file_count: 0,
            rag_chunk_size: 1000,
            rag_chunk_overlap: 200,
            rag_ocr_enabled: true,
            network_whitelist_count: 0,
            filesystem_preopen_count: 0,
            has_recent_errors: false,
            error_patterns: Vec::new(),
        };
        
        if let Some(rag_config) = &context.rag_config {
            analysis.rag_folder_count = rag_config.folders.len();
            analysis.rag_file_count = rag_config.files.len();
            analysis.rag_chunk_size = rag_config.settings.chunk_size;
            analysis.rag_chunk_overlap = rag_config.settings.chunk_overlap;
            analysis.rag_ocr_enabled = rag_config.settings.ocr_enabled;
        }
        
        if let Some(security_config) = &context.security_config {
            analysis.network_whitelist_count = security_config.network_allowlist.len();
            analysis.filesystem_preopen_count = security_config.fs_mounts.len();
        }
        
        if !context.error_logs.is_empty() {
            analysis.has_recent_errors = true;
            analysis.error_patterns = self.extract_error_patterns(&context.error_logs);
        }
        
        Ok(analysis)
    }
    
    fn extract_error_patterns(&self, logs: &[String]) -> Vec<String> {
        let mut patterns = Vec::new();
        
        for log in logs {
            if log.contains("WASI") || log.contains("error") {
                if let Some(pattern) = self.extract_pattern(log) {
                    if !patterns.contains(&pattern) {
                        patterns.push(pattern);
                    }
                }
            }
        }
        
        patterns
    }
    
    fn extract_pattern(&self, log: &str) -> Option<String> {
        if let Some(start) = log.find("error") {
            Some(log[start..].split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        } else if let Some(start) = log.find("WASI") {
            Some(log[start..].split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        } else {
            None
        }
    }
}

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemContext {
    pub fn new() -> Self {
        Self {
            rag_config: None,
            security_config: None,
            error_logs: Vec::new(),
            system_info: SystemInfo::detect(),
        }
    }
    
    pub fn with_rag_config(mut self, config: RagConfig) -> Self {
        self.rag_config = Some(config);
        self
    }
    
    pub fn with_security_config(mut self, config: SecurityConfig) -> Self {
        self.security_config = Some(config);
        self
    }
    
    pub fn with_error_logs(mut self, logs: Vec<String>) -> Self {
        self.error_logs = logs;
        self
    }
}

impl Default for SystemContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemInfo {
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            wasmedge_version: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_context_analysis_empty() {
        let analyzer = ContextAnalyzer::new();
        let context = SystemContext::new();
        
        let analysis = analyzer.analyze(&context).unwrap();
        assert_eq!(analysis.rag_folder_count, 0);
        assert_eq!(analysis.rag_file_count, 0);
        assert!(!analysis.has_recent_errors);
    }
    
    #[test]
    fn test_context_analysis_with_rag() {
        let analyzer = ContextAnalyzer::new();
        let mut rag_config = RagConfig::default();
        rag_config.settings.chunk_size = 512;
        rag_config.settings.ocr_enabled = false;
        
        let context = SystemContext::new().with_rag_config(rag_config);
        
        let analysis = analyzer.analyze(&context).unwrap();
        assert_eq!(analysis.rag_chunk_size, 512);
        assert!(!analysis.rag_ocr_enabled);
    }
    
    #[test]
    fn test_error_pattern_extraction() {
        let analyzer = ContextAnalyzer::new();
        let context = SystemContext::new().with_error_logs(vec![
            "WASI error 8: Bad file descriptor".to_string(),
            "error: file not found".to_string(),
        ]);
        
        let analysis = analyzer.analyze(&context).unwrap();
        assert!(analysis.has_recent_errors);
        assert!(!analysis.error_patterns.is_empty());
    }
}
