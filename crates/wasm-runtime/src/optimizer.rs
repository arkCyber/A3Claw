//! WASM skill execution optimizer.
//!
//! Provides a simple warm-up cache and concurrency limiter so that repeatedly
//! invoked skills pay the instantiation cost only once, and so that a burst of
//! calls to the same skill does not spin up an unbounded number of WASM
//! instances.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info};

// ── ExecutionRecord ───────────────────────────────────────────────────────────

/// One recorded execution sample used for latency-based warm-up decisions.
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub skill_name: String,
    pub duration: Duration,
    pub success: bool,
    pub recorded_at: Instant,
}

// ── WarmupCache ───────────────────────────────────────────────────────────────

/// Tracks which skills are "warm" (recently and frequently used) so the
/// runtime can pre-load their modules instead of instantiating on demand.
#[derive(Debug)]
pub struct WarmupCache {
    /// skill_name → total invocation count
    call_counts: HashMap<String, u64>,
    /// Minimum invocations before a skill is considered warm.
    threshold: u64,
    /// TTL after which a skill's warm status expires.
    ttl: Duration,
    /// skill_name → last call time
    last_call: HashMap<String, Instant>,
}

impl WarmupCache {
    pub fn new(threshold: u64, ttl: Duration) -> Self {
        Self {
            call_counts: HashMap::new(),
            threshold,
            ttl,
            last_call: HashMap::new(),
        }
    }

    /// Record a call to the given skill.
    pub fn record_call(&mut self, skill_name: &str) {
        *self.call_counts.entry(skill_name.to_string()).or_insert(0) += 1;
        self.last_call.insert(skill_name.to_string(), Instant::now());
    }

    /// Returns `true` when the skill should be kept warm in the VM pool.
    pub fn is_warm(&self, skill_name: &str) -> bool {
        let count = self.call_counts.get(skill_name).copied().unwrap_or(0);
        if count < self.threshold {
            return false;
        }
        match self.last_call.get(skill_name) {
            Some(last) => last.elapsed() < self.ttl,
            None => false,
        }
    }

    /// Names of all currently warm skills.
    pub fn warm_skills(&self) -> Vec<String> {
        self.call_counts
            .keys()
            .filter(|name| self.is_warm(name))
            .cloned()
            .collect()
    }

    /// Remove stale entries that have expired.
    pub fn evict_stale(&mut self) {
        let ttl = self.ttl;
        let stale: Vec<String> = self
            .last_call
            .iter()
            .filter(|(_, t)| t.elapsed() >= ttl)
            .map(|(k, _)| k.clone())
            .collect();
        for key in &stale {
            debug!(skill = %key, "Evicting stale warm-up cache entry");
            self.last_call.remove(key);
            self.call_counts.remove(key);
        }
        if !stale.is_empty() {
            info!("Evicted {} stale warm-up cache entries", stale.len());
        }
    }

    /// Total distinct skills tracked.
    pub fn tracked_count(&self) -> usize {
        self.call_counts.len()
    }
}

impl Default for WarmupCache {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(300))
    }
}

// ── ConcurrencyLimiter ────────────────────────────────────────────────────────

/// Per-skill concurrency limiter.  Rejects execution when a skill already has
/// `max_concurrent` in-flight calls, preventing unbounded parallelism.
#[derive(Debug)]
pub struct ConcurrencyLimiter {
    /// Maximum concurrent executions per skill.
    max_concurrent: usize,
    /// skill_name → current in-flight count
    in_flight: HashMap<String, usize>,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            in_flight: HashMap::new(),
        }
    }

    /// Attempt to acquire a slot for `skill_name`.
    ///
    /// Returns `true` on success, `false` when the skill is at capacity.
    pub fn acquire(&mut self, skill_name: &str) -> bool {
        let count = self.in_flight.entry(skill_name.to_string()).or_insert(0);
        if *count >= self.max_concurrent {
            return false;
        }
        *count += 1;
        true
    }

    /// Release a slot for `skill_name` once execution completes.
    pub fn release(&mut self, skill_name: &str) {
        if let Some(count) = self.in_flight.get_mut(skill_name) {
            *count = count.saturating_sub(1);
        }
    }

    /// Current in-flight count for a skill.
    pub fn in_flight_count(&self, skill_name: &str) -> usize {
        self.in_flight.get(skill_name).copied().unwrap_or(0)
    }
}

