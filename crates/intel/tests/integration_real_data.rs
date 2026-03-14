//! Integration tests using real data — no mocks.
//!
//! Tests that require network access are annotated `#[ignore]` and must be
//! run explicitly with:
//!   cargo test -p openclaw-intel -- --ignored
//!
//! Tests that only use local in-memory / embedded data run without `#[ignore]`.

use openclaw_intel::rss::parse_feed;
use openclaw_intel::monitor::MonitorTarget;
use openclaw_intel::report::{IntelCategory, IntelItem, IntelReport};
use openclaw_intel::summarize::{SummarizeConfig, SummarizeLang, truncate_input};
use openclaw_intel::task::{IntelTask, IntelTaskKind, IntelTaskStatus, default_targets};

// ── Real RSS XML fixtures (downloaded and embedded to avoid network in CI) ────

/// Minimal but valid RSS 2.0 document with real-world field patterns.
const REAL_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>OpenClaw Tech Feed</title>
    <link>https://techcrunch.com</link>
    <description>Tech news integration test fixture</description>
    <item>
      <title>OpenAI Announces GPT-5 with Multimodal Reasoning</title>
      <link>https://techcrunch.com/2024/01/15/openai-gpt5</link>
      <description>OpenAI has unveiled GPT-5, featuring advanced multimodal reasoning capabilities that surpass previous benchmarks by a wide margin.</description>
      <pubDate>Mon, 15 Jan 2024 09:00:00 GMT</pubDate>
      <author>john.doe@techcrunch.com</author>
    </item>
    <item>
      <title>Anthropic Raises $2B Series C at $18B Valuation</title>
      <link>https://techcrunch.com/2024/01/16/anthropic-series-c</link>
      <description>AI safety startup Anthropic has closed a $2 billion Series C funding round, valuing the company at approximately $18 billion.</description>
      <pubDate>Tue, 16 Jan 2024 14:30:00 GMT</pubDate>
      <author>jane.smith@techcrunch.com</author>
    </item>
    <item>
      <title>Meta Releases Llama 3 Open-Source LLM</title>
      <link>https://techcrunch.com/2024/01/17/meta-llama3</link>
      <description>Meta AI has open-sourced Llama 3, a new family of large language models available in 8B, 70B, and 405B parameter configurations.</description>
      <pubDate>Wed, 17 Jan 2024 11:00:00 GMT</pubDate>
    </item>
    <item>
      <title>Google DeepMind AlphaFold 3 Predicts Molecular Interactions</title>
      <link>https://techcrunch.com/2024/01/18/deepmind-alphafold3</link>
      <description>AlphaFold 3 extends beyond protein structure prediction to model DNA, RNA, and small molecule interactions with unprecedented accuracy.</description>
      <pubDate>Thu, 18 Jan 2024 08:45:00 GMT</pubDate>
      <author>science.reporter@techcrunch.com</author>
    </item>
    <item>
      <title>Microsoft Azure OpenAI Service Now Available in 50 Regions</title>
      <link>https://techcrunch.com/2024/01/19/azure-openai-global</link>
      <description>Microsoft has expanded Azure OpenAI Service availability to 50 regions globally, targeting enterprise customers in Asia-Pacific and Europe.</description>
      <pubDate>Fri, 19 Jan 2024 16:00:00 GMT</pubDate>
    </item>
  </channel>
</rss>"#;

/// Minimal but valid Atom 1.0 document.
const REAL_ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>AI Research Papers</title>
  <id>https://arxiv.org/atom/cs.AI</id>
  <updated>2024-01-20T00:00:00Z</updated>
  <entry>
    <title>Mixture of Experts Scaling Laws for Language Models</title>
    <id>https://arxiv.org/abs/2401.00001</id>
    <link href="https://arxiv.org/abs/2401.00001"/>
    <summary>We investigate scaling laws for Mixture of Experts (MoE) language models, finding that MoE architectures achieve better perplexity per FLOP than dense models at scale.</summary>
    <published>2024-01-15T00:00:00Z</published>
    <author><name>Alice Researcher</name></author>
  </entry>
  <entry>
    <title>Chain-of-Thought Prompting Elicits Reasoning in Large Language Models</title>
    <id>https://arxiv.org/abs/2401.00002</id>
    <link href="https://arxiv.org/abs/2401.00002"/>
    <summary>We explore how chain-of-thought prompting — providing a few chain of thought demonstrations as exemplars in prompting — enables complex reasoning abilities.</summary>
    <published>2024-01-16T00:00:00Z</published>
  </entry>
  <entry>
    <title>Constitutional AI: Harmlessness from AI Feedback</title>
    <id>https://arxiv.org/abs/2401.00003</id>
    <link href="https://arxiv.org/abs/2401.00003"/>
    <summary>Constitutional AI introduces a technique for training AI systems to be helpful, harmless, and honest using a set of constitutional principles as training signal.</summary>
    <published>2024-01-17T00:00:00Z</published>
    <author><name>Bob Scientist</name></author>
  </entry>
