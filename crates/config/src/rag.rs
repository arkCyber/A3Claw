//! RAG (Retrieval-Augmented Generation) Configuration Module
//! 
//! This module handles all RAG-related configuration including:
//! - Knowledge base folder management
//! - Individual file management  
//! - Indexing status and metadata
//! - File watching and synchronization

use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// Comprehensive RAG configuration for knowledge base management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RagConfig {
    /// Knowledge base folders for batch indexing
    #[serde(default)]
    pub folders: Vec<RagFolder>,
    
    /// Individual files for granular control
    #[serde(default)]
    pub files: Vec<RagFile>,
    
    /// Global RAG settings
    #[serde(default)]
    pub settings: RagSettings,
}

/// RAG knowledge base folder configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagFolder {
    /// Absolute path on the host machine
    pub host_path: PathBuf,
    
    /// Human-readable name for UI display
    pub name: String,
    
    /// Optional description of the knowledge base content
    pub description: String,
    
    /// File extensions to include in indexing (empty = all files)
    pub include_extensions: Vec<String>,
    
    /// Whether the agent can write to this folder
    pub allow_agent_write: bool,
    
    /// Maximum total size limit (MB), None = unlimited
    pub max_size_mb: Option<u32>,
    
    /// Whether this folder is actively watched for changes
    pub watch_enabled: bool,
    
    /// Indexing status and metadata
    #[serde(default)]
    pub indexing_status: IndexingStatus,
    
    /// When this folder was last indexed
    pub last_indexed: Option<SystemTime>,
    
    /// Total number of files indexed
    pub indexed_file_count: u64,
    
    /// Total size of indexed content in bytes
    pub indexed_size_bytes: u64,
}

/// Individual RAG file configuration for granular control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagFile {
    /// Absolute path to the file
    pub file_path: PathBuf,
    
    /// Display name for the UI
    pub name: String,
    
    /// File content type (pdf, md, txt, docx, etc.)
    pub content_type: String,
    
    /// File size in bytes
    pub size_bytes: u64,
    
    /// Content hash for change detection
    pub content_hash: Option<String>,
    
    /// Indexing status
    #[serde(default)]
    pub indexing_status: IndexingStatus,
    
    /// When this file was last indexed
    pub last_indexed: Option<SystemTime>,
    
    /// Indexing error details if any
    pub indexing_error: Option<String>,
    
    /// User-defined tags for categorization
    pub tags: Vec<String>,
    
    /// Priority for search results (higher = more relevant)
    pub priority: u8,
    
    /// Whether this file is enabled for search
    pub enabled: bool,
}

/// Indexing status for RAG content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum IndexingStatus {
    /// Not yet indexed
    #[default]
    Pending,
    
    /// Currently being indexed
    Indexing {
        /// Progress percentage (0-100)
        progress: u8,
        /// Current operation description
        operation: String,
        /// Started timestamp
        started_at: SystemTime,
    },
    
    /// Successfully indexed
    Indexed {
        /// When indexing completed
        completed_at: SystemTime,
        /// Number of chunks/vectors created
        chunk_count: u64,
        /// Indexing duration in milliseconds
        duration_ms: u64,
    },
    
    /// Indexing failed
    Failed {
        /// When the failure occurred
        failed_at: SystemTime,
        /// Error message
        error: String,
        /// Whether to retry automatically
        should_retry: bool,
    },
    
    /// Needs reindexing (content changed)
    NeedsReindex {
        /// Reason for reindexing
        reason: String,
        /// When the change was detected
        detected_at: SystemTime,
    },
}

/// Global RAG settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagSettings {
    /// Maximum total size for all RAG content (MB)
    pub max_total_size_mb: Option<u32>,
    
    /// Maximum file size for individual files (MB)
    pub max_file_size_mb: Option<u32>,
    
    /// Supported file extensions
    pub supported_extensions: Vec<String>,
    
    /// Whether to automatically index new files
    pub auto_index_enabled: bool,
    
    /// Indexing batch size
    pub batch_size: u32,
    
    /// Chunk size for text processing
    pub chunk_size: usize,
    
    /// Overlap between chunks
    pub chunk_overlap: usize,
    
    /// Whether to use OCR for PDFs and images
    pub ocr_enabled: bool,
    
    /// Vector embedding model to use
    pub embedding_model: String,
}


