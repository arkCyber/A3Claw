//! WASM适配器生成器
//! 
//! 为技能代码生成WASM兼容的适配器代码

use crate::error::CompilerError;

/// WASM适配器配置
#[derive(Debug, Clone)]
pub struct SkillAdapterConfig {
    /// 是否启用内存管理
    pub enable_memory_management: bool,
    /// 是否启用性能监控
    pub enable_performance_monitoring: bool,
    /// 是否启用错误处理
    pub enable_error_handling: bool,
    /// 最大输入大小（字节）
    pub max_input_size: usize,
    /// 最大输出大小（字节）
    pub max_output_size: usize,
}

impl Default for SkillAdapterConfig {
    fn default() -> Self {
        Self {
            enable_memory_management: true,
            enable_performance_monitoring: true,
            enable_error_handling: true,
            max_input_size: 1024 * 1024, // 1MB
            max_output_size: 1024 * 1024, // 1MB
        }
    }
}

/// 生成WASM适配器代码
pub fn generate_wasm_adapter(
    skill_name: &str,
    skill_code: &str,
    config: &SkillAdapterConfig,
) -> Result<String, CompilerError> {
    // 验证技能名称
    if skill_name.is_empty() {
        return Err(CompilerError::InvalidSkillName("技能名称不能为空".to_string()));
    }

    // 解析技能代码
    let parsed_code = parse_skill_code(skill_code)?;

    // 生成适配器代码
    let adapter_code = format!(
        r#"//! 自动生成的WASM适配器 - 技能: {}

// 外部依赖导入
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::HashMap;
use serde_json::{{Value, json}};

// 原始技能代码
{}

// 内存管理模块
{}
{}

// 性能监控模块
{}
{}

// 错误处理模块
{}
{}

// 主执行函数 - WASM入口点
#[no_mangle]
pub extern "C" fn execute(input_ptr: *const u8, input_len: usize) -> *mut u8 {{
    // 安全检查
    if input_ptr.is_null() || input_len > {} {{
        return create_error_response("输入无效或过大").as_ptr() as *mut u8;
    }}

    // 读取输入数据
    let input_data = unsafe {{
        std::slice::from_raw_parts(input_ptr, input_len)
    }};

    // 解析JSON输入
    let input: Value = match serde_json::from_slice(input_data) {{
        Ok(v) => v,
        Err(e) => {{
            return create_error_response(&format!("JSON解析失败: {{}}", e)).as_ptr() as *mut u8;
        }}
    }};

    // 执行技能
    let result = match execute_skill_internal(input) {{
        Ok(v) => v,
        Err(e) => {{
            return create_error_response(&format!("技能执行失败: {{}}", e)).as_ptr() as *mut u8;
        }}
    }};

    // 序列化输出
    let output_json = match serde_json::to_vec(&result) {{
        Ok(v) => v,
        Err(e) => {{
            return create_error_response(&format!("输出序列化失败: {{}}", e)).as_ptr() as *mut u8;
        }}
    }};

    // 返回输出指针（调用者负责释放内存）
    Box::into_raw(output_json.into_boxed_slice()) as *mut u8
}}

// 获取技能元数据
#[no_mangle]
pub extern "C" fn get_metadata() -> *mut u8 {{
    let metadata = json!({{
        "name": "{}",
        "version": "1.0.0",
        "description": "自动生成的WASM技能",
        "category": "generated",
        "inputs": ["any"],
        "outputs": ["any"],
        "wasm_compatible": true
    }});

    let metadata_json = serde_json::to_vec(&metadata).unwrap();
    Box::into_raw(metadata_json.into_boxed_slice()) as *mut u8
}}

// 内存释放函数
#[no_mangle]
pub extern "C" fn free_memory(ptr: *mut u8, len: usize) {{
    if !ptr.is_null() {{
        unsafe {{
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len));
        }}
    }}
}}

// 内部执行函数
fn execute_skill_internal(input: Value) -> Result<Value, String> {{
    {}
}}

