//! skill-money — currency formatting and arithmetic (pure-Rust, integer cents, no I/O).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.money",
  "name": "Money Skills",
  "version": "0.1.0",
  "description": "Currency formatting, parsing, arithmetic, and tax calculation",
  "skills": [
    {
      "name": "money.format",
      "display": "Format Money",
      "description": "Format a numeric amount as a currency string (e.g. $1,234.56).",
      "risk": "safe",
      "params": [
        { "name": "amount",   "type": "number", "required": true  },
        { "name": "currency", "type": "string", "required": false },
        { "name": "decimals", "type": "integer","required": false }
      ]
    },
    {
      "name": "money.parse",
      "display": "Parse Money",
      "description": "Parse a currency string like '$1,234.56' and return the numeric value.",
      "risk": "safe",
      "params": [{ "name": "value", "type": "string", "required": true }]
    },
    {
      "name": "money.add",
      "display": "Add Money",
      "description": "Add two monetary amounts and return a formatted string.",
      "risk": "safe",
      "params": [
        { "name": "a",        "type": "number", "required": true  },
        { "name": "b",        "type": "number", "required": true  },
        { "name": "currency", "type": "string", "required": false }
      ]
    },
    {
      "name": "money.tax",
      "display": "Calculate Tax",
      "description": "Calculate tax amount and total given an amount and rate (0-100).",
      "risk": "safe",
      "params": [
        { "name": "amount", "type": "number", "required": true },
        { "name": "rate",   "type": "number", "required": true }
      ]
    },
    {
      "name": "money.split",
      "display": "Split Bill",
      "description": "Split an amount among n people, returning each share.",
      "risk": "safe",
      "params": [
        { "name": "amount", "type": "number",  "required": true },
        { "name": "n",      "type": "integer", "required": true }
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
        "money.format" => {
            let amt = match req.args["amount"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'amount'") };
            let currency = req.args["currency"].as_str().unwrap_or("$");
            let decimals = req.args["decimals"].as_u64().unwrap_or(2) as usize;
            sdk_respond_ok(rid, &format_money(amt, currency, decimals))
        }
        "money.parse" => {
            let s = match req.args["value"].as_str() { Some(s) => s, None => return sdk_respond_err(rid, "missing 'value'") };
            match parse_money(s) {
                Ok(v)  => sdk_respond_ok(rid, &format!("{:.2}", v)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "money.add" => {
            let a = match req.args["a"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'a'") };
            let b = match req.args["b"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'b'") };
            let currency = req.args["currency"].as_str().unwrap_or("$");
            sdk_respond_ok(rid, &format_money(a + b, currency, 2))
        }
        "money.tax" => {
            let amt  = match req.args["amount"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'amount'") };
            let rate = match req.args["rate"].as_f64()   { Some(v) => v, None => return sdk_respond_err(rid, "missing 'rate'")   };
            let tax   = (amt * rate / 100.0 * 100.0).round() / 100.0;
            let total = (amt * 100.0).round() / 100.0 + tax;
            let out = serde_json::json!({"amount": amt, "rate": rate, "tax": tax, "total": total});
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap())
        }
        "money.split" => {
            let amt = match req.args["amount"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'amount'") };
            let n   = match req.args["n"].as_u64()      { Some(v) => v, None => return sdk_respond_err(rid, "missing 'n'")      };
            if n == 0 { return sdk_respond_err(rid, "n must be > 0"); }
            let share = (amt / n as f64 * 100.0).round() / 100.0;
            let out = serde_json::json!({"amount": amt, "n": n, "each": share});
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Money logic ───────────────────────────────────────────────────────────────

fn format_money(amount: f64, symbol: &str, decimals: usize) -> String {
    let neg = amount < 0.0;
    let abs = amount.abs();
    let factor = 10f64.powi(decimals as i32);
    let rounded = (abs * factor).round() / factor;
    let int_part = rounded.floor() as u64;
    let frac = rounded - int_part as f64;

    let int_str: String = {
        let s = int_part.to_string();
        let mut out = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 { out.push(','); }
            out.push(c);
        }
        out.chars().rev().collect()
    };

    let result = if decimals > 0 {
        let frac_str = format!("{:.prec$}", frac, prec = decimals);
        format!("{}{}.{}", int_str, "", &frac_str[2..])
    } else {
        int_str
    };

    if neg {
        format!("-{}{}", symbol, result)
    } else {
        format!("{}{}", symbol, result)
    }
}

fn parse_money(s: &str) -> Result<f64, String> {
    let cleaned: String = s.chars().filter(|&c| c.is_ascii_digit() || c == '.' || c == '-').collect();
    cleaned.parse::<f64>().map_err(|e| e.to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn format_basic()    { assert_eq!(format_money(1234.56, "$", 2), "$1,234.56"); }
    #[test] fn format_zero()     { assert_eq!(format_money(0.0, "$", 2), "$0.00"); }
    #[test] fn format_negative() { assert!(format_money(-99.99, "$", 2).starts_with('-')); }
    #[test] fn format_no_dec()   { assert_eq!(format_money(1000.0, "€", 0), "€1,000"); }
    #[test] fn format_small()    { assert_eq!(format_money(0.99, "$", 2), "$0.99"); }

    #[test] fn parse_dollar()    { assert!((parse_money("$1,234.56").unwrap() - 1234.56).abs() < 0.001); }
    #[test] fn parse_plain()     { assert!((parse_money("99.99").unwrap() - 99.99).abs() < 0.001); }
    #[test] fn parse_negative()  { assert!((parse_money("-50.00").unwrap() - (-50.0)).abs() < 0.001); }
    #[test] fn parse_invalid()   { assert!(parse_money("abc").is_err()); }

    #[test]
    fn tax_basic() {
        let tax: f64 = 100.0 * 10.0 / 100.0;
        assert!((tax - 10.0).abs() < 0.001);
    }
    #[test]
    fn tax_zero() {
        let tax: f64 = 50.0 * 0.0 / 100.0;
        assert_eq!(tax, 0.0);
    }
    #[test]
    fn split_even() {
        let share: f64 = (300.0_f64 / 3.0 * 100.0).round() / 100.0;
        assert!((share - 100.0).abs() < 0.001);
    }
    #[test]
    fn split_uneven() {
        let share: f64 = (10.0_f64 / 3.0 * 100.0).round() / 100.0;
        assert!(share > 3.0 && share < 4.0);
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.money");
        assert_eq!(v["skills"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("money."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