impl RagConfig {
    /// Returns the platform-specific path for the RAG config file.
    /// Path: `{config_dir}/openclaw-plus/rag.toml`
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openclaw-plus")
            .join("rag.toml")
    }

    /// Loads from disk or returns default. Never panics.
    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => toml::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    /// Saves to disk, logging on error instead of panicking.
    pub fn save_or_log(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::warn!("[RagConfig] Could not create config dir: {e}");
                return;
            }
        }
        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    tracing::warn!("[RagConfig] Could not save rag.toml: {e}");
                }
            }
            Err(e) => tracing::warn!("[RagConfig] Serialization failed: {e}"),
        }
    }

    /// Adds a file to the knowledge base if not already present (by path).
    /// Returns `true` if added, `false` if the path was a duplicate.
    pub fn add_file(&mut self, file: RagFile) -> bool {
        if self.files.iter().any(|f| f.file_path == file.file_path) {
            return false;
        }
        self.files.push(file);
        true
    }

    /// Removes the file at `index`. Returns `Some(RagFile)` on success,
    /// `None` if the index is out of bounds.
    pub fn remove_file(&mut self, index: usize) -> Option<RagFile> {
        if index < self.files.len() {
            Some(self.files.remove(index))
        } else {
            None
        }
    }

    /// Replaces the current `RagSettings` with `new_settings`.
    pub fn update_settings(&mut self, new_settings: RagSettings) {
        self.settings = new_settings;
    }

    /// Adds a folder if not already present (by host_path). Returns `true` if added.
    pub fn add_folder(&mut self, folder: RagFolder) -> bool {
        if self.folders.iter().any(|f| f.host_path == folder.host_path) {
            return false;
        }
        self.folders.push(folder);
        true
    }

    /// Removes the folder at `index`. Returns `Some(RagFolder)` on success.
    pub fn remove_folder(&mut self, index: usize) -> Option<RagFolder> {
        if index < self.folders.len() {
            Some(self.folders.remove(index))
        } else {
            None
        }
    }
}

impl Default for RagSettings {
    fn default() -> Self {
        Self {
            max_total_size_mb: Some(1024), // 1GB default
            max_file_size_mb: Some(50),    // 50MB default
            supported_extensions: vec![
                "txt".to_string(),
                "md".to_string(), 
                "pdf".to_string(),
                "docx".to_string(),
                "doc".to_string(),
                "rtf".to_string(),
                "html".to_string(),
                "htm".to_string(),
                "csv".to_string(),
                "json".to_string(),
                "xml".to_string(),
            ],
            auto_index_enabled: true,
            batch_size: 100,
            chunk_size: 1000,
            chunk_overlap: 200,
            ocr_enabled: true,
            embedding_model: "default".to_string(),
        }
    }
}

impl RagFolder {
    /// Creates a new RAG folder with sensible defaults
    pub fn new(host_path: PathBuf, name: &str) -> Self {
        Self {
            host_path,
            name: name.to_string(),
            description: String::new(),
            include_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "pdf".to_string(),
                "docx".to_string(),
            ],
            allow_agent_write: false,
            max_size_mb: Some(500),
            watch_enabled: true,
            indexing_status: IndexingStatus::Pending,
            last_indexed: None,
            indexed_file_count: 0,
            indexed_size_bytes: 0,
        }
    }
    
    /// Returns true if the given file should be included in RAG indexing
    pub fn should_index_file(&self, file_path: &std::path::Path) -> bool {
        // Check if file is within the folder path
        if !file_path.starts_with(&self.host_path) {
            return false;
        }
        
        // If no extensions specified, include all files
        if self.include_extensions.is_empty() {
            return true;
        }
        
        // Check file extension
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            self.include_extensions.contains(&ext.to_lowercase())
        } else {
            false
        }
    }
    
    /// Gets the current indexing progress (0-100)
    pub fn get_indexing_progress(&self) -> u8 {
        match &self.indexing_status {
            IndexingStatus::Indexing { progress, .. } => *progress,
            IndexingStatus::Indexed { .. } => 100,
            _ => 0,
        }
    }
    
    /// Checks if the folder needs reindexing
    pub fn needs_reindexing(&self) -> bool {
        matches!(self.indexing_status, IndexingStatus::NeedsReindex { .. })
    }
}

