//! skill-math — math skill plugin for OpenClaw+
//!
//! Skills:
//!   math.eval      { expr: string }                    → number result
//!   math.round     { value: f64, decimals?: u32 }      → rounded number
//!   math.clamp     { value: f64, min: f64, max: f64 }  → clamped number
//!   math.stats     { values: [f64] }                   → {min,max,sum,mean,median,stddev}
//!   math.convert   { value: f64, from: string, to: string } → converted value
//!   math.format    { value: f64, decimals?: u32, thousands_sep?: bool } → formatted string
//!   math.gcd       { a: i64, b: i64 }                  → GCD
//!   math.lcm       { a: i64, b: i64 }                  → LCM
//!   math.is_prime  { n: u64 }                          → bool

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.math",
  "name": "Math Skills",
  "version": "0.1.0",
  "description": "Numeric operations: expression eval, stats, unit conversion, formatting",
  "skills": [
    {
      "name": "math.eval",
      "display": "Evaluate Expression",
      "description": "Evaluate a simple arithmetic expression (+, -, *, /, ^, %, parentheses, unary minus).",
      "risk": "safe",
      "params": [{ "name": "expr", "type": "string", "required": true }]
    },
    {
      "name": "math.round",
      "display": "Round Number",
      "description": "Round a number to a given decimal place (default 0).",
      "risk": "safe",
      "params": [
        { "name": "value",    "type": "number",  "required": true  },
        { "name": "decimals", "type": "integer", "required": false }
      ]
    },
    {
      "name": "math.clamp",
      "display": "Clamp Number",
      "description": "Restrict a value to [min, max].",
      "risk": "safe",
      "params": [
        { "name": "value", "type": "number", "required": true },
        { "name": "min",   "type": "number", "required": true },
        { "name": "max",   "type": "number", "required": true }
      ]
    },
    {
      "name": "math.stats",
      "display": "Descriptive Statistics",
      "description": "Compute min, max, sum, mean, median, and standard deviation for a list of numbers.",
      "risk": "safe",
      "params": [{ "name": "values", "type": "array", "required": true }]
    },
    {
      "name": "math.convert",
      "display": "Unit Conversion",
      "description": "Convert a numeric value between units (length, mass, temperature, area, volume, speed).",
      "risk": "safe",
      "params": [
        { "name": "value", "type": "number", "required": true },
        { "name": "from",  "type": "string", "required": true },
        { "name": "to",    "type": "string", "required": true }
      ]
    },
    {
      "name": "math.format",
      "display": "Format Number",
      "description": "Format a number with optional decimal places and thousands separator.",
      "risk": "safe",
      "params": [
        { "name": "value",         "type": "number",  "required": true  },
        { "name": "decimals",      "type": "integer", "required": false },
        { "name": "thousands_sep", "type": "boolean", "required": false }
      ]
    },
    {
      "name": "math.gcd",
      "display": "GCD",
      "description": "Compute the greatest common divisor of two integers.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "integer", "required": true },
        { "name": "b", "type": "integer", "required": true }
      ]
    },
    {
      "name": "math.lcm",
      "display": "LCM",
      "description": "Compute the least common multiple of two integers.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "integer", "required": true },
        { "name": "b", "type": "integer", "required": true }
      ]
    },
    {
      "name": "math.is_prime",
      "display": "Is Prime",
      "description": "Test whether a non-negative integer is prime.",
      "risk": "safe",
      "params": [{ "name": "n", "type": "integer", "required": true }]
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

    match req.skill.as_str() {
        "math.eval" => {
            let expr = match req.args["expr"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'expr' argument"),
            };
            match eval_expr(expr) {
                Ok(v)  => sdk_respond_ok(rid, &format_float(v, 10, false)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }

        "math.round" => {
            let value = match req.args["value"].as_f64() {
                Some(v) => v,
                None    => return sdk_respond_err(rid, "missing or invalid 'value'"),
            };
            let decimals = req.args["decimals"].as_u64().unwrap_or(0) as u32;
            let factor = 10f64.powi(decimals as i32);
            let rounded = (value * factor).round() / factor;
            sdk_respond_ok(rid, &format_float(rounded, decimals, false))
        }

        "math.clamp" => {
            let value = match req.args["value"].as_f64() {
                Some(v) => v,
                None    => return sdk_respond_err(rid, "missing 'value'"),
            };
            let min = match req.args["min"].as_f64() {
                Some(v) => v,
                None    => return sdk_respond_err(rid, "missing 'min'"),
            };
            let max = match req.args["max"].as_f64() {
                Some(v) => v,
                None    => return sdk_respond_err(rid, "missing 'max'"),
            };
            if min > max {
                return sdk_respond_err(rid, "'min' must be <= 'max'");
            }
            let clamped = value.max(min).min(max);
            sdk_respond_ok(rid, &format_float(clamped, 10, false))
        }

        "math.stats" => {
            let arr = match req.args["values"].as_array() {
                Some(a) => a,
                None    => return sdk_respond_err(rid, "missing 'values' array"),
            };
            let nums: Result<Vec<f64>, _> = arr.iter()
                .map(|v| v.as_f64().ok_or("non-numeric element in array"))
                .collect();
            match nums {
                Ok(v) => match compute_stats(&v) {
                    Ok(s)  => sdk_respond_ok(rid, &s),
                    Err(e) => sdk_respond_err(rid, &e),
                },
                Err(e) => sdk_respond_err(rid, e),
            }
        }

        "math.convert" => {
            let value = match req.args["value"].as_f64() {
                Some(v) => v,
                None    => return sdk_respond_err(rid, "missing 'value'"),
            };
            let from = match req.args["from"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'from'"),
            };
            let to = match req.args["to"].as_str() {
                Some(s) => s,
                None    => return sdk_respond_err(rid, "missing 'to'"),
            };
            match unit_convert(value, from, to) {
                Ok(v)  => sdk_respond_ok(rid, &format!("{} {}", format_float(v, 6, false), to)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }

        "math.format" => {
            let value = match req.args["value"].as_f64() {
                Some(v) => v,
                None    => return sdk_respond_err(rid, "missing 'value'"),
            };
            let decimals = req.args["decimals"].as_u64().unwrap_or(2) as u32;
            let thousands = req.args["thousands_sep"].as_bool().unwrap_or(false);
            sdk_respond_ok(rid, &format_float(value, decimals, thousands))
        }

        "math.gcd" => {
            let a = match req.args["a"].as_i64() {
                Some(v) => v.abs(),
                None    => return sdk_respond_err(rid, "missing 'a'"),
            };
            let b = match req.args["b"].as_i64() {
                Some(v) => v.abs(),
                None    => return sdk_respond_err(rid, "missing 'b'"),
            };
            sdk_respond_ok(rid, &gcd(a, b).to_string())
        }

        "math.lcm" => {
            let a = match req.args["a"].as_i64() {
                Some(v) => v.abs(),
                None    => return sdk_respond_err(rid, "missing 'a'"),
            };
            let b = match req.args["b"].as_i64() {
                Some(v) => v.abs(),
                None    => return sdk_respond_err(rid, "missing 'b'"),
            };
            if a == 0 || b == 0 {
                return sdk_respond_ok(rid, "0");
            }
            sdk_respond_ok(rid, &(a / gcd(a, b) * b).to_string())
        }

        "math.is_prime" => {
            let n = match req.args["n"].as_u64() {
                Some(v) => v,
                None    => return sdk_respond_err(rid, "missing 'n'"),
            };
            sdk_respond_ok(rid, &is_prime(n).to_string())
        }

        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Arithmetic expression evaluator ──────────────────────────────────────────
// Supports: +, -, *, /, %, ^ (power), unary minus, parentheses, integer/float literals.
// Precedence: ^ > unary > * / % > + -

fn eval_expr(input: &str) -> Result<f64, String> {
    let tokens = tokenize(input)?;
    let mut pos = 0usize;
    let result = parse_additive(&tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err(format!("unexpected token at position {}: {:?}", pos, tokens[pos]));
    }
    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
enum Token { Num(f64), Plus, Minus, Star, Slash, Percent, Caret, LParen, RParen }

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' => { i += 1; }
            '+' => { tokens.push(Token::Plus);    i += 1; }
            '-' => { tokens.push(Token::Minus);   i += 1; }
            '*' => { tokens.push(Token::Star);    i += 1; }
            '/' => { tokens.push(Token::Slash);   i += 1; }
            '%' => { tokens.push(Token::Percent); i += 1; }
            '^' => { tokens.push(Token::Caret);   i += 1; }
            '(' => { tokens.push(Token::LParen);  i += 1; }
            ')' => { tokens.push(Token::RParen);  i += 1; }
            '0'..='9' | '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let v: f64 = s.parse().map_err(|_| format!("invalid number: {s}"))?;
                tokens.push(Token::Num(v));
            }
            c => return Err(format!("unexpected character: {:?}", c)),
        }
    }
    Ok(tokens)
}

fn parse_additive(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_multiplicative(tokens, pos)?;
    while *pos < tokens.len() {
        match tokens[*pos] {
            Token::Plus  => { *pos += 1; left += parse_multiplicative(tokens, pos)?; }
            Token::Minus => { *pos += 1; left -= parse_multiplicative(tokens, pos)?; }
            _            => break,
        }
    }
    Ok(left)
}

fn parse_multiplicative(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_unary(tokens, pos)?;
    while *pos < tokens.len() {
        match tokens[*pos] {
            Token::Star    => { *pos += 1; left *= parse_unary(tokens, pos)?; }
            Token::Slash   => {
                *pos += 1;
                let r = parse_unary(tokens, pos)?;
                if r == 0.0 { return Err("division by zero".into()); }
                left /= r;
            }
            Token::Percent => {
                *pos += 1;
                let r = parse_unary(tokens, pos)?;
                if r == 0.0 { return Err("modulo by zero".into()); }
                left %= r;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_unary(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    if *pos < tokens.len() && tokens[*pos] == Token::Minus {
        *pos += 1;
        return Ok(-parse_power(tokens, pos)?);
    }
    if *pos < tokens.len() && tokens[*pos] == Token::Plus {
        *pos += 1;
    }
    parse_power(tokens, pos)
}

fn parse_power(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    let base = parse_primary(tokens, pos)?;
    if *pos < tokens.len() && tokens[*pos] == Token::Caret {
        *pos += 1;
        let exp = parse_unary(tokens, pos)?;
        Ok(base.powf(exp))
    } else {
        Ok(base)
    }
}

fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    if *pos >= tokens.len() {
        return Err("unexpected end of expression".into());
    }
    match &tokens[*pos] {
        Token::Num(v) => { let v = *v; *pos += 1; Ok(v) }
        Token::LParen => {
            *pos += 1;
            let v = parse_additive(tokens, pos)?;
            if *pos >= tokens.len() || tokens[*pos] != Token::RParen {
                return Err("missing closing ')'".into());
            }
            *pos += 1;
            Ok(v)
        }
        t => Err(format!("unexpected token: {:?}", t)),
    }
}

// ── Statistics ────────────────────────────────────────────────────────────────

fn compute_stats(values: &[f64]) -> Result<String, String> {
    if values.is_empty() {
        return Err("empty values array".into());
    }
    let n = values.len() as f64;
    let sum: f64 = values.iter().sum();
    let mean = sum / n;
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    Ok(format!(
        r#"{{"count":{count},"sum":{sum},"min":{min},"max":{max},"mean":{mean},"median":{median},"stddev":{stddev}}}"#,
        count = values.len(),
        sum    = format_float(sum,    6, false),
        min    = format_float(min,    6, false),
        max    = format_float(max,    6, false),
        mean   = format_float(mean,   6, false),
        median = format_float(median, 6, false),
        stddev = format_float(stddev, 6, false),
    ))
}

// ── Unit conversion ───────────────────────────────────────────────────────────

fn unit_convert(value: f64, from: &str, to: &str) -> Result<f64, String> {
    if from == to { return Ok(value); }

    // Temperature requires special handling
    let is_temp = matches!(from, "C"|"F"|"K") || matches!(to, "C"|"F"|"K");
    if is_temp {
        let celsius = match from {
            "C" => value,
            "F" => (value - 32.0) * 5.0 / 9.0,
            "K" => value - 273.15,
            _   => return Err(format!("unknown temperature unit: {from}")),
        };
        return match to {
            "C" => Ok(celsius),
            "F" => Ok(celsius * 9.0 / 5.0 + 32.0),
            "K" => Ok(celsius + 273.15),
            _   => Err(format!("unknown temperature unit: {to}")),
        };
    }

    // SI-based conversions: convert to a common SI base, then to target.
    // The map contains (unit_name → factor_to_SI_base).
    let to_si = |unit: &str| -> Option<f64> {
        match unit {
            // Length (base: metre)
            "mm" => Some(0.001),       "cm" => Some(0.01),
            "m"  => Some(1.0),         "km" => Some(1000.0),
            "in" => Some(0.0254),      "ft" => Some(0.3048),
            "yd" => Some(0.9144),      "mi" => Some(1609.344),
            "nm" => Some(1e-9),
            // Mass (base: kg)
            "mg" => Some(1e-6),        "g"  => Some(0.001),
            "kg" => Some(1.0),         "t"  => Some(1000.0),
            "lb" => Some(0.453592),    "oz" => Some(0.0283495),
            // Area (base: m²)
            "mm2" => Some(1e-6),       "cm2" => Some(1e-4),
            "m2"  => Some(1.0),        "km2" => Some(1e6),
            "ft2" => Some(0.092903),   "acre"=> Some(4046.86),
            // Volume (base: litre)
            "ml"  => Some(0.001),      "l"   => Some(1.0),
            "m3"  => Some(1000.0),     "gal" => Some(3.78541),
            "qt"  => Some(0.946353),   "pt"  => Some(0.473176),
            "fl_oz"=> Some(0.0295735),
            // Speed (base: m/s)
            "m/s"  => Some(1.0),       "km/h" => Some(1.0/3.6),
            "mph"  => Some(0.44704),   "knot" => Some(0.514444),
            // Data (base: byte)
            "B"   => Some(1.0),        "KB"  => Some(1024.0),
            "MB"  => Some(1024.0_f64.powi(2)), "GB" => Some(1024.0_f64.powi(3)),
            "TB"  => Some(1024.0_f64.powi(4)),
            _     => None,
        }
    };

    let from_si = to_si(from).ok_or_else(|| format!("unknown unit: {from}"))?;
    let to_si_v = to_si(to).ok_or_else(|| format!("unknown unit: {to}"))?;
    Ok(value * from_si / to_si_v)
}

// ── Number formatting ─────────────────────────────────────────────────────────

fn format_float(value: f64, decimals: u32, thousands_sep: bool) -> String {
    if value.is_nan()      { return "NaN".into(); }
    if value.is_infinite() { return if value > 0.0 { "Infinity".into() } else { "-Infinity".into() }; }

    // Build decimal representation
    let factor = 10f64.powi(decimals as i32);
    let rounded = (value * factor).round() / factor;
    let int_part = rounded.trunc() as i64;
    let frac = (rounded.abs().fract() * factor).round() as u64;

    let int_str = if thousands_sep {
        let s = int_part.unsigned_abs().to_string();
        let with_sep = s.as_bytes().rchunks(3)
            .rev()
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join(",");
        if int_part < 0 { format!("-{}", with_sep) } else { with_sep }
    } else {
        int_part.to_string()
    };

    if decimals == 0 {
        int_str
    } else {
        format!("{}.{:0>width$}", int_str, frac, width = decimals as usize)
    }
}

// ── GCD / LCM ─────────────────────────────────────────────────────────────────

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 { let t = b; b = a % b; a = t; }
    a
}

// ── Primality test (deterministic for n < 3,215,031,751) ─────────────────────

fn is_prime(n: u64) -> bool {
    if n < 2  { return false; }
    if n == 2 { return true;  }
    if n % 2 == 0 { return false; }
    let limit = (n as f64).sqrt() as u64 + 1;
    let mut i = 3u64;
    while i < limit {
        if n % i == 0 { return false; }
        i += 2;
    }
    true
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_basic_add() {
        assert_eq!(eval_expr("1 + 2").unwrap(), 3.0);
    }

    #[test]
    fn eval_precedence() {
        assert_eq!(eval_expr("2 + 3 * 4").unwrap(), 14.0);
    }

    #[test]
    fn eval_parens() {
        assert_eq!(eval_expr("(2 + 3) * 4").unwrap(), 20.0);
    }

    #[test]
    fn eval_power() {
        assert_eq!(eval_expr("2 ^ 10").unwrap(), 1024.0);
    }

    #[test]
    fn eval_unary_minus() {
        assert_eq!(eval_expr("-5 + 3").unwrap(), -2.0);
    }

    #[test]
    fn eval_division_by_zero() {
        assert!(eval_expr("1 / 0").is_err());
    }

    #[test]
    fn eval_float() {
        let result = eval_expr("3.14 * 2").unwrap();
        assert!((result - 6.28).abs() < 1e-9);
    }

    #[test]
    fn eval_complex() {
        // (1 + 2) * (3 + 4) / 7 = 3
        assert_eq!(eval_expr("(1 + 2) * (3 + 4) / 7").unwrap(), 3.0);
    }

    #[test]
    fn stats_basic() {
        let s = compute_stats(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["count"].as_u64().unwrap(), 5);
        let min  = v["min"].as_f64().unwrap();
        let max  = v["max"].as_f64().unwrap();
        let mean = v["mean"].as_f64().unwrap();
        assert!((min  - 1.0).abs() < 1e-9);
        assert!((max  - 5.0).abs() < 1e-9);
        assert!((mean - 3.0).abs() < 1e-9);
    }

    #[test]
    fn stats_single_element() {
        let s = compute_stats(&[42.0]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let mean = v["mean"].as_f64().unwrap();
        assert!((mean - 42.0).abs() < 1e-4);
    }

    #[test]
    fn stats_empty_error() {
        assert!(compute_stats(&[]).is_err());
    }

    #[test]
    fn unit_convert_km_to_m() {
        let r = unit_convert(1.0, "km", "m").unwrap();
        assert!((r - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn unit_convert_celsius_to_fahrenheit() {
        let r = unit_convert(0.0, "C", "F").unwrap();
        assert!((r - 32.0).abs() < 1e-9);
    }

    #[test]
    fn unit_convert_fahrenheit_to_celsius() {
        let r = unit_convert(212.0, "F", "C").unwrap();
        assert!((r - 100.0).abs() < 1e-9);
    }

    #[test]
    fn unit_convert_unknown_unit() {
        assert!(unit_convert(1.0, "parsec", "ly").is_err());
    }

    #[test]
    fn format_float_decimals() {
        assert_eq!(format_float(3.14159, 2, false), "3.14");
    }

    #[test]
    fn format_float_thousands() {
        assert_eq!(format_float(1234567.0, 0, true), "1,234,567");
    }

    #[test]
    fn gcd_known() {
        assert_eq!(gcd(48, 18), 6);
    }

    #[test]
    fn gcd_coprime() {
        assert_eq!(gcd(7, 13), 1);
    }

    #[test]
    fn is_prime_known() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(97));
        assert!(!is_prime(100));
    }

    // ── eval extra ───────────────────────────────────────────────────────────────
    #[test]
    fn eval_addition()       { assert!((eval_expr("2 + 3").unwrap() - 5.0).abs() < 1e-9); }
    #[test]
    fn eval_subtraction()    { assert!((eval_expr("10 - 4").unwrap() - 6.0).abs() < 1e-9); }
    #[test]
    fn eval_multiplication() { assert!((eval_expr("3 * 4").unwrap() - 12.0).abs() < 1e-9); }
    #[test]
    fn eval_division()       { assert!((eval_expr("10 / 4").unwrap() - 2.5).abs() < 1e-9); }
    #[test]
    fn eval_parentheses()    { assert!((eval_expr("(2 + 3) * 4").unwrap() - 20.0).abs() < 1e-9); }
    #[test]
    fn eval_negative_result(){ assert!((eval_expr("1 - 5").unwrap() - (-4.0)).abs() < 1e-9); }

    // ── stats ───────────────────────────────────────────────────────────────────
    #[test]
    fn stats_single_element_via_json() {
        let json = compute_stats(&[42.0]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!((v["mean"].as_f64().unwrap() - 42.0).abs() < 1e-4);
        assert!((v["min"].as_f64().unwrap() - 42.0).abs() < 1e-4);
        assert!((v["max"].as_f64().unwrap() - 42.0).abs() < 1e-4);
    }
    #[test]
    fn stats_two_elements_via_json() {
        let json = compute_stats(&[0.0, 10.0]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!((v["mean"].as_f64().unwrap() - 5.0).abs() < 1e-4);
    }
    #[test]
    fn stats_all_same_via_json() {
        let json = compute_stats(&[7.0, 7.0, 7.0]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!((v["min"].as_f64().unwrap() - 7.0).abs() < 1e-4);
        assert!((v["max"].as_f64().unwrap() - 7.0).abs() < 1e-4);
        assert!(v["stddev"].as_f64().unwrap().abs() < 1e-4);
    }

    // ── gcd / lcm ─────────────────────────────────────────────────────────────
    #[test]
    fn lcm_known() {
        let g = gcd(4, 6);
        let l = 4 * 6 / g;
        assert_eq!(l, 12);
    }
    #[test]
    fn gcd_with_zero() {
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(5, 0), 5);
    }
    #[test]
    fn gcd_same_values()  { assert_eq!(gcd(7, 7), 7); }
    #[test]
    fn is_prime_large()   { assert!(is_prime(101)); assert!(is_prime(997)); }
    #[test]
    fn is_prime_composites() { assert!(!is_prime(100)); assert!(!is_prime(49)); }

    // ── unit convert additional ───────────────────────────────────────────────
    #[test]
    fn unit_convert_m_to_km() {
        let r = unit_convert(1000.0, "m", "km").unwrap();
        assert!((r - 1.0).abs() < 1e-9);
    }
    #[test]
    fn unit_convert_kelvin_to_celsius() {
        let r = unit_convert(273.15, "K", "C").unwrap();
        assert!(r.abs() < 0.01, "expected ~0 °C, got {}", r);
    }

    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.math");
        assert_eq!(v["skills"].as_array().unwrap().len(), 9);
    }
    #[test]
    fn manifest_skill_names_start_with_math() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("math."));
        }
    }
}
