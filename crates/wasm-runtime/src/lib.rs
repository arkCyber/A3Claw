//! OpenClaw WASM技能运行时
//! 
//! 在WasmEdge沙箱中执行编译为WASM的技能

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

pub mod error;
pub mod memory;
pub mod optimizer;
pub mod registry;

pub use error::{RuntimeError, ExecutionResult};
pub use memory::WasmMemoryManager;
pub use registry::WasmSkillRegistry;

/// WASM技能运行时
pub struct WasmSkillRuntime {
    registry: Arc<tokio::sync::RwLock<WasmSkillRegistry>>,
    memory_manager: Arc<tokio::sync::Mutex<WasmMemoryManager>>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl WasmSkillRuntime {
    /// 创建新的运行时实例
    pub fn new() -> Result<Self> {
        // Validate that the WasmEdge native library is available by probing
        // the wasmedge_sys Config/Executor APIs.  If this fails the caller
        // gets a clear error.
        let _config = wasmedge_sys::Config::create()
            .map_err(|e| anyhow::anyhow!("WasmEdge Config::create failed — is the native library installed? {:?}", e))?;

        let registry = Arc::new(tokio::sync::RwLock::new(WasmSkillRegistry::new()));
        let memory_manager = Arc::new(tokio::sync::Mutex::new(WasmMemoryManager::new()));
        let performance_metrics = Arc::new(RwLock::new(PerformanceMetrics::new()));

        Ok(Self {
            registry,
            memory_manager,
            performance_metrics,
        })
    }

    /// 执行WASM技能
    pub async fn execute_skill(
        &self,
        skill_name: &str,
        input: &Value,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        info!("执行WASM技能: {}", skill_name);
        
        // 1. 序列化输入
        let input_json = serde_json::to_vec(input)
            .context("Failed to serialize input")?;
        
        // 2. 记录 host-side 分配 (accounting only — real guest alloc happens in execute_wasm_function)
        let input_ptr = self.memory_manager.lock().await.allocate(input_json.len())?;

        // 3. 获取 WASM 字节并执行
        let wasm_bytes = {
            let registry = self.registry.read().await;
            registry.get_bytes(skill_name).await.context("Skill not found")?
        };

        let execution_result = match self.execute_wasm_function(wasm_bytes, &input_json).await {
            Ok(result) => result,
            Err(e) => {
                let _ = self.memory_manager.lock().await.deallocate(input_ptr, input_json.len());
                return Err(e);
            }
        };

        // 4. 清理
        let _ = self.memory_manager.lock().await.deallocate(input_ptr, input_json.len());

        // 5. 性能记录
        let execution_time = start_time.elapsed();
        self.record_performance_metrics(skill_name, execution_time, execution_result.is_success());

        info!("✅ 技能 {} 执行完成，耗时: {:?}", skill_name, execution_time);
        
        Ok(execution_result)
    }

    /// 执行WASM函数 — 使用 wasmedge_sys 为每次调用创建独立的执行环境。
    ///
    /// 协议: guest 导出 `execute(ptr: i32, len: i32) -> i64`，返回值为
    /// packed `(out_ptr: u32) << 32 | (out_len: u32)`。
    async fn execute_wasm_function(
        &self,
        wasm_bytes: Vec<u8>,
        input_json: &[u8],
    ) -> Result<ExecutionResult> {
        use wasmedge_sys::{AsInstance, Config, Executor, Loader, Store, Validator, WasmValue};

        let config = Config::create().map_err(|e| anyhow::anyhow!("Config: {:?}", e))?;
        let loader = Loader::create(Some(&config)).map_err(|e| anyhow::anyhow!("Loader: {:?}", e))?;
        let validator = Validator::create(Some(&config)).map_err(|e| anyhow::anyhow!("Validator: {:?}", e))?;
        let mut executor = Executor::create(Some(&config), None).map_err(|e| anyhow::anyhow!("Executor: {:?}", e))?;
        let mut store = Store::create().map_err(|e| anyhow::anyhow!("Store: {:?}", e))?;

        let module = loader.from_bytes(&wasm_bytes).map_err(|e| anyhow::anyhow!("Loader.from_bytes: {:?}", e))?;
        validator.validate(&module).map_err(|e| anyhow::anyhow!("validate: {:?}", e))?;
        let mut instance = executor
            .register_named_module(&mut store, &module, "skill")
            .map_err(|e| anyhow::anyhow!("register_named_module: {:?}", e))?;

        // Call alloc(size) -> ptr
        let mut alloc_fn = instance.get_func_mut("alloc").map_err(|_| {
            RuntimeError::ExecutionFailed("missing 'alloc' export".into())
        })?;
        let alloc_result = executor
            .call_func(&mut alloc_fn, vec![WasmValue::from_i32(input_json.len() as i32)])
            .map_err(|e| RuntimeError::ExecutionFailed(format!("alloc: {:?}", e)))?;
        let guest_ptr = alloc_result
            .first()
            .map(|v| v.to_i32() as u32)
            .ok_or_else(|| RuntimeError::ExecutionFailed("alloc returned no value".into()))?;

        // Write input bytes into guest memory.
        {
            let mut mem = instance.get_memory_mut("memory").map_err(|_| {
                anyhow::anyhow!("Guest module has no exported 'memory'")
            })?;
            mem.set_data(input_json, guest_ptr).map_err(|e| anyhow::anyhow!("set_data: {:?}", e))?;
        }

        // Call execute(ptr, len) -> i64 packed (out_ptr << 32 | out_len).
        let mut exec_fn = instance.get_func_mut("execute").map_err(|_| {
            RuntimeError::ExecutionFailed("missing 'execute' export".into())
        })?;
        let exec_result = executor
            .call_func(&mut exec_fn, vec![
                WasmValue::from_i32(guest_ptr as i32),
                WasmValue::from_i32(input_json.len() as i32),
            ])
            .map_err(|e| RuntimeError::ExecutionFailed(format!("execute: {:?}", e)))?;

        let packed = exec_result
            .first()
            .map(|v| v.to_i64() as u64)
            .ok_or_else(|| RuntimeError::ExecutionFailed("execute returned no value".into()))?;
        let out_ptr = (packed >> 32) as u32;
        let out_len = (packed & 0xFFFF_FFFF) as u32;

        // Read output bytes from guest memory.
        let output_data = {
            let out_mem = instance.get_memory_ref("memory").map_err(|_| {
                anyhow::anyhow!("Guest module has no exported 'memory'")
            })?;
            out_mem.get_data(out_ptr, out_len).map_err(|e| anyhow::anyhow!("get_data: {:?}", e))?
        };

        // Deserialise output.
        let output_value: Value = serde_json::from_slice(&output_data)
            .map_err(|e| RuntimeError::OutputParseError(e.to_string()))?;

        let success = output_value.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let execution_result = if success {
            ExecutionResult::Success {
                result: output_value.get("result").cloned().unwrap_or(Value::Null),
                execution_time: Duration::from_millis(0),
            }
        } else {
            let err_msg = output_value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            ExecutionResult::Error { error: err_msg }
        };

        Ok(execution_result)
    }

    /// 注册WASM技能
    pub async fn register_skill(&self, skill_name: &str, wasm_binary: Vec<u8>) -> Result<()> {
        info!("注册WASM技能: {} (大小: {} 字节)", skill_name, wasm_binary.len());
        self.validate_wasm_binary(&wasm_binary)?;
        self.registry.write().await.register_skill(skill_name, wasm_binary).await
    }

    /// 验证WASM二进制
    fn validate_wasm_binary(&self, wasm_binary: &[u8]) -> Result<()> {
        use wasmedge_sys::{Config, Loader, Validator};
        let config = Config::create().map_err(|e| RuntimeError::ValidationFailed(format!("{:?}", e)))?;
        let loader = Loader::create(Some(&config)).map_err(|e| RuntimeError::ValidationFailed(format!("{:?}", e)))?;
        let module = loader.from_bytes(wasm_binary).map_err(|e| RuntimeError::ValidationFailed(format!("{:?}", e)))?;
        let validator = Validator::create(Some(&config)).map_err(|e| RuntimeError::ValidationFailed(format!("{:?}", e)))?;
        validator.validate(&module).map_err(|e| RuntimeError::ValidationFailed(format!("{:?}", e)))?;
        Ok(())
    }

    /// 批量注册技能
    pub async fn register_skills_batch(&self, skills: HashMap<String, Vec<u8>>) -> Result<()> {
        info!("批量注册 {} 个WASM技能", skills.len());
        let mut success_count = 0u32;
        let mut failed_count = 0u32;
        for (skill_name, wasm_binary) in skills {
            match self.register_skill(&skill_name, wasm_binary).await {
                Ok(_) => {
                    success_count += 1;
                    debug!("✅ 注册成功: {}", skill_name);
                }
                Err(e) => {
                    failed_count += 1;
                    warn!("❌ 注册失败: {} - {}", skill_name, e);
                }
            }
        }
        info!("批量注册完成: 成功 {}, 失败 {}", success_count, failed_count);
        if failed_count > 0 {
            warn!("有 {} 个技能注册失败", failed_count);
        }
        Ok(())
    }

    /// 获取技能列表
    pub async fn list_skills(&self) -> Vec<String> {
        self.registry.read().await.list_skills().await
    }

    /// 检查技能是否存在
    pub async fn has_skill(&self, skill_name: &str) -> bool {
        self.registry.read().await.has_skill(skill_name).await
    }

    /// 记录性能指标
    fn record_performance_metrics(&self, skill_name: &str, execution_time: Duration, success: bool) {
        let mut metrics = self.performance_metrics.write();
        metrics.record_execution(skill_name, execution_time, success);
    }

    /// 获取性能指标
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.performance_metrics.read().clone()
    }

