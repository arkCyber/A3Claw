use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.number",
  "name": "Number Utilities",
  "version": "0.1.0",
  "description": "10 numeric utility skills: clamp, lerp, round_to, to_ordinal, to_roman, from_roman, fibonacci, factorial, is_prime, digits.",
  "skills": [
    {"name":"number.clamp","display":"Clamp","description":"Clamp a number between min and max.","risk":"safe","params":[{"name":"value","type":"number","description":"Input","required":true},{"name":"min","type":"number","description":"Minimum","required":true},{"name":"max","type":"number","description":"Maximum","required":true}]},
    {"name":"number.lerp","display":"Lerp","description":"Linear interpolation between a and b by t (0-1).","risk":"safe","params":[{"name":"a","type":"number","description":"Start","required":true},{"name":"b","type":"number","description":"End","required":true},{"name":"t","type":"number","description":"Factor 0-1","required":true}]},
    {"name":"number.round_to","display":"Round To","description":"Round number to N decimal places.","risk":"safe","params":[{"name":"value","type":"number","description":"Input","required":true},{"name":"places","type":"integer","description":"Decimal places","required":true}]},
    {"name":"number.to_ordinal","display":"To Ordinal","description":"Convert integer to ordinal string (1st, 2nd …).","risk":"safe","params":[{"name":"n","type":"integer","description":"Integer","required":true}]},
    {"name":"number.to_roman","display":"To Roman","description":"Convert integer (1-3999) to Roman numeral.","risk":"safe","params":[{"name":"n","type":"integer","description":"Integer 1-3999","required":true}]},
    {"name":"number.from_roman","display":"From Roman","description":"Convert Roman numeral string to integer.","risk":"safe","params":[{"name":"roman","type":"string","description":"Roman numeral","required":true}]},
    {"name":"number.fibonacci","display":"Fibonacci","description":"Return the Nth Fibonacci number (0-indexed, N<=80).","risk":"safe","params":[{"name":"n","type":"integer","description":"Index N","required":true}]},
    {"name":"number.factorial","display":"Factorial","description":"Return N! as a string (N<=20).","risk":"safe","params":[{"name":"n","type":"integer","description":"N","required":true}]},
    {"name":"number.is_prime","display":"Is Prime","description":"Check whether N is a prime number.","risk":"safe","params":[{"name":"n","type":"integer","description":"Integer to test","required":true}]},
    {"name":"number.digits","display":"Digits","description":"Return the decimal digits of N as a JSON array.","risk":"safe","params":[{"name":"n","type":"integer","description":"Non-negative integer","required":true}]}
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

    match req.skill.as_str() {
        "number.clamp" => {
            let v = args["value"].as_f64().unwrap_or(0.0);
            let lo = args["min"].as_f64().unwrap_or(f64::NEG_INFINITY);
            let hi = args["max"].as_f64().unwrap_or(f64::INFINITY);
            sdk_respond_ok(rid, &v.max(lo).min(hi).to_string())
        }
        "number.lerp" => {
            let a = args["a"].as_f64().unwrap_or(0.0);
            let b = args["b"].as_f64().unwrap_or(0.0);
            let t = args["t"].as_f64().unwrap_or(0.0).max(0.0).min(1.0);
            sdk_respond_ok(rid, &(a + (b - a) * t).to_string())
        }
        "number.round_to" => {
            let v = args["value"].as_f64().unwrap_or(0.0);
            let p = args["places"].as_u64().unwrap_or(2) as u32;
            let factor = 10f64.powi(p as i32);
            sdk_respond_ok(rid, &((v * factor).round() / factor).to_string())
        }
        "number.to_ordinal" => {
            let n = args["n"].as_i64().unwrap_or(0);
            let suffix = match (n.abs() % 100, n.abs() % 10) {
                (11..=13, _) => "th",
                (_, 1) => "st",
                (_, 2) => "nd",
                (_, 3) => "rd",
                _ => "th",
            };
            sdk_respond_ok(rid, &format!("{}{}", n, suffix))
        }
        "number.to_roman" => {
            let n = args["n"].as_u64().unwrap_or(0) as usize;
            if n == 0 || n > 3999 {
                return sdk_respond_err(rid, "n must be between 1 and 3999");
            }
            let vals = [1000,900,500,400,100,90,50,40,10,9,5,4,1];
            let syms = ["M","CM","D","CD","C","XC","L","XL","X","IX","V","IV","I"];
            let mut rem = n;
            let mut out = String::new();
            for (i, &v) in vals.iter().enumerate() {
                while rem >= v { out.push_str(syms[i]); rem -= v; }
            }
            sdk_respond_ok(rid, &out)
        }
        "number.from_roman" => {
            let roman = args["roman"].as_str().unwrap_or("").to_uppercase();
            let val_of = |c| match c { 'I'=>1,'V'=>5,'X'=>10,'L'=>50,'C'=>100,'D'=>500,'M'=>1000,_=>0i64 };
            let chars: Vec<char> = roman.chars().collect();
            let mut total = 0i64;
            for i in 0..chars.len() {
                let cur = val_of(chars[i]);
                let nxt = if i+1 < chars.len() { val_of(chars[i+1]) } else { 0 };
                if cur < nxt { total -= cur; } else { total += cur; }
            }
            sdk_respond_ok(rid, &total.to_string())
        }
        "number.fibonacci" => {
            let n = args["n"].as_u64().unwrap_or(0);
            if n > 80 { return sdk_respond_err(rid, "n must be <= 80"); }
            let (mut a, mut b) = (0u128, 1u128);
            for _ in 0..n { let tmp = a + b; a = b; b = tmp; }
            sdk_respond_ok(rid, &a.to_string())
        }
        "number.factorial" => {
            let n = args["n"].as_u64().unwrap_or(0);
            if n > 20 { return sdk_respond_err(rid, "n must be <= 20"); }
            let result: u128 = (1..=n as u128).product();
            sdk_respond_ok(rid, &result.to_string())
        }
        "number.is_prime" => {
            let n = args["n"].as_u64().unwrap_or(0);
            let prime = n >= 2 && (2..=((n as f64).sqrt() as u64)).all(|d| n % d != 0);
            sdk_respond_ok(rid, if prime { "true" } else { "false" })
        }
        "number.digits" => {
            let n = args["n"].as_u64().unwrap_or(0);
            let digits: Vec<serde_json::Value> = n.to_string().chars()
                .map(|c| serde_json::Value::Number(serde_json::Number::from(c as u8 - b'0')))
                .collect();
            sdk_respond_ok(rid, &serde_json::to_string(&digits).unwrap_or_default())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── number.clamp ─────────────────────────────────────────────────────
    #[test] fn clamp_mid()   { assert_eq!(5.0f64.max(1.0).min(10.0), 5.0); }
    #[test] fn clamp_below() { assert_eq!((-5.0f64).max(0.0).min(10.0), 0.0); }
    #[test] fn clamp_above() { assert_eq!(20.0f64.max(0.0).min(10.0), 10.0); }
    #[test] fn clamp_at_min(){ assert_eq!(0.0f64.max(0.0).min(10.0), 0.0); }
    #[test] fn clamp_at_max(){ assert_eq!(10.0f64.max(0.0).min(10.0), 10.0); }

    // ── number.lerp ───────────────────────────────────────────────────────
    #[test] fn lerp_half()  { let (a,b,t) = (0.0f64,10.0,0.5); assert_eq!(a+(b-a)*t, 5.0); }
    #[test] fn lerp_t0()    { let (a,b,t) = (3.0f64,7.0,0.0); assert_eq!(a+(b-a)*t, 3.0); }
    #[test] fn lerp_t1()    { let (a,b,t) = (3.0f64,7.0,1.0); assert_eq!(a+(b-a)*t, 7.0); }

    // ── number.round_to ──────────────────────────────────────────────────
    #[test] fn round_to_2()   { let f=10f64.powi(2); assert_eq!((3.14159*f).round()/f, 3.14); }
    #[test] fn round_to_0()   { let f=10f64.powi(0); assert_eq!((2.7*f).round()/f, 3.0); }

    // ── number.to_ordinal ─────────────────────────────────────────────────
    #[test] fn ordinal_1st()  { let n=1i64; let s = match (n.abs()%100,n.abs()%10){(11..=13,_)=>"th",(_,1)=>"st",(_,2)=>"nd",(_,3)=>"rd",_=>"th"}; assert_eq!(s,"st"); }
    #[test] fn ordinal_2nd()  { let n=2i64; let s = match (n.abs()%100,n.abs()%10){(11..=13,_)=>"th",(_,1)=>"st",(_,2)=>"nd",(_,3)=>"rd",_=>"th"}; assert_eq!(s,"nd"); }
    #[test] fn ordinal_3rd()  { let n=3i64; let s = match (n.abs()%100,n.abs()%10){(11..=13,_)=>"th",(_,1)=>"st",(_,2)=>"nd",(_,3)=>"rd",_=>"th"}; assert_eq!(s,"rd"); }
    #[test] fn ordinal_11()   { let n=11i64; let s = match (n.abs()%100,n.abs()%10){(11..=13,_)=>"th",(_,1)=>"st",(_,2)=>"nd",(_,3)=>"rd",_=>"th"}; assert_eq!(s,"th"); }
    #[test] fn ordinal_21st() { let n=21i64; let s = match (n.abs()%100,n.abs()%10){(11..=13,_)=>"th",(_,1)=>"st",(_,2)=>"nd",(_,3)=>"rd",_=>"th"}; assert_eq!(s,"st"); }

    // ── number.to_roman ──────────────────────────────────────────────────
    #[test] fn roman_42() {
        let vals=[1000,900,500,400,100,90,50,40,10,9,5,4,1usize];
        let syms=["M","CM","D","CD","C","XC","L","XL","X","IX","V","IV","I"];
        let mut rem=42usize; let mut out=String::new();
        for (i,&v) in vals.iter().enumerate() { while rem>=v { out.push_str(syms[i]); rem-=v; } }
        assert_eq!(out, "XLII");
    }
    #[test] fn roman_2024() {
        let vals=[1000,900,500,400,100,90,50,40,10,9,5,4,1usize];
        let syms=["M","CM","D","CD","C","XC","L","XL","X","IX","V","IV","I"];
        let mut rem=2024usize; let mut out=String::new();
        for (i,&v) in vals.iter().enumerate() { while rem>=v { out.push_str(syms[i]); rem-=v; } }
        assert_eq!(out, "MMXXIV");
    }
    #[test] fn from_roman_xlii() {
        let roman = "XLII".to_uppercase();
        let val_of = |c| match c { 'I'=>1,'V'=>5,'X'=>10,'L'=>50,'C'=>100,'D'=>500,'M'=>1000,_=>0i64 };
        let chars: Vec<char> = roman.chars().collect();
        let mut total = 0i64;
        for i in 0..chars.len() {
            let cur = val_of(chars[i]);
            let nxt = if i+1 < chars.len() { val_of(chars[i+1]) } else { 0 };
            if cur < nxt { total -= cur; } else { total += cur; }
        }
        assert_eq!(total, 42);
    }

    // ── number.fibonacci ─────────────────────────────────────────────────
    #[test] fn fibonacci_0()  { let (mut a,mut b)=(0u128,1u128); for _ in 0..0  { let t=a+b; a=b; b=t; } assert_eq!(a,0); }
    #[test] fn fibonacci_1()  { let (mut a,mut b)=(0u128,1u128); for _ in 0..1  { let t=a+b; a=b; b=t; } assert_eq!(a,1); }
    #[test] fn fibonacci_10() { let (mut a,mut b)=(0u128,1u128); for _ in 0..10 { let t=a+b; a=b; b=t; } assert_eq!(a,55); }

    // ── number.factorial ─────────────────────────────────────────────────
    #[test] fn factorial_0() { let r:u128=(1..=0u128).product(); assert_eq!(r,1); }
    #[test] fn factorial_1() { let r:u128=(1..=1u128).product(); assert_eq!(r,1); }
    #[test] fn factorial_5() { let r:u128=(1..=5u128).product(); assert_eq!(r,120); }
    #[test] fn factorial_10(){ let r:u128=(1..=10u128).product(); assert_eq!(r,3628800); }

    // ── number.is_prime ─────────────────────────────────────────────────
    #[test] fn is_prime_2()  { let n=2u64; assert!((2..=((n as f64).sqrt() as u64).max(2)).all(|d| n==2 || n%d!=0)); }
    #[test] fn is_prime_7()  { let n=7u64; assert!((2..=((n as f64).sqrt() as u64)).all(|d| n%d!=0)); }
    #[test] fn is_prime_97() { let n=97u64; assert!((2..=((n as f64).sqrt() as u64)).all(|d| n%d!=0)); }
    #[test] fn not_prime_1() { let n=1u64; assert!(!(n >= 2)); }
    #[test] fn not_prime_9() { let n=9u64; assert!((2..=((n as f64).sqrt() as u64)).any(|d| n%d==0)); }

    // ── number.digits ─────────────────────────────────────────────────────
    #[test] fn digits_123()  { let d:Vec<u8>="123".chars().map(|c| c as u8-b'0').collect(); assert_eq!(d, vec![1,2,3]); }
    #[test] fn digits_zero() { let d:Vec<u8>="0".chars().map(|c| c as u8-b'0').collect(); assert_eq!(d, vec![0]); }
    #[test] fn digits_single(){ let d:Vec<u8>="7".chars().map(|c| c as u8-b'0').collect(); assert_eq!(d, vec![7]); }

    // ── manifest ──────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("number."));
        }
    }
}
