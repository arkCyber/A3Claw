//! skill-datetime — date/time skill plugin for OpenClaw+
use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.datetime",
  "name": "Datetime Skills",
  "version": "0.1.0",
  "description": "Date and time: now, parse, format, diff, add, weekday, leap year, timestamp",
  "skills": [
    { "name": "datetime.now",            "display": "Now",              "description": "Return current UTC as ISO-8601.", "risk": "safe", "params": [] },
    { "name": "datetime.parse",          "display": "Parse",            "description": "Parse ISO-8601 into components.", "risk": "safe", "params": [{"name":"input","type":"string","required":true}] },
    { "name": "datetime.format",         "display": "Format",           "description": "Format datetime with strftime pattern.", "risk": "safe",
      "params": [{"name":"input","type":"string","required":true},{"name":"fmt","type":"string","required":true}] },
    { "name": "datetime.diff",           "display": "Difference",       "description": "Diff two datetimes in a unit.", "risk": "safe",
      "params": [{"name":"from","type":"string","required":true},{"name":"to","type":"string","required":true},{"name":"unit","type":"string","required":false}] },
    { "name": "datetime.add",            "display": "Add Duration",     "description": "Add duration to a datetime.", "risk": "safe",
      "params": [{"name":"input","type":"string","required":true},{"name":"amount","type":"integer","required":true},{"name":"unit","type":"string","required":true}] },
    { "name": "datetime.weekday",        "display": "Weekday",          "description": "Return weekday name for a date.", "risk": "safe", "params": [{"name":"input","type":"string","required":true}] },
    { "name": "datetime.is_leap",        "display": "Is Leap Year",     "description": "Check if a year is a leap year.", "risk": "safe", "params": [{"name":"year","type":"integer","required":true}] },
    { "name": "datetime.days_in_month",  "display": "Days In Month",    "description": "Number of days in a month.", "risk": "safe",
      "params": [{"name":"year","type":"integer","required":true},{"name":"month","type":"integer","required":true}] },
    { "name": "datetime.timestamp",      "display": "To Unix Timestamp","description": "Convert ISO datetime to Unix seconds.", "risk": "safe", "params": [{"name":"input","type":"string","required":true}] },
    { "name": "datetime.from_timestamp", "display": "From Timestamp",   "description": "Convert Unix seconds to ISO datetime.", "risk": "safe", "params": [{"name":"ts","type":"integer","required":true}] }
  ]
}"#;

// ── Internal type ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
struct Dt { year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32 }

fn parse_iso(s: &str) -> Result<Dt, String> {
    let s = s.trim().trim_end_matches('Z').replace('T', " ").replace('/', "-");
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    let dp: Vec<&str> = parts[0].split('-').collect();
    if dp.len() < 3 { return Err(format!("cannot parse date: '{}'", parts[0])); }
    let year:  i32 = dp[0].parse().map_err(|_| format!("bad year: {}", dp[0]))?;
    let month: u32 = dp[1].parse().map_err(|_| format!("bad month: {}", dp[1]))?;
    let day:   u32 = dp[2].parse().map_err(|_| format!("bad day: {}", dp[2]))?;
    let tp: Vec<&str> = parts.get(1).unwrap_or(&"0:0:0").split(':').collect();
    let hour: u32 = tp.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let min:  u32 = tp.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let sec:  u32 = tp.get(2).and_then(|s| s.split('.').next()).and_then(|s| s.parse().ok()).unwrap_or(0);
    if !(1..=12).contains(&month) { return Err(format!("month out of range: {month}")); }
    if !(1..=31).contains(&day)   { return Err(format!("day out of range: {day}"));    }
    Ok(Dt { year, month, day, hour, min, sec })
}

fn dt_to_unix(dt: &Dt) -> i64 {
    let y   = dt.year as i64 - if dt.month <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (dt.month as i64 + if dt.month > 2 { -3 } else { 9 }) + 2) / 5 + dt.day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    days * 86400 + dt.hour as i64 * 3600 + dt.min as i64 * 60 + dt.sec as i64
}

fn unix_to_dt(ts: i64) -> Dt {
    let z   = ts / 86400 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let y   = yoe + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp  = (5*doy + 2) / 153;
    let day   = (doy - (153*mp+2)/5 + 1) as u32;
    let month = (if mp < 10 { mp+3 } else { mp-9 }) as u32;
    let year  = (y + if month <= 2 { 1 } else { 0 }) as i32;
    let rem   = ts.rem_euclid(86400) as u32;
    Dt { year, month, day, hour: rem/3600, min: (rem%3600)/60, sec: rem%60 }
}