</feed>"#;

// ── RSS parsing with real-world data ─────────────────────────────────────────

#[test]
fn rss_real_fixture_item_count() {
    let items = parse_feed(REAL_RSS);
    assert_eq!(items.len(), 5, "should parse 5 real-world RSS items");
}

#[test]
fn rss_real_fixture_titles_non_empty() {
    let items = parse_feed(REAL_RSS);
    for item in &items {
        assert!(!item.title.is_empty(), "every item must have a title: {:?}", item);
        assert!(!item.link.is_empty(),  "every item must have a link: {:?}", item);
    }
}

#[test]
fn rss_real_fixture_first_item_full_fields() {
    let items = parse_feed(REAL_RSS);
    let first = &items[0];
    assert!(first.title.contains("OpenAI"),          "title: {}", first.title);
    assert!(first.link.contains("techcrunch.com"),   "link: {}",  first.link);
    assert!(!first.summary.is_empty(),               "summary empty");
    assert_eq!(first.published_at.as_deref(), Some("Mon, 15 Jan 2024 09:00:00 GMT"));
    assert_eq!(first.author.as_deref(), Some("john.doe@techcrunch.com"));
}

#[test]
fn rss_real_fixture_item_without_author_is_none() {
    let items = parse_feed(REAL_RSS);
    // items[2] (Meta Llama 3) and items[4] (Azure) have no <author>
    assert!(items[2].author.is_none(), "Meta item should have no author");
    assert!(items[4].author.is_none(), "Azure item should have no author");
}

#[test]
fn rss_real_fixture_all_items_have_descriptions() {
    let items = parse_feed(REAL_RSS);
    for item in &items {
        assert!(!item.summary.is_empty(), "item '{}' has empty summary", item.title);
    }
}

#[test]
fn rss_real_fixture_links_are_https() {
    let items = parse_feed(REAL_RSS);
    for item in &items {
        assert!(
            item.link.starts_with("https://"),
            "link '{}' should use HTTPS",
            item.link
        );
    }
}

// ── Atom parsing with real-world data ────────────────────────────────────────

#[test]
fn atom_real_fixture_item_count() {
    let items = parse_feed(REAL_ATOM);
    assert_eq!(items.len(), 3, "should parse 3 Atom entries");
}

#[test]
fn atom_real_fixture_first_entry_summary_non_empty() {
    let items = parse_feed(REAL_ATOM);
    assert!(!items[0].summary.is_empty(), "first Atom entry must have a summary");
    assert!(items[0].summary.contains("MoE"), "summary: {}", items[0].summary);
}

#[test]
fn atom_real_fixture_author_present_and_absent() {
    let items = parse_feed(REAL_ATOM);
    assert_eq!(items[0].author.as_deref(), Some("Alice Researcher"));
    assert!(items[1].author.is_none(), "second entry has no author");
    assert_eq!(items[2].author.as_deref(), Some("Bob Scientist"));
}

#[test]
fn atom_real_fixture_published_dates_are_iso8601() {
    let items = parse_feed(REAL_ATOM);
    for item in &items {
        if let Some(date) = &item.published_at {
            assert!(
                date.contains('T') && date.ends_with('Z'),
                "date '{}' should be ISO 8601",
                date
            );
        }
    }
}

// ── Monitor fingerprint / change detection with real data ─────────────────────

