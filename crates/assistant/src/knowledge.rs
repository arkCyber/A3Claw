//! Knowledge base for assistant

use crate::{Intent, Result};
use openclaw_config::RagConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocumentCategory {
    Configuration,
    Troubleshooting,
    BestPractices,
    API,
    Tutorial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub category: DocumentCategory,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub priority: u8,
}

pub struct KnowledgeBase {
    pub documents: Vec<Document>,
    tag_index: HashMap<String, Vec<usize>>,
}

impl KnowledgeBase {
    pub fn new() -> Result<Self> {
        let documents = Self::load_builtin_documents();
        let tag_index = Self::build_tag_index(&documents);
        
        Ok(Self {
            documents,
            tag_index,
        })
    }
    
    pub fn from_rag_config(rag_config: &RagConfig) -> Result<Self> {
        let mut kb = Self::new()?;

        // Index individual files — actually read their content
        for file in &rag_config.files {
            if !file.enabled {
                continue;
            }
            let content = match std::fs::read_to_string(&file.file_path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        "[RAG] Could not read file {}: {}",
                        file.file_path.display(),
                        e
                    );
                    continue;
                }
            };
            // Chunk content at ~2000 chars to stay within context limits
            let chunks = Self::chunk_text(&content, 2000, 200);
            for (i, chunk) in chunks.iter().enumerate() {
                kb.add_document(Document {
                    id: format!("rag-file-{}-{}", file.name, i),
                    category: DocumentCategory::Tutorial,
                    title: if chunks.len() == 1 {
                        file.name.clone()
                    } else {
                        format!("{} (part {}/{})", file.name, i + 1, chunks.len())
                    },
                    content: chunk.clone(),
                    tags: file.tags.iter().cloned()
                        .chain(std::iter::once(file.content_type.clone()))
                        .collect(),
                    priority: file.priority,
                });
            }
        }

        // Index folders — walk each folder and read matching files
        for folder in &rag_config.folders {
            if !folder.watch_enabled && folder.indexed_file_count == 0 {
                // Register folder metadata only
                kb.add_document(Document {
                    id: format!("rag-folder-meta-{}", folder.name),
                    category: DocumentCategory::Configuration,
                    title: format!("Knowledge folder: {}", folder.name),
                    content: format!(
                        "Knowledge base folder at {}\nWatch enabled: {}\nAgent write: {}",
                        folder.host_path.display(),
                        folder.watch_enabled,
                        folder.allow_agent_write
                    ),
                    tags: vec!["rag".to_string(), "folder".to_string()],
                    priority: 3,
                });
                continue;
            }
            // Walk folder and load supported files
            if let Ok(entries) = std::fs::read_dir(&folder.host_path) {
                let mut file_count = 0usize;
                for entry in entries.flatten() {
                    if file_count >= 50 {
                        break; // guard against huge folders
                    }
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    if !folder.should_index_file(&path) {
                        continue;
                    }
                    let Ok(content) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("file")
                        .to_string();
                    let chunks = Self::chunk_text(&content, 2000, 200);
                    for (i, chunk) in chunks.iter().enumerate() {
                        kb.add_document(Document {
                            id: format!(
                                "rag-folder-{}-{}-{}",
                                folder.name, stem, i
                            ),
                            category: DocumentCategory::Tutorial,
                            title: if chunks.len() == 1 {
                                format!("[{}] {}", folder.name, stem)
                            } else {
                                format!("[{}] {} ({}/{})", folder.name, stem, i + 1, chunks.len())
                            },
                            content: chunk.clone(),
                            tags: vec![
                                "rag".to_string(),
                                folder.name.to_lowercase().replace(' ', "-"),
                            ],
                            priority: 6,
                        });
                    }
                    file_count += 1;
                }
            }
        }

        Ok(kb)
    }

    /// Adds a downloaded document to the knowledge base.
    /// Called when official docs are fetched from a URL.
    pub fn add_downloaded_doc(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
    ) {
        self.add_document(Document {
            id: id.into(),
            category: DocumentCategory::API,
            title: title.into(),
            content: content.into(),
            tags,
            priority: 9,
        });
    }

    /// Splits text into overlapping chunks.
    fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
        if text.len() <= chunk_size {
            return vec![text.to_string()];
        }
        let chars: Vec<char> = text.chars().collect();
        let mut chunks = Vec::new();
        let step = chunk_size.saturating_sub(overlap);
        let mut start = 0;
        while start < chars.len() {
            let end = (start + chunk_size).min(chars.len());
            chunks.push(chars[start..end].iter().collect());
            if end == chars.len() {
                break;
            }
            start += step;
        }
        chunks
    }
    
    pub fn search(&self, query: &str, intent: &Intent) -> Result<Vec<Document>> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<(usize, f32)> = Vec::new();
        
        for (idx, doc) in self.documents.iter().enumerate() {
            let mut score = 0.0;
            
            if doc.title.to_lowercase().contains(&query_lower) {
                score += 3.0;
            }
            
            if doc.content.to_lowercase().contains(&query_lower) {
                score += 1.0;
            }
            
            for tag in &doc.tags {
                if query_lower.contains(tag) {
                    score += 2.0;
                }
            }
            
            if Self::matches_intent(&doc.category, intent) {
                score += 1.5;
            }
            
            score += doc.priority as f32 * 0.1;
            
            if score > 0.0 {
                results.push((idx, score));
            }
        }
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        Ok(results.iter()
            .take(5)
            .map(|(idx, _)| self.documents[*idx].clone())
            .collect())
    }
    
    pub fn add_document(&mut self, doc: Document) {
        let idx = self.documents.len();
        
        for tag in &doc.tags {
            self.tag_index.entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(idx);
        }
        
        self.documents.push(doc);
    }
    
    fn matches_intent(category: &DocumentCategory, intent: &Intent) -> bool {
        use DocumentCategory::*;
        use Intent::*;
        
        matches!(
            (category, intent),
            (Configuration, ConfigureRAG { .. }) |
            (Troubleshooting, DiagnoseError { .. }) |
            (BestPractices, OptimizePerformance { .. }) |
            (BestPractices, SecurityAudit) |
            (API | Tutorial, QueryDocumentation { .. })
        )
    }
    
    fn build_tag_index(documents: &[Document]) -> HashMap<String, Vec<usize>> {
        let mut index = HashMap::new();
        
        for (idx, doc) in documents.iter().enumerate() {
            for tag in &doc.tags {
                index.entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }
        
        index
    }
    
    fn load_builtin_documents() -> Vec<Document> {
        vec![
            Document {
                id: "wasmedge-preopen".to_string(),
                category: DocumentCategory::Configuration,
                title: "WasmEdge Filesystem Preopen Configuration".to_string(),
                content: r#"
WasmEdge requires explicit preopen configuration for filesystem access.

Format: GUEST_PATH:HOST_PATH

Example:
  /workspace:/Users/dev/workspace
  /data:/var/data

Common issues:
- Path doesn't exist on host
- Incorrect order (should be GUEST:HOST)
- Missing parent directories

Best practices:
- Use absolute paths
- Verify paths exist before configuration
- Grant minimal necessary permissions
"#.to_string(),
                tags: vec!["wasmedge".to_string(), "filesystem".to_string(), "preopen".to_string(), "configuration".to_string()],
                priority: 10,
            },
            Document {
                id: "wasi-error-8".to_string(),
                category: DocumentCategory::Troubleshooting,
                title: "WASI Error 8 (EBADF) Troubleshooting".to_string(),
                content: r#"
WASI Error 8: EBADF (Bad File Descriptor)

Root causes:
1. Preopen path doesn't exist on host system
2. Incorrect preopen format (GUEST:HOST order)
3. File permissions issue
4. Path not included in preopen list

Diagnostic steps:
1. Check if host path exists: ls -la /path/to/host
2. Verify preopen format in config
3. Check file permissions: chmod 755 /path
4. Ensure parent directories are accessible

Fix:
1. Create missing directories: mkdir -p /path/to/host
2. Update config with correct format
3. Restart WasmEdge runtime
4. Verify with test file access
"#.to_string(),
                tags: vec!["wasi".to_string(), "error".to_string(), "filesystem".to_string(), "troubleshooting".to_string()],
                priority: 9,
            },
            Document {
                id: "rag-optimization".to_string(),
                category: DocumentCategory::BestPractices,
                title: "RAG Performance Optimization".to_string(),
                content: r#"
RAG Indexing Performance Optimization

Key parameters:
- chunk_size: 512-1024 (default: 1000)
- chunk_overlap: 50-200 (default: 200)
- batch_size: 100-500 (default: 100)
- ocr_enabled: true/false

Optimization strategies:

1. For large PDFs (>10MB):
   - Disable OCR (saves 80% time)
   - Increase chunk_size to 1024
   - Set max_file_size_mb: 10

2. For many small files:
   - Increase batch_size to 200-500
   - Reduce chunk_overlap to 50-100
   - Enable parallel processing

3. For better search precision:
   - Reduce chunk_size to 512-800
   - Increase chunk_overlap to 150-200
   - Enable semantic chunking

Monitoring:
- Track indexing duration
- Monitor memory usage
- Check chunk count vs file size
"#.to_string(),
                tags: vec!["rag".to_string(), "optimization".to_string(), "performance".to_string(), "indexing".to_string()],
                priority: 8,
            },
            Document {
                id: "security-production".to_string(),
                category: DocumentCategory::BestPractices,
                title: "Production Security Configuration".to_string(),
                content: r#"
Production Security Best Practices

Network Security:
- Enable network whitelist
- Only allow necessary domains
- Use HTTPS for all external connections
- Set connection timeouts (30-60s)

Filesystem Security:
- Minimal preopen paths
- Read-only access by default
- Separate workspace directories
- No access to system directories

Resource Limits:
- Max memory: 512MB-2GB
- Max execution time: 30-300s
- Max file size: 50-100MB
- Max concurrent agents: 5-10

Sensitive Operations:
- Require user confirmation for:
  * Python code execution
  * SSH connections
  * File system writes
  * Network requests
  * Document conversion

Monitoring:
- Log all security events
- Alert on policy violations
- Regular security audits
- Update whitelist as needed
"#.to_string(),
                tags: vec!["security".to_string(), "production".to_string(), "best-practices".to_string(), "configuration".to_string()],
                priority: 10,
            },
            Document {
                id: "wasi-error-2".to_string(),
                category: DocumentCategory::Troubleshooting,
                title: "WASI Error 2 (ENOENT) Troubleshooting".to_string(),
                content: r#"
WASI Error 2: ENOENT (No Such File or Directory)

Common causes:
1. File or directory doesn't exist
2. Typo in file path
3. Relative path without proper base
4. File deleted after configuration

Diagnostic steps:
1. Verify path exists: ls /full/path/to/file
2. Check for typos in configuration
3. Ensure using absolute paths
4. Verify file wasn't deleted

Fix:
1. Create missing file/directory
2. Correct path in configuration
3. Use absolute paths instead of relative
4. Ensure preopen includes parent directory
"#.to_string(),
                tags: vec!["wasi".to_string(), "error".to_string(), "filesystem".to_string(), "troubleshooting".to_string()],
                priority: 8,
            },
            Document {
                id: "wasi-error-13".to_string(),
                category: DocumentCategory::Troubleshooting,
                title: "WASI Error 13 (EACCES) Troubleshooting".to_string(),
                content: r#"
WASI Error 13: EACCES (Permission Denied)

The process attempted to access a resource it does not have permission for.

Common causes:
1. File/directory permissions are too restrictive (chmod)
2. Preopen path is configured as read-only but write was attempted
3. Running as wrong user
4. SELinux / AppArmor policy blocking access

Diagnostic steps:
1. Check file permissions: ls -la /path/to/resource
2. Verify preopen config allows the required access mode
3. Check `allow_agent_write` flag for RAG folders
4. Confirm the host user has read/write access

Fix:
1. chmod 755 /path/to/directory  OR  chmod 644 /path/to/file
2. Set allow_agent_write = true in RAG folder config if write is needed
3. Ensure the WasmEdge process runs with sufficient privileges
"#.to_string(),
                tags: vec!["wasi".to_string(), "error".to_string(), "permission".to_string(), "troubleshooting".to_string()],
                priority: 8,
            },
            Document {
                id: "wasi-error-22".to_string(),
                category: DocumentCategory::Troubleshooting,
                title: "WASI Error 22 (EINVAL) Troubleshooting".to_string(),
                content: r#"
WASI Error 22: EINVAL (Invalid Argument)

An invalid argument was passed to a system call.

Common causes:
1. Incorrect preopen format (must be GUEST:HOST, not HOST:GUEST)
2. Invalid file descriptor passed to WASI call
3. Unsupported flag or option in syscall
4. Path contains invalid characters

Diagnostic steps:
1. Double-check the preopen format: it must be "GUEST_PATH:HOST_PATH"
2. Verify no null bytes or control characters in paths
3. Check WasmEdge version compatibility

Fix:
1. Correct preopen format: /workspace:/home/user/workspace
2. Remove invalid characters from path
3. Update WasmEdge to latest stable version
"#.to_string(),
                tags: vec!["wasi".to_string(), "error".to_string(), "invalid".to_string(), "troubleshooting".to_string()],
                priority: 7,
            },
            Document {
                id: "policy-denied".to_string(),
                category: DocumentCategory::Troubleshooting,
                title: "PolicyDenied Event — Security Policy Violation".to_string(),
                content: r#"
PolicyDenied Security Event

The sandbox rejected an operation because it violates the active security policy.

Event types that trigger PolicyDenied:
- Network request to a host not in network_allowlist
- Filesystem write to a read-only preopen path
- Skill execution (Python, SSH, document conversion) without user confirmation
- Resource limit exceeded (memory, execution time, file size)

How to investigate:
1. Open the Dashboard event log
2. Look for PolicyDenied entries with operation details
3. Note the blocked host, path, or skill name

Resolution:
- Network block: Add host to security.network_allowlist
- Write block: Set allow_agent_write = true on the RAG folder
- Skill block: Add skill to the allowed_without_confirmation list
- Resource block: Increase limits in Security Settings

Example config fix:
[security]
network_allowlist = ["api.openai.com", "blocked-host.example.com"]
max_execution_time_secs = 120
"#.to_string(),
                tags: vec![
                    "policydenied".to_string(), "security".to_string(),
                    "policy".to_string(), "denied".to_string(), "troubleshooting".to_string(),
                ],
                priority: 9,
            },
            Document {
                id: "network-timeout".to_string(),
                category: DocumentCategory::Troubleshooting,
                title: "Network Timeout and Connectivity Issues".to_string(),
                content: r#"
Network Timeout Troubleshooting

Symptoms:
- Agent hangs and eventually fails with "timeout" or "timed out"
- Connection refused error
- DNS resolution failure
- TLS handshake failure

Common causes:
1. Target host is slow or unreachable
2. max_execution_time_secs is too low
3. DNS resolution fails inside the sandbox
4. TLS certificate issue on the server

Fix steps:
1. Increase execution time limit:
   max_execution_time_secs = 120  (default: 30)

2. Verify host reachability from the host machine:
   curl -v https://target-host.example.com

3. Check if the host is in network_allowlist

4. For TLS errors: ensure system CA certificates are accessible
   - Add /etc/ssl/certs as a preopen path

5. For DNS issues: try using an IP address instead of hostname
"#.to_string(),
                tags: vec![
                    "timeout".to_string(), "network".to_string(), "tls".to_string(),
                    "dns".to_string(), "connection".to_string(), "troubleshooting".to_string(),
                ],
                priority: 8,
            },
            Document {
                id: "rag-config-guide".to_string(),
                category: DocumentCategory::Configuration,
                title: "RAG Knowledge Base Configuration Guide".to_string(),
                content: r#"
RAG Knowledge Base Configuration

The RAG (Retrieval-Augmented Generation) system indexes local files and folders
so the AI assistant can reference them when answering queries.

Adding a folder:
1. Click "Add Folder" in the Assistant Settings
2. Select a directory containing your documentation
3. Configure watch_enabled = true for automatic re-indexing
4. Set allow_agent_write = false unless the agent needs to write back

Adding individual files:
1. Click "Add File" to pick specific files
2. Supported formats: .md, .txt, .pdf, .rst, .adoc
3. Files are chunked at ~2000 chars with 200-char overlap

Indexing settings (Settings → RAG):
- chunk_size: 512–2000 (default: 1000) — smaller = more precise recall
- chunk_overlap: 50–400 (default: 200) — higher = better context continuity
- ocr_enabled: true/false — disable for non-scanned PDFs to speed up indexing
- max_file_size_mb: limits files larger than this value

Download official docs:
Click "Download Official Docs" to fetch WasmEdge and OpenClaw reference docs
from the official website directly into the knowledge base.
"#.to_string(),
                tags: vec![
                    "rag".to_string(), "configuration".to_string(), "knowledge".to_string(),
                    "indexing".to_string(), "folder".to_string(),
                ],
                priority: 9,
            },
        ]
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new().expect("Failed to create default knowledge base")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_knowledge_base_creation() {
        let kb = KnowledgeBase::new().unwrap();
        assert!(!kb.documents.is_empty());
    }
    
    #[test]
    fn test_search_wasmedge() {
        let kb = KnowledgeBase::new().unwrap();
        let intent = Intent::DiagnoseError {
            error_type: "wasi error 8".to_string(),
        };
        
        let results = kb.search("wasi error 8", &intent).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].title.contains("WASI Error 8"));
    }
    
    #[test]
    fn test_search_rag_optimization() {
        let kb = KnowledgeBase::new().unwrap();
        let intent = Intent::OptimizePerformance {
            target: "RAG".to_string(),
        };
        
        let results = kb.search("RAG slow indexing", &intent).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|d| d.tags.contains(&"rag".to_string())));
    }
    
    #[test]
    fn test_add_document() {
        let mut kb = KnowledgeBase::new().unwrap();
        let initial_count = kb.documents.len();
        
        kb.add_document(Document {
            id: "test-doc".to_string(),
            category: DocumentCategory::Tutorial,
            title: "Test Document".to_string(),
            content: "Test content".to_string(),
            tags: vec!["test".to_string()],
            priority: 5,
        });
        
        assert_eq!(kb.documents.len(), initial_count + 1);
    }

    #[test]
    fn test_chunk_text_short() {
        let text = "hello world";
        let chunks = KnowledgeBase::chunk_text(text, 2000, 200);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_chunk_text_long() {
        let text = "a".repeat(5000);
        let chunks = KnowledgeBase::chunk_text(&text, 2000, 200);
        assert!(chunks.len() >= 3);
        assert!(chunks[0].len() <= 2000);
        for chunk in &chunks {
            assert!(!chunk.is_empty());
        }
        // Verify all content is covered — concatenation with overlap
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert!(total >= 5000);
    }

    #[test]
    fn test_chunk_text_exact_boundary() {
        let text = "x".repeat(2000);
        let chunks = KnowledgeBase::chunk_text(&text, 2000, 200);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_add_downloaded_doc() {
        let mut kb = KnowledgeBase::new().unwrap();
        let initial_count = kb.documents.len();

        kb.add_downloaded_doc(
            "official-wasmedge-v1",
            "WasmEdge Official Guide",
            "WasmEdge is a lightweight, high-performance WebAssembly runtime.",
            vec!["wasmedge".to_string(), "official".to_string()],
        );

        assert_eq!(kb.documents.len(), initial_count + 1);
        let added = kb.documents.last().unwrap();
        assert_eq!(added.id, "official-wasmedge-v1");
        assert_eq!(added.priority, 9);
        assert!(matches!(added.category, DocumentCategory::API));
        assert!(added.tags.contains(&"official".to_string()));
    }

    #[test]
    fn test_search_downloaded_doc() {
        let mut kb = KnowledgeBase::new().unwrap();
        kb.add_downloaded_doc(
            "wasmedge-rust-sdk",
            "WasmEdge Rust SDK Reference",
            "Use WasmEdge Rust SDK to embed WasmEdge into your Rust application. \
             The SDK provides Host Function support.",
            vec!["wasmedge".to_string(), "rust".to_string(), "sdk".to_string()],
        );

        let intent = Intent::QueryDocumentation {
            topic: "rust sdk".to_string(),
        };
        let results = kb.search("WasmEdge Rust SDK", &intent).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|d| d.id == "wasmedge-rust-sdk"));
    }

    #[test]
    fn test_from_rag_config_empty() {
        let config = RagConfig::default();
        let kb = KnowledgeBase::from_rag_config(&config).unwrap();
        // Builtin docs should still be present
        assert!(!kb.documents.is_empty());
    }

    #[test]
    fn test_from_rag_config_with_nonexistent_file() {
        use openclaw_config::{RagFile, RagConfig};
        let mut config = RagConfig::default();
        let rag_file = RagFile::new(
            std::path::PathBuf::from("/nonexistent/path/doc.txt"),
            "Nonexistent Doc",
            "txt",
        );
        config.add_file(rag_file);
        // Should not panic — unreadable files are skipped
        let kb = KnowledgeBase::from_rag_config(&config).unwrap();
        assert!(!kb.documents.is_empty());
        // The nonexistent file should not have been indexed
        assert!(!kb.documents.iter().any(|d| d.title == "Nonexistent Doc"));
    }
}