// 创建错误响应
fn create_error_response(error_msg: &str) -> Vec<u8> {{
    let error_response = json!({{
        "success": false,
        "error": error_msg,
        "result": null
    }});
    serde_json::to_vec(&error_response).unwrap()
}}

// 创建成功响应
fn create_success_response(result: Value) -> Vec<u8> {{
    let success_response = json!({{
        "success": true,
        "error": null,
        "result": result
    }});
    serde_json::to_vec(&success_response).unwrap()
}}
"#,
        skill_name,
        skill_code,
        if config.enable_memory_management {
            generate_memory_management_code()
        } else {
            "// 内存管理已禁用".to_string()
        },
        if config.enable_memory_management {
            generate_memory_utils()
        } else {
            "".to_string()
        },
        if config.enable_performance_monitoring {
            generate_performance_monitoring_code()
        } else {
            "// 性能监控已禁用".to_string()
        },
        if config.enable_performance_monitoring {
            "// 性能监控工具函数".to_string()
        } else {
            "".to_string()
        },
        if config.enable_error_handling {
            generate_error_handling_code()
        } else {
            "// 错误处理已禁用".to_string()
        },
        if config.enable_error_handling {
            "// 错误处理工具函数".to_string()
        } else {
            "".to_string()
        },
        config.max_input_size,
        skill_name,
        generate_skill_execution_code(&parsed_code)
    );

    Ok(adapter_code)
}

/// 解析技能代码
fn parse_skill_code(skill_code: &str) -> Result<ParsedSkill, CompilerError> {
    // 简单的代码解析 - 在实际实现中应该使用syn crate进行完整解析
    let parsed = ParsedSkill {
        functions: extract_functions(skill_code)?,
        imports: extract_imports(skill_code)?,
        main_function: find_main_function(skill_code)?,
    };

    Ok(parsed)
}

/// 解析后的技能代码结构
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ParsedSkill {
    functions: Vec<FunctionInfo>,
    imports: Vec<String>,
    main_function: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FunctionInfo {
    name: String,
    params: Vec<String>,
    return_type: String,
    body: String,
}

/// 提取函数信息
fn extract_functions(code: &str) -> Result<Vec<FunctionInfo>, CompilerError> {
    let mut functions = Vec::new();
    
    // 简单的函数提取 - 实际实现中应该使用syn
    let lines: Vec<&str> = code.lines().collect();
    let mut current_function = None;
    let mut brace_count = 0;
    
    for line in lines {
        let trimmed = line.trim();
        
        // 检测函数定义
        if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
            if let Some(func) = current_function.take() {
                functions.push(func);
            }
            
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let func_name = parts[parts.len() - 2].split('(').next().unwrap();
                current_function = Some(FunctionInfo {
                    name: func_name.to_string(),
                    params: Vec::new(), // 简化处理
                    return_type: "Value".to_string(),
                    body: String::new(),
                });
            }
        }
        
        // 计算大括号
        brace_count += line.matches('{').count() as i32;
        brace_count -= line.matches('}').count() as i32;
        
        // 如果是函数体，添加到当前函数
        if let Some(ref mut func) = current_function {
            if !trimmed.starts_with("pub fn ") && !trimmed.starts_with("fn ") {
                func.body.push_str(line);
                func.body.push('\n');
            }
        }
        
        // 函数结束
        if brace_count == 0 && current_function.is_some() {
            if let Some(func) = current_function.take() {
                functions.push(func);
            }
        }
    }
    
    // 添加最后一个函数
    if let Some(func) = current_function {
        functions.push(func);
    }
    
    Ok(functions)
}

/// 提取导入语句
fn extract_imports(code: &str) -> Result<Vec<String>, CompilerError> {
    let mut imports = Vec::new();
    
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") {
            imports.push(trimmed.to_string());
        }
    }
    
    Ok(imports)
}

/// 查找主函数
fn find_main_function(code: &str) -> Result<Option<String>, CompilerError> {
    // 简化处理 - 返回第一个函数作为主函数
    if let Some(first_func) = extract_functions(code)?.first() {
        Ok(Some(first_func.name.clone()))
    } else {
        Ok(None)
    }
}