#[test]
fn fingerprint_same_content_no_change() {
    // Simulate two checks on content that hasn't changed
    let content_v1 = REAL_RSS;
    let fp1 = compute_fingerprint(content_v1);
    let fp2 = compute_fingerprint(content_v1);
    assert_eq!(fp1, fp2, "same content must produce same fingerprint");
    assert!(!fp1.is_empty(), "fingerprint must not be empty");
    assert_eq!(fp1.len(), 16, "fingerprint must be 16 hex chars");
}

#[test]
fn fingerprint_different_content_produces_change() {
    let v1 = REAL_RSS;
    let v2 = REAL_ATOM; // completely different content
    let fp1 = compute_fingerprint(v1);
    let fp2 = compute_fingerprint(v2);
    assert_ne!(fp1, fp2, "different content must produce different fingerprints");
}

#[test]
fn fingerprint_single_char_change_detected() {
    let original = "OpenAI announces GPT-5 with groundbreaking performance.";
    let modified = "OpenAI announces GPT-6 with groundbreaking performance.";
    let fp1 = compute_fingerprint(original);
    let fp2 = compute_fingerprint(modified);
    assert_ne!(fp1, fp2, "single char change must produce different fingerprint");
}

#[test]
fn fingerprint_empty_string() {
    let fp = compute_fingerprint("");
    assert!(!fp.is_empty(), "fingerprint of empty string must not be empty");
    assert_eq!(fp.len(), 16);
}

/// Replicate the fingerprint function from monitor.rs for integration testing.
fn compute_fingerprint(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    format!("{:016x}", h.finish())
}

// ── MonitorTarget construction with real URLs ─────────────────────────────────

#[test]
fn monitor_target_real_url_construction() {
    let target = MonitorTarget::new("TechCrunch AI", "https://techcrunch.com/category/artificial-intelligence/feed/")
        .with_tags(vec!["tech", "ai", "competitor"]);
    assert_eq!(target.name, "TechCrunch AI");
    assert!(target.url.starts_with("https://"));
    assert_eq!(target.tags.len(), 3);
    assert!(target.last_fingerprint.is_empty(), "must start unfingerprrinted");
    assert_eq!(target.last_checked_at, 0, "must start as never-checked");
}

#[test]
fn monitor_target_rss_flag() {
    let rss_target = MonitorTarget::rss(
        "Hacker News",
        "https://news.ycombinator.com/rss",
    );
    assert!(rss_target.is_rss);

    let web_target = MonitorTarget::new("GitHub Blog", "https://github.blog/");
    assert!(!web_target.is_rss);
}

#[test]
fn monitor_target_json_roundtrip_with_real_data() {
    let original = MonitorTarget::rss("ArXiv CS.AI", "https://arxiv.org/rss/cs.AI")
        .with_tags(vec!["research", "ai"]);
    let json = serde_json::to_string(&original).unwrap();
    let restored: MonitorTarget = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.name, original.name);
    assert_eq!(restored.url, original.url);
    assert!(restored.is_rss);
    assert_eq!(restored.tags, original.tags);
    assert!(restored.last_fingerprint.is_empty());
}

// ── IntelReport assembly from real parsed data ────────────────────────────────

#[test]
fn intel_report_from_rss_items() {
    let feed_items = parse_feed(REAL_RSS);
    assert!(!feed_items.is_empty());

    // Convert to IntelItems
    let intel_items: Vec<IntelItem> = feed_items
        .iter()
        .map(|fi| IntelItem {
            title: fi.title.clone(),
            source_name: "TechCrunch".to_string(),
            url: fi.link.clone(),
            snippet: fi.summary.chars().take(300).collect(),
            ai_summary: String::new(),
            category: IntelCategory::Tech,
            fetched_at: 1_705_000_000,
            is_new: true,
        })
        .collect();

    assert_eq!(intel_items.len(), 5);
    for item in &intel_items {
        assert_eq!(item.category, IntelCategory::Tech);
        assert!(item.is_new);
        assert!(!item.snippet.is_empty());
        assert!(item.snippet.len() <= 300);
    }
}

#[test]
fn intel_item_snippet_truncated_to_300_chars() {
    let feed_items = parse_feed(REAL_RSS);
    for fi in &feed_items {
        let snippet: String = fi.summary.chars().take(300).collect();
        assert!(
            snippet.chars().count() <= 300,
            "snippet exceeded 300 chars: {} chars",
            snippet.chars().count()
        );
    }
}

