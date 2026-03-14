//! `data.*` — structured data processing without external dependencies.
//!
//! ## Skills
//! | skill                | description |
//! |----------------------|-------------|
//! | `data.csv_read`      | Parse a CSV file or inline string into JSON rows |
//! | `data.csv_write`     | Write JSON rows (array of objects) to a CSV file |
//! | `data.json_transform`| Extract / reshape JSON using JSON Pointer paths |
//!
//! ## Design
//! - Zero new dependencies: CSV is parsed manually (RFC 4180 compliant).
//! - `data.json_transform` uses serde_json's built-in JSON Pointer support.
//! - All size limits guard against LLM context overflow.

const MAX_ROWS: usize = 10_000;
const MAX_OUTPUT_CHARS: usize = 32_000;

// ── data.csv_read ─────────────────────────────────────────────────────────────

pub struct CsvReadArgs {
    /// Path to a CSV file (mutually exclusive with `content`).
    pub path: Option<String>,
    /// Inline CSV string (mutually exclusive with `path`).
    pub content: Option<String>,
    /// Field delimiter character (default: ',').
    pub delimiter: char,
    /// Whether the first row is a header (default: true).
    pub has_header: bool,
    /// Maximum number of rows to return (default: 1000, max: 10000).
    pub max_rows: usize,
}

impl CsvReadArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        let path = v["path"].as_str().map(|s| s.to_string());
        let content = v["content"].as_str().map(|s| s.to_string());
        if path.is_none() && content.is_none() {
            return Err("data.csv_read: one of 'path' or 'content' is required".into());
        }
        if path.is_some() && content.is_some() {
            return Err("data.csv_read: 'path' and 'content' are mutually exclusive".into());
        }
        let delimiter = v["delimiter"]
            .as_str()
            .and_then(|s| s.chars().next())
            .unwrap_or(',');
        let has_header = v["has_header"].as_bool().unwrap_or(true);
        let max_rows = v["max_rows"]
            .as_u64()
            .map(|n| (n as usize).min(MAX_ROWS))
            .unwrap_or(1000);
        Ok(Self { path, content, delimiter, has_header, max_rows })
    }
}

pub fn csv_read(args: &CsvReadArgs) -> Result<String, String> {
    let raw = if let Some(p) = &args.path {
        std::fs::read_to_string(p)
            .map_err(|e| format!("data.csv_read: cannot read '{}': {}", p, e))?
    } else {
        args.content.clone().unwrap_or_default()
    };

    let rows = parse_csv(&raw, args.delimiter, args.has_header, args.max_rows)?;

    let out = serde_json::json!({
        "row_count": rows.len(),
        "rows": rows,
        "truncated": rows.len() == args.max_rows,
    });

    let s = serde_json::to_string(&out)
        .map_err(|e| format!("data.csv_read: serialization error: {}", e))?;

    if s.len() > MAX_OUTPUT_CHARS {
        let truncated = serde_json::json!({
            "row_count": rows.len(),
            "rows": &rows[..rows.len().min(100)],
            "truncated": true,
            "note": "Output truncated to first 100 rows to fit context window",
        });
        return serde_json::to_string(&truncated)
            .map_err(|e| format!("data.csv_read: serialization error: {}", e));
    }

    Ok(s)
}

// ── data.csv_write ────────────────────────────────────────────────────────────

pub struct CsvWriteArgs {
    /// JSON array of objects to write as CSV rows.
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    /// Output file path.
    pub path: String,
    /// Field delimiter (default: ',').
    pub delimiter: char,
    /// Whether to emit a header row (default: true).
    pub has_header: bool,
}

impl CsvWriteArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        let path = v["path"]
            .as_str()
            .ok_or("data.csv_write: missing 'path'")?
            .to_string();

        let rows: Vec<serde_json::Map<String, serde_json::Value>> = v["rows"]
            .as_array()
            .ok_or("data.csv_write: 'rows' must be a JSON array")?
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.as_object()
                    .cloned()
                    .ok_or_else(|| format!("data.csv_write: row {} is not an object", i))
            })
            .collect::<Result<_, _>>()?;

        if rows.is_empty() {
            return Err("data.csv_write: 'rows' array must not be empty".into());
        }

        let delimiter = v["delimiter"]
            .as_str()
            .and_then(|s| s.chars().next())
            .unwrap_or(',');

        Ok(Self { rows, path, delimiter, has_header: v["has_header"].as_bool().unwrap_or(true) })
    }
}

