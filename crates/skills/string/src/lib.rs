use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.string",
  "name": "String Utilities",
  "version": "0.1.0",
  "description": "10 string manipulation skills: reverse, repeat, contains, starts_with, ends_with, char_count, lines, trim, indent, dedent.",
  "skills": [
    {"name":"string.reverse","display":"Reverse","description":"Reverse a string.","risk":"safe","params":[{"name":"text","type":"string","description":"Input string","required":true}]},
    {"name":"string.repeat","display":"Repeat","description":"Repeat a string N times.","risk":"safe","params":[{"name":"text","type":"string","description":"String to repeat","required":true},{"name":"count","type":"integer","description":"Repetition count","required":true}]},
    {"name":"string.contains","display":"Contains","description":"Check if string contains a substring.","risk":"safe","params":[{"name":"text","type":"string","description":"Haystack","required":true},{"name":"needle","type":"string","description":"Needle","required":true}]},
    {"name":"string.starts_with","display":"Starts With","description":"Check if string starts with prefix.","risk":"safe","params":[{"name":"text","type":"string","description":"String","required":true},{"name":"prefix","type":"string","description":"Prefix","required":true}]},
    {"name":"string.ends_with","display":"Ends With","description":"Check if string ends with suffix.","risk":"safe","params":[{"name":"text","type":"string","description":"String","required":true},{"name":"suffix","type":"string","description":"Suffix","required":true}]},
    {"name":"string.char_count","display":"Char Count","description":"Count Unicode characters (not bytes).","risk":"safe","params":[{"name":"text","type":"string","description":"Input","required":true}]},
    {"name":"string.lines","display":"Lines","description":"Split string into lines, return JSON array.","risk":"safe","params":[{"name":"text","type":"string","description":"Input","required":true}]},
    {"name":"string.trim","display":"Trim","description":"Trim leading/trailing whitespace (or custom chars).","risk":"safe","params":[{"name":"text","type":"string","description":"Input","required":true},{"name":"chars","type":"string","description":"Optional chars to trim","required":false}]},
    {"name":"string.indent","display":"Indent","description":"Indent every line by N spaces.","risk":"safe","params":[{"name":"text","type":"string","description":"Input","required":true},{"name":"spaces","type":"integer","description":"Number of spaces","required":true}]},
    {"name":"string.dedent","display":"Dedent","description":"Remove common leading whitespace from all lines.","risk":"safe","params":[{"name":"text","type":"string","description":"Input","required":true}]}
  ]
}"#;

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
        "string.reverse" => {
            let t = str_arg!("text");
            sdk_respond_ok(rid, &t.chars().rev().collect::<String>())
        }
        "string.repeat" => {
            let t = str_arg!("text");
            let n = args["count"].as_u64().unwrap_or(1) as usize;
            sdk_respond_ok(rid, &t.repeat(n))
        }
        "string.contains" => {
            let t = str_arg!("text");
            let n = str_arg!("needle");
            sdk_respond_ok(rid, if t.contains(n) { "true" } else { "false" })
        }
        "string.starts_with" => {
            let t = str_arg!("text");
            let p = str_arg!("prefix");
            sdk_respond_ok(rid, if t.starts_with(p) { "true" } else { "false" })
        }
        "string.ends_with" => {
            let t = str_arg!("text");
            let s = str_arg!("suffix");
            sdk_respond_ok(rid, if t.ends_with(s) { "true" } else { "false" })
        }
        "string.char_count" => {
            let t = str_arg!("text");
            sdk_respond_ok(rid, &t.chars().count().to_string())
        }
        "string.lines" => {
            let t = str_arg!("text");
            let arr: Vec<serde_json::Value> = t.lines().map(|l| serde_json::Value::String(l.to_string())).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&arr).unwrap_or_default())
        }
        "string.trim" => {
            let t = str_arg!("text");
            let result = if let Some(chars) = args["chars"].as_str() {
                let set: std::collections::HashSet<char> = chars.chars().collect();
                t.trim_matches(|c| set.contains(&c)).to_string()
            } else {
                t.trim().to_string()
            };
            sdk_respond_ok(rid, &result)
        }
        "string.indent" => {
            let t = str_arg!("text");
            let n = args["spaces"].as_u64().unwrap_or(2) as usize;
            let pad = " ".repeat(n);
            let result = t.lines().map(|l| format!("{}{}", pad, l)).collect::<Vec<_>>().join("\n");
            sdk_respond_ok(rid, &result)
        }
        "string.dedent" => {
            let t = str_arg!("text");
            let min_indent = t.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            let result = t.lines().map(|l| {
                if l.len() >= min_indent { &l[min_indent..] } else { l }
            }).collect::<Vec<_>>().join("\n");
            sdk_respond_ok(rid, &result)
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── string.reverse ──────────────────────────────────────────────────
    #[test] fn reverse_ascii()   { assert_eq!("cba", "abc".chars().rev().collect::<String>()); }
    #[test] fn reverse_empty()   { assert_eq!("", "".chars().rev().collect::<String>()); }
    #[test] fn reverse_unicode() {
        let s = "中文";
        let r: String = s.chars().rev().collect();
        assert_eq!(r, "文中");
    }
    #[test] fn reverse_palindrome() { assert_eq!("racecar", "racecar".chars().rev().collect::<String>()); }

    // ── string.repeat ───────────────────────────────────────────────────
    #[test] fn repeat_x3()  { assert_eq!("aaa", "a".repeat(3)); }
    #[test] fn repeat_zero() { assert_eq!("", "abc".repeat(0)); }
    #[test] fn repeat_one()  { assert_eq!("xy", "xy".repeat(1)); }

    // ── string.contains / starts_with / ends_with ─────────────────────────
    #[test] fn contains_true()      { assert!("hello world".contains("world")); }
    #[test] fn contains_false()     { assert!(!"hello".contains("xyz")); }
    #[test] fn starts_with_ok()     { assert!("foobar".starts_with("foo")); }
    #[test] fn starts_with_fail()   { assert!(!"foobar".starts_with("bar")); }
    #[test] fn ends_with_ok()       { assert!("foobar".ends_with("bar")); }
    #[test] fn ends_with_fail()     { assert!(!"foobar".ends_with("foo")); }

    // ── string.char_count ────────────────────────────────────────────────
    #[test] fn char_count_ascii()   { assert_eq!(3, "abc".chars().count()); }
    #[test] fn char_count_unicode() {
        assert_eq!(3, "中文A".chars().count(), "3 unicode chars");
    }
    #[test] fn char_count_empty()   { assert_eq!(0, "".chars().count()); }

    // ── string.lines ─────────────────────────────────────────────────────
    #[test] fn lines_two()   { assert_eq!(2, "a\nb".lines().count()); }
    #[test] fn lines_one()   { assert_eq!(1, "hello".lines().count()); }
    #[test] fn lines_empty() { assert_eq!(0, "".lines().count()); }

    // ── string.trim ──────────────────────────────────────────────────────
    #[test] fn trim_spaces()  { assert_eq!("hi", "  hi  ".trim()); }
    #[test] fn trim_chars()   {
        let chars: std::collections::HashSet<char> = "/*".chars().collect();
        assert_eq!("hello", "/*hello*/".trim_matches(|c| chars.contains(&c)));
    }
    #[test] fn trim_already_clean() { assert_eq!("hi", "hi".trim()); }

    // ── string.indent / dedent ───────────────────────────────────────────
    #[test] fn indent_two() {
        let r = "a\nb".lines().map(|l| format!("  {}", l)).collect::<Vec<_>>().join("\n");
        assert_eq!("  a\n  b", r);
    }
    #[test] fn indent_zero() {
        let r = "a\nb".lines().map(|l| format!("{}", l)).collect::<Vec<_>>().join("\n");
        assert_eq!("a\nb", r);
    }
    #[test] fn dedent_common_indent() {
        let text = "    hello\n    world";
        let min_indent = text.lines().filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
        let result = text.lines().map(|l| if l.len() >= min_indent { &l[min_indent..] } else { l })
            .collect::<Vec<_>>().join("\n");
        assert_eq!(result, "hello\nworld");
    }
    #[test] fn dedent_mixed_indent() {
        let text = "  a\n    b";
        let min_indent = text.lines().filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
        assert_eq!(min_indent, 2);
    }

    // ── manifest ──────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("string."));
        }
    }
}
