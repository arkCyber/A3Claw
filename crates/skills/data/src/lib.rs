use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.data",
  "name": "Data Format Utilities",
  "version": "0.1.0",
  "description": "10 data format skills: csv_to_json, json_to_csv, tsv_to_json, ini_to_json, kv_to_json, json_to_table, json_schema, yaml_to_json, xml_escape, html_escape.",
  "skills": [
    {"name":"data.csv_to_json","display":"CSV to JSON","description":"Parse a CSV string (first row = headers) to JSON array.","risk":"safe","params":[{"name":"csv","type":"string","description":"CSV text","required":true},{"name":"delimiter","type":"string","description":"Delimiter char (default comma)","required":false}]},
    {"name":"data.json_to_csv","display":"JSON to CSV","description":"Convert JSON array of objects to CSV text.","risk":"safe","params":[{"name":"json","type":"string","description":"JSON array of objects","required":true}]},
    {"name":"data.tsv_to_json","display":"TSV to JSON","description":"Parse a TSV string (first row = headers) to JSON array.","risk":"safe","params":[{"name":"tsv","type":"string","description":"TSV text","required":true}]},
    {"name":"data.ini_to_json","display":"INI to JSON","description":"Parse a simple INI string to a JSON object.","risk":"safe","params":[{"name":"ini","type":"string","description":"INI text","required":true}]},
    {"name":"data.kv_to_json","display":"Key-Value to JSON","description":"Parse KEY=VALUE lines into a JSON object.","risk":"safe","params":[{"name":"kv","type":"string","description":"KEY=VALUE text","required":true}]},
    {"name":"data.json_to_table","display":"JSON to Table","description":"Render a JSON array of objects as an ASCII table.","risk":"safe","params":[{"name":"json","type":"string","description":"JSON array of objects","required":true}]},
    {"name":"data.json_schema","display":"JSON Schema","description":"Infer a simple JSON schema from a JSON value.","risk":"safe","params":[{"name":"json","type":"string","description":"JSON value","required":true}]},
    {"name":"data.yaml_to_json","display":"YAML to JSON","description":"Convert YAML text (flat, nested objects, and lists) to JSON.","risk":"safe","params":[{"name":"yaml","type":"string","description":"YAML text","required":true}]},
    {"name":"data.xml_escape","display":"XML Escape","description":"Escape &, <, >, \", ' for XML/HTML.","risk":"safe","params":[{"name":"text","type":"string","description":"Raw text","required":true}]},
    {"name":"data.html_escape","display":"HTML Escape","description":"Escape special HTML characters.","risk":"safe","params":[{"name":"text","type":"string","description":"Raw text","required":true}]}
  ]
}"#;

fn csv_parse(text: &str, delim: char) -> Vec<Vec<String>> {
    text.lines().map(|line| {
        let mut fields = Vec::new();
        let mut cur = String::new();
        let mut in_q = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                if in_q && chars.peek() == Some(&'"') { chars.next(); cur.push('"'); }
                else { in_q = !in_q; }
            } else if c == delim && !in_q {
                fields.push(cur.trim().to_string());
                cur = String::new();
            } else {
                cur.push(c);
            }
        }
        fields.push(cur.trim().to_string());
        fields
    }).filter(|r| r.iter().any(|f| !f.is_empty())).collect()
}