pub fn csv_write(args: &CsvWriteArgs) -> Result<String, String> {
    let headers: Vec<String> = args.rows[0].keys().cloned().collect();
    let mut out = String::new();

    if args.has_header {
        out.push_str(&csv_row(&headers.iter().map(|s| s.as_str()).collect::<Vec<_>>(), args.delimiter));
        out.push('\n');
    }

    for row in &args.rows {
        let values: Vec<String> = headers
            .iter()
            .map(|h| json_value_to_csv_field(row.get(h)))
            .collect();
        out.push_str(&csv_row(&values.iter().map(|s| s.as_str()).collect::<Vec<_>>(), args.delimiter));
        out.push('\n');
    }

    // Create parent directories if needed.
    if let Some(parent) = std::path::Path::new(&args.path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("data.csv_write: cannot create dir: {}", e))?;
        }
    }

    std::fs::write(&args.path, out.as_bytes())
        .map_err(|e| format!("data.csv_write: cannot write '{}': {}", args.path, e))?;

    Ok(format!(
        "Wrote {} row(s) to '{}'",
        args.rows.len(),
        args.path
    ))
}

// ── data.json_transform ───────────────────────────────────────────────────────

pub struct JsonTransformArgs {
    /// JSON value to transform (inline object/array).
    pub data: serde_json::Value,
    /// List of JSON Pointer expressions (RFC 6901) to extract.
    /// If empty, the whole document is returned.
    pub pointers: Vec<String>,
    /// If true, flatten the result to a single JSON object keyed by pointer.
    pub flatten: bool,
}

impl JsonTransformArgs {
    pub fn from_json(v: &serde_json::Value) -> Result<Self, String> {
        let data = v["data"].clone();
        if data.is_null() {
            return Err("data.json_transform: missing 'data'".into());
        }
        let pointers = v["pointers"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        let flatten = v["flatten"].as_bool().unwrap_or(false);
        Ok(Self { data, pointers, flatten })
    }
}

pub fn json_transform(args: &JsonTransformArgs) -> Result<String, String> {
    if args.pointers.is_empty() {
        return serde_json::to_string_pretty(&args.data)
            .map_err(|e| format!("data.json_transform: serialization error: {}", e));
    }

    if args.flatten {
        let mut map = serde_json::Map::new();
        for ptr in &args.pointers {
            let val = args.data.pointer(ptr).cloned().unwrap_or(serde_json::Value::Null);
            map.insert(ptr.clone(), val);
        }
        return serde_json::to_string_pretty(&serde_json::Value::Object(map))
            .map_err(|e| format!("data.json_transform: serialization error: {}", e));
    }

    if args.pointers.len() == 1 {
        let val = args.data.pointer(&args.pointers[0]).cloned().unwrap_or(serde_json::Value::Null);
        return serde_json::to_string_pretty(&val)
            .map_err(|e| format!("data.json_transform: serialization error: {}", e));
    }

    // Multiple pointers → return as array of values in same order.
    let values: Vec<serde_json::Value> = args.pointers
        .iter()
        .map(|ptr| args.data.pointer(ptr).cloned().unwrap_or(serde_json::Value::Null))
        .collect();
    serde_json::to_string_pretty(&serde_json::Value::Array(values))
        .map_err(|e| format!("data.json_transform: serialization error: {}", e))
}

// ── CSV parser (RFC 4180) ─────────────────────────────────────────────────────

fn parse_csv(
    input: &str,
    delimiter: char,
    has_header: bool,
    max_rows: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let logical_rows = parse_csv_records(input, delimiter);
    if logical_rows.is_empty() {
        return Ok(vec![]);
    }

    let mut iter = logical_rows.into_iter();

    let headers: Vec<String> = if has_header {
        iter.next().unwrap_or_default()
    } else {
        vec![]
    };

    let mut rows = Vec::new();
    for fields in iter {
        if rows.len() >= max_rows {
            break;
        }
        // Skip rows that are entirely empty
        if fields.iter().all(|f| f.is_empty()) {
            continue;
        }
        if has_header {
            let mut obj = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                let val = fields.get(i).cloned().unwrap_or_default();
                obj.insert(h.clone(), serde_json::Value::String(val));
            }
            rows.push(serde_json::Value::Object(obj));
        } else {
            rows.push(serde_json::Value::Array(
                fields.into_iter().map(serde_json::Value::String).collect(),
            ));
        }
    }
    Ok(rows)
}

