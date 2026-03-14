//! skill-duration — time duration parsing, formatting, and arithmetic (pure-Rust).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.duration",
  "name": "Duration Skills",
  "version": "0.1.0",
  "description": "Duration parsing, formatting, arithmetic, and human-readable output",
  "skills": [
    {
      "name": "duration.parse",
      "display": "Parse Duration",
      "description": "Parse a duration string like '1h30m' into total seconds.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "duration.format",
      "display": "Format Duration",
      "description": "Format a number of seconds as a human-readable duration string.",
      "risk": "safe",
      "params": [
        { "name": "seconds", "type": "number",  "required": true  },
        { "name": "style",   "type": "string",  "required": false }
      ]
    },
    {
      "name": "duration.add",
      "display": "Add Durations",
      "description": "Add two duration strings and return total seconds.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "string", "required": true },
        { "name": "b", "type": "string", "required": true }
      ]
    },
    {
      "name": "duration.between",
      "display": "Duration Between",
      "description": "Compute the duration in seconds between two Unix timestamps.",
      "risk": "safe",
      "params": [
        { "name": "start", "type": "number", "required": true },
        { "name": "end",   "type": "number", "required": true }
      ]
    },
    {
      "name": "duration.convert",
      "display": "Convert Duration",
      "description": "Convert seconds to a specific unit: seconds/minutes/hours/days.",
      "risk": "safe",
      "params": [
        { "name": "seconds", "type": "number", "required": true },
        { "name": "unit",    "type": "string", "required": true }
      ]
    }
  ]
}"#;

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r) => r, Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();

    match req.skill.as_str() {
        "duration.parse" => {
            let s = match req.args["value"].as_str() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'value'") };
            match parse_duration(s) {
                Ok(secs) => sdk_respond_ok(rid, &secs.to_string()),
                Err(e)   => sdk_respond_err(rid, &e),
            }
        }
        "duration.format" => {
            let secs = match req.args["seconds"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'seconds'") };
            let style = req.args["style"].as_str().unwrap_or("short");
            sdk_respond_ok(rid, &format_duration(secs as u64, style))
        }
        "duration.add" => {
            let a = match req.args["a"].as_str() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_str() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'b'") };
            match (parse_duration(a), parse_duration(b)) {
                (Ok(sa), Ok(sb)) => sdk_respond_ok(rid, &(sa + sb).to_string()),
                (Err(e), _) | (_, Err(e)) => sdk_respond_err(rid, &e),
            }
        }
        "duration.between" => {
            let start = match req.args["start"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'start'") };
            let end   = match req.args["end"].as_f64()   { Some(v) => v, None => return sdk_respond_err(rid, "missing 'end'")   };
            sdk_respond_ok(rid, &((end - start).abs() as u64).to_string())
        }
        "duration.convert" => {
            let secs = match req.args["seconds"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'seconds'") };
            let unit = match req.args["unit"].as_str()    { Some(v) => v, None => return sdk_respond_err(rid, "missing 'unit'")    };
            match convert_duration(secs, unit) {
                Ok(v)  => sdk_respond_ok(rid, &format!("{:.6}", v)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Duration logic ────────────────────────────────────────────────────────────

fn parse_duration(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Ok(n) = s.parse::<u64>() { return Ok(n); }
    let mut total = 0u64;
    let mut num_buf = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            let n: u64 = num_buf.parse().unwrap_or(0);
            num_buf.clear();
            total += match ch {
                'w' => n * 604800,
                'd' => n * 86400,
                'h' => n * 3600,
                'm' => n * 60,
                's' => n,
                _   => return Err(format!("unknown unit '{ch}' in duration '{s}'")),
            };
        }
    }
    if !num_buf.is_empty() {
        total += num_buf.parse::<u64>().map_err(|e| e.to_string())?;
    }
    Ok(total)
}

fn format_duration(secs: u64, style: &str) -> String {
    let weeks   = secs / 604800;
    let days    = (secs % 604800) / 86400;
    let hours   = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if style == "long" {
        let mut parts = Vec::new();
        if weeks   > 0 { parts.push(format!("{} week{}", weeks,   if weeks==1 {""} else {"s"})); }
        if days    > 0 { parts.push(format!("{} day{}", days,     if days==1 {""} else {"s"})); }
        if hours   > 0 { parts.push(format!("{} hour{}", hours,   if hours==1 {""} else {"s"})); }
        if minutes > 0 { parts.push(format!("{} minute{}", minutes, if minutes==1 {""} else {"s"})); }
        if seconds > 0 || parts.is_empty() { parts.push(format!("{} second{}", seconds, if seconds==1 {""} else {"s"})); }
        parts.join(", ")
    } else {
        let mut s = String::new();
        if weeks   > 0 { s.push_str(&format!("{}w", weeks)); }
        if days    > 0 { s.push_str(&format!("{}d", days)); }
        if hours   > 0 { s.push_str(&format!("{}h", hours)); }
        if minutes > 0 { s.push_str(&format!("{}m", minutes)); }
        if seconds > 0 || s.is_empty() { s.push_str(&format!("{}s", seconds)); }
        s
    }
}

fn convert_duration(secs: f64, unit: &str) -> Result<f64, String> {
    match unit {
        "seconds" | "s"       => Ok(secs),
        "minutes" | "m" | "min" => Ok(secs / 60.0),
        "hours"   | "h" | "hr"  => Ok(secs / 3600.0),
        "days"    | "d"          => Ok(secs / 86400.0),
        "weeks"   | "w"          => Ok(secs / 604800.0),
        other => Err(format!("unknown unit: {other}")),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn parse_seconds()    { assert_eq!(parse_duration("30s").unwrap(), 30); }
    #[test] fn parse_minutes()    { assert_eq!(parse_duration("5m").unwrap(), 300); }
    #[test] fn parse_hours()      { assert_eq!(parse_duration("2h").unwrap(), 7200); }
    #[test] fn parse_days()       { assert_eq!(parse_duration("1d").unwrap(), 86400); }
    #[test] fn parse_combined()   { assert_eq!(parse_duration("1h30m").unwrap(), 5400); }
    #[test] fn parse_hms()        { assert_eq!(parse_duration("1h30m10s").unwrap(), 5410); }
    #[test] fn parse_plain_int()  { assert_eq!(parse_duration("120").unwrap(), 120); }
    #[test] fn parse_unknown_unit(){ assert!(parse_duration("5x").is_err()); }

    #[test] fn format_short_basic() { assert_eq!(format_duration(3661, "short"), "1h1m1s"); }
    #[test] fn format_short_zero()  { assert_eq!(format_duration(0, "short"), "0s"); }
    #[test] fn format_long_basic()  { assert!(format_duration(7200, "long").contains("hour")); }
    #[test] fn format_long_plural() { assert!(format_duration(7200, "long").contains("hours")); }
    #[test] fn format_short_days()  { assert!(format_duration(90061, "short").contains('d')); }

    #[test] fn convert_to_min()   { assert!((convert_duration(3600.0, "minutes").unwrap() - 60.0).abs() < 1e-9); }
    #[test] fn convert_to_hours() { assert!((convert_duration(3600.0, "hours").unwrap() - 1.0).abs() < 1e-9); }
    #[test] fn convert_to_days()  { assert!((convert_duration(86400.0, "days").unwrap() - 1.0).abs() < 1e-9); }
    #[test] fn convert_unknown()  { assert!(convert_duration(60.0, "fortnights").is_err()); }
    #[test] fn convert_seconds()  { assert!((convert_duration(42.0, "s").unwrap() - 42.0).abs() < 1e-9); }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.duration");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("duration."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