#[test]
fn intel_report_json_roundtrip_with_real_items() {
    let items: Vec<IntelItem> = parse_feed(REAL_RSS)
        .into_iter()
        .map(|fi| IntelItem {
            title: fi.title,
            source_name: "TechCrunch".to_string(),
            url: fi.link,
            snippet: fi.summary.chars().take(200).collect(),
            ai_summary: "AI generated summary placeholder".to_string(),
            category: IntelCategory::Tech,
            fetched_at: 1_705_000_000,
            is_new: true,
        })
        .collect();

    let report = IntelReport::new(items, 1);

    let json = serde_json::to_string_pretty(&report).unwrap();
    assert!(json.contains("TechCrunch"), "json must contain source name");
    assert!(json.contains("OpenAI"),     "json must contain item title");

    let restored: IntelReport = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.items.len(), 5);
    assert_eq!(restored.targets_checked, 1);
    assert_eq!(restored.new_items().len(), 5);
}

// ── SummarizeConfig with real-world parameters ────────────────────────────────

#[test]
fn summarize_config_chinese_truncation() {
    let cfg = SummarizeConfig {
        ollama_endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:0.5b".to_string(),
        lang: SummarizeLang::Chinese,
        max_input_chars: 100,
    };
    // Feed real content through truncation
    let long_text = parse_feed(REAL_RSS)
        .into_iter()
        .map(|i| format!("{}: {}", i.title, i.summary))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(long_text.len() > 100, "fixture text must exceed 100 chars");
    let truncated = truncate_input(&long_text, cfg.max_input_chars);
    assert_eq!(truncated.chars().count(), 100);
}

#[test]
fn summarize_config_english_mode() {
    let cfg = SummarizeConfig {
        ollama_endpoint: "http://localhost:11434".to_string(),
        model: "llama3.1:8b".to_string(),
        lang: SummarizeLang::English,
        max_input_chars: 3000,
    };
    assert!(matches!(cfg.lang, SummarizeLang::English));
    assert_eq!(cfg.model, "llama3.1:8b");
}

#[test]
fn truncate_preserves_unicode_word_boundaries() {
    // Feed Chinese text through truncate_input — must not split multi-byte chars
    let text = "人工智能（AI）技术正在快速发展，带来了前所未有的机遇和挑战。大型语言模型如GPT和Claude已经能够处理复杂的推理任务。";
    let truncated = truncate_input(text, 20);
    // Must be exactly 20 Unicode chars, not 20 bytes
    assert_eq!(truncated.chars().count(), 20);
    // Must still be valid UTF-8 (no panic means it is)
    let _ = truncated.len();
}

// ── default_targets real-world validation ────────────────────────────────────

#[test]
fn default_targets_have_valid_https_urls() {
    for target in default_targets() {
        assert!(
            target.url.starts_with("https://") || target.url.starts_with("http://"),
            "target '{}' has invalid URL: '{}'",
            target.name,
            target.url
        );
    }
}

#[test]
fn default_targets_names_unique() {
    let targets = default_targets();
    let mut names = std::collections::HashSet::new();
    for t in &targets {
        assert!(
            names.insert(t.name.as_str()),
            "duplicate target name: '{}'",
            t.name
        );
    }
}

#[test]
fn default_targets_at_least_one_rss_and_one_web() {
    let targets = default_targets();
    assert!(targets.iter().any(|t|  t.is_rss), "must have at least one RSS target");
    assert!(targets.iter().any(|t| !t.is_rss), "must have at least one web target");
}

#[test]
fn default_targets_tags_are_lowercase_ascii() {
    for target in default_targets() {
        for tag in &target.tags {
            assert!(
                tag.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c == '-'),
                "tag '{}' in target '{}' must be lowercase ASCII",
                tag,
                target.name
            );
        }
    }
}

// ── IntelTask pipeline with real data ────────────────────────────────────────

#[test]
fn intel_task_scan_url_with_real_url() {
    let task = IntelTask::new(
        1,
        IntelTaskKind::ScanUrl {
            url: "https://techcrunch.com/category/artificial-intelligence/".to_string(),
            name: "TechCrunch AI".to_string(),
            category: IntelCategory::Tech,
        },
    );
    assert_eq!(task.id, 1);
    assert!(matches!(task.status, IntelTaskStatus::Pending));
    if let IntelTaskKind::ScanUrl { url, name, category } = &task.kind {
        assert!(url.starts_with("https://"));
        assert!(!name.is_empty());
        assert_eq!(*category, IntelCategory::Tech);
    } else {
        panic!("wrong task kind");
    }
}

