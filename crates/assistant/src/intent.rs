//! Intent parsing for user queries

use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    ConfigureRAG { action: String },
    DiagnoseError { error_type: String },
    OptimizePerformance { target: String },
    SecurityAudit,
    QueryDocumentation { topic: String },
    Unknown,
}

pub struct IntentParser {
    patterns: Vec<IntentPattern>,
}

struct IntentPattern {
    keywords: Vec<&'static str>,
    intent_type: IntentType,
}

#[derive(Debug, Clone, Copy)]
enum IntentType {
    ConfigureRAG,
    DiagnoseError,
    OptimizePerformance,
    SecurityAudit,
    QueryDocumentation,
}

impl IntentParser {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                IntentPattern {
                    keywords: vec![
                        "add", "create", "new", "configure", "setup",
                        "知识库", "文件夹", "folder", "rag",
                        "knowledge base", "knowledge_base", "索引状态",
                    ],
                    intent_type: IntentType::ConfigureRAG,
                },
                IntentPattern {
                    keywords: vec![
                        "error", "fail", "错误", "失败", "问题", "bug", "crash",
                        "wasi", "wasmedge",
                        "analyze", "recent", "security events", "suggest fixes",
                        "diagnose", "诊断", "分析",
                        "policydenied", "policy_denied", "policy denied", "blocked",
                        "timeout", "timed out", "timed",
                        "connection refused", "dns", "tls",
                        "network error", "denied",
                    ],
                    intent_type: IntentType::DiagnoseError,
                },
                IntentPattern {
                    keywords: vec![
                        "slow", "慢", "optimize", "优化", "performance", "性能",
                        "speed", "fast", "indexing", "索引",
                        "improve", "improvement", "improvements", "review config",
                        "建议", "优化建议",
                    ],
                    intent_type: IntentType::OptimizePerformance,
                },
                IntentPattern {
                    keywords: vec![
                        "security", "安全", "audit", "审计", "permission", "权限",
                        "policy", "gaps", "over-permissive", "rules", "漏洞",
                    ],
                    intent_type: IntentType::SecurityAudit,
                },
                IntentPattern {
                    keywords: vec![
                        "how", "what", "怎么", "如何", "文档", "doc", "help",
                        "帮助", "guide", "use", "skill", "python",
                    ],
                    intent_type: IntentType::QueryDocumentation,
                },
            ],
        }
    }
    
    pub fn parse(&self, query: &str) -> Result<Intent> {
        let query_lower = query.to_lowercase();
        
        let mut scores: Vec<(IntentType, usize)> = Vec::new();
        
        for pattern in &self.patterns {
            let mut score = 0;
            for keyword in &pattern.keywords {
                if query_lower.contains(keyword) {
                    score += 1;
                }
            }
            if score > 0 {
                scores.push((pattern.intent_type, score));
            }
        }
        
        scores.sort_by(|a, b| b.1.cmp(&a.1));
        
        if let Some((intent_type, _)) = scores.first() {
            Ok(match intent_type {
                IntentType::ConfigureRAG => Intent::ConfigureRAG {
                    action: self.extract_action(&query_lower),
                },
                IntentType::DiagnoseError => Intent::DiagnoseError {
                    error_type: self.extract_error_type(&query_lower),
                },
                IntentType::OptimizePerformance => Intent::OptimizePerformance {
                    target: self.extract_optimization_target(&query_lower),
                },
                IntentType::SecurityAudit => Intent::SecurityAudit,
                IntentType::QueryDocumentation => Intent::QueryDocumentation {
                    topic: self.extract_topic(&query_lower),
                },
            })
        } else {
            Ok(Intent::Unknown)
        }
    }
    
    fn extract_action(&self, query: &str) -> String {
        if query.contains("add") || query.contains("new") || query.contains("create") {
            "add".to_string()
        } else if query.contains("remove") || query.contains("delete") {
            "remove".to_string()
        } else if query.contains("update") || query.contains("modify") {
            "update".to_string()
        } else {
            "configure".to_string()
        }
    }
    
    fn extract_error_type(&self, query: &str) -> String {
        if let Some(start) = query.find("error") {
            query[start..].split_whitespace().take(3).collect::<Vec<_>>().join(" ")
        } else if let Some(start) = query.find("wasi") {
            query[start..].split_whitespace().take(3).collect::<Vec<_>>().join(" ")
        } else {
            query.to_string()
        }
    }
    
    fn extract_optimization_target(&self, query: &str) -> String {
        if query.contains("rag") || query.contains("index") || query.contains("知识库") {
            "RAG".to_string()
        } else if query.contains("wasm") || query.contains("agent") {
            "WasmEdge".to_string()
        } else {
            "general".to_string()
        }
    }
    
    fn extract_topic(&self, query: &str) -> String {
        let stop_words = ["how", "what", "to", "do", "i", "can", "the", "a", "an"];
        query
            .split_whitespace()
            .filter(|w| !stop_words.contains(&w.to_lowercase().as_str()))
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for IntentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_configure_rag() {
        let parser = IntentParser::new();
        
        let intent = parser.parse("I want to add a new knowledge base folder").unwrap();
        assert!(matches!(intent, Intent::ConfigureRAG { .. }));
        
        let intent = parser.parse("如何配置RAG知识库").unwrap();
        assert!(matches!(intent, Intent::ConfigureRAG { .. }));
    }
    
    #[test]
    fn test_parse_diagnose_error() {
        let parser = IntentParser::new();
        
        let intent = parser.parse("WasmEdge error 8").unwrap();
        assert!(matches!(intent, Intent::DiagnoseError { .. }));
        
        let intent = parser.parse("启动失败，WASI错误").unwrap();
        assert!(matches!(intent, Intent::DiagnoseError { .. }));
    }
    
    #[test]
    fn test_parse_optimize() {
        let parser = IntentParser::new();
        
        let intent = parser.parse("RAG indexing is too slow").unwrap();
        assert!(matches!(intent, Intent::OptimizePerformance { .. }));
        
        let intent = parser.parse("如何优化性能").unwrap();
        assert!(matches!(intent, Intent::OptimizePerformance { .. }));
    }
    
    #[test]
    fn test_parse_security() {
        let parser = IntentParser::new();
        
        let intent = parser.parse("Run security audit").unwrap();
        assert!(matches!(intent, Intent::SecurityAudit));
        
        let intent = parser.parse("检查安全配置").unwrap();
        assert!(matches!(intent, Intent::SecurityAudit));
    }
    
    #[test]
    fn test_parse_documentation() {
        let parser = IntentParser::new();
        
        let intent = parser.parse("How to use Python skill?").unwrap();
        assert!(matches!(intent, Intent::QueryDocumentation { .. }));
        
        let intent = parser.parse("WasmEdge help documentation").unwrap();
        assert!(matches!(intent, Intent::QueryDocumentation { .. }));
    }
    
    #[test]
    fn test_parse_unknown() {
        let parser = IntentParser::new();
        
        let intent = parser.parse("xyz abc def").unwrap();
        assert!(matches!(intent, Intent::Unknown));
    }
}
