//! 批量编译OpenClaw技能为WASM
//! 
//! 使用方法: cargo run --bin compile_skills_to_wasm

use anyhow::{Context, Result};
use a3office_wasm_compiler::{SkillWasmCompiler, CompilationResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn, error};
use tracing_subscriber;

/// 核心技能定义 - 选择第一批WASM化的技能
fn get_core_skills() -> HashMap<String, String> {
    let mut skills = HashMap::new();

    // Hash技能 (10个)
    skills.insert("hash.sha256".to_string(), r#"
pub fn sha256(text: String) -> Result<String, String> {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}
    "#.to_string());

    skills.insert("hash.md5".to_string(), r#"
pub fn md5(text: String) -> Result<String, String> {
    use md5::Md5;
    let mut hasher = Md5::new();
    hasher.update(text.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}
    "#.to_string());

    // Encode技能 (8个)
    skills.insert("encode.base64".to_string(), r#"
pub fn encode_base64(text: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose};
    Ok(general_purpose::STANDARD.encode(text.as_bytes()))
}
    "#.to_string());

    skills.insert("encode.hex".to_string(), r#"
pub fn encode_hex(text: String) -> Result<String, String> {
    Ok(hex::encode(text.as_bytes()))
}
    "#.to_string());

    // Math技能 (10个)
    skills.insert("math.add".to_string(), r#"
pub fn add(numbers: Vec<f64>) -> Result<f64, String> {
    Ok(numbers.iter().sum())
}
    "#.to_string());

    skills.insert("math.multiply".to_string(), r#"
pub fn multiply(numbers: Vec<f64>) -> Result<f64, String> {
    let mut result = 1.0;
    for num in numbers {
        result *= num;
    }
    Ok(result)
}
    "#.to_string());

    skills.insert("math.factorial".to_string(), r#"
pub fn factorial(n: u64) -> Result<u64, String> {
    if n > 20 {
        return Err("输入太大，可能导致溢出".to_string());
    }
    let mut result = 1u64;
    for i in 2..=n {
        result *= i;
    }
    Ok(result)
}
    "#.to_string());

    // Stat技能 (6个)
    skills.insert("stat.mean".to_string(), r#"
pub fn mean(numbers: Vec<f64>) -> Result<f64, String> {
    if numbers.is_empty() {
        return Err("输入不能为空".to_string());
    }
    let sum: f64 = numbers.iter().sum();
    Ok(sum / numbers.len() as f64)
}
    "#.to_string());

    skills.insert("stat.median".to_string(), r#"
pub fn median(mut numbers: Vec<f64>) -> Result<f64, String> {
    if numbers.is_empty() {
        return Err("输入不能为空".to_string());
    }
    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = numbers.len();
    if len % 2 == 0 {
        Ok((numbers[len/2 - 1] + numbers[len/2]) / 2.0)
    } else {
        Ok(numbers[len/2])
    }
}
    "#.to_string());

    // Time技能 (4个)
    skills.insert("time.now".to_string(), r#"
pub fn now(format: String) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now();
    match format.as_str() {
        "unix" => Ok(now.duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()),
        "iso" => Ok(chrono::Utc::now().to_rfc3339()),
        "ms" => Ok(now.duration_since(UNIX_EPOCH).unwrap().as_millis().to_string()),
        _ => Err("不支持的格式".to_string()),
    }
}
    "#.to_string());

    // URL技能 (6个)
    skills.insert("url.parse".to_string(), r#"
pub fn parse_url(url: String) -> Result<serde_json::Value, String> {
    use url::Url;
    match Url::parse(&url) {
        Ok(parsed) => Ok(json!({
            "scheme": parsed.scheme(),
            "host": parsed.host_str(),
            "port": parsed.port(),
            "path": parsed.path(),
            "query": parsed.query(),
            "fragment": parsed.fragment()
        })),
        Err(e) => Err(format!("URL解析失败: {}", e))
    }
}
    "#.to_string());

    skills.insert("url.encode".to_string(), r#"
pub fn encode_url(text: String) -> Result<String, String> {
    Ok(percent_encoding::utf8_percent_encode(&text, percent_encoding::NON_ALPHANUMERIC).to_string())
}
    "#.to_string());

    // IP技能 (4个)
    skills.insert("ip.is_valid".to_string(), r#"
pub fn is_valid_ip(ip: String) -> Result<bool, String> {
    Ok(ip.parse::<std::net::IpAddr>().is_ok())
}
    "#.to_string());

    skills.insert("ip.is_private".to_string(), r#"
pub fn is_private_ip(ip: String) -> Result<bool, String> {
    match ip.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(addr)) => Ok(addr.is_private()),
        Ok(std::net::IpAddr::V6(addr)) => Ok(addr.segments()[0] == 0xfc00 || addr.segments()[0] == 0xfe80),
        Err(_) => Err("无效的IP地址".to_string())
    }
}
    "#.to_string());

    // UUID技能 (3个)
    skills.insert("uuid.v4".to_string(), r#"
pub fn generate_uuid_v4() -> Result<String, String> {
    Ok(uuid::Uuid::new_v4().to_string())
}
    "#.to_string());

    // Random技能 (4个)
    skills.insert("random.int".to_string(), r#"
pub fn random_int(min: i32, max: i32) -> Result<i32, String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    Ok(rng.gen_range(min..=max))
}
    "#.to_string());

    skills.insert("random.string".to_string(), r#"
pub fn random_string(length: usize) -> Result<String, String> {
    use rand::{Rng, distributions::Alphanumeric};
    let mut rng = rand::thread_rng();
    let result: String = (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect();
    Ok(result)
}
    "#.to_string());

    // Text技能 (6个)
    skills.insert("text.upper".to_string(), r#"
pub fn to_upper(text: String) -> Result<String, String> {
    Ok(text.to_uppercase())
}
    "#.to_string());

    skills.insert("text.lower".to_string(), r#"
pub fn to_lower(text: String) -> Result<String, String> {
    Ok(text.to_lowercase())
}
    "#.to_string());

    skills.insert("text.replace".to_string(), r#"
pub fn replace_text(text: String, from: String, to: String) -> Result<String, String> {
    Ok(text.replace(&from, &to))
}
    "#.to_string());

    // Logic技能 (6个)
    skills.insert("logic.and".to_string(), r#"
pub fn logical_and(values: Vec<bool>) -> Result<bool, String> {
    Ok(values.iter().all(|&v| v))
}
    "#.to_string());

    skills.insert("logic.or".to_string(), r#"
pub fn logical_or(values: Vec<bool>) -> Result<bool, String> {
    Ok(values.iter().any(|&v| v))
}
    "#.to_string());

    skills.insert("logic.not".to_string(), r#"
pub fn logical_not(value: bool) -> Result<bool, String> {
    Ok(!value)
}
    "#.to_string());

    // AI技能 (4个)
    skills.insert("ai.levenshtein".to_string(), r#"
pub fn levenshtein_distance(a: String, b: String) -> Result<usize, String> {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    
    if m == 0 { return Ok(n); }
    if n == 0 { return Ok(m); }
    
    let mut matrix = vec![vec![0; n + 1]; m + 1];
    
    for i in 0..=m {
        matrix[i][0] = i;
    }
    for j in 0..=n {
        matrix[0][j] = j;
    }
    
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i-1] == b_chars[j-1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i-1][j] + 1,      // deletion
                    matrix[i][j-1] + 1       // insertion
                ),
                matrix[i-1][j-1] + cost    // substitution
            );
        }
    }
    
    Ok(matrix[m][n])
}
    "#.to_string());

    skills.insert("ai.cosine_similarity".to_string(), r#"
pub fn cosine_similarity(a: Vec<f64>, b: Vec<f64>) -> Result<f64, String> {
    if a.len() != b.len() {
        return Err("向量长度必须相同".to_string());
    }
    
    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return Err("向量不能为零向量".to_string());
    }
    
    Ok(dot_product / (norm_a * norm_b))
}
    "#.to_string());

    skills
}

/// 保存WASM技能到文件
async fn save_wasm_skill(skill_name: &str, wasm_binary: Vec<u8>) -> Result<()> {
    let wasm_dir = Path::new("assets/wasm-skills");
    fs::create_dir_all(wasm_dir).context("创建WASM技能目录失败")?;

    let file_path = wasm_dir.join(format!("{}.wasm", skill_name.replace('.', "_")));
    fs::write(&file_path, wasm_binary).context("写入WASM文件失败")?;

    info!("💾 WASM技能已保存: {}", file_path.display());
    Ok(())
}

/// 生成编译报告
fn generate_compilation_report(results: &HashMap<String, CompilationResult>) -> Result<()> {
    let mut successful = Vec::new();
    let mut failed = Vec::new();
    let mut total_size = 0;

    for (skill_name, result) in results {
        match result {
            CompilationResult::Success { size_bytes, .. } => {
                successful.push(skill_name.clone());
                total_size += size_bytes;
            }
            CompilationResult::Error { .. } => {
                failed.push(skill_name.clone());
            }
        }
    }

    let report = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "total_skills": results.len(),
            "successful": successful.len(),
            "failed": failed.len(),
            "success_rate": format!("{:.1}%", (successful.len() as f64 / results.len() as f64) * 100.0),
            "total_wasm_size_bytes": total_size,
            "total_wasm_size_mb": format!("{:.2}", total_size as f64 / 1024.0 / 1024.0)
        },
        "successful_skills": successful,
        "failed_skills": failed,
        "performance_metrics": {
            "avg_skill_size_bytes": if successful.len() > 0 { total_size / successful.len() } else { 0 },
            "estimated_load_time_ms": total_size / 1024, // 简化估算
            "memory_usage_estimate_mb": format!("{:.2}", total_size as f64 / 1024.0 / 1024.0 * 2.0) // 2倍内存使用估算
        }
    });

    let report_path = "assets/wasm-compilation-report.json";
    fs::write(report_path, serde_json::to_string_pretty(&report)?)?;

    info!("📊 编译报告已生成: {}", report_path);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 开始批量编译OpenClaw技能为WASM...");

    // 创建WASM编译器
    let compiler = SkillWasmCompiler::new().context("创建WASM编译器失败")?;

    // 获取核心技能
    let skills = get_core_skills();
    info!("📋 准备编译 {} 个核心技能", skills.len());

    // 批量编译
    let results = compiler.compile_skills_batch(skills).await
        .context("批量编译失败")?;

    // 保存成功的WASM技能
    let mut saved_count = 0;
    for (skill_name, result) in &results {
        if let CompilationResult::Success { wasm_binary, .. } = result {
            match save_wasm_skill(skill_name, wasm_binary.clone()).await {
                Ok(_) => {
                    saved_count += 1;
                }
                Err(e) => {
                    warn!("保存技能 {} 失败: {}", skill_name, e);
                }
            }
        }
    }

    // 生成编译报告
    generate_compilation_report(&results)?;

    // 输出结果统计
    let successful = results.values().filter(|r| r.is_success()).count();
    let failed = results.values().filter(|r| r.is_error()).count();

    println!("\n🎉 WASM编译完成!");
    println!("📊 统计信息:");
    println!("   ✅ 成功: {} 个技能", successful);
    println!("   ❌ 失败: {} 个技能", failed);
    println!("   💾 已保存: {} 个技能", saved_count);
    println!("   📈 成功率: {:.1}%", (successful as f64 / results.len() as f64) * 100.0);

    if failed > 0 {
        println!("\n⚠️  失败的技能:");
        for (skill_name, result) in &results {
            if let CompilationResult::Error { error } = result {
                println!("   - {}: {}", skill_name, error);
            }
        }
    }

    println!("\n📁 WASM文件位置: assets/wasm-skills/");
    println!("📊 编译报告: assets/wasm-compilation-report.json");

    Ok(())
}