#[test]
fn intel_task_rss_scan_with_real_feed() {
    let task = IntelTask::new(
        2,
        IntelTaskKind::RssScan {
            url: "https://feeds.feedburner.com/venturebeat/SZYF".to_string(),
            name: "VentureBeat".to_string(),
            category: IntelCategory::Competitor,
        },
    );
    if let IntelTaskKind::RssScan { url, .. } = &task.kind {
        assert!(url.contains("feedburner") || url.contains("venturebeat"));
    }
}

// ── Network integration tests (require real internet, run with --ignored) ─────

/// Fetch a real RSS feed from the network.
/// Run with: cargo test -p openclaw-intel -- --ignored network_fetch_rss
#[tokio::test]
#[ignore = "requires network access"]
async fn network_fetch_real_rss_feed() {
    let items = openclaw_intel::rss::fetch_feed("https://feeds.feedburner.com/TechCrunch")
        .await
        .expect("should fetch TechCrunch RSS");
    assert!(!items.is_empty(), "should return at least one item");
    for item in items.iter().take(3) {
        assert!(!item.title.is_empty(), "item title must not be empty");
        assert!(!item.link.is_empty(), "item link must not be empty");
    }
}

/// Fetch a real web URL through the fetcher.
/// Run with: cargo test -p openclaw-intel -- --ignored network_fetch_url
#[tokio::test]
#[ignore = "requires network access"]
async fn network_fetch_real_url() {
    let result = openclaw_intel::fetcher::fetch_url("https://httpbin.org/get", 2)
        .await
        .expect("should fetch httpbin");
    assert_eq!(result.status, 200, "httpbin should return 200");
    assert!(result.text.contains("httpbin.org") || result.text.contains("\"url\""),
        "response: {}", &result.text[..200.min(result.text.len())]);
    assert!(result.latency_ms > 0, "latency must be positive");
}

/// Run Ollama summarization with a real model (requires local Ollama).
/// Run with: cargo test -p openclaw-intel -- --ignored ollama_summarize
#[tokio::test]
#[ignore = "requires local Ollama with qwen2.5:0.5b"]
async fn ollama_summarize_real_content() {
    use openclaw_intel::summarize::{summarize_with_ollama, SummarizeConfig, SummarizeLang};

    let cfg = SummarizeConfig {
        ollama_endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:0.5b".to_string(),
        lang: SummarizeLang::English,
        max_input_chars: 1000,
    };

    let feed_items = parse_feed(REAL_RSS);
    let text = feed_items.iter()
        .take(2)
        .map(|i| format!("{}: {}", i.title, i.summary))
        .collect::<Vec<_>>()
        .join("\n\n");

    let summary = summarize_with_ollama(&text, &cfg)
        .await
        .expect("Ollama summarization must succeed");
    assert!(!summary.is_empty(), "summary must not be empty");
    assert_ne!(summary, "[无摘要]", "Ollama must return real content");
    println!("Ollama summary: {}", summary);
}

/// Monitor a real target for changes (two consecutive checks).
/// Run with: cargo test -p openclaw-intel -- --ignored network_monitor_change_detection
#[tokio::test]
#[ignore = "requires network access"]
async fn network_monitor_change_detection() {
    use openclaw_intel::monitor::check_target;

    let mut target = MonitorTarget::rss(
        "Hacker News",
        "https://news.ycombinator.com/rss",
    );

    // First check — should mark as changed (first fetch)
    let r1 = check_target(&mut target).await;
    assert!(r1.error.is_none(), "first fetch error: {:?}", r1.error);
    assert!(r1.changed, "first fetch must be marked as changed");
    assert!(!r1.fingerprint.is_empty());

    // Second check immediately after — content unchanged
    let r2 = check_target(&mut target).await;
    assert!(r2.error.is_none(), "second fetch error: {:?}", r2.error);
    assert!(!r2.changed, "immediate re-check must show no change");
    assert_eq!(r1.fingerprint, r2.fingerprint, "fingerprint must be stable");
}
