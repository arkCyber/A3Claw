//! WASM linear-memory manager.
//!
//! Tracks allocations made inside a WASM guest's linear memory so the host
//! can pass data in and read results out without leaking memory.
//!
//! In the current architecture the host never directly manages guest memory
//! pages — allocations are driven by calling the guest's `alloc` / `dealloc`
//! exports.  This module provides a lightweight accounting layer that records
//! live host-side buffer allocations so the runtime can detect leaks.

use anyhow::{bail, Result};
use std::collections::HashMap;
use tracing::warn;

// ── WasmMemoryManager ────────────────────────────────────────────────────────

/// Tracks host-side allocations that are about to be written into a WASM
/// guest's linear memory.
///
/// This is intentionally a *host-side* view: actual memory lives inside the
/// WASM VM.  The manager only records (ptr, len) pairs so callers can detect
/// double-frees and forgotten allocations.
pub struct WasmMemoryManager {
    /// Live allocations: ptr → len.
    live: HashMap<usize, usize>,
    /// Running count of all allocations since creation.
    total_allocated: u64,
    /// Running count of all deallocations since creation.
    total_freed: u64,
}

impl WasmMemoryManager {
    pub fn new() -> Self {
        Self {
            live: HashMap::new(),
            total_allocated: 0,
            total_freed: 0,
        }
    }

    /// Record that `len` bytes have been allocated at `ptr` in the guest.
    ///
    /// Returns `ptr` unchanged so callers can chain the call.
    pub fn allocate(&mut self, len: usize) -> Result<usize> {
        if len == 0 {
            bail!("WasmMemoryManager::allocate called with len=0");
        }
        // ptr=0 is reserved as null in WASM; the caller must supply the real
        // ptr returned by the guest's `alloc` export.  We use a sentinel here
        // since this layer does not drive the actual allocation.
        let ptr = self.next_sentinel();
        self.live.insert(ptr, len);
        self.total_allocated += 1;
        Ok(ptr)
    }

    /// Record that a previous allocation at `ptr` has been freed.
    pub fn deallocate(&mut self, ptr: usize, len: usize) -> Result<()> {
        match self.live.remove(&ptr) {
            Some(recorded_len) => {
                if recorded_len != len {
                    warn!(
                        ptr,
                        recorded = recorded_len,
                        given = len,
                        "WasmMemoryManager: dealloc len mismatch — possible memory corruption"
                    );
                }
                self.total_freed += 1;
                Ok(())
            }
            None => {
                warn!(ptr, "WasmMemoryManager: dealloc of unknown ptr — possible double-free");
                Ok(())
            }
        }
    }

    /// Stub: in a real implementation this would forward the read to the VM's
    /// linear memory.  Here it returns an empty slice so the rest of the
    /// runtime can be tested without a live WasmEdge VM.
    pub fn read_memory(&self, _ptr: usize, len: usize) -> Result<Vec<u8>> {
        Ok(vec![0u8; len])
    }

    /// Stub: records the write in the live map; a real implementation would
    /// forward the bytes to the VM's linear memory.
    pub fn write_memory(&mut self, ptr: usize, data: &[u8]) -> Result<()> {
        self.live.entry(ptr).and_modify(|l| *l = data.len());
        Ok(())
    }

    /// Number of live (unfreed) allocations.
    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    /// Total bytes tracked as live (sum of all live lengths).
    pub fn live_bytes(&self) -> usize {
        self.live.values().sum()
    }

    /// Cumulative allocation count since creation.
    pub fn total_allocated(&self) -> u64 {
        self.total_allocated
    }

    /// Cumulative deallocation count since creation.
    pub fn total_freed(&self) -> u64 {
        self.total_freed
    }

    /// Release all tracked allocations (used during cleanup).
    pub fn cleanup(&mut self) {
        let leaked = self.live.len();
        if leaked > 0 {
            warn!(
                leaked,
                "WasmMemoryManager::cleanup: {} allocation(s) were never freed",
                leaked
            );
        }
        self.live.clear();
    }

    // ── private helpers ───────────────────────────────────────────────────────

    /// Returns a monotonically increasing sentinel ptr (starting at 1).
    /// Used only for host-side accounting; the real WASM ptr comes from the guest.
    fn next_sentinel(&self) -> usize {
        self.total_allocated as usize + 1
    }
}

impl Default for WasmMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_has_no_live_allocations() {
        let mgr = WasmMemoryManager::new();
        assert_eq!(mgr.live_count(), 0);
        assert_eq!(mgr.live_bytes(), 0);
    }

    #[test]
    fn allocate_increases_live_count() {
        let mut mgr = WasmMemoryManager::new();
        mgr.allocate(64).unwrap();
        assert_eq!(mgr.live_count(), 1);
        assert_eq!(mgr.total_allocated(), 1);
    }

    #[test]
    fn allocate_zero_len_errors() {
        let mut mgr = WasmMemoryManager::new();
        assert!(mgr.allocate(0).is_err());
    }

    #[test]
    fn deallocate_removes_from_live() {
        let mut mgr = WasmMemoryManager::new();
        let ptr = mgr.allocate(128).unwrap();
        mgr.deallocate(ptr, 128).unwrap();
        assert_eq!(mgr.live_count(), 0);
        assert_eq!(mgr.total_freed(), 1);
    }

    #[test]
    fn deallocate_unknown_ptr_does_not_panic() {
        let mut mgr = WasmMemoryManager::new();
        assert!(mgr.deallocate(0xDEAD_BEEF, 32).is_ok());
    }

    #[test]
    fn multiple_allocations_tracked_independently() {
        let mut mgr = WasmMemoryManager::new();
        let p1 = mgr.allocate(32).unwrap();
        let p2 = mgr.allocate(64).unwrap();
        assert_ne!(p1, p2);
        assert_eq!(mgr.live_count(), 2);
        assert_eq!(mgr.live_bytes(), 32 + 64);
    }

    #[test]
    fn cleanup_clears_all_live_allocations() {
        let mut mgr = WasmMemoryManager::new();
        mgr.allocate(16).unwrap();
        mgr.allocate(32).unwrap();
        mgr.cleanup();
        assert_eq!(mgr.live_count(), 0);
    }

    #[test]
    fn read_memory_returns_zero_filled_slice() {
        let mgr = WasmMemoryManager::new();
        let data = mgr.read_memory(100, 8).unwrap();
        assert_eq!(data.len(), 8);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn total_allocated_counts_all_allocations() {
        let mut mgr = WasmMemoryManager::new();
        for _ in 0..5 {
            mgr.allocate(10).unwrap();
        }
        assert_eq!(mgr.total_allocated(), 5);
    }
}
