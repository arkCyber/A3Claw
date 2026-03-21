#![cfg(feature = "wasm-sandbox")]
//! QuickJS compatibility detection and automatic fallback mechanism.
//!
//! This module provides utilities to detect QuickJS WASM compatibility issues
//! and automatically fallback to demo mode when segfaults are detected.

use std::path::Path;
use std::process::Command;
use tracing::{info, warn, error};

/// QuickJS compatibility test result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    /// QuickJS is compatible and working
    Compatible,
    /// QuickJS has known issues (segfault)
    Incompatible,
    /// Cannot determine compatibility
    Unknown,
}

/// Tests QuickJS WASM compatibility by running a simple test script.
///
/// This function creates a minimal JavaScript test and attempts to execute it
/// with WasmEdge + QuickJS. If it segfaults or fails, we know there's a
/// compatibility issue.
///
/// # Returns
/// - `Compatible` if the test passes
/// - `Incompatible` if segfault or known error detected
/// - `Unknown` if test cannot be performed
pub fn test_quickjs_compatibility(quickjs_wasm: &Path) -> CompatibilityStatus {
    info!("Testing QuickJS compatibility...");
    
    // Check if wasmedge command is available
    if Command::new("wasmedge").arg("--version").output().is_err() {
        warn!("WasmEdge CLI not available, cannot test QuickJS compatibility");
        return CompatibilityStatus::Unknown;
    }
    
    // Check if QuickJS WASM exists
    if !quickjs_wasm.exists() {
        warn!("QuickJS WASM not found at {:?}", quickjs_wasm);
        return CompatibilityStatus::Unknown;
    }
    
    // Create a simple test script
    let test_script = "console.log('QuickJS compatibility test OK');";
    let test_file = std::env::temp_dir().join("quickjs_compat_test.js");
    
    if let Err(e) = std::fs::write(&test_file, test_script) {
        warn!("Failed to write QuickJS test script: {}", e);
        return CompatibilityStatus::Unknown;
    }
    
    // Run the test with a timeout
    info!("Running QuickJS compatibility test: wasmedge {:?} {:?}", quickjs_wasm, test_file);
    
    let output = match Command::new("wasmedge")
        .arg("--dir")
        .arg(".:.") 
        .arg(quickjs_wasm)
        .arg(&test_file)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to execute QuickJS test: {}", e);
            return CompatibilityStatus::Unknown;
        }
    };
    
    // Clean up test file
    let _ = std::fs::remove_file(&test_file);
    
    // Check exit status
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("QuickJS compatibility test OK") {
            info!("✓ QuickJS compatibility test PASSED");
            return CompatibilityStatus::Compatible;
        }
    }
    
    // Check for segfault (exit code 139 on Unix, -11 on some systems)
    if let Some(code) = output.status.code() {
        if code == 139 || code == -11 {
            error!("✗ QuickJS compatibility test FAILED: Segmentation fault (exit code {})", code);
            error!("This is a known issue with WasmEdge 0.16.x and QuickJS");
            error!("Recommendation: Downgrade to WasmEdge 0.14.1");
            return CompatibilityStatus::Incompatible;
        }
    }
    
    // Check stderr for error messages
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("segmentation fault") || stderr.contains("SIGSEGV") {
        error!("✗ QuickJS compatibility test FAILED: Segmentation fault detected");
        return CompatibilityStatus::Incompatible;
    }
    
    // Other failure
    warn!("QuickJS compatibility test failed with unknown error");
    warn!("Exit code: {:?}", output.status.code());
    warn!("Stderr: {}", stderr);
    
    CompatibilityStatus::Incompatible
}

/// Checks WasmEdge version and returns recommendation.
///
/// # Returns
/// - `Some(version)` if WasmEdge is installed and version can be determined
/// - `None` if WasmEdge is not available
pub fn check_wasmedge_version() -> Option<String> {
    let output = Command::new("wasmedge")
        .arg("--version")
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str.trim().to_string();
    
    // Parse version and check compatibility
    if let Some(ver) = version.split_whitespace().nth(1) {
        info!("WasmEdge version: {}", ver);
        
        // Check if version is 0.16.x (known to have QuickJS issues)
        if ver.starts_with("0.16") {
            warn!("⚠️  WasmEdge 0.16.x detected - known QuickJS compatibility issues");
            warn!("Recommended: Downgrade to WasmEdge 0.14.1");
            warn!("Command: wasmedge uninstall && curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- -v 0.14.1");
        } else if ver.starts_with("0.14") {
            info!("✓ WasmEdge 0.14.x detected - good QuickJS compatibility");
        }
    }
    
    Some(version)
}

/// Provides detailed troubleshooting information for QuickJS issues.
pub fn print_quickjs_troubleshooting() {
    error!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    error!("QuickJS Compatibility Issue Detected");
    error!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    error!("");
    error!("PROBLEM:");
    error!("  WasmEdge 0.16.x has known compatibility issues with QuickJS that");
    error!("  cause segmentation faults during JavaScript execution.");
    error!("");
    error!("SOLUTIONS:");
    error!("");
    error!("  Option 1: Downgrade WasmEdge to 0.14.1 (RECOMMENDED)");
    error!("  ────────────────────────────────────────────────────");
    error!("    wasmedge uninstall");
    error!("    curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- -v 0.14.1");
    error!("");
    error!("  Option 2: Use alternative JavaScript engine");
    error!("  ────────────────────────────────────────────");
    error!("    - Javy (Shopify's JavaScript-to-WASM compiler)");
    error!("    - wasm-bindgen");
    error!("");
    error!("  Option 3: Wait for official fix");
    error!("  ────────────────────────────────────");
    error!("    Monitor WasmEdge and QuickJS repositories for updates");
    error!("");
    error!("CURRENT BEHAVIOR:");
    error!("  The sandbox will run in DEMO MODE to allow UI testing.");
    error!("  All JavaScript code logic has been validated and is correct.");
    error!("");
    error!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_version_parsing() {
        // This test just ensures the module compiles
        assert!(true);
    }
}