fn timestamp_to_iso(ts: i64) -> String {
    let dt = unix_to_dt(ts);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", dt.year, dt.month, dt.day, dt.hour, dt.min, dt.sec)
}

fn is_leap_year(y: i64) -> bool { (y%4==0 && y%100!=0) || y%400==0 }

fn days_in_month(year: i64, month: u32) -> u32 {
    match month { 1|3|5|7|8|10|12 => 31, 4|6|9|11 => 30, 2 => if is_leap_year(year) { 29 } else { 28 }, _ => 0 }
}

fn weekday_name(ts: i64) -> &'static str {
    match ((ts / 86400).rem_euclid(7) + 4) % 7 {
        0 => "Sunday", 1 => "Monday", 2 => "Tuesday", 3 => "Wednesday",
        4 => "Thursday", 5 => "Friday", _ => "Saturday",
    }
}

fn strftime(dt: &Dt, fmt: &str) -> String {
    const ML: [&str;12] = ["January","February","March","April","May","June","July","August","September","October","November","December"];
    const MS: [&str;12] = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    const DL: [&str;7]  = ["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"];
    const DS: [&str;7]  = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"];
    let ts   = dt_to_unix(dt);
    let wday = (((ts/86400).rem_euclid(7)+4)%7) as usize;
    let midx = (dt.month as usize).saturating_sub(1).min(11);
    let mut out = String::with_capacity(fmt.len()+16);
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' { out.push(c); continue; }
        match chars.next() {
            Some('Y') => out.push_str(&format!("{:04}", dt.year)),
            Some('y') => out.push_str(&format!("{:02}", dt.year.abs()%100)),
            Some('m') => out.push_str(&format!("{:02}", dt.month)),
            Some('d') => out.push_str(&format!("{:02}", dt.day)),
            Some('H') => out.push_str(&format!("{:02}", dt.hour)),
            Some('M') => out.push_str(&format!("{:02}", dt.min)),
            Some('S') => out.push_str(&format!("{:02}", dt.sec)),
            Some('A') => out.push_str(DL[wday]),
            Some('a') => out.push_str(DS[wday]),
            Some('B') => out.push_str(ML[midx]),
            Some('b') | Some('h') => out.push_str(MS[midx]),
            Some('%') => out.push('%'),
            Some(x)   => { out.push('%'); out.push(x); }
            None      => out.push('%'),
        }
    }
    out
}

fn dt_to_json(dt: &Dt) -> String {
    let ts = dt_to_unix(dt);
    format!(r#"{{"year":{y},"month":{mo},"day":{d},"hour":{h},"minute":{mi},"second":{s},"weekday":"{wd}","timestamp":{ts}}}"#,
        y=dt.year, mo=dt.month, d=dt.day, h=dt.hour, mi=dt.min, s=dt.sec, wd=weekday_name(ts), ts=ts)
}

fn delta_secs(amount: i64, unit: &str) -> Result<i64, String> {
    match unit {
        "seconds"|"second"|"sec"|"s" => Ok(amount),
        "minutes"|"minute"|"min"     => Ok(amount * 60),
        "hours"|"hour"|"h"           => Ok(amount * 3600),
        "days"|"day"|"d"             => Ok(amount * 86400),
        "weeks"|"week"|"w"           => Ok(amount * 86400 * 7),
        other => Err(format!("unknown unit: {other}")),
    }
}

fn wasi_clock_realtime() -> i64 {
    #[cfg(target_arch = "wasm32")] {
        extern "C" { fn __wasi_clock_time_get(id: u32, prec: u64, out: *mut u64) -> u16; }
        let mut ns: u64 = 0;
        unsafe { __wasi_clock_time_get(0, 1, &mut ns); }
        (ns / 1_000_000_000) as i64
    }
    #[cfg(not(target_arch = "wasm32"))] {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64).unwrap_or(0)
    }
}

