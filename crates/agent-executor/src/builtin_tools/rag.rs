//! RAG -- in-process TF-IDF vector index.
//! Skills: rag.ingest, rag.search, rag.delete, rag.list, rag.status

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use crate::dispatch::SkillHandler;

const CHUNK_SIZE: usize = 400;
const CHUNK_OVERLAP: usize = 80;
const MAX_VOCAB: usize = 4096;
const SNIPPET_CHARS: usize = 350;

// -- Data types ---------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RagChunk {
    pub id: String,
    pub doc_id: String,
    pub chunk_index: usize,
    pub title: String,
    pub source: Option<String>,
    pub text: String,
    pub(crate) vector: HashMap<usize, f32>,
    pub(crate) norm: f32,
    pub ingested_at: u64,
}

#[derive(Debug, Clone)]
pub struct RagDocMeta {
    pub doc_id: String,
    pub title: String,
    pub source: Option<String>,
    pub chunk_count: usize,
    pub total_chars: usize,
    pub ingested_at: u64,
}

#[derive(Debug, Clone)]
pub struct RagSearchResult {
    pub chunk_id: String,
    pub doc_id: String,
    pub title: String,
    pub source: Option<String>,
    pub score: f32,
    pub snippet: String,
}

// -- Vocabulary ---------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct Vocabulary {
    pub term_to_idx: HashMap<String, usize>,
    pub next_id: usize,
}

impl Vocabulary {
    pub fn get_or_insert(&mut self, term: &str) -> usize {
        if let Some(&idx) = self.term_to_idx.get(term) { return idx; }
        if self.next_id >= MAX_VOCAB { return MAX_VOCAB; }
        let idx = self.next_id;
        self.term_to_idx.insert(term.to_string(), idx);
        self.next_id += 1;
        idx
    }
    pub fn get(&self, term: &str) -> Option<usize> {
        self.term_to_idx.get(term).copied()
    }
}

// -- Text utilities -----------------------------------------------------------

pub fn chunk_text(text: &str) -> Vec<String> {
    if text.is_empty() { return vec![]; }
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= CHUNK_SIZE { return vec![text.to_string()]; }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < len {
        let end = (start + CHUNK_SIZE).min(len);
        chunks.push(chars[start..end].iter().collect());
        if end == len { break; }
        start += CHUNK_SIZE - CHUNK_OVERLAP;
    }
    chunks
}

pub fn tokenize(text: &str) -> Vec<String> {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_string())
        .collect()
}

pub fn tfidf_vector(
    text: &str,
    vocab: &mut Vocabulary,
    df: &HashMap<usize, usize>,
    num_docs: usize,
) -> (HashMap<usize, f32>, f32) {
    let tokens = tokenize(text);
    if tokens.is_empty() { return (HashMap::new(), 0.0); }
    let n = tokens.len() as f32;
    let mut tf: HashMap<usize, f32> = HashMap::new();
    for t in &tokens {
        let idx = vocab.get_or_insert(t);
        if idx < MAX_VOCAB { *tf.entry(idx).or_insert(0.0) += 1.0 / n; }
    }
    let nd = num_docs.max(1) as f32;
    let mut vec: HashMap<usize, f32> = HashMap::new();
    for (idx, tv) in &tf {
        let df_c = df.get(idx).copied().unwrap_or(0) as f32;
        let idf = (nd / (df_c + 1.0)).ln() + 1.0;
        vec.insert(*idx, tv * idf);
    }
    let norm = vec.values().map(|v| v * v).sum::<f32>().sqrt();
    (vec, norm)
}

