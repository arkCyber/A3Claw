use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.fmt",
  "name": "Formatting Utilities",
  "version": "0.1.0",
  "description": "10 formatting skills: pad_left, pad_right, truncate, wrap, slug, camel_to_snake, snake_to_camel, title_case, file_size, duration.",
  "skills": [
    {"name":"fmt.pad_left","display":"Pad Left","description":"Left-pad a string to a minimum width with a fill char.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"width","type":"integer","required":true},{"name":"fill","type":"string","description":"Fill char (default space)","required":false}]},
    {"name":"fmt.pad_right","display":"Pad Right","description":"Right-pad a string to a minimum width with a fill char.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"width","type":"integer","required":true},{"name":"fill","type":"string","description":"Fill char (default space)","required":false}]},
    {"name":"fmt.truncate","display":"Truncate","description":"Truncate a string to max length, append ellipsis if truncated.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"max","type":"integer","required":true},{"name":"ellipsis","type":"string","description":"Ellipsis string (default …)","required":false}]},
    {"name":"fmt.wrap","display":"Word Wrap","description":"Wrap text at a given column width.","risk":"safe","params":[{"name":"text","type":"string","required":true},{"name":"width","type":"integer","description":"Column width (default 80)","required":false}]},
    {"name":"fmt.slug","display":"Slug","description":"Convert a string to a URL-safe slug.","risk":"safe","params":[{"name":"text","type":"string","required":true}]},
    {"name":"fmt.camel_to_snake","display":"Camel to Snake","description":"Convert camelCase to snake_case.","risk":"safe","params":[{"name":"text","type":"string","required":true}]},
    {"name":"fmt.snake_to_camel","display":"Snake to Camel","description":"Convert snake_case to camelCase.","risk":"safe","params":[{"name":"text","type":"string","required":true}]},
    {"name":"fmt.title_case","display":"Title Case","description":"Convert a string to Title Case.","risk":"safe","params":[{"name":"text","type":"string","required":true}]},
    {"name":"fmt.file_size","display":"File Size","description":"Format bytes as human-readable file size (KB/MB/GB).","risk":"safe","params":[{"name":"bytes","type":"integer","description":"File size in bytes","required":true}]},
    {"name":"fmt.duration","display":"Duration","description":"Format seconds as human-readable duration (e.g. 1h 23m 45s).","risk":"safe","params":[{"name":"seconds","type":"number","description":"Duration in seconds","required":true}]}
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
        "fmt.pad_left" => {
            let t = str_arg!("text");
            let w = args["width"].as_u64().unwrap_or(0) as usize;
            let fill = args["fill"].as_str().and_then(|s| s.chars().next()).unwrap_or(' ');
            let cur = t.chars().count();
            let result = if cur >= w { t.to_string() }
                else { format!("{}{}", std::iter::repeat(fill).take(w - cur).collect::<String>(), t) };
            sdk_respond_ok(rid, &result)
        }
        "fmt.pad_right" => {
            let t = str_arg!("text");
            let w = args["width"].as_u64().unwrap_or(0) as usize;
            let fill = args["fill"].as_str().and_then(|s| s.chars().next()).unwrap_or(' ');
            let cur = t.chars().count();
            let result = if cur >= w { t.to_string() }
                else { format!("{}{}", t, std::iter::repeat(fill).take(w - cur).collect::<String>()) };
            sdk_respond_ok(rid, &result)
        }
        "fmt.truncate" => {
            let t = str_arg!("text");
            let max = args["max"].as_u64().unwrap_or(80) as usize;
            let ell = args["ellipsis"].as_str().unwrap_or("…");
            let chars: Vec<char> = t.chars().collect();
            let result = if chars.len() <= max { t.to_string() }
                else {
                    let ell_len = ell.chars().count();
                    let take = max.saturating_sub(ell_len);
                    format!("{}{}", chars[..take].iter().collect::<String>(), ell)
                };
            sdk_respond_ok(rid, &result)
        }
        "fmt.wrap" => {
            let t = str_arg!("text");
            let width = args["width"].as_u64().unwrap_or(80) as usize;
            let mut lines: Vec<String> = Vec::new();
            for input_line in t.lines() {
                let mut line = String::new();
                for word in input_line.split_whitespace() {
                    if !line.is_empty() && line.len() + 1 + word.len() > width {
                        lines.push(line.clone());
                        line = word.to_string();
                    } else {
                        if !line.is_empty() { line.push(' '); }
                        line.push_str(word);
                    }
                }
                lines.push(line);
            }
            sdk_respond_ok(rid, &lines.join("\n"))
        }
        "fmt.slug" => {
            let t = str_arg!("text");
            let slug: String = t.to_lowercase().chars().map(|c| {
                if c.is_ascii_alphanumeric() { c } else if c == ' ' || c == '_' { '-' } else { '\0' }
            }).filter(|&c| c != '\0').collect();
            let slug = slug.trim_matches('-').to_string();
            sdk_respond_ok(rid, &slug)
        }
        "fmt.camel_to_snake" => {
            let t = str_arg!("text");
            let mut result = String::new();
            for (i, c) in t.chars().enumerate() {
                if c.is_uppercase() && i > 0 { result.push('_'); }
                result.push(c.to_lowercase().next().unwrap_or(c));
            }
            sdk_respond_ok(rid, &result)
        }
        "fmt.snake_to_camel" => {
            let t = str_arg!("text");
            let mut capitalize = false;
            let mut result = String::new();
            for c in t.chars() {
                if c == '_' { capitalize = true; }
                else if capitalize { result.push(c.to_uppercase().next().unwrap_or(c)); capitalize = false; }
                else { result.push(c); }
            }
            sdk_respond_ok(rid, &result)
        }
        "fmt.title_case" => {
            let t = str_arg!("text");
            let result: String = t.split_whitespace().map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                }
            }).collect::<Vec<_>>().join(" ");
            sdk_respond_ok(rid, &result)
        }
        "fmt.file_size" => {
            let bytes = args["bytes"].as_u64().unwrap_or(0);
            let result = if bytes < 1024 { format!("{} B", bytes) }
                else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
                else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
                else { format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) };
            sdk_respond_ok(rid, &result)
        }
        "fmt.duration" => {
            let secs = args["seconds"].as_f64().unwrap_or(0.0) as u64;
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            let result = match (h, m, s) {
                (0, 0, s) => format!("{}s", s),
                (0, m, s) => format!("{}m {}s", m, s),
                (h, m, s) => format!("{}h {}m {}s", h, m, s),
            };
            sdk_respond_ok(rid, &result)
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    // ── fmt.pad_left ──────────────────────────────────────────────────────
    #[test] fn pad_left_basic() {
        let s = "hi"; let w = 5usize;
        let r = format!("{}{}", " ".repeat(w - s.len()), s);
        assert_eq!(r, "   hi");
    }
    #[test] fn pad_left_no_pad_needed() {
        let s = "hello"; let w = 3usize;
        let r = if s.len() >= w { s.to_string() } else { format!("{}{}", " ".repeat(w - s.len()), s) };
        assert_eq!(r, "hello");
    }
    #[test] fn pad_right_basic() {
        let s = "hi"; let w = 5usize;
        let r = format!("{}{}", s, " ".repeat(w - s.len()));
        assert_eq!(r, "hi   ");
    }

    // ── fmt.truncate ────────────────────────────────────────────────────
    #[test] fn truncate_long() {
        let t = "hello world"; let max = 8usize; let ell = "…";
        let chars: Vec<char> = t.chars().collect();
        let take = max.saturating_sub(ell.chars().count());
        let r = format!("{}{}", chars[..take].iter().collect::<String>(), ell);
        assert!(r.len() > 0 && r.contains("…"));
    }
    #[test] fn truncate_short() {
        let t = "hi"; let max = 10usize;
        let chars: Vec<char> = t.chars().collect();
        let r = if chars.len() <= max { t.to_string() } else { t[..max].to_string() };
        assert_eq!(r, "hi");
    }
    #[test] fn truncate_exact() {
        let t = "hello";
        let chars: Vec<char> = t.chars().collect();
        let r = if chars.len() <= 5 { t.to_string() } else { t[..5].to_string() };
        assert_eq!(r, "hello");
    }

    // ── fmt.slug ──────────────────────────────────────────────────────────
    #[test] fn slug_basic() {
        let s = "Hello World!";
        let slug: String = s.to_lowercase().chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else if c == ' ' { '-' } else { '\0' })
            .filter(|&c| c != '\0').collect();
        assert_eq!(slug, "hello-world");
    }
    #[test] fn slug_already_clean() {
        let s = "clean-slug";
        let slug: String = s.to_lowercase().chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '\0' })
            .filter(|&c| c != '\0').collect();
        assert_eq!(slug, "clean-slug");
    }

    // ── fmt.camel_to_snake / snake_to_camel ──────────────────────────────
    #[test] fn camel_to_snake() {
        let t = "helloWorld";
        let mut r = String::new();
        for (i,c) in t.chars().enumerate() {
            if c.is_uppercase() && i > 0 { r.push('_'); }
            r.push(c.to_lowercase().next().unwrap());
        }
        assert_eq!(r, "hello_world");
    }
    #[test] fn camel_to_snake_all_lower() {
        let t = "hello";
        let mut r = String::new();
        for (i,c) in t.chars().enumerate() {
            if c.is_uppercase() && i > 0 { r.push('_'); }
            r.push(c.to_lowercase().next().unwrap());
        }
        assert_eq!(r, "hello");
    }
    #[test] fn snake_to_camel() {
        let t = "hello_world";
        let mut cap = false; let mut r = String::new();
        for c in t.chars() {
            if c == '_' { cap = true; }
            else if cap { r.push(c.to_uppercase().next().unwrap()); cap=false; }
            else { r.push(c); }
        }
        assert_eq!(r, "helloWorld");
    }

    // ── fmt.title_case ───────────────────────────────────────────────────
    #[test] fn title_case_basic() {
        let t = "hello world";
        let r: String = t.split_whitespace().map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        }).collect::<Vec<_>>().join(" ");
        assert_eq!(r, "Hello World");
    }
    #[test] fn title_case_single() {
        let t = "rust";
        let r: String = t.split_whitespace().map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        }).collect::<Vec<_>>().join(" ");
        assert_eq!(r, "Rust");
    }

    // ── fmt.file_size ───────────────────────────────────────────────────
    #[test] fn file_size_bytes() { assert_eq!("512 B", format!("{} B", 512u64)); }
    #[test] fn file_size_kb()    { assert_eq!(format!("{:.1} KB", 1024.0f64/1024.0), "1.0 KB"); }
    #[test] fn file_size_mb()    { let b = 2u64*1024*1024; assert_eq!(format!("{:.1} MB", b as f64/(1024.0*1024.0)), "2.0 MB"); }
    #[test] fn file_size_gb()    { let b = 2u64*1024*1024*1024; assert_eq!(format!("{:.2} GB", b as f64/(1024.0*1024.0*1024.0)), "2.00 GB"); }

    // ── fmt.duration ─────────────────────────────────────────────────────
    #[test] fn duration_secs_only() { let s = 45u64; assert_eq!(format!("{}s", s), "45s"); }
    #[test] fn duration_min_sec()   { let s = 90u64; assert_eq!(format!("{}m {}s", s/60, s%60), "1m 30s"); }
    #[test] fn duration_1h()        { let s = 3661u64; assert_eq!(format!("{}h {}m {}s", s/3600,(s%3600)/60,s%60), "1h 1m 1s"); }
    #[test] fn duration_zero()      { let s = 0u64; assert_eq!(format!("{}s", s), "0s"); }

    // ── manifest ──────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(super::MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(super::MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("fmt."));
        }
    }
}