    /// 清理资源
    pub async fn cleanup(&self) {
        info!("清理WASM运行时资源");
        self.memory_manager.lock().await.cleanup();
        self.registry.write().await.cleanup();
    }
}

impl Default for WasmSkillRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default WASM runtime — ensure WasmEdge native library is installed")
    }
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    executions: HashMap<String, ExecutionStats>,
    total_executions: u64,
    total_successes: u64,
    total_failures: u64,
    total_execution_time: Duration,
}

#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub count: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            executions: HashMap::new(),
            total_executions: 0,
            total_successes: 0,
            total_failures: 0,
            total_execution_time: Duration::ZERO,
        }
    }

    pub fn record_execution(&mut self, skill_name: &str, execution_time: Duration, success: bool) {
        let stats = self.executions.entry(skill_name.to_string()).or_insert_with(|| ExecutionStats {
            count: 0,
            successes: 0,
            failures: 0,
            total_time: Duration::ZERO,
            avg_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
        });

        stats.count += 1;
        stats.total_time += execution_time;
        stats.avg_time = stats.total_time / stats.count as u32;
        
        if execution_time < stats.min_time {
            stats.min_time = execution_time;
        }
        if execution_time > stats.max_time {
            stats.max_time = execution_time;
        }

        if success {
            stats.successes += 1;
            self.total_successes += 1;
        } else {
            stats.failures += 1;
            self.total_failures += 1;
        }

        self.total_executions += 1;
        self.total_execution_time += execution_time;
    }

    pub fn get_skill_stats(&self, skill_name: &str) -> Option<&ExecutionStats> {
        self.executions.get(skill_name)
    }

    pub fn get_overall_stats(&self) -> (ExecutionStats, f64) {
        let overall = ExecutionStats {
            count: self.total_executions,
            successes: self.total_successes,
            failures: self.total_failures,
            total_time: self.total_execution_time,
            avg_time: if self.total_executions > 0 {
                self.total_execution_time / self.total_executions as u32
            } else {
                Duration::ZERO
            },
            min_time: Duration::ZERO,
            max_time: Duration::ZERO,
        };

        let success_rate = if self.total_executions > 0 {
            (self.total_successes as f64 / self.total_executions as f64) * 100.0
        } else {
            0.0
        };

        (overall, success_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn execute_skill_unregistered_returns_error() {
        let runtime = WasmSkillRuntime::new().unwrap();
        let result = runtime.execute_skill("no.such.skill", &serde_json::json!({})).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("no.such.skill") || msg.contains("not found") || msg.contains("Skill"),
            "error should mention missing skill: {msg}");
    }

    #[tokio::test]
    async fn register_then_has_skill() {
        let runtime = WasmSkillRuntime::new().unwrap();
        // minimal valid WASM magic header (passes validate_wasm_binary)
        let minimal_wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        runtime.register_skill("test.ping", minimal_wasm).await.unwrap();
        assert!(runtime.has_skill("test.ping").await);
        let names = runtime.list_skills().await;
        assert!(names.contains(&"test.ping".to_string()));
    }

    #[tokio::test]
    async fn register_skills_batch_counts_failures() {
        let runtime = WasmSkillRuntime::new().unwrap();
        let mut skills = HashMap::new();
        skills.insert("ok.skill".to_string(), vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
        skills.insert("bad.skill".to_string(), b"not-wasm".to_vec());
        // Should not panic; bad skills are logged and skipped
        let result = runtime.register_skills_batch(skills).await;
        assert!(result.is_ok());
        // Only valid skill is registered
        assert!(runtime.has_skill("ok.skill").await);
        assert!(!runtime.has_skill("bad.skill").await);
    }

    #[tokio::test]
    async fn runtime_creation_succeeds() {
        let runtime = WasmSkillRuntime::new();
        assert!(runtime.is_ok(), "WasmEdge runtime must be creatable: {:?}", runtime.err());
    }

    #[tokio::test]
    async fn register_invalid_wasm_returns_error() {
        let runtime = WasmSkillRuntime::new().unwrap();
        let bad_wasm = b"not-wasm".to_vec();
        let result = runtime.register_skill("bad.skill", bad_wasm).await;
        assert!(result.is_err(), "invalid WASM bytes must be rejected by validate_wasm_binary");
    }

    #[tokio::test]
    async fn has_skill_returns_false_for_unknown() {
        let runtime = WasmSkillRuntime::new().unwrap();
        assert!(!runtime.has_skill("no.such").await);
    }

    #[tokio::test]
    async fn list_skills_empty_on_new_runtime() {
        let runtime = WasmSkillRuntime::new().unwrap();
        assert!(runtime.list_skills().await.is_empty());
    }

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();
        
        metrics.record_execution("test.skill", Duration::from_millis(100), true);
        metrics.record_execution("test.skill", Duration::from_millis(200), false);
        
        let stats = metrics.get_skill_stats("test.skill").unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 1);
        
        let (overall, success_rate) = metrics.get_overall_stats();
        assert_eq!(overall.count, 2);
        assert_eq!(success_rate, 50.0);
    }
}