pub fn cosine_similarity(
    a: &HashMap<usize, f32>, an: f32,
    b: &HashMap<usize, f32>, bn: f32,
) -> f32 {
    if an == 0.0 || bn == 0.0 { return 0.0; }
    let dot: f32 = a.iter().filter_map(|(k, v)| b.get(k).map(|bv| v * bv)).sum();
    dot / (an * bn)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn derive_doc_id(title: &str, content: &str) -> String {
    // FNV-1a 32-bit over title bytes
    let title_h = title.bytes().fold(0x811c9dc5u32, |a, b| {
        (a ^ b as u32).wrapping_mul(0x01000193)
    });
    // FNV-1a 32-bit over content bytes
    let content_h = content.bytes().fold(0x811c9dc5u32, |a, b| {
        (a ^ b as u32).wrapping_mul(0x01000193)
    });
    format!("rag_{:08x}{:08x}", title_h, content_h)
}

// -- RagIndex -----------------------------------------------------------------

#[derive(Clone)]
pub struct RagIndex { inner: Arc<RwLock<RagIndexInner>> }

struct RagIndexInner {
    chunks: Vec<RagChunk>,
    vocab: Vocabulary,
    df: HashMap<usize, usize>,
}

impl Default for RagIndex { fn default() -> Self { Self::new() } }

impl RagIndex {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(RagIndexInner {
            chunks: vec![], vocab: Vocabulary::default(), df: HashMap::new(),
        })) }
    }

    pub async fn ingest(&self, content: &str, title: &str, source: Option<&str>) -> (String, usize) {
        let doc_id = derive_doc_id(title, content);
        let now = unix_now();
        let raw = chunk_text(content);
        let num = raw.len();
        let mut inner = self.inner.write().await;
        inner.chunks.retain(|c| c.doc_id != doc_id);
        inner.df.clear();
        let existing_keys: Vec<Vec<usize>> = inner.chunks.iter()
            .map(|c| c.vector.keys().copied().collect())
            .collect();
        for keys in &existing_keys { for &idx in keys { *inner.df.entry(idx).or_insert(0) += 1; } }
        let total = inner.chunks.len() + num;
        for (i, text) in raw.into_iter().enumerate() {
            let df_snap = inner.df.clone();
            let (vector, norm) = tfidf_vector(&text, &mut inner.vocab, &df_snap, total);
            let new_keys: Vec<usize> = vector.keys().copied().collect();
            for idx in new_keys { *inner.df.entry(idx).or_insert(0) += 1; }
            inner.chunks.push(RagChunk {
                id: format!("{doc_id}_{i:04}"),
                doc_id: doc_id.clone(), chunk_index: i,
                title: title.to_string(), source: source.map(|s| s.to_string()),
                text, vector, norm, ingested_at: now,
            });
        }
        (doc_id, num)
    }

    pub async fn search(&self, query: &str, top_k: usize) -> Vec<RagSearchResult> {
        if query.trim().is_empty() || top_k == 0 { return vec![]; }
        // Phase 1: clone vocab+df under read lock so we don't block writers
        let (mut vocab_snap, df_snap, nd) = {
            let inner = self.inner.read().await;
            (inner.vocab.clone(), inner.df.clone(), inner.chunks.len())
        };
        // Compute query vector (may expand vocab snapshot, but NOT the shared one)
        let (qv, qn) = tfidf_vector(query, &mut vocab_snap, &df_snap, nd);
        // Phase 2: score under read lock
        let results = {
            let inner = self.inner.read().await;
            let mut scored: Vec<(f32, usize)> = inner.chunks.iter().enumerate()
                .map(|(i, c)| (cosine_similarity(&qv, qn, &c.vector, c.norm), i))
                .filter(|(s, _)| *s > 0.0)
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.into_iter().take(top_k).map(|(score, i)| {
                let c = &inner.chunks[i];
                RagSearchResult {
                    chunk_id: c.id.clone(), doc_id: c.doc_id.clone(), title: c.title.clone(),
                    source: c.source.clone(), score,
                    snippet: c.text.chars().take(SNIPPET_CHARS).collect(),
                }
            }).collect::<Vec<_>>()
        };
        results
    }

    pub async fn delete(&self, doc_id: &str) -> bool {
        let mut inner = self.inner.write().await;
        let before = inner.chunks.len();
        inner.chunks.retain(|c| c.doc_id != doc_id);
        let removed = inner.chunks.len() < before;
        if removed {
            inner.df.clear();
            let keys_list: Vec<Vec<usize>> = inner.chunks.iter()
                .map(|c| c.vector.keys().copied().collect())
                .collect();
            for keys in keys_list { for idx in keys { *inner.df.entry(idx).or_insert(0) += 1; } }
        }
        removed
    }

    pub async fn list(&self) -> Vec<RagDocMeta> {
        let inner = self.inner.read().await;
        let mut map: HashMap<String, RagDocMeta> = HashMap::new();
        for c in &inner.chunks {
            let e = map.entry(c.doc_id.clone()).or_insert_with(|| RagDocMeta {
                doc_id: c.doc_id.clone(), title: c.title.clone(), source: c.source.clone(),
                chunk_count: 0, total_chars: 0, ingested_at: c.ingested_at,
            });
            e.chunk_count += 1; e.total_chars += c.text.len();
        }
        let mut docs: Vec<RagDocMeta> = map.into_values().collect();
        docs.sort_by(|a, b| b.ingested_at.cmp(&a.ingested_at));
        docs
    }

    pub async fn status(&self) -> (usize, usize, usize) {
        let inner = self.inner.read().await;
        let docs: std::collections::HashSet<&str> =
            inner.chunks.iter().map(|c| c.doc_id.as_str()).collect();
        (docs.len(), inner.chunks.len(), inner.vocab.next_id)
    }
}

