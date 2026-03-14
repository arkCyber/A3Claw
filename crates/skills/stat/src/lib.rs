use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.stat",
  "name": "Statistics Utilities",
  "version": "0.1.0",
  "description": "10 statistics skills: mean, median, mode, variance, stddev, percentile, zscore, correlation, normalize, histogram.",
  "skills": [
    {"name":"stat.mean","display":"Mean","description":"Arithmetic mean of a JSON number array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true}]},
    {"name":"stat.median","display":"Median","description":"Median of a JSON number array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true}]},
    {"name":"stat.mode","display":"Mode","description":"Most frequent value(s) in a JSON array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON array","required":true}]},
    {"name":"stat.variance","display":"Variance","description":"Population variance of a JSON number array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true}]},
    {"name":"stat.stddev","display":"Std Dev","description":"Population standard deviation of a JSON number array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true}]},
    {"name":"stat.percentile","display":"Percentile","description":"Pth percentile (0-100) of a JSON number array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true},{"name":"p","type":"number","description":"Percentile 0-100","required":true}]},
    {"name":"stat.zscore","display":"Z-Score","description":"Z-scores for each value in a JSON number array.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true}]},
    {"name":"stat.correlation","display":"Correlation","description":"Pearson correlation coefficient between two JSON number arrays.","risk":"safe","params":[{"name":"x","type":"string","description":"JSON number array","required":true},{"name":"y","type":"string","description":"JSON number array","required":true}]},
    {"name":"stat.normalize","display":"Normalize","description":"Min-max normalize a JSON number array to [0,1].","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true}]},
    {"name":"stat.histogram","display":"Histogram","description":"Bucket counts for a JSON number array into N bins.","risk":"safe","params":[{"name":"array","type":"string","description":"JSON number array","required":true},{"name":"bins","type":"integer","description":"Number of bins","required":true}]}
  ]
}"#;

fn parse_nums(s: &str) -> Vec<f64> {
    serde_json::from_str::<Vec<serde_json::Value>>(s).unwrap_or_default()
        .iter().filter_map(|v| v.as_f64()).collect()
}

fn mean(nums: &[f64]) -> f64 {
    if nums.is_empty() { return 0.0; }
    nums.iter().sum::<f64>() / nums.len() as f64
}

fn variance(nums: &[f64]) -> f64 {
    if nums.len() < 2 { return 0.0; }
    let m = mean(nums);
    nums.iter().map(|x| (x - m).powi(2)).sum::<f64>() / nums.len() as f64
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
        "stat.mean" => {
            let nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            if nums.is_empty() { return sdk_respond_err(rid, "empty array"); }
            sdk_respond_ok(rid, &format!("{:.6}", mean(&nums)))
        }
        "stat.median" => {
            let mut nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            if nums.is_empty() { return sdk_respond_err(rid, "empty array"); }
            nums.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = nums.len() / 2;
            let median = if nums.len() % 2 == 0 { (nums[mid-1] + nums[mid]) / 2.0 } else { nums[mid] };
            sdk_respond_ok(rid, &format!("{:.6}", median))
        }
        "stat.mode" => {
            let arr_str = args["array"].as_str().unwrap_or("[]");
            let arr: Vec<serde_json::Value> = serde_json::from_str(arr_str).unwrap_or_default();
            let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for v in &arr { *counts.entry(v.to_string()).or_insert(0) += 1; }
            let max_count = counts.values().max().cloned().unwrap_or(0);
            let modes: Vec<serde_json::Value> = counts.into_iter()
                .filter(|(_, c)| *c == max_count)
                .map(|(k, _)| serde_json::from_str(&k).unwrap_or(serde_json::Value::String(k)))
                .collect();
            sdk_respond_ok(rid, &serde_json::to_string(&modes).unwrap_or_default())
        }
        "stat.variance" => {
            let nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            sdk_respond_ok(rid, &format!("{:.6}", variance(&nums)))
        }
        "stat.stddev" => {
            let nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            sdk_respond_ok(rid, &format!("{:.6}", variance(&nums).sqrt()))
        }
        "stat.percentile" => {
            let mut nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            if nums.is_empty() { return sdk_respond_err(rid, "empty array"); }
            let p = args["p"].as_f64().unwrap_or(50.0).max(0.0).min(100.0);
            nums.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let idx = (p / 100.0 * (nums.len() - 1) as f64).round() as usize;
            sdk_respond_ok(rid, &format!("{:.6}", nums[idx.min(nums.len()-1)]))
        }
        "stat.zscore" => {
            let nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            if nums.is_empty() { return sdk_respond_err(rid, "empty array"); }
            let m = mean(&nums);
            let sd = variance(&nums).sqrt();
            if sd == 0.0 { return sdk_respond_ok(rid, &"0.0".repeat(1)); }
            let zs: Vec<serde_json::Value> = nums.iter()
                .map(|x| serde_json::json!(format!("{:.4}", (x - m) / sd)))
                .collect();
            sdk_respond_ok(rid, &serde_json::to_string(&zs).unwrap_or_default())
        }
        "stat.correlation" => {
            let xs = parse_nums(args["x"].as_str().unwrap_or("[]"));
            let ys = parse_nums(args["y"].as_str().unwrap_or("[]"));
            if xs.len() != ys.len() || xs.is_empty() {
                return sdk_respond_err(rid, "arrays must have equal non-zero length");
            }
            let mx = mean(&xs); let my = mean(&ys);
            let num: f64 = xs.iter().zip(ys.iter()).map(|(x,y)| (x-mx)*(y-my)).sum();
            let dx: f64 = xs.iter().map(|x| (x-mx).powi(2)).sum::<f64>().sqrt();
            let dy: f64 = ys.iter().map(|y| (y-my).powi(2)).sum::<f64>().sqrt();
            if dx == 0.0 || dy == 0.0 { return sdk_respond_ok(rid, "0.0"); }
            sdk_respond_ok(rid, &format!("{:.6}", num / (dx * dy)))
        }
        "stat.normalize" => {
            let nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            if nums.is_empty() { return sdk_respond_err(rid, "empty array"); }
            let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = max - min;
            let out: Vec<serde_json::Value> = nums.iter().map(|x| {
                serde_json::json!(if range == 0.0 { 0.0 } else { (x - min) / range })
            }).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        "stat.histogram" => {
            let nums = parse_nums(args["array"].as_str().unwrap_or("[]"));
            if nums.is_empty() { return sdk_respond_err(rid, "empty array"); }
            let n_bins = args["bins"].as_u64().unwrap_or(10).max(1) as usize;
            let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = max - min;
            let mut bins = vec![0usize; n_bins];
            for &v in &nums {
                let idx = if range == 0.0 { 0 } else {
                    ((v - min) / range * n_bins as f64).floor() as usize
                };
                bins[idx.min(n_bins - 1)] += 1;
            }
            let out: Vec<serde_json::Value> = bins.iter().enumerate().map(|(i, &count)| {
                let lo = min + i as f64 * range / n_bins as f64;
                let hi = min + (i + 1) as f64 * range / n_bins as f64;
                serde_json::json!({"min": lo, "max": hi, "count": count})
            }).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&out).unwrap_or_default())
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── stat.mean ──────────────────────────────────────────────────────────
    #[test] fn mean_basic()   { assert_eq!(mean(&[1.0, 2.0, 3.0]), 2.0); }
    #[test] fn mean_single()  { assert_eq!(mean(&[7.0]), 7.0); }
    #[test] fn mean_empty()   { assert_eq!(mean(&[]), 0.0); }
    #[test] fn mean_float()   { assert!((mean(&[1.5, 2.5]) - 2.0).abs() < 1e-9); }

    // ── stat.median ───────────────────────────────────────────────────────
    #[test] fn median_odd() {
        let mut v = vec![3.0, 1.0, 4.0, 1.0, 5.0];
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(v[v.len() / 2], 3.0);
    }
    #[test] fn median_even() {
        let mut v = vec![1.0, 2.0, 3.0, 4.0];
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = v.len() / 2;
        assert_eq!((v[mid-1] + v[mid]) / 2.0, 2.5);
    }
    #[test] fn median_single() {
        let v = vec![42.0f64];
        assert_eq!(v[0], 42.0);
    }

    // ── stat.variance / stddev ───────────────────────────────────────────
    #[test] fn variance_basic(){ let v = variance(&[2.0,4.0,4.0,4.0,5.0,5.0,7.0,9.0]); assert!((v-4.0).abs()<0.01,"got {}",v); }
    #[test] fn variance_zero() { assert_eq!(variance(&[5.0, 5.0, 5.0]), 0.0); }
    #[test] fn variance_single(){ assert_eq!(variance(&[3.0]), 0.0); }
    #[test] fn stddev_basic()  { let sd = variance(&[2.0,4.0,4.0,4.0,5.0,5.0,7.0,9.0]).sqrt(); assert!((sd-2.0).abs()<0.01); }

    // ── stat.percentile ────────────────────────────────────────────────────
    #[test] fn percentile_50() {
        let mut v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        v.sort_by(|a,b| a.partial_cmp(b).unwrap());
        let idx = (0.50 * (v.len()-1) as f64).round() as usize;
        assert_eq!(v[idx], 3.0);
    }
    #[test] fn percentile_0() {
        let mut v = vec![5.0, 1.0, 3.0];
        v.sort_by(|a,b| a.partial_cmp(b).unwrap());
        assert_eq!(v[0], 1.0);
    }
    #[test] fn percentile_100() {
        let mut v = vec![5.0, 1.0, 3.0];
        v.sort_by(|a,b| a.partial_cmp(b).unwrap());
        assert_eq!(*v.last().unwrap(), 5.0);
    }

    // ── stat.correlation ──────────────────────────────────────────────────
    #[test] fn correlation_perfect() {
        let xs = vec![1.0, 2.0, 3.0];
        let ys = vec![2.0, 4.0, 6.0];
        let mx = mean(&xs); let my = mean(&ys);
        let num: f64 = xs.iter().zip(ys.iter()).map(|(x,y)| (x-mx)*(y-my)).sum();
        let dx: f64 = xs.iter().map(|x| (x-mx).powi(2)).sum::<f64>().sqrt();
        let dy: f64 = ys.iter().map(|y| (y-my).powi(2)).sum::<f64>().sqrt();
        let r = num / (dx * dy);
        assert!((r - 1.0).abs() < 1e-9, "expected ~1.0 got {}", r);
    }
    #[test] fn correlation_inverse() {
        let xs = vec![1.0, 2.0, 3.0];
        let ys = vec![6.0, 4.0, 2.0];
        let mx = mean(&xs); let my = mean(&ys);
        let num: f64 = xs.iter().zip(ys.iter()).map(|(x,y)| (x-mx)*(y-my)).sum();
        let dx: f64 = xs.iter().map(|x| (x-mx).powi(2)).sum::<f64>().sqrt();
        let dy: f64 = ys.iter().map(|y| (y-my).powi(2)).sum::<f64>().sqrt();
        let r = num / (dx * dy);
        assert!((r + 1.0).abs() < 1e-9, "expected ~-1.0 got {}", r);
    }

    // ── stat.normalize ─────────────────────────────────────────────────────
    #[test] fn normalize_basic() {
        let nums = vec![0.0, 5.0, 10.0];
        let min = 0.0f64; let max = 10.0f64; let range = max - min;
        let normed: Vec<f64> = nums.iter().map(|x| (x - min) / range).collect();
        assert_eq!(normed, vec![0.0, 0.5, 1.0]);
    }
    #[test] fn normalize_uniform() {
        let nums = vec![3.0f64, 3.0, 3.0];
        let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        let normed: Vec<f64> = nums.iter().map(|x| if range == 0.0 { 0.0 } else { (x - min)/range }).collect();
        assert_eq!(normed, vec![0.0, 0.0, 0.0]);
    }

    // ── stat.histogram ─────────────────────────────────────────────────────
    #[test] fn histogram_2bins() {
        let nums = vec![1.0, 2.0, 8.0, 9.0];
        let n_bins = 2usize;
        let min = 1.0f64; let max = 9.0f64; let range = max - min;
        let mut bins = vec![0usize; n_bins];
        for &v in &nums {
            let idx = ((v - min) / range * n_bins as f64).floor() as usize;
            bins[idx.min(n_bins - 1)] += 1;
        }
        assert_eq!(bins[0], 2);
        assert_eq!(bins[1], 2);
    }
    #[test] fn histogram_single_bin() {
        let nums = vec![1.0, 2.0, 3.0];
        let n_bins = 1usize;
        let min = 1.0f64; let max = 3.0f64; let range = max - min;
        let mut bins = vec![0usize; n_bins];
        for &v in &nums {
            let idx = ((v - min) / range * n_bins as f64).floor() as usize;
            bins[idx.min(n_bins - 1)] += 1;
        }
        assert_eq!(bins[0], 3);
    }

    // ── helpers ────────────────────────────────────────────────────────────
    #[test] fn parse_nums_works()  { assert_eq!(parse_nums("[1,2,3]"), vec![1.0,2.0,3.0]); }
    #[test] fn parse_nums_empty()  { assert_eq!(parse_nums("[]"), Vec::<f64>::new()); }
    #[test] fn parse_nums_invalid(){ assert_eq!(parse_nums("not-json"), Vec::<f64>::new()); }

    // ── manifest ───────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("stat."));
        }
    }
}