impl Default for ConcurrencyLimiter {
    fn default() -> Self {
        Self::new(4)
    }
}

// ── SkillOptimizer ────────────────────────────────────────────────────────────

/// Combines the warm-up cache and concurrency limiter into a single coordinator.
pub struct SkillOptimizer {
    pub warmup: WarmupCache,
    pub limiter: ConcurrencyLimiter,
    /// Recent execution history (bounded ring-buffer).
    history: VecDeque<ExecutionRecord>,
    history_limit: usize,
}

impl SkillOptimizer {
    pub fn new(warmup_threshold: u64, ttl: Duration, max_concurrent: usize) -> Self {
        Self {
            warmup: WarmupCache::new(warmup_threshold, ttl),
            limiter: ConcurrencyLimiter::new(max_concurrent),
            history: VecDeque::new(),
            history_limit: 1000,
        }
    }

    /// Called before execution starts — records the call and acquires a slot.
    ///
    /// Returns `false` when the skill is at concurrency capacity.
    pub fn before_execute(&mut self, skill_name: &str) -> bool {
        self.warmup.record_call(skill_name);
        self.limiter.acquire(skill_name)
    }

    /// Called after execution completes — releases the concurrency slot and
    /// appends to history.
    pub fn after_execute(&mut self, skill_name: &str, duration: Duration, success: bool) {
        self.limiter.release(skill_name);
        if self.history.len() >= self.history_limit {
            self.history.pop_front();
        }
        self.history.push_back(ExecutionRecord {
            skill_name: skill_name.to_string(),
            duration,
            success,
            recorded_at: Instant::now(),
        });
    }

    /// Average execution duration for `skill_name` over the recorded history.
    pub fn avg_duration(&self, skill_name: &str) -> Option<Duration> {
        let records: Vec<_> = self
            .history
            .iter()
            .filter(|r| r.skill_name == skill_name)
            .collect();
        if records.is_empty() {
            return None;
        }
        let total: Duration = records.iter().map(|r| r.duration).sum();
        Some(total / records.len() as u32)
    }

    /// Success rate (0.0–1.0) for `skill_name` over recorded history.
    pub fn success_rate(&self, skill_name: &str) -> f64 {
        let records: Vec<_> = self
            .history
            .iter()
            .filter(|r| r.skill_name == skill_name)
            .collect();
        if records.is_empty() {
            return 1.0;
        }
        let successes = records.iter().filter(|r| r.success).count();
        successes as f64 / records.len() as f64
    }

    /// Purge stale warm-up entries.
    pub fn evict_stale(&mut self) {
        self.warmup.evict_stale();
    }
}