// -- RagSkillHandler ----------------------------------------------------------

pub struct RagSkillHandler { pub index: RagIndex }

impl RagSkillHandler { pub fn new(index: RagIndex) -> Self { Self { index } } }

#[async_trait]
impl SkillHandler for RagSkillHandler {
    fn skill_names(&self) -> &[&'static str] {
        &["rag.ingest", "rag.search", "rag.delete", "rag.list", "rag.status"]
    }

    async fn execute(&self, skill_name: &str, args: &serde_json::Value) -> Result<String, String> {
        match skill_name {
            "rag.ingest" => {
                let content = args["content"].as_str()
                    .ok_or("rag.ingest: missing 'content'")?;
                if content.trim().is_empty() {
                    return Err("rag.ingest: content must not be empty".into());
                }
                let title = args["title"].as_str().unwrap_or("untitled");
                let source = args["source"].as_str();
                let (doc_id, n) = self.index.ingest(content, title, source).await;
                Ok(format!("Ingested '{title}' -> doc_id={doc_id}, {n} chunk(s), {} chars.", content.len()))
            }
            "rag.search" => {
                let query = args["query"].as_str()
                    .or_else(|| args["question"].as_str())
                    .ok_or("rag.search: missing 'query'")?
                    .trim();
                if query.is_empty() {
                    return Err("rag.search: query must not be empty".into());
                }
                let top_k = args["top_k"].as_u64().unwrap_or(5).min(20) as usize;
                let results = self.index.search(query, top_k).await;
                if results.is_empty() {
                    return Ok(format!(
                        "No RAG results for '{query}'. Use rag.ingest to add documents."
                    ));
                }
                let lines: Vec<String> = results.iter().map(|r| {
                    let src = r.source.as_deref().unwrap_or("-");
                    format!("[{:.3}] {} ({})\n{}", r.score, r.title, src, r.snippet)
                }).collect();
                Ok(format!("RAG: {} result(s) for '{query}':\n\n{}", lines.len(), lines.join("\n\n---\n\n")))
            }
            "rag.delete" => {
                let doc_id = args["doc_id"].as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or("rag.delete: missing 'doc_id'")?;
                if self.index.delete(doc_id).await {
                    Ok(format!("Deleted document '{doc_id}' from RAG index."))
                } else {
                    Err(format!("rag.delete: document '{doc_id}' not found"))
                }
            }
            "rag.list" => {
                let docs = self.index.list().await;
                if docs.is_empty() {
                    return Ok("RAG index is empty. Use rag.ingest to add documents.".into());
                }
                let lines: Vec<String> = docs.iter().map(|d| {
                    let src = d.source.as_deref().unwrap_or("-");
                    format!("  [{}] {} | {} chunks | {} chars | src: {}",
                        d.doc_id, d.title, d.chunk_count, d.total_chars, src)
                }).collect();
                Ok(format!("{} document(s) in RAG index:\n{}", docs.len(), lines.join("\n")))
            }
            "rag.status" => {
                let (docs, chunks, vocab) = self.index.status().await;
                Ok(format!("RAG index: {docs} documents, {chunks} chunks, {vocab} vocab terms."))
            }
            other => Err(format!("RagSkillHandler: unknown skill '{other}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> RagSkillHandler { RagSkillHandler::new(RagIndex::new()) }

    #[test]
    fn chunk_text_short_returns_single() {
        let c = chunk_text("hello world");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0], "hello world");
    }

    #[test]
    fn chunk_text_long_splits() {
        let text = "a".repeat(1000);
        let chunks = chunk_text(&text);
        assert!(chunks.len() > 1);
        for ch in &chunks { assert!(ch.len() <= CHUNK_SIZE); }
    }

    #[test]
    fn chunk_text_empty() { assert!(chunk_text("").is_empty()); }

    #[test]
    fn tokenize_basic() {
        let t = tokenize("Hello, World! 123");
        assert!(t.contains(&"hello".to_string()));
        assert!(t.contains(&"world".to_string()));
        assert!(t.contains(&"123".to_string()));
    }

    #[test]
    fn tokenize_filters_short() {
        let t = tokenize("a bb ccc");
        assert!(!t.contains(&"a".to_string()));
        assert!(t.contains(&"bb".to_string()));
        assert!(t.contains(&"ccc".to_string()));
    }

    #[test]
    fn cosine_identical_vectors() {
        let mut v = HashMap::new();
        v.insert(0usize, 1.0f32); v.insert(1, 2.0);
        let norm = (1.0f32 + 4.0f32).sqrt();
        let sim = cosine_similarity(&v, norm, &v, norm);
        assert!((sim - 1.0).abs() < 1e-5, "expected ~1.0, got {sim}");
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let mut a = HashMap::new(); a.insert(0usize, 1.0f32);
        let mut b = HashMap::new(); b.insert(1usize, 1.0f32);
        assert_eq!(cosine_similarity(&a, 1.0, &b, 1.0), 0.0);
    }

    #[test]
    fn cosine_zero_norm() {
        let a: HashMap<usize, f32> = HashMap::new();
        let b: HashMap<usize, f32> = HashMap::new();
        assert_eq!(cosine_similarity(&a, 0.0, &b, 0.0), 0.0);
    }

    #[test]
    fn derive_doc_id_deterministic() {
        assert_eq!(derive_doc_id("t", "c"), derive_doc_id("t", "c"));
    }

    #[test]
    fn derive_doc_id_different_for_different_input() {
        assert_ne!(derive_doc_id("t", "c"), derive_doc_id("t", "cc"));
    }

    #[test]
    fn vocabulary_get_or_insert() {
        let mut v = Vocabulary::default();
        let i1 = v.get_or_insert("rust");
        let i2 = v.get_or_insert("rust");
        assert_eq!(i1, i2);
        let i3 = v.get_or_insert("python");
        assert_ne!(i1, i3);
    }

    #[test]
    fn vocabulary_get_existing() {
        let mut v = Vocabulary::default();
        v.get_or_insert("rust");
        assert_eq!(v.get("rust"), Some(0));
        assert_eq!(v.get("nope"), None);
    }

    #[tokio::test]
    async fn ingest_and_status() {
        let idx = RagIndex::new();
        let (doc_id, n) = idx.ingest("Rust is a systems language.", "Rust", None).await;
        assert!(doc_id.starts_with("rag_"));
        assert_eq!(n, 1);
        let (docs, chunks, _) = idx.status().await;
        assert_eq!(docs, 1); assert_eq!(chunks, 1);
    }

    #[tokio::test]
    async fn ingest_with_source() {
        let idx = RagIndex::new();
        let (doc_id, _) = idx.ingest("content", "Title", Some("README.md")).await;
        let docs = idx.list().await;
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].source.as_deref(), Some("README.md"));
        let _ = doc_id;
    }