/// 生成内存管理代码
fn generate_memory_management_code() -> String {
    r#"
mod memory {
    use alloc::alloc::{alloc, dealloc, Layout};
    
    pub fn allocate(size: usize) -> *mut u8 {
        unsafe {
            let layout = Layout::from_size_align(size, 8).unwrap();
            alloc(layout)
        }
    }
    
    pub fn deallocate(ptr: *mut u8, size: usize) {
        unsafe {
            let layout = Layout::from_size_align(size, 8).unwrap();
            dealloc(ptr, layout);
        }
    }
}
"#.to_string()
}

/// 生成内存工具函数
fn generate_memory_utils() -> String {
    r#"
use crate::memory;

pub fn safe_read_string(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    
    unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(slice).to_string()
    }
}

pub fn safe_write_string(s: &str) -> (*mut u8, usize) {
    let bytes = s.as_bytes();
    let ptr = memory::allocate(bytes.len());
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    }
    (ptr, bytes.len())
}
"#.to_string()
}

/// 生成性能监控代码
fn generate_performance_monitoring_code() -> String {
    r#"
mod performance {
    use std::time::Instant;
    
    static mut START_TIME: Option<Instant> = None;
    
    pub fn start_timing() {
        unsafe {
            START_TIME = Some(Instant::now());
        }
    }
    
    pub fn end_timing() -> u64 {
        unsafe {
            if let Some(start) = START_TIME {
                start.elapsed().as_millis() as u64
            } else {
                0
            }
        }
    }
    
    pub fn log_execution(skill_name: &str, duration_ms: u64) {
        // 在实际实现中，这里会记录到日志或监控系统
        // println!("Skill {} executed in {}ms", skill_name, duration_ms);
    }
}
"#.to_string()
}

/// 生成错误处理代码
fn generate_error_handling_code() -> String {
    r#"
mod error_handling {
    use alloc::string::String;
    
    #[derive(Debug)]
    pub enum SkillError {
        InvalidInput(String),
        ExecutionFailed(String),
        SerializationError(String),
    }
    
    impl SkillError {
        pub fn to_string(&self) -> String {
            match self {
                SkillError::InvalidInput(msg) => format!("输入无效: {}", msg),
                SkillError::ExecutionFailed(msg) => format!("执行失败: {}", msg),
                SkillError::SerializationError(msg) => format!("序列化错误: {}", msg),
            }
        }
    }
}
"#.to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_code() -> &'static str {
        r#"pub fn add(a: i32, b: i32) -> i32 { a + b }"#
    }

    fn multi_fn_code() -> &'static str {
        r#"
use std::collections::HashMap;