impl Default for SkillOptimizer {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(300), 4)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── WarmupCache ───────────────────────────────────────────────────────────

    #[test]
    fn new_cache_has_no_warm_skills() {
        let cache = WarmupCache::new(3, Duration::from_secs(60));
        assert!(cache.warm_skills().is_empty());
        assert_eq!(cache.tracked_count(), 0);
    }

    #[test]
    fn skill_becomes_warm_after_threshold_calls() {
        let mut cache = WarmupCache::new(3, Duration::from_secs(60));
        for _ in 0..3 {
            cache.record_call("math.add");
        }
        assert!(cache.is_warm("math.add"));
    }

    #[test]
    fn skill_not_warm_below_threshold() {
        let mut cache = WarmupCache::new(3, Duration::from_secs(60));
        cache.record_call("math.add");
        cache.record_call("math.add");
        assert!(!cache.is_warm("math.add"));
    }

    #[test]
    fn warm_skills_returns_all_warm_names() {
        let mut cache = WarmupCache::new(2, Duration::from_secs(60));
        for _ in 0..2 {
            cache.record_call("skill.a");
            cache.record_call("skill.b");
        }
        let mut warm = cache.warm_skills();
        warm.sort();
        assert_eq!(warm, vec!["skill.a", "skill.b"]);
    }

    #[test]
    fn evict_stale_removes_expired_entries() {
        let mut cache = WarmupCache::new(1, Duration::from_millis(1));
        cache.record_call("fast.skill");
        std::thread::sleep(Duration::from_millis(5));
        cache.evict_stale();
        assert_eq!(cache.tracked_count(), 0);
    }

    // ── ConcurrencyLimiter ────────────────────────────────────────────────────

    #[test]
    fn acquire_within_limit_succeeds() {
        let mut lim = ConcurrencyLimiter::new(2);
        assert!(lim.acquire("skill.a"));
        assert!(lim.acquire("skill.a"));
    }

    #[test]
    fn acquire_at_capacity_fails() {
        let mut lim = ConcurrencyLimiter::new(2);
        lim.acquire("skill.a");
        lim.acquire("skill.a");
        assert!(!lim.acquire("skill.a"));
    }

    #[test]
    fn release_frees_slot() {
        let mut lim = ConcurrencyLimiter::new(1);
        assert!(lim.acquire("skill.a"));
        assert!(!lim.acquire("skill.a"));
        lim.release("skill.a");
        assert!(lim.acquire("skill.a"));
    }

    #[test]
    fn in_flight_count_tracks_correctly() {
        let mut lim = ConcurrencyLimiter::new(4);
        lim.acquire("skill.b");
        lim.acquire("skill.b");
        assert_eq!(lim.in_flight_count("skill.b"), 2);
        lim.release("skill.b");
        assert_eq!(lim.in_flight_count("skill.b"), 1);
    }

    #[test]
    fn release_unknown_skill_does_not_panic() {
        let mut lim = ConcurrencyLimiter::new(4);
        lim.release("ghost"); // must not panic
    }

    // ── SkillOptimizer ────────────────────────────────────────────────────────

    #[test]
    fn before_execute_acquires_slot() {
        let mut opt = SkillOptimizer::new(3, Duration::from_secs(60), 4);
        assert!(opt.before_execute("math.add"));
        assert_eq!(opt.limiter.in_flight_count("math.add"), 1);
    }

    #[test]
    fn after_execute_releases_slot_and_records_history() {
        let mut opt = SkillOptimizer::new(3, Duration::from_secs(60), 4);
        opt.before_execute("math.add");
        opt.after_execute("math.add", Duration::from_millis(50), true);
        assert_eq!(opt.limiter.in_flight_count("math.add"), 0);
        assert!(opt.avg_duration("math.add").is_some());
    }

    #[test]
    fn success_rate_all_success() {
        let mut opt = SkillOptimizer::default();
        opt.before_execute("x");
        opt.after_execute("x", Duration::from_millis(10), true);
        opt.before_execute("x");
        opt.after_execute("x", Duration::from_millis(10), true);
        assert_eq!(opt.success_rate("x"), 1.0);
    }

    #[test]
    fn success_rate_mixed() {
        let mut opt = SkillOptimizer::default();
        opt.before_execute("y");
        opt.after_execute("y", Duration::from_millis(10), true);
        opt.before_execute("y");
        opt.after_execute("y", Duration::from_millis(10), false);
        assert_eq!(opt.success_rate("y"), 0.5);
    }

    #[test]
    fn avg_duration_unknown_skill_is_none() {
        let opt = SkillOptimizer::default();
        assert!(opt.avg_duration("unknown.skill").is_none());
    }

    #[test]
    fn success_rate_unknown_skill_is_one() {
        let opt = SkillOptimizer::default();
        assert_eq!(opt.success_rate("unknown.skill"), 1.0);
    }
}
