//! WASM编译器错误类型

use thiserror::Error;

/// WASM编译器错误
#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("WASM生成失败: {0}")]
    WasmGenerationFailed(String),

    #[error("技能代码解析失败: {0}")]
    SkillCodeParseError(String),

    #[error("适配器生成失败: {0}")]
    AdapterGenerationFailed(String),

    #[error("Rust编译失败: {0}")]
    RustCompilationFailed(String),

    #[error("WASM验证失败: {0}")]
    WasmValidationFailed(String),

    #[error("优化失败: {0}")]
    OptimizationFailed(String),

    #[error("文件操作失败: {0}")]
    FileOperationFailed(#[from] std::io::Error),

    #[error("临时目录创建失败: {0}")]
    TempDirCreationFailed(String),

    #[error("技能名称无效: {0}")]
    InvalidSkillName(String),

    #[error("依赖注入失败: {0}")]
    DependencyInjectionFailed(String),
}

/// 编译结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CompilationResult {
    /// 编译成功
    Success {
        wasm_binary: Vec<u8>,
        size_bytes: usize,
    },
    /// 编译失败
    Error {
        error: String,
    },
}

impl CompilationResult {
    /// 是否成功
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// 是否失败
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// 获取WASM二进制（如果成功）
    pub fn wasm_binary(&self) -> Option<&[u8]> {
        match self {
            Self::Success { wasm_binary, .. } => Some(wasm_binary),
            Self::Error { .. } => None,
        }
    }

    /// 获取错误信息（如果失败）
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Success { .. } => None,
            Self::Error { error } => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CompilationResult ─────────────────────────────────────────────────

    #[test]
    fn compilation_result_success_is_success() {
        let r = CompilationResult::Success { wasm_binary: vec![0u8; 8], size_bytes: 8 };
        assert!(r.is_success());
        assert!(!r.is_error());
    }

    #[test]
    fn compilation_result_error_is_error() {
        let r = CompilationResult::Error { error: "compile failed".into() };
        assert!(r.is_error());
        assert!(!r.is_success());
    }

    #[test]
    fn compilation_result_success_wasm_binary_some() {
        let bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let r = CompilationResult::Success { wasm_binary: bytes.clone(), size_bytes: 8 };
        assert_eq!(r.wasm_binary(), Some(bytes.as_slice()));
        assert_eq!(r.error(), None);
    }

    #[test]
    fn compilation_result_error_wasm_binary_none() {
        let r = CompilationResult::Error { error: "oops".into() };
        assert_eq!(r.wasm_binary(), None);
        assert_eq!(r.error(), Some("oops"));
    }

    #[test]
    fn compilation_result_success_error_is_none() {
        let r = CompilationResult::Success { wasm_binary: vec![], size_bytes: 0 };
        assert_eq!(r.error(), None);
    }

    #[test]
    fn compilation_result_roundtrip_serde() {
        let r = CompilationResult::Success {
            wasm_binary: vec![0, 1, 2],
            size_bytes: 3,
        };
        let json = serde_json::to_string(&r).unwrap();
        let d: CompilationResult = serde_json::from_str(&json).unwrap();
        assert!(d.is_success());
        assert_eq!(d.wasm_binary(), Some(&[0u8, 1, 2][..]));
    }

    #[test]
    fn compilation_result_error_roundtrip_serde() {
        let r = CompilationResult::Error { error: "bad input".into() };
        let json = serde_json::to_string(&r).unwrap();
        let d: CompilationResult = serde_json::from_str(&json).unwrap();
        assert!(d.is_error());
        assert_eq!(d.error(), Some("bad input"));
    }

    // ── CompilerError display ──────────────────────────────────────────────

    #[test]
    fn compiler_error_wasm_generation_failed() {
        let e = CompilerError::WasmGenerationFailed("linker error".into());
        assert!(e.to_string().contains("linker error"));
    }

    #[test]
    fn compiler_error_skill_code_parse_error() {
        let e = CompilerError::SkillCodeParseError("unexpected token".into());
        assert!(e.to_string().contains("unexpected token"));
    }

    #[test]
    fn compiler_error_adapter_generation_failed() {
        let e = CompilerError::AdapterGenerationFailed("missing fn".into());
        assert!(e.to_string().contains("missing fn"));
    }

    #[test]
    fn compiler_error_rust_compilation_failed() {
        let e = CompilerError::RustCompilationFailed("E0308".into());
        assert!(e.to_string().contains("E0308"));
    }

    #[test]
    fn compiler_error_wasm_validation_failed() {
        let e = CompilerError::WasmValidationFailed("bad magic".into());
        assert!(e.to_string().contains("bad magic"));
    }

    #[test]
    fn compiler_error_optimization_failed() {
        let e = CompilerError::OptimizationFailed("wasm-opt crash".into());
        assert!(e.to_string().contains("wasm-opt crash"));
    }

    #[test]
    fn compiler_error_file_operation_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e: CompilerError = io_err.into();
        assert!(e.to_string().contains("no such file") || e.to_string().contains("文件操作"));
    }

    #[test]
    fn compiler_error_temp_dir_creation_failed() {
        let e = CompilerError::TempDirCreationFailed("/tmp/x".into());
        assert!(e.to_string().contains("/tmp/x"));
    }

    #[test]
    fn compiler_error_invalid_skill_name() {
        let e = CompilerError::InvalidSkillName("".into());
        assert!(e.to_string().contains("技能名称"));
    }

    #[test]
    fn compiler_error_dependency_injection_failed() {
        let e = CompilerError::DependencyInjectionFailed("missing dep".into());
        assert!(e.to_string().contains("missing dep"));
    }
}
