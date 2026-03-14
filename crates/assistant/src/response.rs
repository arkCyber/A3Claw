//! Response types for assistant

use crate::SuggestedAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantResponse {
    pub text: String,
    pub actions: Vec<SuggestedAction>,
    pub code_snippets: Vec<CodeSnippet>,
    pub related_docs: Vec<DocumentLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    pub language: String,
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLink {
    pub title: String,
    pub url: String,
    pub category: String,
}

impl AssistantResponse {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            actions: Vec::new(),
            code_snippets: Vec::new(),
            related_docs: Vec::new(),
        }
    }
    
    pub fn simple(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            actions: Vec::new(),
            code_snippets: Vec::new(),
            related_docs: Vec::new(),
        }
    }
    
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }
    
    pub fn with_action(mut self, action: SuggestedAction) -> Self {
        self.actions.push(action);
        self
    }
    
    pub fn with_code(mut self, snippet: CodeSnippet) -> Self {
        self.code_snippets.push(snippet);
        self
    }
    
    pub fn with_doc(mut self, link: DocumentLink) -> Self {
        self.related_docs.push(link);
        self
    }
}

impl Default for AssistantResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::SuggestedAction;

    #[test]
    fn new_response_is_empty() {
        let r = AssistantResponse::new();
        assert!(r.text.is_empty());
        assert!(r.actions.is_empty());
        assert!(r.code_snippets.is_empty());
        assert!(r.related_docs.is_empty());
    }

    #[test]
    fn simple_sets_text() {
        let r = AssistantResponse::simple("Hello world");
        assert_eq!(r.text, "Hello world");
        assert!(r.actions.is_empty());
    }

    #[test]
    fn with_text_replaces_text() {
        let r = AssistantResponse::new().with_text("updated");
        assert_eq!(r.text, "updated");
    }

    #[test]
    fn with_action_appends() {
        let action = SuggestedAction::ClearEventLog;
        let r = AssistantResponse::new()
            .with_text("test")
            .with_action(action);
        assert_eq!(r.actions.len(), 1);
        assert!(matches!(r.actions[0], SuggestedAction::ClearEventLog));
    }

    #[test]
    fn with_code_appends_snippet() {
        let snippet = CodeSnippet {
            language: "toml".to_string(),
            code: "[section]\nkey = \"value\"".to_string(),
            description: "Example config".to_string(),
        };
        let r = AssistantResponse::new().with_code(snippet);
        assert_eq!(r.code_snippets.len(), 1);
        assert_eq!(r.code_snippets[0].language, "toml");
        assert!(r.code_snippets[0].code.contains("key"));
    }

    #[test]
    fn with_doc_appends_link() {
        let link = DocumentLink {
            title: "Guide".to_string(),
            url: "#guide".to_string(),
            category: "Tutorial".to_string(),
        };
        let r = AssistantResponse::new().with_doc(link);
        assert_eq!(r.related_docs.len(), 1);
        assert_eq!(r.related_docs[0].title, "Guide");
    }

    #[test]
    fn builder_chain_all_fields() {
        let r = AssistantResponse::new()
            .with_text("text")
            .with_action(SuggestedAction::ClearEventLog)
            .with_code(CodeSnippet {
                language: "rust".to_string(),
                code: "let x = 1;".to_string(),
                description: "Rust snippet".to_string(),
            })
            .with_doc(DocumentLink {
                title: "Doc".to_string(),
                url: "#doc".to_string(),
                category: "Reference".to_string(),
            });
        assert_eq!(r.text, "text");
        assert_eq!(r.actions.len(), 1);
        assert_eq!(r.code_snippets.len(), 1);
        assert_eq!(r.related_docs.len(), 1);
    }

    #[test]
    fn default_equals_new() {
        let a = AssistantResponse::default();
        let b = AssistantResponse::new();
        assert_eq!(a.text, b.text);
        assert_eq!(a.actions.len(), b.actions.len());
    }

    #[test]
    fn response_serde_roundtrip() {
        let r = AssistantResponse::simple("serde test")
            .with_code(CodeSnippet {
                language: "json".to_string(),
                code: "{}".to_string(),
                description: "empty".to_string(),
            });
        let json = serde_json::to_string(&r).unwrap();
        let r2: AssistantResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.text, "serde test");
        assert_eq!(r2.code_snippets[0].language, "json");
    }

    #[test]
    fn code_snippet_fields_accessible() {
        let s = CodeSnippet {
            language: "python".to_string(),
            code: "print('hi')".to_string(),
            description: "Hello".to_string(),
        };
        assert_eq!(s.language, "python");
        assert_eq!(s.code, "print('hi')");
        assert_eq!(s.description, "Hello");
    }

    #[test]
    fn document_link_fields_accessible() {
        let d = DocumentLink {
            title: "WasmEdge Docs".to_string(),
            url: "https://wasmedge.org".to_string(),
            category: "External".to_string(),
        };
        assert_eq!(d.title, "WasmEdge Docs");
        assert_eq!(d.url, "https://wasmedge.org");
        assert_eq!(d.category, "External");
    }
}