impl RagFile {
    /// Creates a new RAG file entry
    pub fn new(file_path: PathBuf, name: &str, content_type: &str) -> Self {
        Self {
            size_bytes: 0, // Will be calculated when file is processed
            content_hash: None,
            indexing_status: IndexingStatus::Pending,
            last_indexed: None,
            indexing_error: None,
            tags: Vec::new(),
            priority: 50, // Default priority
            enabled: true,
            file_path,
            name: name.to_string(),
            content_type: content_type.to_string(),
        }
    }
    
    /// Gets the current indexing progress (0-100)
    pub fn get_indexing_progress(&self) -> u8 {
        match &self.indexing_status {
            IndexingStatus::Indexing { progress, .. } => *progress,
            IndexingStatus::Indexed { .. } => 100,
            _ => 0,
        }
    }
    
    /// Checks if the file has indexing errors
    pub fn has_indexing_errors(&self) -> bool {
        matches!(self.indexing_status, IndexingStatus::Failed { .. })
    }
    
    /// Gets a human-readable status description
    pub fn get_status_description(&self) -> &'static str {
        match &self.indexing_status {
            IndexingStatus::Pending => "Pending",
            IndexingStatus::Indexing { .. } => "Indexing",
            IndexingStatus::Indexed { .. } => "Indexed",
            IndexingStatus::Failed { .. } => "Failed",
            IndexingStatus::NeedsReindex { .. } => "Needs Reindex",
        }
    }
    
    /// Checks if the file needs reindexing
    pub fn needs_reindexing(&self) -> bool {
        matches!(self.indexing_status, IndexingStatus::NeedsReindex { .. })
    }
}

impl IndexingStatus {
    /// Checks if indexing is currently in progress
    pub fn is_indexing(&self) -> bool {
        matches!(self, IndexingStatus::Indexing { .. })
    }
    
    /// Checks if indexing is complete and successful
    pub fn is_successfully_indexed(&self) -> bool {
        matches!(self, IndexingStatus::Indexed { .. })
    }
    