/// Parse an entire CSV input into logical records, handling RFC 4180 quoted
/// fields that may span multiple lines.
fn parse_csv_records(input: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut current_record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                // Newlines inside quotes are preserved as-is.
                field.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == delimiter {
            current_record.push(field.trim().to_string());
            field = String::new();
        } else if ch == '\r' {
            // Consume optional \n after \r.
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            current_record.push(field.trim().to_string());
            field = String::new();
            records.push(current_record);
            current_record = Vec::new();
        } else if ch == '\n' {
            current_record.push(field.trim().to_string());
            field = String::new();
            records.push(current_record);
            current_record = Vec::new();
        } else {
            field.push(ch);
        }
    }

    // Push trailing field / record if any.
    current_record.push(field.trim().to_string());
    if !current_record.iter().all(|f| f.is_empty()) {
        records.push(current_record);
    }

    records
}

/// Parse a single CSV row respecting double-quote escaping (used in tests).
#[allow(dead_code)]
fn parse_csv_row(line: &str, delimiter: char) -> Vec<String> {
    let records = parse_csv_records(line, delimiter);
    records.into_iter().next().unwrap_or_default()
}

fn csv_row(fields: &[&str], delimiter: char) -> String {
    fields
        .iter()
        .map(|f| {
            if f.contains(delimiter) || f.contains('"') || f.contains('\n') {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(&delimiter.to_string())
}

fn json_value_to_csv_field(val: Option<&serde_json::Value>) -> String {
    match val {
        None => String::new(),
        Some(serde_json::Value::Null) => String::new(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── CsvReadArgs ───────────────────────────────────────────────────────

    #[test]
    fn csv_read_args_neither_errors() {
        assert!(CsvReadArgs::from_json(&serde_json::json!({})).is_err());
    }

    #[test]
    fn csv_read_args_both_errors() {
        let v = serde_json::json!({"path": "/a.csv", "content": "a,b"});
        assert!(CsvReadArgs::from_json(&v).is_err());
    }

    #[test]
    fn csv_read_args_content_ok() {
        let v = serde_json::json!({"content": "a,b\n1,2"});
        let a = CsvReadArgs::from_json(&v).unwrap();
        assert_eq!(a.delimiter, ',');
        assert!(a.has_header);
    }

    #[test]
    fn csv_read_args_custom_delimiter() {
        let v = serde_json::json!({"content": "a;b", "delimiter": ";"});
        let a = CsvReadArgs::from_json(&v).unwrap();
        assert_eq!(a.delimiter, ';');
    }

    #[test]
    fn csv_read_args_max_rows_capped() {
        let v = serde_json::json!({"content": "x", "max_rows": 99999});
        let a = CsvReadArgs::from_json(&v).unwrap();
        assert_eq!(a.max_rows, MAX_ROWS);
    }

    // ── parse_csv ─────────────────────────────────────────────────────────

    #[test]
    fn parse_csv_simple_with_header() {
        let input = "name,age\nAlice,30\nBob,25";
        let rows = parse_csv(input, ',', true, 100).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["age"], "30");
        assert_eq!(rows[1]["name"], "Bob");
    }

    #[test]
    fn parse_csv_no_header_returns_arrays() {
        let input = "1,2,3\n4,5,6";
        let rows = parse_csv(input, ',', false, 100).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_array());
        assert_eq!(rows[0][0], "1");
    }

    #[test]
    fn parse_csv_quoted_field_with_comma() {
        let input = "name,city\n\"Smith, Jr.\",NYC";
        let rows = parse_csv(input, ',', true, 100).unwrap();
        assert_eq!(rows[0]["name"], "Smith, Jr.");
    }

    #[test]
    fn parse_csv_escaped_quote() {
        let input = "name\n\"say \"\"hello\"\"\"";
        let rows = parse_csv(input, ',', true, 100).unwrap();
        assert_eq!(rows[0]["name"], "say \"hello\"");
    }

    #[test]
    fn parse_csv_max_rows_respected() {
        let header = "x";
        let data: String = (0..50).map(|i| format!("{}\n", i)).collect();
        let input = format!("{}\n{}", header, data);
        let rows = parse_csv(&input, ',', true, 10).unwrap();
        assert_eq!(rows.len(), 10);
    }

    #[test]
    fn parse_csv_empty_input() {
        let rows = parse_csv("", ',', true, 100).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn parse_csv_semicolon_delimiter() {
        let input = "a;b\n1;2";
        let rows = parse_csv(input, ';', true, 100).unwrap();
        assert_eq!(rows[0]["a"], "1");
        assert_eq!(rows[0]["b"], "2");
    }

    // ── csv_read (inline content) ─────────────────────────────────────────

    #[test]
    fn csv_read_inline_content() {
        let args = CsvReadArgs {
            content: Some("col1,col2\nval1,val2".to_string()),
            path: None,
            delimiter: ',',
            has_header: true,
            max_rows: 100,
        };
        let out = csv_read(&args).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["row_count"], 1);
        assert_eq!(v["rows"][0]["col1"], "val1");
    }

    // ── CsvWriteArgs ──────────────────────────────────────────────────────

    #[test]
    fn csv_write_args_missing_path_errors() {
        let v = serde_json::json!({"rows": [{"a": "1"}]});
        assert!(CsvWriteArgs::from_json(&v).is_err());
    }

    #[test]
    fn csv_write_args_missing_rows_errors() {
        let v = serde_json::json!({"path": "/tmp/out.csv"});
        assert!(CsvWriteArgs::from_json(&v).is_err());
    }

    #[test]
    fn csv_write_args_empty_rows_errors() {
        let v = serde_json::json!({"path": "/tmp/out.csv", "rows": []});
        assert!(CsvWriteArgs::from_json(&v).is_err());
    }

    #[test]
    fn csv_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.csv");
        let args = CsvWriteArgs {
            rows: vec![
                [("name".to_string(), serde_json::Value::String("Alice".to_string())),
                 ("age".to_string(), serde_json::Value::String("30".to_string()))]
                    .into_iter().collect(),
            ],
            path: path.to_string_lossy().to_string(),
            delimiter: ',',
            has_header: true,
        };
        csv_write(&args).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("name,age") || content.contains("age,name"));
        assert!(content.contains("Alice"));
        assert!(content.contains("30"));
    }

    // ── JsonTransformArgs ─────────────────────────────────────────────────

    #[test]
    fn json_transform_missing_data_errors() {
        let v = serde_json::json!({"pointers": ["/name"]});
        assert!(JsonTransformArgs::from_json(&v).is_err());
    }

    #[test]
    fn json_transform_no_pointers_returns_whole() {
        let v = serde_json::json!({"data": {"x": 1, "y": 2}});
        let args = JsonTransformArgs::from_json(&v).unwrap();
        let out = json_transform(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(result["x"], 1);
    }

    #[test]
    fn json_transform_single_pointer() {
        let v = serde_json::json!({"data": {"user": {"name": "Alice"}}, "pointers": ["/user/name"]});
        let args = JsonTransformArgs::from_json(&v).unwrap();
        let out = json_transform(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(result, "Alice");
    }

    #[test]
    fn json_transform_multiple_pointers_returns_array() {
        let v = serde_json::json!({
            "data": {"a": 1, "b": 2, "c": 3},
            "pointers": ["/a", "/c"]
        });
        let args = JsonTransformArgs::from_json(&v).unwrap();
        let out = json_transform(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(result.is_array());
        assert_eq!(result[0], 1);
        assert_eq!(result[1], 3);
    }

    #[test]
    fn json_transform_flatten_mode() {
        let v = serde_json::json!({
            "data": {"name": "Bob", "age": 42},
            "pointers": ["/name", "/age"],
            "flatten": true
        });
        let args = JsonTransformArgs::from_json(&v).unwrap();
        let out = json_transform(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(result["/name"], "Bob");
        assert_eq!(result["/age"], 42);
    }

    #[test]
    fn json_transform_missing_pointer_returns_null() {
        let v = serde_json::json!({
            "data": {"a": 1},
            "pointers": ["/nonexistent"]
        });
        let args = JsonTransformArgs::from_json(&v).unwrap();
        let out = json_transform(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(result.is_null());
    }

    // ── csv_row helper ────────────────────────────────────────────────────

    #[test]
    fn csv_row_simple() {
        assert_eq!(csv_row(&["a", "b", "c"], ','), "a,b,c");
    }

    #[test]
    fn csv_row_quotes_field_with_comma() {
        let r = csv_row(&["Smith, Jr.", "NYC"], ',');
        assert!(r.contains("\"Smith, Jr.\""));
    }

    #[test]
    fn csv_row_escapes_double_quote() {
        let r = csv_row(&["say \"hello\""], ',');
        assert!(r.contains("\"say \"\"hello\"\"\""));
    }

    // ── TSV delimiter ─────────────────────────────────────────────────────

    #[test]
    fn csv_read_tsv_delimiter() {
        let v = serde_json::json!({
            "content": "name\tage\nAlice\t30\nBob\t25",
            "delimiter": "\t"
        });
        let args = CsvReadArgs::from_json(&v).unwrap();
        let out = csv_read(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(result["row_count"], 2);
        assert_eq!(result["rows"][0]["name"], "Alice");
        assert_eq!(result["rows"][1]["age"], "25");
    }

    #[test]
    fn csv_row_tsv_uses_tab() {
        let r = csv_row(&["a", "b"], '\t');
        assert_eq!(r, "a\tb");
    }

    // ── Quoted fields with embedded newline (RFC 4180) ────────────────────

    #[test]
    fn csv_read_quoted_field_with_embedded_newline() {
        let content = "name,bio\nAlice,\"line1\nline2\"\nBob,simple";
        let v = serde_json::json!({"content": content});
        let args = CsvReadArgs::from_json(&v).unwrap();
        let out = csv_read(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(result["row_count"], 2);
        assert!(result["rows"][0]["bio"].as_str().unwrap().contains("line1"));
        assert!(result["rows"][0]["bio"].as_str().unwrap().contains("line2"));
        assert_eq!(result["rows"][1]["name"], "Bob");
    }

    #[test]
    fn csv_read_quoted_field_with_embedded_comma() {
        let content = "a,b\n\"hello, world\",42";
        let v = serde_json::json!({"content": content});
        let args = CsvReadArgs::from_json(&v).unwrap();
        let out = csv_read(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(result["rows"][0]["a"], "hello, world");
    }

    #[test]
    fn csv_read_no_header_produces_arrays() {
        let v = serde_json::json!({"content": "Alice,30", "has_header": false});
        let args = CsvReadArgs::from_json(&v).unwrap();
        let out = csv_read(&args).unwrap();
        let result: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(result["row_count"], 1);
        // headerless rows are returned as JSON arrays of strings
        assert_eq!(result["rows"][0][0], "Alice");
        assert_eq!(result["rows"][0][1], "30");
    }

    // ── csv_read path/content mutual exclusion ────────────────────────────

    #[test]
    fn csv_read_path_and_content_mutual_exclusion() {
        let v = serde_json::json!({"path": "/tmp/f.csv", "content": "a,b"});
        assert!(CsvReadArgs::from_json(&v).is_err());
    }


    // ── tempfile uniqueness (python.rs helper logic equivalence) ─────────

    #[test]
    fn csv_write_multiple_files_no_collision() {
        let dir = tempfile::tempdir().unwrap();
        let make_args = |name: &str| CsvWriteArgs {
            rows: vec![[("x".to_string(), serde_json::Value::String("1".to_string()))].into_iter().collect()],
            path: dir.path().join(name).to_string_lossy().to_string(),
            delimiter: ',',
            has_header: true,
        };
        csv_write(&make_args("a.csv")).unwrap();
        csv_write(&make_args("b.csv")).unwrap();
        assert!(dir.path().join("a.csv").exists());
        assert!(dir.path().join("b.csv").exists());
    }
}
