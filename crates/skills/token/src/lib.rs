use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.token",
  "name": "Tokenization Utilities",
  "version": "0.1.0",
  "description": "10 text/tokenization skills: word_count, sentence_count, char_freq, top_words, ngrams, levenshtein, jaccard, longest_common_substr, word_wrap_words, is_pangram.",
  "skills": [
    {"name":"token.word_count","display":"Word Count","description":"Count words in a text.","risk":"safe","params":[{"name":"text","type":"string","required":true}]},
    {"name":"token.sentence_count","display":"Sentence Count","description":"Count sentences (split on .!?) in a text.","risk":"safe","params":[{"name":"text","type":"string","required":true}]},
    {"name":"token.char_freq","display":"Char Frequency","description":"Return character frequency as a JSON object.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"lowercase","type":"boolean","description":"Normalize to lowercase","required":false}]},
    {"name":"token.top_words","display":"Top Words","description":"Return the top N most frequent words as a JSON array.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"n","type":"integer","description":"Number of top words (default 10)","required":false}]},
    {"name":"token.ngrams","display":"N-grams","description":"Return all N-grams (word-level) of a text as a JSON array.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"n","type":"integer","description":"N (default 2 = bigrams)","required":false}]},
    {"name":"token.levenshtein","display":"Levenshtein","description":"Edit distance between two strings.","risk":"safe","params":[{"name":"a","type":"string","required":true},{"name":"b","type":"string","required":true}]},
    {"name":"token.jaccard","display":"Jaccard Similarity","description":"Jaccard similarity (word-set overlap) between two texts.","risk":"safe","params":[{"name":"a","type":"string","required":true},{"name":"b","type":"string","required":true}]},
    {"name":"token.longest_common_substr","display":"Longest Common Substring","description":"Find the longest common substring of two strings.","risk":"safe","params":[{"name":"a","type":"string","required":true},{"name":"b","type":"string","required":true}]},
    {"name":"token.word_wrap_words","display":"Word Wrap (words)","description":"Return words split into lines of at most N words each.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"words_per_line","type":"integer","description":"Words per line (default 10)","required":false}]},
    {"name":"token.is_pangram","display":"Is Pangram","description":"Check if text contains every letter of the alphabet.","risk":"safe","params":[{"name":"text","type":"string","required":true}]}
  ]
}"#;

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] { dp[i-1][j-1] }
                       else { 1 + dp[i-1][j].min(dp[i][j-1]).min(dp[i-1][j-1]) };
        }
    }
    dp[m][n]
}

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r)  => r,
        Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();
    let args = &req.args;

    macro_rules! str_arg {
        ($k:literal) => {
            match args[$k].as_str() {
                Some(s) => s,
                None => return sdk_respond_err(rid, concat!("missing '", $k, "'")),
            }
        };
    }

    match req.skill.as_str() {
        "token.word_count" => {
            let t = str_arg!("text");
            sdk_respond_ok(rid, &words(t).len().to_string())
        }
        "token.sentence_count" => {
            let t = str_arg!("text");
            let count = t.chars().filter(|&c| c == '.' || c == '!' || c == '?').count();
            sdk_respond_ok(rid, &count.max(if t.trim().is_empty() { 0 } else { 1 }).to_string())
        }
        "token.char_freq" => {
            let t = str_arg!("text");
            let lower = args["lowercase"].as_bool().unwrap_or(true);
            let mut freq: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
            for c in t.chars() {
                if c.is_alphabetic() {
                    let ch = if lower { c.to_lowercase().next().unwrap_or(c) } else { c };
                    *freq.entry(ch.to_string()).or_insert(0) += 1;
                }
            }
            let obj: serde_json::Map<String, serde_json::Value> = freq.into_iter()
                .map(|(k, v)| (k, serde_json::Value::Number(serde_json::Number::from(v))))
                .collect();
            sdk_respond_ok(rid, &serde_json::to_string(&obj).unwrap_or_default())
        }
        "token.top_words" => {
            let t = str_arg!("text");
            let n = args["n"].as_u64().unwrap_or(10) as usize;
            let ws = words(t);
            let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for w in &ws { *freq.entry(w.clone()).or_insert(0) += 1; }
            let mut pairs: Vec<(String, usize)> = freq.into_iter().collect();
            pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let out: Vec<serde_json::Value> = pairs.into_iter().take(n).map(|(w, c)| {
                serde_json::json!({"word": w, "count": c})
            }).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "token.ngrams" => {
            let t = str_arg!("text");
            let n = args["n"].as_u64().unwrap_or(2) as usize;
            let ws = words(t);
            if ws.len() < n { return sdk_respond_ok(rid, "[]"); }
            let out: Vec<serde_json::Value> = ws.windows(n).map(|w| {
                serde_json::Value::String(w.join(" "))
            }).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "token.levenshtein" => {
            let a = str_arg!("a");
            let b = str_arg!("b");
            sdk_respond_ok(rid, &levenshtein(a, b).to_string())
        }
        "token.jaccard" => {
            let a = str_arg!("a");
            let b = str_arg!("b");
            let wa: std::collections::HashSet<String> = words(a).into_iter().collect();
            let wb: std::collections::HashSet<String> = words(b).into_iter().collect();
            let inter = wa.intersection(&wb).count();
            let union = wa.union(&wb).count();
            if union == 0 { return sdk_respond_ok(rid, "1.0"); }
            sdk_respond_ok(rid, &format!("{:.4}", inter as f64 / union as f64))
        }
        "token.longest_common_substr" => {
            let a: Vec<char> = str_arg!("a").chars().collect();
            let b: Vec<char> = str_arg!("b").chars().collect();
            let (m, n) = (a.len(), b.len());
            let mut best = 0usize;
            let mut best_end = 0usize;
            let mut dp = vec![vec![0usize; n + 1]; m + 1];
            for i in 1..=m {
                for j in 1..=n {
                    if a[i-1] == b[j-1] {
                        dp[i][j] = dp[i-1][j-1] + 1;
                        if dp[i][j] > best { best = dp[i][j]; best_end = i; }
                    }
                }
            }
            let result: String = a[best_end.saturating_sub(best)..best_end].iter().collect();
            sdk_respond_ok(rid, &result)
        }
        "token.word_wrap_words" => {
            let t = str_arg!("text");
            let wpl = args["words_per_line"].as_u64().unwrap_or(10) as usize;
            let ws = words(t);
            let lines: Vec<String> = ws.chunks(wpl).map(|c| c.join(" ")).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&lines).unwrap_or_default())
        }
        "token.is_pangram" => {
            let t = str_arg!("text").to_lowercase();
            let missing: Vec<char> = ('a'..='z').filter(|&c| !t.contains(c)).collect();
            sdk_respond_ok(rid, if missing.is_empty() { "true" } else { "false" })
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── token.word_count ──────────────────────────────────────────────────
    #[test] fn word_count_basic()  { assert_eq!(words("hello world foo").len(), 3); }
    #[test] fn word_count_empty()  { assert_eq!(words("").len(), 0); }
    #[test] fn word_count_punct()  { assert_eq!(words("hello, world!").len(), 2); }

    // ── token.sentence_count ──────────────────────────────────────────────
    #[test] fn sentence_one_dot()   { let t="Hello."; let c=t.chars().filter(|&c| c=='.'||c=='!'||c=='?').count(); assert_eq!(c.max(1), 1); }
    #[test] fn sentence_two()       { let t="Hi! How are you?"; let c=t.chars().filter(|&ch| ch=='.'||ch=='!'||ch=='?').count(); assert_eq!(c, 2); }
    #[test] fn sentence_empty()     { let t=""; assert_eq!(t.trim().is_empty(), true); }

    // ── token.levenshtein ────────────────────────────────────────────────
    #[test] fn levenshtein_same()     { assert_eq!(levenshtein("abc", "abc"), 0); }
    #[test] fn levenshtein_diff()     { assert_eq!(levenshtein("kitten", "sitting"), 3); }
    #[test] fn levenshtein_empty_a()  { assert_eq!(levenshtein("", "abc"), 3); }
    #[test] fn levenshtein_empty_b()  { assert_eq!(levenshtein("abc", ""), 3); }
    #[test] fn levenshtein_one_char() { assert_eq!(levenshtein("a", "b"), 1); }

    // ── token.jaccard ─────────────────────────────────────────────────────
    #[test] fn jaccard_identical() {
        let wa: std::collections::HashSet<String> = words("hello world").into_iter().collect();
        let wb: std::collections::HashSet<String> = words("hello world").into_iter().collect();
        let inter = wa.intersection(&wb).count();
        let union = wa.union(&wb).count();
        assert_eq!(inter as f64 / union as f64, 1.0);
    }
    #[test] fn jaccard_disjoint() {
        let wa: std::collections::HashSet<String> = words("foo bar").into_iter().collect();
        let wb: std::collections::HashSet<String> = words("baz qux").into_iter().collect();
        let inter = wa.intersection(&wb).count();
        assert_eq!(inter, 0);
    }
    #[test] fn jaccard_partial() {
        let wa: std::collections::HashSet<String> = words("a b c").into_iter().collect();
        let wb: std::collections::HashSet<String> = words("b c d").into_iter().collect();
        let inter = wa.intersection(&wb).count();
        let union = wa.union(&wb).count();
        let j = inter as f64 / union as f64;
        assert!((j - 0.5).abs() < 1e-9, "expected 0.5 got {}", j);
    }

    // ── token.longest_common_substr ───────────────────────────────────────
    #[test] fn lcs_basic() {
        let a: Vec<char> = "abcdef".chars().collect();
        let b: Vec<char> = "zbcdf".chars().collect();
        let (m, n) = (a.len(), b.len());
        let mut best = 0; let mut best_end = 0;
        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 1..=m { for j in 1..=n { if a[i-1]==b[j-1] { dp[i][j]=dp[i-1][j-1]+1; if dp[i][j]>best { best=dp[i][j]; best_end=i; } } } }
        let result: String = a[best_end.saturating_sub(best)..best_end].iter().collect();
        assert_eq!(result, "bcd");
    }
    #[test] fn lcs_no_common() {
        let a: Vec<char> = "abc".chars().collect();
        let b: Vec<char> = "xyz".chars().collect();
        let (m, n) = (a.len(), b.len());
        let mut best = 0; let mut best_end = 0;
        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 1..=m { for j in 1..=n { if a[i-1]==b[j-1] { dp[i][j]=dp[i-1][j-1]+1; if dp[i][j]>best { best=dp[i][j]; best_end=i; } } } }
        let result: String = a[best_end.saturating_sub(best)..best_end].iter().collect();
        assert_eq!(result, "");
    }

    // ── token.ngrams ──────────────────────────────────────────────────────
    #[test] fn ngrams_bigrams() {
        let ws = vec!["a","b","c"];
        let ng: Vec<String> = ws.windows(2).map(|w| w.join(" ")).collect();
        assert_eq!(ng, vec!["a b", "b c"]);
    }
    #[test] fn ngrams_trigrams() {
        let ws = vec!["a","b","c","d"];
        let ng: Vec<String> = ws.windows(3).map(|w| w.join(" ")).collect();
        assert_eq!(ng, vec!["a b c", "b c d"]);
    }
    #[test] fn ngrams_too_short() {
        let ws = vec!["a"];
        assert!(ws.windows(2).count() == 0);
    }

    // ── token.is_pangram ─────────────────────────────────────────────────
    #[test] fn is_pangram_true() {
        let t = "the quick brown fox jumps over the lazy dog";
        let missing: Vec<char> = ('a'..='z').filter(|&c| !t.contains(c)).collect();
        assert!(missing.is_empty());
    }
    #[test] fn is_pangram_false() {
        let t = "hello world";
        let missing: Vec<char> = ('a'..='z').filter(|&c| !t.contains(c)).collect();
        assert!(!missing.is_empty());
    }
    #[test] fn is_pangram_empty() {
        let missing: Vec<char> = ('a'..='z').filter(|&c| !"".contains(c)).collect();
        assert_eq!(missing.len(), 26);
    }

    // ── manifest ──────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("token."));
        }
    }
}