    #[tokio::test]
    async fn ingest_long_document_produces_multiple_chunks() {
        let idx = RagIndex::new();
        let content = "word ".repeat(300);
        let (_, n) = idx.ingest(&content, "Long", None).await;
        assert!(n > 1, "expected multiple chunks, got {n}");
        let (_, chunks, _) = idx.status().await;
        assert_eq!(chunks, n);
    }

    #[tokio::test]
    async fn search_finds_relevant() {
        let idx = RagIndex::new();
        idx.ingest("Rust ownership and borrowing rules", "Rust Guide", None).await;
        idx.ingest("Python is a dynamic scripting language", "Python Guide", None).await;
        let results = idx.search("rust ownership", 5).await;
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Rust Guide");
    }

    #[tokio::test]
    async fn search_returns_score_in_range() {
        let idx = RagIndex::new();
        idx.ingest("async await rust tokio", "Async", None).await;
        let r = idx.search("async rust", 5).await;
        for res in &r {
            assert!(res.score >= 0.0 && res.score <= 1.0, "score out of range: {}", res.score);
        }
    }

    #[tokio::test]
    async fn search_top_k_limits_results() {
        let idx = RagIndex::new();
        for i in 0..10 {
            idx.ingest(&format!("rust content number {i} tokio async"), &format!("Doc{i}"), None).await;
        }
        let r = idx.search("rust tokio", 3).await;
        assert!(r.len() <= 3);
    }

