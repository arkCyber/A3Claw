//! skill-matrix — 2D matrix arithmetic skills (pure-Rust, no I/O).

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "openclaw.matrix",
  "name": "Matrix Skills",
  "version": "0.1.0",
  "description": "2D matrix add, multiply, transpose, determinant, dot product",
  "skills": [
    {
      "name": "matrix.add",
      "display": "Matrix Add",
      "description": "Element-wise addition of two matrices.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "array", "required": true },
        { "name": "b", "type": "array", "required": true }
      ]
    },
    {
      "name": "matrix.multiply",
      "display": "Matrix Multiply",
      "description": "Standard matrix multiplication (A x B).",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "array", "required": true },
        { "name": "b", "type": "array", "required": true }
      ]
    },
    {
      "name": "matrix.transpose",
      "display": "Matrix Transpose",
      "description": "Return the transpose of a matrix.",
      "risk": "safe",
      "params": [{ "name": "a", "type": "array", "required": true }]
    },
    {
      "name": "matrix.det",
      "display": "Matrix Determinant",
      "description": "Compute the determinant of a 2x2 or 3x3 square matrix.",
      "risk": "safe",
      "params": [{ "name": "a", "type": "array", "required": true }]
    },
    {
      "name": "matrix.scale",
      "display": "Matrix Scale",
      "description": "Multiply every element by a scalar value.",
      "risk": "safe",
      "params": [
        { "name": "a",      "type": "array",  "required": true },
        { "name": "scalar", "type": "number", "required": true }
      ]
    },
    {
      "name": "matrix.dot",
      "display": "Dot Product",
      "description": "Compute the dot product of two 1D vectors.",
      "risk": "safe",
      "params": [
        { "name": "a", "type": "array", "required": true },
        { "name": "b", "type": "array", "required": true }
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
        "matrix.add" => {
            let a = parse_matrix(&req.args["a"]);
            let b = parse_matrix(&req.args["b"]);
            match mat_add(&a, &b) {
                Ok(m)  => sdk_respond_ok(rid, &serde_json::to_string(&m).unwrap()),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "matrix.multiply" => {
            let a = parse_matrix(&req.args["a"]);
            let b = parse_matrix(&req.args["b"]);
            match mat_mul(&a, &b) {
                Ok(m)  => sdk_respond_ok(rid, &serde_json::to_string(&m).unwrap()),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "matrix.transpose" => {
            let a = parse_matrix(&req.args["a"]);
            sdk_respond_ok(rid, &serde_json::to_string(&transpose(&a)).unwrap())
        }
        "matrix.det" => {
            let a = parse_matrix(&req.args["a"]);
            match determinant(&a) {
                Ok(d)  => sdk_respond_ok(rid, &format!("{}", d)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        "matrix.scale" => {
            let a = parse_matrix(&req.args["a"]);
            let s = match req.args["scalar"].as_f64() { Some(v) => v, None => return sdk_respond_err(rid, "missing 'scalar'") };
            let scaled: Vec<Vec<f64>> = a.iter().map(|row| row.iter().map(|&x| x * s).collect()).collect();
            sdk_respond_ok(rid, &serde_json::to_string(&scaled).unwrap())
        }
        "matrix.dot" => {
            let a = parse_vec(&req.args["a"]);
            let b = parse_vec(&req.args["b"]);
            match dot(&a, &b) {
                Ok(d)  => sdk_respond_ok(rid, &format!("{}", d)),
                Err(e) => sdk_respond_err(rid, &e),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

// ── Matrix logic ──────────────────────────────────────────────────────────────

fn parse_matrix(v: &serde_json::Value) -> Vec<Vec<f64>> {
    v.as_array().map(|rows| {
        rows.iter().map(|row| {
            row.as_array().map(|cells| cells.iter().filter_map(|c| c.as_f64()).collect()).unwrap_or_default()
        }).collect()
    }).unwrap_or_default()
}

fn parse_vec(v: &serde_json::Value) -> Vec<f64> {
    v.as_array().map(|arr| arr.iter().filter_map(|c| c.as_f64()).collect()).unwrap_or_default()
}

fn mat_add(a: &[Vec<f64>], b: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if a.len() != b.len() { return Err("row count mismatch".to_string()); }
    a.iter().zip(b.iter()).map(|(ra, rb)| {
        if ra.len() != rb.len() { return Err("col count mismatch".to_string()); }
        Ok(ra.iter().zip(rb.iter()).map(|(x, y)| x + y).collect())
    }).collect()
}

fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if a.is_empty() || b.is_empty() { return Ok(vec![]); }
    let cols_a = a[0].len();
    let rows_b = b.len();
    if cols_a != rows_b { return Err(format!("dimension mismatch: A cols={cols_a}, B rows={rows_b}")); }
    let cols_b = b[0].len();
    Ok((0..a.len()).map(|i| {
        (0..cols_b).map(|j| {
            (0..cols_a).map(|k| a[i][k] * b[k][j]).sum()
        }).collect()
    }).collect())
}

fn transpose(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if a.is_empty() || a[0].is_empty() { return vec![]; }
    let rows = a.len(); let cols = a[0].len();
    (0..cols).map(|j| (0..rows).map(|i| a[i][j]).collect()).collect()
}

fn determinant(a: &[Vec<f64>]) -> Result<f64, String> {
    let n = a.len();
    if a.iter().any(|r| r.len() != n) { return Err("matrix must be square".to_string()); }
    match n {
        1 => Ok(a[0][0]),
        2 => Ok(a[0][0]*a[1][1] - a[0][1]*a[1][0]),
        3 => Ok(
            a[0][0]*(a[1][1]*a[2][2] - a[1][2]*a[2][1])
          - a[0][1]*(a[1][0]*a[2][2] - a[1][2]*a[2][0])
          + a[0][2]*(a[1][0]*a[2][1] - a[1][1]*a[2][0])
        ),
        _ => Err(format!("determinant only supported for 1x1, 2x2, 3x3, got {}x{}", n, n)),
    }
}

fn dot(a: &[f64], b: &[f64]) -> Result<f64, String> {
    if a.len() != b.len() { return Err(format!("length mismatch: {} vs {}", a.len(), b.len())); }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mat_add_basic() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        let r = mat_add(&a, &b).unwrap();
        assert_eq!(r, vec![vec![6.0, 8.0], vec![10.0, 12.0]]);
    }
    #[test]
    fn mat_add_mismatch() {
        let a = vec![vec![1.0]];
        let b = vec![vec![1.0], vec![2.0]];
        assert!(mat_add(&a, &b).is_err());
    }
    #[test]
    fn mat_mul_basic() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let r = mat_mul(&a, &b).unwrap();
        assert_eq!(r, a);
    }
    #[test]
    fn mat_mul_mismatch() {
        let a = vec![vec![1.0, 2.0]];
        let b = vec![vec![1.0]];
        assert!(mat_mul(&a, &b).is_err());
    }
    #[test]
    fn transpose_basic() {
        let a = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let t = transpose(&a);
        assert_eq!(t, vec![vec![1.0, 4.0], vec![2.0, 5.0], vec![3.0, 6.0]]);
    }
    #[test]
    fn transpose_square() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let t = transpose(&a);
        assert_eq!(t, vec![vec![1.0, 3.0], vec![2.0, 4.0]]);
    }
    #[test]
    fn det_2x2() {
        let a = vec![vec![3.0, 8.0], vec![4.0, 6.0]];
        assert!((determinant(&a).unwrap() - (-14.0)).abs() < 1e-9);
    }
    #[test]
    fn det_3x3() {
        let a = vec![vec![6.0,1.0,1.0], vec![4.0,-2.0,5.0], vec![2.0,8.0,7.0]];
        assert!((determinant(&a).unwrap() - (-306.0)).abs() < 1e-6);
    }
    #[test]
    fn det_non_square() {
        let a = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        assert!(determinant(&a).is_err());
    }
    #[test]
    fn dot_basic() {
        assert!((dot(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]).unwrap() - 32.0).abs() < 1e-9);
    }
    #[test]
    fn dot_mismatch() {
        assert!(dot(&[1.0, 2.0], &[1.0]).is_err());
    }
    #[test]
    fn scale_basic() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let s: Vec<Vec<f64>> = a.iter().map(|r| r.iter().map(|&x| x * 2.0).collect()).collect();
        assert_eq!(s, vec![vec![2.0, 4.0], vec![6.0, 8.0]]);
    }
    #[test]
    fn manifest_valid() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["id"], "openclaw.matrix");
        assert_eq!(v["skills"].as_array().unwrap().len(), 6);
    }
    #[test]
    fn manifest_skill_names_prefix() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("matrix."));
        }
    }
    #[test]
    fn all_skills_have_risk() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() { assert!(s["risk"].is_string()); }
    }
}