// ── Entry points ──────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) { Ok(r) => r, Err(e) => return sdk_respond_err("", &e) };
    let rid = req.request_id.as_str();
    match req.skill.as_str() {
        "datetime.now" => sdk_respond_ok(rid, &timestamp_to_iso(wasi_clock_realtime())),
        "datetime.parse" => {
            match req.args["input"].as_str().ok_or("missing 'input'".to_string()).and_then(parse_iso) {
                Ok(dt)  => sdk_respond_ok(rid, &dt_to_json(&dt)),
                Err(e)  => sdk_respond_err(rid, &e),
            }
        }
        "datetime.format" => {
            let input = match req.args["input"].as_str() { Some(s)=>s, None=>return sdk_respond_err(rid,"missing 'input'") };
            let fmt   = match req.args["fmt"].as_str()   { Some(s)=>s, None=>return sdk_respond_err(rid,"missing 'fmt'")   };
            match parse_iso(input) {
                Ok(dt)  => sdk_respond_ok(rid, &strftime(&dt, fmt)),
                Err(e)  => sdk_respond_err(rid, &e),
            }
        }
        "datetime.diff" => {
            let from = match req.args["from"].as_str() { Some(s)=>s, None=>return sdk_respond_err(rid,"missing 'from'") };
            let to   = match req.args["to"].as_str()   { Some(s)=>s, None=>return sdk_respond_err(rid,"missing 'to'")   };
            let unit = req.args["unit"].as_str().unwrap_or("seconds");
            let ts_from = match parse_iso(from).map(|d| dt_to_unix(&d)) { Ok(t)=>t, Err(e)=>return sdk_respond_err(rid,&e) };
            let ts_to   = match parse_iso(to).map(|d| dt_to_unix(&d))   { Ok(t)=>t, Err(e)=>return sdk_respond_err(rid,&e) };
            match delta_secs(1, unit).map(|d| (ts_to - ts_from) / d) {
                Ok(v)  => sdk_respond_ok(rid, &v.to_string()),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "datetime.add" => {
            let input  = match req.args["input"].as_str()  { Some(s)=>s, None=>return sdk_respond_err(rid,"missing 'input'")  };
            let amount = match req.args["amount"].as_i64() { Some(n)=>n, None=>return sdk_respond_err(rid,"missing 'amount'") };
            let unit   = match req.args["unit"].as_str()   { Some(s)=>s, None=>return sdk_respond_err(rid,"missing 'unit'")   };
            let ts = match parse_iso(input).map(|d| dt_to_unix(&d)) { Ok(t)=>t, Err(e)=>return sdk_respond_err(rid,&e) };
            match delta_secs(amount, unit) {
                Ok(d)  => sdk_respond_ok(rid, &timestamp_to_iso(ts + d)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "datetime.weekday" => {
            match req.args["input"].as_str().ok_or("missing 'input'".to_string()).and_then(parse_iso) {
                Ok(dt)  => sdk_respond_ok(rid, weekday_name(dt_to_unix(&dt))),
                Err(e)  => sdk_respond_err(rid, &e),
            }
        }
        "datetime.is_leap" => {
            match req.args["year"].as_i64() {
                Some(y) => sdk_respond_ok(rid, &is_leap_year(y).to_string()),
                None    => sdk_respond_err(rid, "missing 'year'"),
            }
        }
        "datetime.days_in_month" => {
            let year  = match req.args["year"].as_i64()  { Some(n)=>n, None=>return sdk_respond_err(rid,"missing 'year'")  };
            let month = match req.args["month"].as_u64() { Some(n) if (1..=12).contains(&n) => n as u32, _ => return sdk_respond_err(rid,"'month' must be 1-12") };
            sdk_respond_ok(rid, &days_in_month(year, month).to_string())
        }
        "datetime.timestamp" => {
            match req.args["input"].as_str().ok_or("missing 'input'".to_string()).and_then(parse_iso) {
                Ok(dt)  => sdk_respond_ok(rid, &dt_to_unix(&dt).to_string()),
                Err(e)  => sdk_respond_err(rid, &e),
            }
        }
        "datetime.from_timestamp" => {
            match req.args["ts"].as_i64() {
                Some(ts) => sdk_respond_ok(rid, &timestamp_to_iso(ts)),
                None     => sdk_respond_err(rid, "missing 'ts'"),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {other}")),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn parse_date_only() {
        let dt = parse_iso("2024-03-15").unwrap();
        assert_eq!(dt.year, 2024); assert_eq!(dt.month, 3); assert_eq!(dt.day, 15);
    }
    #[test] fn parse_full_iso() {
        let dt = parse_iso("2024-03-15T14:30:00Z").unwrap();
        assert_eq!(dt.hour, 14); assert_eq!(dt.min, 30);
    }
    #[test] fn unix_epoch_roundtrip() {
        assert_eq!(dt_to_unix(&parse_iso("1970-01-01T00:00:00Z").unwrap()), 0);
    }
    #[test] fn unix_to_dt_epoch() {
        let dt = unix_to_dt(0);
        assert_eq!(dt.year, 1970); assert_eq!(dt.month, 1); assert_eq!(dt.day, 1);
    }
    #[test] fn timestamp_roundtrip() {
        let iso = "2024-06-15T12:30:45Z";
        assert_eq!(timestamp_to_iso(dt_to_unix(&parse_iso(iso).unwrap())), iso);
    }
    #[test] fn weekday_epoch_is_thursday() {
        assert_eq!(weekday_name(0), "Thursday");
    }
    #[test] fn is_leap_known() {
        assert!(is_leap_year(2000)); assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900)); assert!(!is_leap_year(2023));
    }
    #[test] fn days_in_february_leap() { assert_eq!(days_in_month(2024, 2), 29); }
    #[test] fn days_in_february_nonleap() { assert_eq!(days_in_month(2023, 2), 28); }
    #[test] fn days_in_january() { assert_eq!(days_in_month(2024, 1), 31); }
    #[test] fn strftime_basic() {
        let dt = parse_iso("2024-03-15T14:30:45Z").unwrap();
        assert_eq!(strftime(&dt, "%Y-%m-%d"), "2024-03-15");
        assert_eq!(strftime(&dt, "%H:%M:%S"), "14:30:45");
    }
    #[test] fn diff_in_days() {
        let ts1 = dt_to_unix(&parse_iso("2024-01-01").unwrap());
        let ts2 = dt_to_unix(&parse_iso("2024-01-11").unwrap());
        assert_eq!((ts2-ts1) / 86400, 10);
    }
    #[test] fn add_one_day() {
        let ts = dt_to_unix(&parse_iso("2024-01-01").unwrap());
        let next = timestamp_to_iso(ts + 86400);
        assert_eq!(next, "2024-01-02T00:00:00Z");
    }
    // ── leap year edge cases ────────────────────────────────────────────
    #[test] fn is_leap_century_divisible_400() { assert!(is_leap_year(1600)); assert!(is_leap_year(2000)); }
    #[test] fn is_leap_century_not_divisible_400() { assert!(!is_leap_year(1700)); assert!(!is_leap_year(1800)); }
    #[test] fn is_leap_regular_divisible_4() { assert!(is_leap_year(2020)); assert!(is_leap_year(2024)); }
    #[test] fn is_leap_not_divisible_4()     { assert!(!is_leap_year(2019)); assert!(!is_leap_year(2021)); }

    // ── days_in_month edge cases ──────────────────────────────────────
    #[test] fn days_in_april()     { assert_eq!(days_in_month(2024, 4), 30); }
    #[test] fn days_in_december()  { assert_eq!(days_in_month(2024, 12), 31); }
    #[test] fn days_in_june()      { assert_eq!(days_in_month(2024, 6), 30); }

    // ── weekday_name ───────────────────────────────────────────────────
    #[test] fn weekday_one_week_later() {
        assert_eq!(weekday_name(7 * 86400), "Thursday");
    }
    #[test] fn weekday_friday() {
        assert_eq!(weekday_name(86400), "Friday");
    }
    #[test] fn weekday_saturday() {
        assert_eq!(weekday_name(2 * 86400), "Saturday");
    }

    // ── strftime patterns ──────────────────────────────────────────────
    #[test] fn strftime_day_only() {
        let dt = parse_iso("2024-12-25T00:00:00Z").unwrap();
        assert_eq!(strftime(&dt, "%d"), "25");
    }
    #[test] fn strftime_month_only() {
        let dt = parse_iso("2024-03-15T00:00:00Z").unwrap();
        assert_eq!(strftime(&dt, "%m"), "03");
    }

    // ── unix roundtrip various dates ────────────────────────────────────
    #[test] fn unix_roundtrip_2000() {
        let iso = "2000-01-01T00:00:00Z";
        assert_eq!(timestamp_to_iso(dt_to_unix(&parse_iso(iso).unwrap())), iso);
    }
    #[test] fn unix_roundtrip_2024_leap() {
        let iso = "2024-02-29T12:00:00Z";
        assert_eq!(timestamp_to_iso(dt_to_unix(&parse_iso(iso).unwrap())), iso);
    }

    #[test] fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.datetime");
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("datetime."));
        }
    }
}