    #[tokio::test]
    async fn search_empty_query_returns_empty() {
        let idx = RagIndex::new();
        idx.ingest("some content", "doc", None).await;
        assert!(idx.search("", 5).await.is_empty());
    }

    #[tokio::test]
    async fn search_zero_top_k_returns_empty() {
        let idx = RagIndex::new();
        idx.ingest("some content", "doc", None).await;
        assert!(idx.search("content", 0).await.is_empty());
    }

    #[tokio::test]
    async fn search_no_match_returns_empty() {
        let idx = RagIndex::new();
        idx.ingest("completely unrelated text about cooking", "Food", None).await;
        let results = idx.search("quantum physics wormhole", 5).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn delete_removes_document() {
        let idx = RagIndex::new();
        let (doc_id, _) = idx.ingest("to be deleted", "Delete Me", None).await;
        assert!(idx.delete(&doc_id).await);
        let (docs, chunks, _) = idx.status().await;
        assert_eq!(docs, 0); assert_eq!(chunks, 0);
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_false() {
        let idx = RagIndex::new();
        assert!(!idx.delete("rag_00000000").await);
    }

    #[tokio::test]
    async fn delete_only_removes_target_doc() {
        let idx = RagIndex::new();
        let (id1, _) = idx.ingest("doc one", "One", None).await;
        idx.ingest("doc two", "Two", None).await;
        assert!(idx.delete(&id1).await);
        let (docs, _, _) = idx.status().await;
        assert_eq!(docs, 1);
        let remaining = idx.list().await;
        assert_eq!(remaining[0].title, "Two");
    }

    #[tokio::test]
    async fn list_returns_docs() {
        let idx = RagIndex::new();
        idx.ingest("content a", "Doc A", Some("file_a.md")).await;
        idx.ingest("content b", "Doc B", None).await;
        let docs = idx.list().await;
        assert_eq!(docs.len(), 2);
        let titles: Vec<&str> = docs.iter().map(|d| d.title.as_str()).collect();
        assert!(titles.contains(&"Doc A")); assert!(titles.contains(&"Doc B"));
    }

    #[tokio::test]
    async fn list_empty_index() {
        let idx = RagIndex::new();
        assert!(idx.list().await.is_empty());
    }

    #[tokio::test]
    async fn handler_ingest_ok() {
        let h = make_handler();
        let r = h.execute("rag.ingest", &serde_json::json!({"content":"hello world","title":"Test"})).await.unwrap();
        assert!(r.contains("rag_")); assert!(r.contains("chunk"));
    }

    #[tokio::test]
    async fn handler_ingest_no_title_uses_untitled() {
        let h = make_handler();
        let r = h.execute("rag.ingest", &serde_json::json!({"content":"hello world"})).await.unwrap();
        assert!(r.contains("untitled"));
    }

    #[tokio::test]
    async fn handler_ingest_missing_content_errors() {
        let h = make_handler();
        assert!(h.execute("rag.ingest", &serde_json::json!({"title":"T"})).await.is_err());
    }

    #[tokio::test]
    async fn handler_ingest_empty_content_errors() {
        let h = make_handler();
        assert!(h.execute("rag.ingest", &serde_json::json!({"content":"   ","title":"T"})).await.is_err());
    }

    #[tokio::test]
    async fn handler_search_after_ingest() {
        let h = make_handler();
        h.execute("rag.ingest", &serde_json::json!({"content":"async rust tokio runtime","title":"Async"})).await.unwrap();
        let r = h.execute("rag.search", &serde_json::json!({"query":"async tokio"})).await.unwrap();
        assert!(r.to_lowercase().contains("async"));
    }

    #[tokio::test]
    async fn handler_search_empty_index() {
        let h = make_handler();
        let r = h.execute("rag.search", &serde_json::json!({"query":"rust"})).await.unwrap();
        assert!(r.contains("rag.ingest"));
    }

    #[tokio::test]
    async fn handler_search_missing_query_errors() {
        let h = make_handler();
        assert!(h.execute("rag.search", &serde_json::json!({})).await.is_err());
    }

    #[tokio::test]
    async fn handler_search_accepts_question_key() {
        let h = make_handler();
        h.execute("rag.ingest", &serde_json::json!({"content":"memory management in rust","title":"Mem"})).await.unwrap();
        let r = h.execute("rag.search", &serde_json::json!({"question":"memory rust"})).await.unwrap();
        assert!(!r.contains("missing"));
    }

    #[tokio::test]
    async fn handler_delete_ok() {
        let h = make_handler();
        let ingest_r = h.execute("rag.ingest",
            &serde_json::json!({"content":"delete me","title":"Gone"})).await.unwrap();
        let doc_id = ingest_r.split("doc_id=").nth(1).unwrap().split(',').next().unwrap().trim();
        let r = h.execute("rag.delete", &serde_json::json!({"doc_id": doc_id})).await.unwrap();
        assert!(r.contains("Deleted"));
    }

    #[tokio::test]
    async fn handler_delete_accepts_id_key() {
        let h = make_handler();
        let ingest_r = h.execute("rag.ingest",
            &serde_json::json!({"content":"delete me too","title":"Gone2"})).await.unwrap();
        let doc_id = ingest_r.split("doc_id=").nth(1).unwrap().split(',').next().unwrap().trim();
        let r = h.execute("rag.delete", &serde_json::json!({"id": doc_id})).await.unwrap();
        assert!(r.contains("Deleted"));
    }

    #[tokio::test]
    async fn handler_delete_not_found_errors() {
        let h = make_handler();
        assert!(h.execute("rag.delete", &serde_json::json!({"doc_id":"rag_00000000"})).await.is_err());
    }

    #[tokio::test]
    async fn handler_list_empty() {
        let h = make_handler();
        let r = h.execute("rag.list", &serde_json::json!({})).await.unwrap();
        assert!(r.contains("empty"));
    }

    #[tokio::test]
    async fn handler_list_shows_docs() {
        let h = make_handler();
        h.execute("rag.ingest", &serde_json::json!({"content":"doc a content","title":"Alpha"})).await.unwrap();
        h.execute("rag.ingest", &serde_json::json!({"content":"doc b content","title":"Beta"})).await.unwrap();
        let r = h.execute("rag.list", &serde_json::json!({})).await.unwrap();
        assert!(r.contains("Alpha")); assert!(r.contains("Beta")); assert!(r.contains("2 document"));
    }

    #[tokio::test]
    async fn handler_status_after_ingest() {
        let h = make_handler();
        h.execute("rag.ingest", &serde_json::json!({"content":"status test content here","title":"S"})).await.unwrap();
        let r = h.execute("rag.status", &serde_json::json!({})).await.unwrap();
        assert!(r.contains("1 document")); assert!(r.contains("chunk"));
    }

    #[tokio::test]
    async fn handler_status_empty_index() {
        let h = make_handler();
        let r = h.execute("rag.status", &serde_json::json!({})).await.unwrap();
        assert!(r.contains("0 document"));
    }

    #[tokio::test]
    async fn handler_unknown_skill_errors() {
        let h = make_handler();
        assert!(h.execute("rag.unknown", &serde_json::json!({})).await.is_err());
    }

    #[test]
    fn handler_skill_names() {
        let h = make_handler();
        let names = h.skill_names();
        assert!(names.contains(&"rag.ingest"));
        assert!(names.contains(&"rag.search"));
        assert!(names.contains(&"rag.delete"));
        assert!(names.contains(&"rag.list"));
        assert!(names.contains(&"rag.status"));
    }

    // ── derive_doc_id: FNV collision resistance ───────────────────────────

    #[test]
    fn derive_doc_id_different_title_same_length() {
        // Two docs with same content length but different titles must differ
        let id1 = derive_doc_id("Title A", "hello world!!");
        let id2 = derive_doc_id("Title B", "hello world!!");
        assert_ne!(id1, id2, "same-length content, different title → must differ");
    }

    #[test]
    fn derive_doc_id_same_title_different_content() {
        let id1 = derive_doc_id("Same", "content one");
        let id2 = derive_doc_id("Same", "content two");
        assert_ne!(id1, id2, "same title, different content → must differ");
    }

    #[test]
    fn derive_doc_id_format_16_hex_chars() {
        // New format: rag_ + 16 hex chars (two 32-bit FNV hashes)
        let id = derive_doc_id("t", "c");
        assert!(id.starts_with("rag_"), "must start with rag_");
        assert_eq!(id.len(), 4 + 16, "must be rag_ + 16 hex chars, got: {id}");
        assert!(id[4..].chars().all(|c| c.is_ascii_hexdigit()), "suffix must be hex");
    }

    // ── chunk_text: overlap verification ─────────────────────────────────

    #[test]
    fn chunk_text_overlap_correct() {
        // Build a string exactly 2*CHUNK_SIZE - CHUNK_OVERLAP chars long → 2 chunks
        let text = "x".repeat(CHUNK_SIZE * 2 - CHUNK_OVERLAP);
        let chunks = chunk_text(&text);
        assert_eq!(chunks.len(), 2, "expected exactly 2 chunks");
        assert_eq!(chunks[0].len(), CHUNK_SIZE);
        assert_eq!(chunks[1].len(), CHUNK_SIZE);
    }

    #[test]
    fn chunk_text_overlap_content_overlap() {
        // Verify that the overlap region is identical between consecutive chunks
        let text: String = (0u8..=127u8).map(|b| (b % 26 + b'a') as char).collect::<String>().repeat(6);
        let chunks = chunk_text(&text);
        if chunks.len() >= 2 {
            let tail_of_first: String = chunks[0].chars().rev().take(CHUNK_OVERLAP).collect::<String>().chars().rev().collect();
            let head_of_second: String = chunks[1].chars().take(CHUNK_OVERLAP).collect();
            assert_eq!(tail_of_first, head_of_second, "overlap region must be identical");
        }
    }

    #[test]
    fn chunk_text_exactly_chunk_size() {
        let text = "a".repeat(CHUNK_SIZE);
        let chunks = chunk_text(&text);
        assert_eq!(chunks.len(), 1, "exactly CHUNK_SIZE → single chunk");
        assert_eq!(chunks[0].len(), CHUNK_SIZE);
    }

    #[test]
    fn chunk_text_chunk_size_plus_one() {
        let text = "a".repeat(CHUNK_SIZE + 1);
        let chunks = chunk_text(&text);
        assert_eq!(chunks.len(), 2, "CHUNK_SIZE+1 → two chunks");
    }

    // ── ingest idempotency: re-ingest replaces, not duplicates ────────────

    #[tokio::test]
    async fn reingest_same_doc_replaces_not_duplicates() {
        let idx = RagIndex::new();
        let content = "rust memory safety";
        let (id1, _) = idx.ingest(content, "Rust", None).await;
        let (id2, _) = idx.ingest(content, "Rust", None).await;
        assert_eq!(id1, id2, "same title+content → same doc_id");
        let (docs, chunks, _) = idx.status().await;
        assert_eq!(docs, 1, "re-ingest must not create duplicate doc");
        assert_eq!(chunks, 1);
    }

    #[tokio::test]
    async fn reingest_different_content_same_title_creates_new_doc() {
        let idx = RagIndex::new();
        let (id1, _) = idx.ingest("content version one", "MyDoc", None).await;
        let (id2, _) = idx.ingest("content version two", "MyDoc", None).await;
        assert_ne!(id1, id2, "different content → different doc_id");
        let (docs, _, _) = idx.status().await;
        assert_eq!(docs, 2);
    }

    // ── concurrent safety ─────────────────────────────────────────────────

    #[tokio::test]
    async fn concurrent_ingest_and_search_no_panic() {
        use std::sync::Arc;
        let idx = Arc::new(RagIndex::new());
        // Pre-populate with some docs
        for i in 0..5 {
            idx.ingest(&format!("rust tokio async doc {i}"), &format!("Doc{i}"), None).await;
        }
        // Concurrent ingest + search
        let idx2 = idx.clone();
        let ingest_task = tokio::spawn(async move {
            for i in 5..15 {
                idx2.ingest(&format!("concurrent ingest doc {i} rust"), &format!("C{i}"), None).await;
            }
        });
        let idx3 = idx.clone();
        let search_task = tokio::spawn(async move {
            for _ in 0..10 {
                let _ = idx3.search("rust tokio", 5).await;
            }
        });
        ingest_task.await.unwrap();
        search_task.await.unwrap();
        let (docs, _, _) = idx.status().await;
        assert!(docs >= 5, "must have at least initial docs after concurrent ops");
    }

    #[tokio::test]
    async fn concurrent_multi_search_same_index() {
        use std::sync::Arc;
        let idx = Arc::new(RagIndex::new());
        idx.ingest("rust ownership memory model", "Rust", None).await;
        idx.ingest("python dynamic typing garbage collection", "Python", None).await;
        let handles: Vec<_> = (0..8).map(|_| {
            let i = idx.clone();
            tokio::spawn(async move { i.search("rust ownership", 3).await })
        }).collect();
        for h in handles {
            let results = h.await.unwrap();
            assert!(!results.is_empty(), "concurrent search must return results");
            assert_eq!(results[0].title, "Rust");
        }
    }

    // ── handler: top_k cap at 20 ──────────────────────────────────────────

    #[tokio::test]
    async fn handler_search_top_k_capped_at_20() {
        let h = make_handler();
        for i in 0..30 {
            h.execute("rag.ingest",
                &serde_json::json!({"content": format!("rust doc number {i} tokio async wasm"), "title": format!("D{i}")}))
                .await.unwrap();
        }
        let r = h.execute("rag.search",
            &serde_json::json!({"query": "rust tokio", "top_k": 999})).await.unwrap();
        // Count result separators; max results ≤ 20
        let sep_count = r.matches("---").count();
        assert!(sep_count < 20, "top_k must be capped at 20, separators={sep_count}");
    }

    #[tokio::test]
    async fn handler_search_top_k_zero_in_handler_falls_through() {
        // top_k=0 as JSON → as_u64()=0 → capped to 0 → search returns empty → guidance msg
        let h = make_handler();
        h.execute("rag.ingest", &serde_json::json!({"content":"some content","title":"T"})).await.unwrap();
        let r = h.execute("rag.search", &serde_json::json!({"query":"content","top_k":0})).await.unwrap();
        assert!(r.contains("rag.ingest"), "top_k=0 → no results → guidance message");
    }

    // ── search: snippet length bounded ───────────────────────────────────

    #[tokio::test]
    async fn search_snippet_bounded_by_snippet_chars() {
        let idx = RagIndex::new();
        let long_text = "rust ".repeat(200);
        idx.ingest(&long_text, "Long", None).await;
        let results = idx.search("rust", 1).await;
        assert!(!results.is_empty());
        assert!(results[0].snippet.chars().count() <= SNIPPET_CHARS,
            "snippet must be ≤ SNIPPET_CHARS chars");
    }

    // ── vocab: MAX_VOCAB cap does not panic ───────────────────────────────

    #[test]
    fn vocabulary_max_vocab_cap_returns_sentinel() {
        let mut v = Vocabulary::default();
        // Fill up to MAX_VOCAB
        for i in 0..MAX_VOCAB {
            v.get_or_insert(&format!("term_{i}"));
        }
        assert_eq!(v.next_id, MAX_VOCAB);
        // Next insert must return MAX_VOCAB sentinel without panic
        let sentinel = v.get_or_insert("overflow_term");
        assert_eq!(sentinel, MAX_VOCAB);
        assert_eq!(v.next_id, MAX_VOCAB, "next_id must not exceed MAX_VOCAB");
    }

    // ── tfidf_vector: empty text returns zero norm ────────────────────────

    #[test]
    fn tfidf_vector_empty_text_returns_zero() {
        let mut vocab = Vocabulary::default();
        let df: HashMap<usize, usize> = HashMap::new();
        let (vec, norm) = tfidf_vector("", &mut vocab, &df, 1);
        assert!(vec.is_empty());
        assert_eq!(norm, 0.0);
    }

    #[test]
    fn tfidf_vector_single_token() {
        let mut vocab = Vocabulary::default();
        let df: HashMap<usize, usize> = HashMap::new();
        let (vec, norm) = tfidf_vector("rust", &mut vocab, &df, 1);
        assert!(!vec.is_empty());
        assert!(norm > 0.0);
    }

    // ── delete: df rebuilt correctly after deletion ───────────────────────

    #[tokio::test]
    async fn search_still_works_after_delete_and_reingest() {
        let idx = RagIndex::new();
        let (id, _) = idx.ingest("rust ownership memory model", "Rust", None).await;
        idx.ingest("python scripting language", "Python", None).await;
        idx.delete(&id).await;
        // Re-ingest after delete
        idx.ingest("rust async runtime tokio", "Rust2", None).await;
        let results = idx.search("rust tokio", 5).await;
        assert!(!results.is_empty(), "search must work after delete+reingest");
        assert_eq!(results[0].title, "Rust2");
    }
}