    /// Checks if indexing has failed
    pub fn has_failed(&self) -> bool {
        matches!(self, IndexingStatus::Failed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rag_folder_new_defaults() {
        let folder = RagFolder::new(PathBuf::from("/docs"), "Docs");
        
        assert_eq!(folder.name, "Docs");
        assert_eq!(folder.host_path, PathBuf::from("/docs"));
        assert!(folder.watch_enabled);
        assert!(!folder.allow_agent_write);
        assert_eq!(folder.get_indexing_progress(), 0);
    }
    
    #[test]
    fn test_rag_folder_should_index_file() {
        let folder = RagFolder::new(PathBuf::from("/docs"), "Docs");
        
        // File within folder with supported extension
        assert!(folder.should_index_file(&PathBuf::from("/docs/test.txt")));
        assert!(folder.should_index_file(&PathBuf::from("/docs/subdir/file.md")));
        
        // File outside folder
        assert!(!folder.should_index_file(&PathBuf::from("/other/test.txt")));
        
        // File with unsupported extension
        assert!(!folder.should_index_file(&PathBuf::from("/docs/test.exe")));
    }
    
    #[test]
    fn test_rag_folder_empty_extensions_indexes_all() {
        let mut folder = RagFolder::new(PathBuf::from("/docs"), "Docs");
        folder.include_extensions.clear(); // Empty = index all
        
        assert!(folder.should_index_file(&PathBuf::from("/docs/test.txt")));
        assert!(folder.should_index_file(&PathBuf::from("/docs/test.exe")));
        assert!(folder.should_index_file(&PathBuf::from("/docs/test.any")));
    }
    
    #[test]
    fn test_rag_file_new() {
        let file = RagFile::new(
            PathBuf::from("/docs/test.pdf"),
            "Test Document",
            "pdf"
        );
        
        assert_eq!(file.name, "Test Document");
        assert_eq!(file.content_type, "pdf");
        assert_eq!(file.file_path, PathBuf::from("/docs/test.pdf"));
        assert!(file.enabled);
        assert_eq!(file.priority, 50);
        assert_eq!(file.get_indexing_progress(), 0);
    }
    
    #[test]
    fn test_indexing_status() {
        let now = SystemTime::now();
        
        let pending = IndexingStatus::Pending;
        assert!(!pending.is_indexing());
        assert!(!pending.is_successfully_indexed());
        assert!(!pending.has_failed());
        
        let indexing = IndexingStatus::Indexing {
            progress: 50,
            operation: "Processing".to_string(),
            started_at: now,
        };
        assert!(indexing.is_indexing());
        assert!(!indexing.is_successfully_indexed());
        assert!(!indexing.has_failed());
        
        let indexed = IndexingStatus::Indexed {
            completed_at: now,
            chunk_count: 10,
            duration_ms: 1000,
        };
        assert!(!indexed.is_indexing());
        assert!(indexed.is_successfully_indexed());
        assert!(!indexed.has_failed());
        
        let failed = IndexingStatus::Failed {
            failed_at: now,
            error: "Test error".to_string(),
            should_retry: false,
        };
        assert!(!failed.is_indexing());
        assert!(!failed.is_successfully_indexed());
        assert!(failed.has_failed());
    }
    
    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        
        assert!(config.folders.is_empty());
        assert!(config.files.is_empty());
        assert!(config.settings.auto_index_enabled);
        assert_eq!(config.settings.chunk_size, 1000);
        assert_eq!(config.settings.supported_extensions.len(), 11);
    }
    
    #[test]
    fn test_rag_settings_default() {
        let settings = RagSettings::default();
        
        assert_eq!(settings.max_total_size_mb, Some(1024));
        assert_eq!(settings.max_file_size_mb, Some(50));
        assert!(settings.supported_extensions.contains(&"pdf".to_string()));
        assert!(settings.auto_index_enabled);
        assert_eq!(settings.batch_size, 100);
    }

    // ── Serialization / Deserialization ─────────────────────────────────────

    #[test]
    fn test_rag_config_round_trip_toml() {
        let mut cfg = RagConfig::default();
        cfg.folders.push(RagFolder::new(PathBuf::from("/tmp/docs"), "Docs"));
        cfg.settings.chunk_size = 512;
        cfg.settings.ocr_enabled = false;

        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
        let restored: RagConfig = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(restored.folders.len(), 1);
        assert_eq!(restored.folders[0].name, "Docs");
        assert_eq!(restored.settings.chunk_size, 512);
        assert!(!restored.settings.ocr_enabled);
    }

    #[test]
    fn test_rag_file_round_trip_toml() {
        let file = RagFile::new(PathBuf::from("/tmp/report.pdf"), "Report", "pdf");
        let mut cfg = RagConfig::default();
        cfg.files.push(file.clone());

        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
        let restored: RagConfig = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(restored.files.len(), 1);
        assert_eq!(restored.files[0].name, "Report");
        assert_eq!(restored.files[0].content_type, "pdf");
        assert!(restored.files[0].enabled);
    }

    #[test]
    fn test_rag_config_deserialize_missing_optional_fields() {
        let minimal = r#"
[settings]
chunk_size = 500
chunk_overlap = 100
auto_index_enabled = true
batch_size = 50
ocr_enabled = false
embedding_model = "default"
supported_extensions = ["txt"]
"#;
        let cfg: RagConfig = toml::from_str(minimal).expect("deserialize minimal");
        assert_eq!(cfg.settings.chunk_size, 500);
        assert!(cfg.folders.is_empty());
        assert!(cfg.files.is_empty());
        assert_eq!(cfg.settings.max_total_size_mb, None);
    }

    #[test]
    fn test_indexing_status_round_trip() {
        let now = SystemTime::now();
        let status = IndexingStatus::Indexed {
            completed_at: now,
            chunk_count: 42,
            duration_ms: 200,
        };
        let json = serde_json::to_string(&status).expect("json serialize");
        let restored: IndexingStatus = serde_json::from_str(&json).expect("json deserialize");
        assert!(restored.is_successfully_indexed());
    }

    // ── RagConfig::add_file / remove_file ───────────────────────────────────

    #[test]
    fn test_add_file_succeeds_for_new_path() {
        let mut cfg = RagConfig::default();
        let f = RagFile::new(PathBuf::from("/tmp/a.txt"), "A", "txt");
        assert!(cfg.add_file(f));
        assert_eq!(cfg.files.len(), 1);
    }

    #[test]
    fn test_add_file_rejects_duplicate_path() {
        let mut cfg = RagConfig::default();
        let f1 = RagFile::new(PathBuf::from("/tmp/a.txt"), "A", "txt");
        let f2 = RagFile::new(PathBuf::from("/tmp/a.txt"), "A-dup", "txt");
        assert!(cfg.add_file(f1));
        assert!(!cfg.add_file(f2)); // duplicate → false
        assert_eq!(cfg.files.len(), 1);
    }

    #[test]
    fn test_remove_file_valid_index() {
        let mut cfg = RagConfig::default();
        cfg.add_file(RagFile::new(PathBuf::from("/tmp/a.txt"), "A", "txt"));
        cfg.add_file(RagFile::new(PathBuf::from("/tmp/b.txt"), "B", "txt"));

        let removed = cfg.remove_file(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "A");
        assert_eq!(cfg.files.len(), 1);
        assert_eq!(cfg.files[0].name, "B");
    }

    #[test]
    fn test_remove_file_out_of_bounds_returns_none() {
        let mut cfg = RagConfig::default();
        assert!(cfg.remove_file(0).is_none());
        cfg.add_file(RagFile::new(PathBuf::from("/tmp/a.txt"), "A", "txt"));
        assert!(cfg.remove_file(5).is_none());
        assert_eq!(cfg.files.len(), 1); // unchanged
    }

    // ── RagConfig::update_settings ──────────────────────────────────────────

    #[test]
    fn test_update_settings_replaces_entire_settings() {
        let mut cfg = RagConfig::default();
        assert_eq!(cfg.settings.chunk_size, 1000);

        let mut new_settings = RagSettings::default();
        new_settings.chunk_size = 256;
        new_settings.auto_index_enabled = false;
        new_settings.max_total_size_mb = None;

        cfg.update_settings(new_settings);

        assert_eq!(cfg.settings.chunk_size, 256);
        assert!(!cfg.settings.auto_index_enabled);
        assert!(cfg.settings.max_total_size_mb.is_none());
    }

    // ── RagConfig::add_folder / remove_folder ───────────────────────────────

    #[test]
    fn test_add_folder_succeeds_for_new_path() {
        let mut cfg = RagConfig::default();
        let f = RagFolder::new(PathBuf::from("/home/docs"), "Docs");
        assert!(cfg.add_folder(f));
        assert_eq!(cfg.folders.len(), 1);
    }

    #[test]
    fn test_add_folder_rejects_duplicate_path() {
        let mut cfg = RagConfig::default();
        let f1 = RagFolder::new(PathBuf::from("/home/docs"), "Docs");
        let f2 = RagFolder::new(PathBuf::from("/home/docs"), "Docs-dup");
        assert!(cfg.add_folder(f1));
        assert!(!cfg.add_folder(f2));
        assert_eq!(cfg.folders.len(), 1);
    }

    #[test]
    fn test_remove_folder_valid_index() {
        let mut cfg = RagConfig::default();
        cfg.add_folder(RagFolder::new(PathBuf::from("/a"), "A"));
        cfg.add_folder(RagFolder::new(PathBuf::from("/b"), "B"));

        let removed = cfg.remove_folder(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "A");
        assert_eq!(cfg.folders.len(), 1);
        assert_eq!(cfg.folders[0].name, "B");
    }

    #[test]
    fn test_remove_folder_out_of_bounds_returns_none() {
        let mut cfg = RagConfig::default();
        assert!(cfg.remove_folder(99).is_none());
    }

    // ── save_or_log / load_or_default ───────────────────────────────────────

    #[test]
    fn test_save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("rag.toml");

        let mut cfg = RagConfig::default();
        cfg.settings.chunk_size = 777;
        cfg.add_folder(RagFolder::new(PathBuf::from("/test/folder"), "Test"));

        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
        std::fs::write(&path, toml_str).expect("write");

        let loaded: RagConfig = toml::from_str(
            &std::fs::read_to_string(&path).expect("read")
        ).expect("deserialize");

        assert_eq!(loaded.settings.chunk_size, 777);
        assert_eq!(loaded.folders.len(), 1);
        assert_eq!(loaded.folders[0].name, "Test");
    }

