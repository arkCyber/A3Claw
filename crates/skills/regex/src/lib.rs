//! skill-regex — simple regex-like pattern matching without external crates.
//!
//! Implements a subset of regex for wasm32 with zero external dependencies:
//!   - Literal chars, `.` (any char), `*` (0+), `+` (1+), `?` (0/1)
//!   - `^` (start anchor), `$` (end anchor)
//!   - `[abc]` character classes, `[^abc]` negated classes
//!   - `\d` `\w` `\s` escape sequences
//!
//! Skills exposed:
//!   regex.test     { text: string, pattern: string }  → "true" | "false"
//!   regex.find     { text: string, pattern: string }  → first match or ""
//!   regex.find_all { text: string, pattern: string }  → JSON array of matches
//!   regex.replace  { text: string, pattern: string, replacement: string } → string
//!   regex.split    { text: string, pattern: string }  → JSON array of parts
//!   regex.count    { text: string, pattern: string }  → count as string

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.regex",
  "name": "Regex Skills",
  "version": "0.1.0",
  "description": "Regex-like pattern matching: test, find, find_all, replace, split, count",
  "skills": [
    {
      "name": "regex.test",
      "display": "Test Pattern",
      "description": "Test if a pattern matches anywhere in the text. Returns 'true' or 'false'.",
      "risk": "safe",
      "params": [
        { "name": "text",    "type": "string", "description": "Input text",    "required": true },
        { "name": "pattern", "type": "string", "description": "Regex pattern", "required": true }
      ]
    },
    {
      "name": "regex.find",
      "display": "Find First Match",
      "description": "Find the first match of a pattern in text. Returns matched string or empty.",
      "risk": "safe",
      "params": [
        { "name": "text",    "type": "string", "description": "Input text",    "required": true },
        { "name": "pattern", "type": "string", "description": "Regex pattern", "required": true }
      ]
    },
    {
      "name": "regex.find_all",
      "display": "Find All Matches",
      "description": "Find all non-overlapping matches. Returns a JSON array of strings.",
      "risk": "safe",
      "params": [
        { "name": "text",    "type": "string", "description": "Input text",    "required": true },
        { "name": "pattern", "type": "string", "description": "Regex pattern", "required": true }
      ]
    },
    {
      "name": "regex.replace",
      "display": "Replace First Match",
      "description": "Replace the first match of a pattern with a replacement string.",
      "risk": "safe",
      "params": [
        { "name": "text",        "type": "string", "description": "Input text",        "required": true },
        { "name": "pattern",     "type": "string", "description": "Regex pattern",     "required": true },
        { "name": "replacement", "type": "string", "description": "Replacement string","required": true }
      ]
    },
    {
      "name": "regex.split",
      "display": "Split by Pattern",
      "description": "Split text at each match of pattern. Returns JSON array of parts.",
      "risk": "safe",
      "params": [
        { "name": "text",    "type": "string", "description": "Input text",    "required": true },
        { "name": "pattern", "type": "string", "description": "Regex pattern", "required": true }
      ]
    },
    {
      "name": "regex.count",
      "display": "Count Matches",
      "description": "Count the number of non-overlapping matches in text.",
      "risk": "safe",
      "params": [
        { "name": "text",    "type": "string", "description": "Input text",    "required": true },
        { "name": "pattern", "type": "string", "description": "Regex pattern", "required": true }
      ]
    }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 {
    sdk_export_str(MANIFEST)
}

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r)  => r,
        Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();

    let text    = match req.args["text"].as_str()    { Some(s) => s, None => return sdk_respond_err(rid, "missing 'text'") };
    let pattern = match req.args["pattern"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'pattern'") };

    match req.skill.as_str() {
        "regex.test" => {
            sdk_respond_ok(rid, if regex_find(text, pattern).is_some() { "true" } else { "false" })
        }
        "regex.find" => {
            match regex_find(text, pattern) {
                Some(m) => sdk_respond_ok(rid, &m),
                None    => sdk_respond_ok(rid, ""),
            }
        }
        "regex.find_all" => {
            let matches = regex_find_all(text, pattern);
            let json: Vec<serde_json::Value> = matches.into_iter().map(serde_json::Value::String).collect();
            sdk_respond_ok(rid, &serde_json::Value::Array(json).to_string())
        }
        "regex.replace" => {
            let replacement = match req.args["replacement"].as_str() {
                Some(s) => s, None => return sdk_respond_err(rid, "missing 'replacement'"),
            };
            sdk_respond_ok(rid, &regex_replace(text, pattern, replacement))
        }
        "regex.split" => {
            let parts = regex_split(text, pattern);
            let json: Vec<serde_json::Value> = parts.into_iter().map(serde_json::Value::String).collect();
            sdk_respond_ok(rid, &serde_json::Value::Array(json).to_string())
        }
        "regex.count" => {
            let count = regex_find_all(text, pattern).len();
            sdk_respond_ok(rid, &count.to_string())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Minimalist NFA-based regex engine ─────────────────────────────────────────
// Compiles pattern to a Vec<Token>, then runs NFA simulation.

#[derive(Debug, Clone)]
enum Token {
    Literal(char),
    AnyChar,
    CharClass(Vec<char>, bool), // chars, negated
    Anchor(bool),               // true=start, false=end
}

#[derive(Debug, Clone)]
struct Atom {
    token: Token,
    quant: Quant,
}

#[derive(Debug, Clone, PartialEq)]
enum Quant { One, ZeroOrMore, OneOrMore, ZeroOrOne }

fn parse_pattern(pattern: &str) -> Vec<Atom> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut atoms = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (token, advance) = match chars[i] {
            '^' => { atoms.push(Atom { token: Token::Anchor(true),  quant: Quant::One }); i += 1; continue; }
            '$' => { atoms.push(Atom { token: Token::Anchor(false), quant: Quant::One }); i += 1; continue; }
            '.' => (Token::AnyChar, 1),
            '[' => {
                let (cls, len) = parse_class(&chars[i..]);
                (cls, len)
            }
            '\\' if i + 1 < chars.len() => {
                let t = match chars[i+1] {
                    'd' => Token::CharClass("0123456789".chars().collect(), false),
                    'D' => Token::CharClass("0123456789".chars().collect(), true),
                    'w' => Token::CharClass("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_".chars().collect(), false),
                    'W' => Token::CharClass("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_".chars().collect(), true),
                    's' => Token::CharClass(vec![' ', '\t', '\n', '\r'], false),
                    'S' => Token::CharClass(vec![' ', '\t', '\n', '\r'], true),
                    c   => Token::Literal(c),
                };
                (t, 2)
            }
            c => (Token::Literal(c), 1),
        };
        i += advance;
        let quant = if i < chars.len() {
            match chars[i] {
                '*' => { i += 1; Quant::ZeroOrMore }
                '+' => { i += 1; Quant::OneOrMore }
                '?' => { i += 1; Quant::ZeroOrOne }
                _   => Quant::One,
            }
        } else { Quant::One };
        atoms.push(Atom { token, quant });
    }
    atoms
}

fn parse_class(chars: &[char]) -> (Token, usize) {
    let negated = chars.len() > 1 && chars[1] == '^';
    let start = if negated { 2 } else { 1 };
    let mut cls = Vec::new();
    let mut i = start;
    while i < chars.len() && chars[i] != ']' {
        if i + 2 < chars.len() && chars[i+1] == '-' && chars[i+2] != ']' {
            let from = chars[i] as u32;
            let to   = chars[i+2] as u32;
            for cp in from..=to {
                if let Some(c) = char::from_u32(cp) { cls.push(c); }
            }
            i += 3;
        } else {
            cls.push(chars[i]);
            i += 1;
        }
    }
    let len = i + if i < chars.len() { 1 } else { 0 }; // skip ']'
    (Token::CharClass(cls, negated), len)
}

fn token_matches(token: &Token, c: char) -> bool {
    match token {
        Token::Literal(l)       => *l == c,
        Token::AnyChar          => c != '\n',
        Token::CharClass(cs, neg) => cs.contains(&c) ^ neg,
        Token::Anchor(_)        => false,
    }
}

fn match_at(text: &[char], pos: usize, atoms: &[Atom]) -> Option<usize> {
    if atoms.is_empty() { return Some(pos); }
    let atom = &atoms[0];
    let rest = &atoms[1..];

    if let Token::Anchor(start) = &atom.token {
        return if *start {
            if pos == 0 { match_at(text, pos, rest) } else { None }
        } else {
            if pos == text.len() { match_at(text, pos, rest) } else { None }
        };
    }

    match &atom.quant {
        Quant::One => {
            if pos < text.len() && token_matches(&atom.token, text[pos]) {
                match_at(text, pos + 1, rest)
            } else { None }
        }
        Quant::ZeroOrOne => {
            // Try consuming one, then try consuming none
            if pos < text.len() && token_matches(&atom.token, text[pos]) {
                if let Some(end) = match_at(text, pos + 1, rest) { return Some(end); }
            }
            match_at(text, pos, rest)
        }
        Quant::ZeroOrMore => {
            // Greedy: find max match, try each
            let mut end = pos;
            while end < text.len() && token_matches(&atom.token, text[end]) { end += 1; }
            while end >= pos {
                if let Some(r) = match_at(text, end, rest) { return Some(r); }
                if end == pos { break; }
                end -= 1;
            }
            None
        }
        Quant::OneOrMore => {
            if pos >= text.len() || !token_matches(&atom.token, text[pos]) { return None; }
            let mut end = pos + 1;
            while end < text.len() && token_matches(&atom.token, text[end]) { end += 1; }
            while end > pos {
                if let Some(r) = match_at(text, end, rest) { return Some(r); }
                end -= 1;
            }
            None
        }
    }
}

fn find_in(text: &[char], atoms: &[Atom], start_from: usize) -> Option<(usize, usize)> {
    let anchored_start = atoms.first().map(|a| matches!(a.token, Token::Anchor(true))).unwrap_or(false);
    let search_range = if anchored_start { start_from..=start_from } else { start_from..=text.len() };
    for start in search_range {
        if let Some(end) = match_at(text, start, atoms) {
            if end == start && !atoms.is_empty() { continue; } // skip zero-length matches in find_all
            return Some((start, end));
        }
    }
    None
}

fn regex_find(text: &str, pattern: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let atoms = parse_pattern(pattern);
    find_in(&chars, &atoms, 0).map(|(s, e)| chars[s..e].iter().collect())
}

fn regex_find_all(text: &str, pattern: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let atoms = parse_pattern(pattern);
    let mut results = Vec::new();
    let mut pos = 0;
    while pos <= chars.len() {
        match find_in(&chars, &atoms, pos) {
            Some((s, e)) if e > s => {
                results.push(chars[s..e].iter().collect());
                pos = e;
            }
            _ => break,
        }
    }
    results
}

fn regex_replace(text: &str, pattern: &str, replacement: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let atoms = parse_pattern(pattern);
    if let Some((s, e)) = find_in(&chars, &atoms, 0) {
        let mut result: String = chars[..s].iter().collect();
        result.push_str(replacement);
        result.extend(chars[e..].iter());
        result
    } else {
        text.to_string()
    }
}

fn regex_split(text: &str, pattern: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let atoms = parse_pattern(pattern);
    let mut parts = Vec::new();
    let mut pos = 0;
    loop {
        match find_in(&chars, &atoms, pos) {
            Some((s, e)) if e > s => {
                parts.push(chars[pos..s].iter().collect());
                pos = e;
            }
            _ => break,
        }
    }
    parts.push(chars[pos..].iter().collect());
    parts
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_match() {
        assert_eq!(regex_find("hello world", "world"), Some("world".into()));
    }

    #[test]
    fn test_literal_no_match() {
        assert!(regex_find("hello", "xyz").is_none());
    }

    #[test]
    fn test_dot_star() {
        assert!(regex_find("abc123", r"a.*3").is_some());
    }

    #[test]
    fn test_digit_class() {
        assert_eq!(regex_find("price: 42 USD", r"\d+"), Some("42".into()));
    }

    #[test]
    fn test_find_all() {
        let all = regex_find_all("a1b2c3", r"\d");
        assert_eq!(all, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_replace() {
        let result = regex_replace("hello world", "world", "Rust");
        assert_eq!(result, "hello Rust");
    }

    #[test]
    fn test_split() {
        let parts = regex_split("one,two,three", ",");
        assert_eq!(parts, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_anchor_start() {
        assert!(regex_find("hello", "^hello").is_some());
        assert!(regex_find("say hello", "^hello").is_none());
    }

    #[test]
    fn test_anchor_end() {
        assert!(regex_find("world", r"world$").is_some());
        assert!(regex_find("worldwide", r"world$").is_none());
    }

    #[test]
    fn test_char_class() {
        assert_eq!(regex_find("aeiou", "[aeiou]+"), Some("aeiou".into()));
    }

    #[test]
    fn test_negated_class() {
        assert_eq!(regex_find("abc123", "[^a-z]+"), Some("123".into()));
    }

    #[test]
    fn test_count() {
        assert_eq!(regex_find_all("banana", "a").len(), 3);
    }

    #[test]
    fn test_plus_quantifier() {
        assert_eq!(regex_find("aaa", "a+"), Some("aaa".into()));
        assert!(regex_find("bbb", "a+").is_none());
    }

    #[test]
    fn test_question_quantifier() {
        assert_eq!(regex_find("colour", "colou?r"), Some("colour".into()));
        assert_eq!(regex_find("color", "colou?r"), Some("color".into()));
    }

    // ── extra match tests ───────────────────────────────────────────────────────
    #[test]
    fn test_match_digit_class() {
        assert!(regex_find("abc123", r"\d+").is_some());
    }
    #[test]
    fn test_match_word_class() {
        assert!(regex_find("hello", r"\w+").is_some());
    }
    #[test]
    fn test_no_match_empty_pattern() {
        assert!(regex_find("anything", ".*").is_some());
    }
    #[test]
    fn test_dot_matches_any() {
        assert!(regex_find("abc", "a.c").is_some());
        assert!(regex_find("ac", "a.c").is_none());
    }
    #[test]
    fn test_star_quantifier_zero() {
        assert!(regex_find("b", "a*b").is_some());
    }
    #[test]
    fn test_star_quantifier_many() {
        assert!(regex_find("aaab", "a*b").is_some());
    }

    // ── find all ──────────────────────────────────────────────────────────────────
    #[test]
    fn test_find_all_words() {
        let all = regex_find_all("the cat sat", r"\w+");
        assert_eq!(all.len(), 3);
    }
    #[test]
    fn test_find_all_empty_result() {
        let all = regex_find_all("abc", r"\d+");
        assert!(all.is_empty());
    }

    // ── replace all ─────────────────────────────────────────────────────────────
    #[test]
    fn test_replace_no_match() {
        assert_eq!(regex_replace("hello", "xyz", "ZZZ"), "hello");
    }
    #[test]
    fn test_replace_first_only() {
        let result = regex_replace("aaa", "a", "b");
        assert!(result.starts_with('b'), "first replacement should be b: {result}");
    }

    // ── split ─────────────────────────────────────────────────────────────────────
    #[test]
    fn test_split_pipe_delim() {
        let parts = regex_split("a|b|c", r"\|");
        assert_eq!(parts, vec!["a", "b", "c"]);
    }
    #[test]
    fn test_split_no_delim_returns_whole() {
        let parts = regex_split("hello", ",");
        assert_eq!(parts, vec!["hello"]);
    }

    #[test]
    fn manifest_is_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.regex");
        assert_eq!(v["skills"].as_array().unwrap().len(), 6);
    }
    #[test]
    fn manifest_skill_names_start_with_regex() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("regex."));
        }
    }
}