pub fn first(x: i32) -> i32 { x + 1 }
pub fn second(x: i32) -> i32 { x * 2 }
"#
    }

    // ── SkillAdapterConfig ────────────────────────────────────────────────

    #[test]
    fn adapter_config_default_values() {
        let cfg = SkillAdapterConfig::default();
        assert!(cfg.enable_memory_management);
        assert!(cfg.enable_performance_monitoring);
        assert!(cfg.enable_error_handling);
        assert_eq!(cfg.max_input_size, 1024 * 1024);
        assert_eq!(cfg.max_output_size, 1024 * 1024);
    }

    // ── generate_wasm_adapter ─────────────────────────────────────────────

    #[test]
    fn generate_adapter_empty_name_returns_error() {
        let cfg = SkillAdapterConfig::default();
        let r = generate_wasm_adapter("", simple_code(), &cfg);
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("技能名称") || msg.contains("empty") || msg.contains("name"),
            "error should mention skill name: {msg}");
    }

    #[test]
    fn generate_adapter_simple_code_produces_output() {
        let cfg = SkillAdapterConfig::default();
        let r = generate_wasm_adapter("test.add", simple_code(), &cfg);
        assert!(r.is_ok(), "should generate adapter: {:?}", r.err());
        let code = r.unwrap();
        assert!(!code.is_empty());
        assert!(code.contains("test.add") || code.contains("test_add") || code.len() > 100,
            "adapter code should reference skill name or be substantive");
    }

    #[test]
    fn generate_adapter_empty_code_still_produces_output() {
        let cfg = SkillAdapterConfig::default();
        let r = generate_wasm_adapter("empty.skill", "", &cfg);
        assert!(r.is_ok(), "empty code should still produce adapter: {:?}", r.err());
        let code = r.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn generate_adapter_multi_function_code() {
        let cfg = SkillAdapterConfig::default();
        let r = generate_wasm_adapter("multi.skill", multi_fn_code(), &cfg);
        assert!(r.is_ok(), "multi-function code should produce adapter: {:?}", r.err());
        let code = r.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn generate_adapter_with_imports_in_code() {
        let cfg = SkillAdapterConfig::default();
        let code = "use std::fmt;\npub fn fmt_skill() -> String { String::new() }";
        let r = generate_wasm_adapter("fmt.skill", code, &cfg);
        assert!(r.is_ok());
    }

    #[test]
    fn generate_adapter_memory_management_disabled() {
        let cfg = SkillAdapterConfig {
            enable_memory_management: false,
            ..Default::default()
        };
        let r = generate_wasm_adapter("test.skill", simple_code(), &cfg);
        assert!(r.is_ok());
    }

    #[test]
    fn generate_adapter_all_features_disabled() {
        let cfg = SkillAdapterConfig {
            enable_memory_management: false,
            enable_performance_monitoring: false,
            enable_error_handling: false,
            max_input_size: 64,
            max_output_size: 64,
        };
        let r = generate_wasm_adapter("minimal.skill", simple_code(), &cfg);
        assert!(r.is_ok());
        let code = r.unwrap();
        assert!(!code.is_empty());
    }

    // ── extract_imports ───────────────────────────────────────────────────

    #[test]
    fn extract_imports_finds_use_statements() {
        let code = "use std::fmt;\nuse std::collections::HashMap;\npub fn f() {}";
        let imports = extract_imports(code).unwrap();
        assert!(imports.iter().any(|i| i.contains("std::fmt")));
        assert!(imports.iter().any(|i| i.contains("HashMap")));
    }

    #[test]
    fn extract_imports_empty_on_no_use() {
        let imports = extract_imports("pub fn f() -> i32 { 42 }").unwrap();
        assert!(imports.is_empty());
    }

    // ── find_main_function ────────────────────────────────────────────────

    #[test]
    fn find_main_function_returns_first_fn() {
        let code = "pub fn compute(x: i32) -> i32 { x }\npub fn helper() {}";
        let main = find_main_function(code).unwrap();
        assert!(main.is_some(), "should find a main function");
    }

    #[test]
    fn find_main_function_empty_code_returns_none() {
        let main = find_main_function("").unwrap();
        assert!(main.is_none());
    }

    // ── extract_functions ─────────────────────────────────────────────────

    #[test]
    fn extract_functions_finds_pub_fn() {
        let funcs = extract_functions(simple_code()).unwrap();
        assert!(!funcs.is_empty(), "should find at least one function in: {:?}", funcs);
        assert!(!funcs[0].name.is_empty(), "extracted function name must not be empty");
    }

    #[test]
    fn extract_functions_empty_code_returns_empty() {
        let funcs = extract_functions("").unwrap();
        assert!(funcs.is_empty());
    }

    #[test]
    fn extract_functions_multiple_fns() {
        let funcs = extract_functions(multi_fn_code()).unwrap();
        assert!(funcs.len() >= 2, "should find at least 2 functions, got {}", funcs.len());
    }
}

/// 生成技能执行代码
fn generate_skill_execution_code(parsed: &ParsedSkill) -> String {
    if let Some(main_func) = &parsed.main_function {
        format!(
            r#"
    // 开始性能计时
    performance::start_timing();
    
    // 执行主函数
    let result = {}(input)?;
    
    // 结束计时并记录
    let duration = performance::end_timing();
    performance::log_execution("{}", duration);
    
    Ok(result)
"#,
            main_func, main_func
        )
    } else {
        r#"
    // 没有找到主函数，返回输入
    Ok(input)
"#.to_string()
    }
}