    #[test]
    fn test_load_or_default_nonexistent_path_returns_default() {
        // Test the deserialization path for a nonexistent file by directly
        // verifying default is produced when content is absent.
        let result: RagConfig = toml::from_str("").unwrap_or_default();
        assert!(result.folders.is_empty());
        assert_eq!(result.settings.chunk_size, 1000);
    }

    #[test]
    fn test_load_or_default_corrupt_toml_returns_default() {
        let corrupt = "{ this is not valid toml !!!";
        let result: RagConfig = toml::from_str(corrupt).unwrap_or_default();
        assert!(result.folders.is_empty());
        assert_eq!(result.settings.chunk_size, 1000);
    }

    // ── Boundary / edge cases ────────────────────────────────────────────────

    #[test]
    fn test_rag_settings_unlimited_size() {
        let mut s = RagSettings::default();
        s.max_total_size_mb = None;
        s.max_file_size_mb = None;
        let toml_str = toml::to_string_pretty(&s).expect("serialize");
        let restored: RagSettings = toml::from_str(&toml_str).expect("deserialize");
        assert!(restored.max_total_size_mb.is_none());
        assert!(restored.max_file_size_mb.is_none());
    }

    #[test]
    fn test_rag_settings_zero_chunk_overlap() {
        let mut s = RagSettings::default();
        s.chunk_overlap = 0;
        let toml_str = toml::to_string_pretty(&s).expect("serialize");
        let restored: RagSettings = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(restored.chunk_overlap, 0);
    }

    #[test]
    fn test_rag_file_toggle_enabled() {
        let mut f = RagFile::new(PathBuf::from("/tmp/x.md"), "X", "md");
        assert!(f.enabled);
        f.enabled = false;
        assert!(!f.enabled);
    }

    #[test]
    fn test_rag_folder_should_index_extension_filter() {
        let mut folder = RagFolder::new(PathBuf::from("/docs"), "Docs");
        folder.include_extensions = vec!["pdf".to_string(), "md".to_string()];

        assert!(folder.should_index_file(&PathBuf::from("/docs/report.pdf")));
        assert!(folder.should_index_file(&PathBuf::from("/docs/readme.md")));
        assert!(!folder.should_index_file(&PathBuf::from("/docs/image.png")));
    }

    #[test]
    fn test_rag_folder_watch_write_toggles() {
        let mut folder = RagFolder::new(PathBuf::from("/data"), "Data");
        assert!(folder.watch_enabled);
        assert!(!folder.allow_agent_write);

        folder.watch_enabled = false;
        folder.allow_agent_write = true;

        assert!(!folder.watch_enabled);
        assert!(folder.allow_agent_write);
    }
}
