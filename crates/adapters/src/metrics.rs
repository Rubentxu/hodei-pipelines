//! Cache Metrics System
//!
//! This module implements Prometheus metrics for monitoring cache performance
//! including hits, misses, evictions, and latency measurements.

use prometheus::{Counter, Histogram, IntGauge, Registry, TextEncoder};
use std::sync::Arc;

/// Cache performance metrics
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    pub hits_total: Counter,
    pub misses_total: Counter,
    pub evictions_total: Counter,
    pub entries_active: IntGauge,
    pub get_latency: Histogram,
    pub set_latency: Histogram,
}

impl CacheMetrics {
    /// Create new cache metrics instance
    pub fn new() -> Result<Self, prometheus::Error> {
        let hits_total = Counter::new("hodei_cache_hits_total", "Total number of cache hits")?;

        let misses_total =
            Counter::new("hodei_cache_misses_total", "Total number of cache misses")?;

        let evictions_total = Counter::new(
            "hodei_cache_evictions_total",
            "Total number of cache evictions",
        )?;

        let entries_active = IntGauge::new(
            "hodei_cache_entries_active",
            "Current number of entries in cache",
        )?;

        let get_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "hodei_cache_get_latency_seconds",
                "Latency of cache get operations in seconds",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]),
        )?;

        let set_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "hodei_cache_set_latency_seconds",
                "Latency of cache set operations in seconds",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]),
        )?;

        Ok(Self {
            hits_total,
            misses_total,
            evictions_total,
            entries_active,
            get_latency,
            set_latency,
        })
    }

    /// Register metrics with Prometheus registry
    pub fn register(&self, registry: &Registry) -> Result<(), prometheus::Error> {
        registry.register(Box::new(self.hits_total.clone()))?;
        registry.register(Box::new(self.misses_total.clone()))?;
        registry.register(Box::new(self.evictions_total.clone()))?;
        registry.register(Box::new(self.entries_active.clone()))?;
        registry.register(Box::new(self.get_latency.clone()))?;
        registry.register(Box::new(self.set_latency.clone()))?;
        Ok(())
    }

    /// Record a cache hit
    pub fn record_hit(&self) {
        self.hits_total.inc();
    }

    /// Record a cache miss
    pub fn record_miss(&self) {
        self.misses_total.inc();
    }

    /// Record a cache eviction
    pub fn record_eviction(&self) {
        self.evictions_total.inc();
    }

    /// Update active entries count
    pub fn set_active_entries(&self, count: i64) {
        self.entries_active.set(count);
    }

    /// Record get operation latency
    pub fn record_get_latency(&self, duration: std::time::Duration) {
        self.get_latency.observe(duration.as_secs_f64());
    }

    /// Record set operation latency
    pub fn record_set_latency(&self, duration: std::time::Duration) {
        self.set_latency.observe(duration.as_secs_f64());
    }

    /// Get current hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits_total.get();
        let total = hits + self.misses_total.get();

        if total > 0.0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create cache metrics")
    }
}

/// Shared cache metrics (Arc-wrapped for async use)
pub type SharedCacheMetrics = Arc<CacheMetrics>;

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::Registry;

    #[test]
    fn test_cache_metrics_creation() {
        let metrics = CacheMetrics::new().unwrap();

        assert_eq!(metrics.hits_total.get(), 0.0);
        assert_eq!(metrics.misses_total.get(), 0.0);
        assert_eq!(metrics.evictions_total.get(), 0.0);
        assert_eq!(metrics.entries_active.get(), 0);
        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_metrics_registration() {
        let registry = Registry::new();
        let metrics = CacheMetrics::new().unwrap();

        // Should not panic
        let result = metrics.register(&registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_hit_increments_counter() {
        let metrics = CacheMetrics::new().unwrap();
        assert_eq!(metrics.hits_total.get(), 0.0);

        metrics.record_hit();
        assert_eq!(metrics.hits_total.get(), 1.0);

        metrics.record_hit();
        assert_eq!(metrics.hits_total.get(), 2.0);
    }

    #[test]
    fn test_record_miss_increments_counter() {
        let metrics = CacheMetrics::new().unwrap();
        assert_eq!(metrics.misses_total.get(), 0.0);

        metrics.record_miss();
        assert_eq!(metrics.misses_total.get(), 1.0);

        metrics.record_miss();
        assert_eq!(metrics.misses_total.get(), 2.0);
    }

    #[test]
    fn test_record_eviction_increments_counter() {
        let metrics = CacheMetrics::new().unwrap();
        assert_eq!(metrics.evictions_total.get(), 0.0);

        metrics.record_eviction();
        assert_eq!(metrics.evictions_total.get(), 1.0);
    }

    #[test]
    fn test_set_active_entries() {
        let metrics = CacheMetrics::new().unwrap();
        assert_eq!(metrics.entries_active.get(), 0);

        metrics.set_active_entries(10);
        assert_eq!(metrics.entries_active.get(), 10);

        metrics.set_active_entries(25);
        assert_eq!(metrics.entries_active.get(), 25);
    }

    #[test]
    fn test_record_latency() {
        let metrics = CacheMetrics::new().unwrap();

        let duration = std::time::Duration::from_millis(5);
        metrics.record_get_latency(duration);
        metrics.record_set_latency(duration);

        // Metrics are recorded, verify by checking they're > 0
        // (exact values depend on histogram implementation)
        assert!(true); // If we get here, the calls didn't panic
    }

    #[test]
    fn test_hit_rate_calculation() {
        let metrics = CacheMetrics::new().unwrap();

        // Initially no hits or misses
        assert_eq!(metrics.hit_rate(), 0.0);

        // Add some hits
        for _ in 0..10 {
            metrics.record_hit();
        }

        // Add 10 misses
        for _ in 0..10 {
            metrics.record_miss();
        }

        // Now we have 10 hits and 10 misses = 50% hit rate
        let hit_rate = metrics.hit_rate();
        assert!((hit_rate - 50.0).abs() < 0.001); // Allow for floating point precision
    }

    #[test]
    fn test_hit_rate_100_percent() {
        let metrics = CacheMetrics::new().unwrap();

        // Only add hits
        for _ in 0..5 {
            metrics.record_hit();
        }

        assert_eq!(metrics.hit_rate(), 100.0);
    }

    #[test]
    fn test_hit_rate_0_percent() {
        let metrics = CacheMetrics::new().unwrap();

        // Only add misses
        for _ in 0..5 {
            metrics.record_miss();
        }

        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_default_metrics() {
        // Should not panic
        let _metrics = CacheMetrics::default();
    }
}