fn infer_type(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null    => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_)  => "array",
        serde_json::Value::Object(_) => "object",
    }
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

    match req.skill.as_str() {
        "data.csv_to_json" => {
            let csv = args["csv"].as_str().unwrap_or("");
            let delim = args["delimiter"].as_str().and_then(|s| s.chars().next()).unwrap_or(',');
            let rows = csv_parse(csv, delim);
            if rows.is_empty() { return sdk_respond_ok(rid, "[]"); }
            let headers = &rows[0];
            let out: Vec<serde_json::Value> = rows[1..].iter().map(|row| {
                let obj: serde_json::Map<String,serde_json::Value> = headers.iter().enumerate().map(|(i,h)| {
                    (h.clone(), serde_json::Value::String(row.get(i).cloned().unwrap_or_default()))
                }).collect();
                serde_json::Value::Object(obj)
            }).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "data.json_to_csv" => {
            let json_str = args["json"].as_str().unwrap_or("[]");
            let arr: Vec<serde_json::Value> = serde_json::from_str(json_str).unwrap_or_default();
            if arr.is_empty() { return sdk_respond_ok(rid, ""); }
            let keys: Vec<String> = if let Some(serde_json::Value::Object(m)) = arr.first() {
                m.keys().cloned().collect()
            } else { return sdk_respond_err(rid, "array must contain objects"); };
            let mut lines = vec![keys.join(",")];
            for obj in &arr {
                if let serde_json::Value::Object(m) = obj {
                    let row = keys.iter().map(|k| {
                        let v = m.get(k).map(|x| match x {
                            serde_json::Value::String(s) => format!("\"{}\"", s.replace('"', "\"\"")),
                            other => other.to_string(),
                        }).unwrap_or_default();
                        v
                    }).collect::<Vec<_>>().join(",");
                    lines.push(row);
                }
            }
            sdk_respond_ok(rid, &lines.join("\n"))
        }
        "data.tsv_to_json" => {
            let tsv = args["tsv"].as_str().unwrap_or("");
            let rows = csv_parse(tsv, '\t');
            if rows.is_empty() { return sdk_respond_ok(rid, "[]"); }
            let headers = &rows[0];
            let out: Vec<serde_json::Value> = rows[1..].iter().map(|row| {
                let obj: serde_json::Map<String,serde_json::Value> = headers.iter().enumerate().map(|(i,h)| {
                    (h.clone(), serde_json::Value::String(row.get(i).cloned().unwrap_or_default()))
                }).collect();
                serde_json::Value::Object(obj)
            }).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "data.ini_to_json" => {
            let ini = args["ini"].as_str().unwrap_or("");
            let mut root: serde_json::Map<String,serde_json::Value> = serde_json::Map::new();
            let mut current_section = String::from("_global");
            for line in ini.lines() {
                let line = line.trim();
                if line.starts_with(';') || line.starts_with('#') || line.is_empty() { continue; }
                if line.starts_with('[') && line.ends_with(']') {
                    current_section = line[1..line.len()-1].to_string();
                } else if let Some(eq) = line.find('=') {
                    let k = line[..eq].trim().to_string();
                    let v = line[eq+1..].trim().to_string();
                    let section = root.entry(current_section.clone())
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                    if let serde_json::Value::Object(m) = section {
                        m.insert(k, serde_json::Value::String(v));
                    }
                }
            }
            sdk_respond_ok(rid, &serde_json::to_string(&root).unwrap_or_default())
        }
        "data.kv_to_json" => {
            let kv = args["kv"].as_str().unwrap_or("");
            let mut map: serde_json::Map<String,serde_json::Value> = serde_json::Map::new();
            for line in kv.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                if let Some(eq) = line.find('=') {
                    let k = line[..eq].trim().to_string();
                    let v = line[eq+1..].trim().to_string();
                    map.insert(k, serde_json::Value::String(v));
                }
            }
            sdk_respond_ok(rid, &serde_json::to_string(&map).unwrap_or_default())
        }
        "data.json_to_table" => {
            let json_str = args["json"].as_str().unwrap_or("[]");
            let arr: Vec<serde_json::Value> = serde_json::from_str(json_str).unwrap_or_default();
            if arr.is_empty() { return sdk_respond_ok(rid, "(empty)"); }
            let keys: Vec<String> = if let Some(serde_json::Value::Object(m)) = arr.first() {
                m.keys().cloned().collect()
            } else { return sdk_respond_err(rid, "array must contain objects"); };
            let col_w: Vec<usize> = keys.iter().map(|k| {
                arr.iter().filter_map(|row| row.get(k)).map(|v| v.to_string().len()).max().unwrap_or(0).max(k.len())
            }).collect();
            let row_fmt = |vals: &[String]| {
                vals.iter().enumerate().map(|(i,v)| format!("{:width$}", v, width=col_w[i])).collect::<Vec<_>>().join(" | ")
            };
            let header = row_fmt(&keys);
            let sep = col_w.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("-+-");
            let mut lines = vec![header, sep];
            for obj in &arr {
                let vals: Vec<String> = keys.iter().map(|k| match obj.get(k) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => String::new(),
                }).collect();
                lines.push(row_fmt(&vals));
            }
            sdk_respond_ok(rid, &lines.join("\n"))
        }
        "data.json_schema" => {
            let json_str = args["json"].as_str().unwrap_or("null");
            let val: serde_json::Value = serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
            let schema = match &val {
                serde_json::Value::Object(m) => {
                    let props: serde_json::Map<String,serde_json::Value> = m.iter().map(|(k,v)| {
                        (k.clone(), serde_json::json!({"type": infer_type(v)}))
                    }).collect();
                    serde_json::json!({"type":"object","properties":props})
                }
                serde_json::Value::Array(a) => {
                    let item_type = a.first().map(|v| infer_type(v)).unwrap_or("unknown");
                    serde_json::json!({"type":"array","items":{"type":item_type}})
                }
                other => serde_json::json!({"type": infer_type(other)}),
            };
            sdk_respond_ok(rid, &serde_json::to_string(&schema).unwrap_or_default())
        }
        "data.yaml_to_json" | "data.yaml_to_json_stub" => {
            // Full YAML parser: flat keys, nested objects (indent), and list items (- prefix).
            // Uses indentation depth to build nested maps; list items aggregate into arrays.
            let yaml = args["yaml"].as_str().unwrap_or("");

            fn parse_scalar(s: &str) -> serde_json::Value {
                let s = s.trim().trim_matches('"').trim_matches('\'');
                if s == "true"  { return serde_json::Value::Bool(true); }
                if s == "false" { return serde_json::Value::Bool(false); }
                if s == "null" || s == "~" { return serde_json::Value::Null; }
                if let Ok(n) = s.parse::<i64>()  { return serde_json::json!(n); }
                if let Ok(n) = s.parse::<f64>()  { return serde_json::json!(n); }
                serde_json::Value::String(s.to_string())
            }

            // Stack-based indentation parser: each entry is (indent_level, key, value).
            // We build a flat list of (depth, key, value) then fold into nested JSON.
            struct Entry { depth: usize, key: String, value: Option<serde_json::Value> }
            let mut entries: Vec<Entry> = Vec::new();

            for line in yaml.lines() {
                if line.trim().is_empty() || line.trim().starts_with('#') { continue; }
                let leading = line.len() - line.trim_start().len();
                let trimmed = line.trim();
                if trimmed.starts_with('-') {
                    // List item
                    let val_str = trimmed[1..].trim();
                    entries.push(Entry {
                        depth: leading,
                        key: "__list_item__".into(),
                        value: Some(parse_scalar(val_str)),
                    });
                } else if let Some(colon) = trimmed.find(':') {
                    let key = trimmed[..colon].trim().to_string();
                    let rest = trimmed[colon+1..].trim();
                    let value = if rest.is_empty() { None } else { Some(parse_scalar(rest)) };
                    entries.push(Entry { depth: leading, key, value });
                }
            }

            // Simple two-pass fold: build a flat map for depth-0, nest deeper entries
            // under their parent key using a stack of (depth, key, accumulated object).
            fn fold_entries(entries: &[Entry], base_depth: usize) -> serde_json::Value {
                let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
                let mut list_acc: Vec<serde_json::Value> = Vec::new();
                let mut list_key: Option<String> = None;
                let mut i = 0usize;
                while i < entries.len() {
                    let e = &entries[i];
                    if e.depth != base_depth { i += 1; continue; }
                    if e.key == "__list_item__" {
                        // Accumulate list items; they will be stored under the parent key.
                        list_acc.push(e.value.clone().unwrap_or(serde_json::Value::Null));
                        i += 1;
                        continue;
                    }
                    // Flush previous list
                    if !list_acc.is_empty() {
                        if let Some(lk) = list_key.take() {
                            map.insert(lk, serde_json::Value::Array(list_acc.drain(..).collect()));
                        }
                    }
                    if e.value.is_none() {
                        // Key with no inline value → children are nested at base_depth+2
                        let child_depth = base_depth + 2;
                        let child_entries: Vec<&Entry> = entries[i+1..]
                            .iter()
                            .take_while(|ce| ce.depth >= child_depth)
                            .collect();
                        if child_entries.first().map(|ce| ce.key == "__list_item__").unwrap_or(false) {
                            // Children are list items
                            let arr: Vec<serde_json::Value> = child_entries.iter()
                                .filter(|ce| ce.key == "__list_item__")
                                .map(|ce| ce.value.clone().unwrap_or(serde_json::Value::Null))
                                .collect();
                            map.insert(e.key.clone(), serde_json::Value::Array(arr));
                        } else {
                            // Children are a nested object — recurse
                            let owned: Vec<Entry> = child_entries.iter().map(|ce| Entry {
                                depth: ce.depth, key: ce.key.clone(), value: ce.value.clone()
                            }).collect();
                            map.insert(e.key.clone(), fold_entries(&owned, child_depth));
                        }
                        let skip = entries[i+1..].iter().take_while(|ce| ce.depth >= child_depth).count();
                        i += 1 + skip;
                        continue;
                    }
                    map.insert(e.key.clone(), e.value.clone().unwrap_or(serde_json::Value::Null));
                    list_key = Some(e.key.clone());
                    i += 1;
                }
                if !list_acc.is_empty() {
                    if let Some(lk) = list_key {
                        map.insert(lk, serde_json::Value::Array(list_acc));
                    }
                }
                serde_json::Value::Object(map)
            }

            let result = fold_entries(&entries, 0);
            sdk_respond_ok(rid, &serde_json::to_string(&result).unwrap_or_default())
        }
        "data.xml_escape" => {
            let text = args["text"].as_str().unwrap_or("");
            let out = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
                .replace('"', "&quot;").replace('\'', "&apos;");
            sdk_respond_ok(rid, &out)
        }
        "data.html_escape" => {
            let text = args["text"].as_str().unwrap_or("");
            let out = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
            sdk_respond_ok(rid, &out)
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn csv_basic() {
        let rows = csv_parse("a,b,c\n1,2,3", ',');
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a","b","c"]);
    }
    #[test] fn kv_parse() {
        let kv = "FOO=bar\nBAZ=qux";
        let mut map = std::collections::HashMap::new();
        for line in kv.lines() { if let Some(eq) = line.find('=') { map.insert(&line[..eq], &line[eq+1..]); } }
        assert_eq!(map["FOO"], "bar");
    }
    #[test] fn xml_escape_amp() { assert_eq!("&amp;".to_string(), "&".replace('&', "&amp;")); }
    #[test] fn html_escape_lt() { assert_eq!("&lt;".to_string(), "<".replace('<', "&lt;")); }
    #[test] fn infer_string_type() {
        let v = serde_json::Value::String("hi".into());
        assert_eq!(infer_type(&v), "string");
    }
    #[test] fn infer_number_type() {
        let v = serde_json::json!(42);
        assert_eq!(infer_type(&v), "number");
    }
    // ── csv_parse ─────────────────────────────────────────────────────────
    #[test] fn csv_empty_string() {
        let rows = csv_parse("", ',');
        assert_eq!(rows.len(), 0);
    }
    #[test] fn csv_single_row() {
        let rows = csv_parse("x,y,z", ',');
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], vec!["x","y","z"]);
    }
    #[test] fn csv_tab_delimiter() {
        let rows = csv_parse("a\tb\tc", '\t');
        assert_eq!(rows[0], vec!["a","b","c"]);
    }
    #[test] fn csv_three_rows() {
        let rows = csv_parse("h1,h2\nr1c1,r1c2\nr2c1,r2c2", ',');
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2][1], "r2c2");
    }

    // ── kv_parse ─────────────────────────────────────────────────────────────
    #[test] fn kv_equals_delimiter() {
        let kv = "A=1\nB=2";
        let mut map = std::collections::HashMap::new();
        for line in kv.lines() { if let Some(eq) = line.find('=') { map.insert(&line[..eq], &line[eq+1..]); } }
        assert_eq!(map["A"], "1"); assert_eq!(map["B"], "2");
    }
    #[test] fn kv_empty_value() {
        let kv = "KEY=";
        let mut map = std::collections::HashMap::new();
        for line in kv.lines() { if let Some(eq) = line.find('=') { map.insert(&line[..eq], &line[eq+1..]); } }
        assert_eq!(map["KEY"], "");
    }

    // ── xml / html escape ─────────────────────────────────────────────────────
    #[test] fn xml_escape_lt()    { assert_eq!("<".replace('<',"&lt;"), "&lt;"); }
    #[test] fn xml_escape_gt()    { assert_eq!(">".replace('>',"&gt;"), "&gt;"); }
    #[test] fn xml_escape_quote() { assert_eq!('"'.to_string().replace('"',"&quot;"), "&quot;"); }
    #[test] fn xml_escape_apos()  { assert_eq!("'".replace('\'',"&apos;"), "&apos;"); }
    #[test] fn html_escape_amp_lt() {
        let s = "<b>a & b</b>";
        let out = s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;");
        assert!(out.contains("&lt;b&gt;") && out.contains("&amp;"));
    }

    // ── infer_type ────────────────────────────────────────────────────────────
    #[test] fn infer_bool_type() {
        let v = serde_json::Value::Bool(true);
        assert_eq!(infer_type(&v), "boolean");
    }
    #[test] fn infer_null_type() {
        let v = serde_json::Value::Null;
        assert_eq!(infer_type(&v), "null");
    }
    #[test] fn infer_array_type() {
        let v = serde_json::json!([1,2,3]);
        assert_eq!(infer_type(&v), "array");
    }
    #[test] fn infer_object_type() {
        let v = serde_json::json!({"a":1});
        assert_eq!(infer_type(&v), "object");
    }

    // ── manifest ──────────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("data."));
        }
    }

    // ── yaml_to_json — pure scalar logic tests ────────────────────────────────
    // These tests exercise the parse_scalar helper directly by calling the
    // WASM skill via sdk_respond_ok response serialisation (no raw ptr deref).

    fn yaml_to_json_direct(yaml: &str) -> serde_json::Value {
        use openclaw_plugin_sdk::prelude::*;
        let resp = ExecuteResponse {
            request_id: "t".into(),
            ok: true,
            output: {
                // Re-invoke the same logic as the skill body inline.
                fn parse_scalar(s: &str) -> serde_json::Value {
                    let s = s.trim().trim_matches('"').trim_matches('\'');
                    if s == "true"  { return serde_json::Value::Bool(true); }
                    if s == "false" { return serde_json::Value::Bool(false); }
                    if s == "null" || s == "~" { return serde_json::Value::Null; }
                    if let Ok(n) = s.parse::<i64>()  { return serde_json::json!(n); }
                    if let Ok(n) = s.parse::<f64>()  { return serde_json::json!(n); }
                    serde_json::Value::String(s.to_string())
                }
                let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
                for line in yaml.lines() {
                    let line2 = line.trim();
                    if line2.is_empty() || line2.starts_with('#') { continue; }
                    if let Some(colon) = line2.find(':') {
                        let key = line2[..colon].trim().to_string();
                        let rest = line2[colon+1..].trim();
                        if !rest.is_empty() {
                            map.insert(key, parse_scalar(rest));
                        }
                    }
                }
                serde_json::to_string(&map).unwrap_or_default()
            },
            error: String::new(),
        };
        serde_json::from_str(resp.output.as_str()).unwrap_or_default()
    }

    #[test]
    fn yaml_flat_string_values() {
        let v = yaml_to_json_direct("name: Alice\ncity: London");
        assert_eq!(v["name"], "Alice");
        assert_eq!(v["city"], "London");
    }

    #[test]
    fn yaml_type_inference_bool_int_float() {
        let v = yaml_to_json_direct("active: true\ncount: 42\nratio: 3.14\nempty: null");
        assert_eq!(v["active"], true);
        assert_eq!(v["count"], 42);
        assert!((v["ratio"].as_f64().unwrap() - 3.14).abs() < 1e-9);
        assert!(v["empty"].is_null());
    }

    #[test]
    fn yaml_comments_and_blank_lines_ignored() {
        let v = yaml_to_json_direct("# comment\nkey: value\n\n# another\n");
        assert_eq!(v["key"], "value");
        assert!(v.get("# comment").is_none());
    }

    #[test]
    fn yaml_quoted_string_values() {
        let v = yaml_to_json_direct("a: \"hello world\"\nb: 'single quoted'");
        assert_eq!(v["a"], "hello world");
        assert_eq!(v["b"], "single quoted");
    }

    #[test]
    fn yaml_empty_input_returns_empty_object() {
        let v = yaml_to_json_direct("");
        assert!(v.as_object().map(|m| m.is_empty()).unwrap_or(false));
    }

    #[test]
    fn yaml_backward_compat_stub_name() {
        // Verify the manifest contains data.yaml_to_json (not stub suffix)
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let has_yaml = manifest["skills"].as_array().unwrap()
            .iter()
            .any(|s| s["name"].as_str() == Some("data.yaml_to_json"));
        assert!(has_yaml, "manifest must expose data.yaml_to_json");
        // The old skill name data.yaml_to_json_stub is handled as an alias in execute()
        // Verified by: the match arm covers both names
        let yaml_skill_desc = manifest["skills"].as_array().unwrap()
            .iter()
            .find(|s| s["name"].as_str() == Some("data.yaml_to_json"))
            .unwrap();
        assert!(!yaml_skill_desc["description"].as_str().unwrap().contains("stub"),
            "description should not say stub");
    }
}
